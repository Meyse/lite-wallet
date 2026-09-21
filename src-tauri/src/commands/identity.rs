//
// Identity transaction commands (update/revoke/recover) and identity linking/discovery commands.
// Security: preflight stores backend-owned payload; send accepts preflight_id only.

use std::collections::HashSet;
use std::sync::Arc;

use crate::core::auth::session::ActiveWalletAccessContext;
use crate::core::auth::{
    capture_active_wallet_access_context, load_primary_private_scalar_for_context, SessionManager,
};
use crate::core::channels::vrpc::identity as vrpc_identity;
use crate::core::channels::vrpc::{self, VrpcProvider, VrpcProviderPool};
use crate::core::channels::PreflightStore;
use crate::core::coins::CoinRegistry;
use crate::core::crypto::wif_encoding::Network;
use crate::core::identity_display::{ensure_identity_handle_suffix, format_identity_display_name};
use crate::core::wallet::AccountStateStore;
use crate::types::wallet::WalletNetwork;
use crate::types::{
    IdentityDetailWarning, IdentityDetails, IdentityPreflightParams, IdentityPreflightResult,
    IdentityProfileLoadResult, IdentityProfilePreflightRequest, IdentityProfilePreflightResult,
    IdentitySendRequest, IdentitySendResult, LinkIdentityRequest, LinkableIdentity, LinkedIdentity,
    PendingIdentityProfileUpdate, SetLinkedIdentityFavoriteRequest, UnlinkIdentityRequest,
    WalletError,
};
use serde_json::Value;
use tauri::State;
use tokio::sync::Mutex;

const MAX_LINKED_IDENTITIES: usize = 100;
const MAX_FAVORITE_LINKED_IDENTITIES: usize = 2;

pub(crate) struct IdentitySessionContext {
    pub(crate) access: ActiveWalletAccessContext,
    pub(crate) session_id: String,
    pub(crate) account_id: String,
    pub(crate) network: WalletNetwork,
    pub(crate) primary_address: String,
    pub(crate) account_state_store: AccountStateStore,
}

#[derive(Clone)]
struct DiscoveryCandidate {
    identity_address: String,
    name: Option<String>,
    fully_qualified_name: Option<String>,
    status: Option<String>,
}

pub(crate) struct ParsedGetIdentityPayload {
    pub(crate) status: Option<String>,
    pub(crate) identity: Value,
    pub(crate) fully_qualified_name: Option<String>,
    pub(crate) friendly_name: Option<String>,
}

pub(crate) fn normalize_non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn value_as_non_empty_string(value: Option<&Value>) -> Option<String> {
    normalize_non_empty(value?.as_str()?)
}

fn first_non_empty_field(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(found) = value_as_non_empty_string(value.get(*key)) {
            return Some(found);
        }
    }
    None
}

fn resolve_identity_display_name(
    identity: &Value,
    payload_fully_qualified_name: Option<&str>,
    payload_friendly_name: Option<&str>,
) -> Option<String> {
    if let Some(friendly) = payload_friendly_name.and_then(ensure_identity_handle_suffix) {
        return Some(friendly);
    }

    let identity_fqn =
        first_non_empty_field(identity, &["fullyqualifiedname", "fullyQualifiedName"])
            .or_else(|| payload_fully_qualified_name.and_then(normalize_non_empty));
    let identity_name = first_non_empty_field(identity, &["name"]);
    format_identity_display_name(identity_fqn.as_deref(), identity_name.as_deref())
}

fn dedupe_case_insensitive(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::<String>::new();

    for value in values {
        let normalized = value.trim();
        if normalized.is_empty() {
            continue;
        }

        let key = normalized.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }

        out.push(normalized.to_string());
    }

    out
}

fn extract_primary_addresses(identity: &Value) -> Vec<String> {
    let candidates = identity
        .get("primaryaddresses")
        .or_else(|| identity.get("primaryAddresses"))
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| normalize_non_empty(entry.as_str().unwrap_or_default()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    dedupe_case_insensitive(candidates)
}

fn extract_identity_address(identity: &Value, fallback: Option<&str>) -> Option<String> {
    first_non_empty_field(
        identity,
        &["identityaddress", "identityAddress", "iaddress"],
    )
    .or_else(|| fallback.and_then(normalize_non_empty))
}

pub(crate) fn map_identity_lookup_error(err: WalletError) -> WalletError {
    match err {
        WalletError::IdentityRpcUnsupported => WalletError::IdentityRpcUnsupported,
        WalletError::NetworkError => WalletError::NetworkError,
        WalletError::IdentityNotFound
        | WalletError::InvalidAddress
        | WalletError::OperationFailed => WalletError::IdentityNotFound,
        _ => WalletError::IdentityNotFound,
    }
}

pub(crate) fn parse_getidentity_payload(
    raw: Value,
) -> Result<ParsedGetIdentityPayload, WalletError> {
    let status = value_as_non_empty_string(raw.get("status"));
    let fully_qualified_name =
        first_non_empty_field(&raw, &["fullyqualifiedname", "fullyQualifiedName"]);
    let friendly_name = first_non_empty_field(&raw, &["friendlyname", "friendlyName"]);
    let Some(identity) = raw.get("identity").cloned() else {
        return Err(WalletError::IdentityNotFound);
    };

    if !identity.is_object() {
        return Err(WalletError::IdentityNotFound);
    }

    Ok(ParsedGetIdentityPayload {
        status,
        identity,
        fully_qualified_name,
        friendly_name,
    })
}

async fn resolve_identity_reference_display_name(
    provider: &VrpcProvider,
    identity_id: Option<&str>,
    current_identity_address: &str,
    current_identity_display_name: Option<&str>,
) -> Option<String> {
    let identity_id = identity_id.and_then(normalize_non_empty)?;

    if identity_id.eq_ignore_ascii_case(current_identity_address) {
        return current_identity_display_name.and_then(normalize_non_empty);
    }

    let raw = provider.getidentity(&identity_id).await.ok()?;
    let parsed = parse_getidentity_payload(raw).ok()?;
    resolve_identity_display_name(
        &parsed.identity,
        parsed.fully_qualified_name.as_deref(),
        parsed.friendly_name.as_deref(),
    )
}

fn registered_system_display_name(
    coin_registry: &CoinRegistry,
    system_id: Option<&str>,
    is_testnet: bool,
) -> Option<String> {
    let system_id = system_id.and_then(normalize_non_empty)?;
    coin_registry
        .get_all()
        .into_iter()
        .find(|coin| {
            coin.is_testnet == is_testnet
                && coin.system_id.eq_ignore_ascii_case(&system_id)
                && coin.currency_id.eq_ignore_ascii_case(&system_id)
        })
        .and_then(|coin| normalize_non_empty(&coin.display_name))
}

fn identity_label_as_system_name(label: Option<String>) -> Option<String> {
    let label = normalize_non_empty(&label?)?;
    normalize_non_empty(label.trim_end_matches('@'))
}

fn is_owned_by_primary(primary_addresses: &[String], session_primary_address: &str) -> bool {
    primary_addresses
        .iter()
        .any(|address| address.eq_ignore_ascii_case(session_primary_address))
}

fn build_identity_warnings(
    identity_address: &str,
    primary_addresses: &[String],
    owned_by_primary_address: bool,
    revocation_authority: Option<&String>,
    recovery_authority: Option<&String>,
) -> Vec<IdentityDetailWarning> {
    let mut warnings = Vec::<IdentityDetailWarning>::new();

    if primary_addresses.len() > 1 || !owned_by_primary_address {
        warnings.push(IdentityDetailWarning {
            warning_type: "spend_and_sign".to_string(),
            message: "Funds can be spent by other addresses in the primary address list."
                .to_string(),
        });
    }

    if revocation_authority
        .map(|authority| !authority.eq_ignore_ascii_case(identity_address))
        .unwrap_or(false)
    {
        warnings.push(IdentityDetailWarning {
            warning_type: "revoke".to_string(),
            message: "Revocation authority is set to another VerusID.".to_string(),
        });
    }

    if recovery_authority
        .map(|authority| !authority.eq_ignore_ascii_case(identity_address))
        .unwrap_or(false)
    {
        warnings.push(IdentityDetailWarning {
            warning_type: "recover".to_string(),
            message: "Recovery authority is set to another VerusID.".to_string(),
        });
    }

    warnings
}

pub(crate) fn build_identity_details_from_payload(
    identity: &Value,
    status: Option<String>,
    session_primary_address: &str,
    fallback_identity_address: Option<&str>,
    payload_fully_qualified_name: Option<&str>,
    payload_friendly_name: Option<&str>,
) -> Result<IdentityDetails, WalletError> {
    let identity_address = extract_identity_address(identity, fallback_identity_address)
        .ok_or(WalletError::IdentityNotFound)?;
    let primary_addresses = extract_primary_addresses(identity);
    let revocation_authority =
        first_non_empty_field(identity, &["revocationauthority", "revocationAuthority"]);
    let recovery_authority =
        first_non_empty_field(identity, &["recoveryauthority", "recoveryAuthority"]);
    let parent = first_non_empty_field(identity, &["parent", "parentid", "parentID"]);
    let owned_by_primary_address = is_owned_by_primary(&primary_addresses, session_primary_address);
    let minimum_signatures = identity
        .get("minimumsignatures")
        .or_else(|| identity.get("minimumSignatures"))
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(1);
    let flags = identity
        .get("flags")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or_default();
    let tokenized_control = flags & 0x4 != 0;
    let is_active = status
        .as_deref()
        .map(|value| value.eq_ignore_ascii_case("active"))
        .unwrap_or(false);
    let profile_editability_reason = if !is_active {
        Some("inactive".to_string())
    } else if !owned_by_primary_address {
        Some("not_owned".to_string())
    } else if minimum_signatures != 1 || primary_addresses.len() != 1 {
        Some("unsupported_control".to_string())
    } else if tokenized_control {
        Some("tokenized_control".to_string())
    } else {
        None
    };

    let warnings = build_identity_warnings(
        &identity_address,
        &primary_addresses,
        owned_by_primary_address,
        revocation_authority.as_ref(),
        recovery_authority.as_ref(),
    );

    Ok(IdentityDetails {
        identity_address,
        name: first_non_empty_field(identity, &["name"]),
        fully_qualified_name: resolve_identity_display_name(
            identity,
            payload_fully_qualified_name,
            payload_friendly_name,
        ),
        status,
        system: first_non_empty_field(identity, &["systemid", "system"]),
        system_display_name: None,
        parent,
        revocation_authority,
        revocation_authority_name: None,
        recovery_authority,
        recovery_authority_name: None,
        primary_addresses,
        private_address: first_non_empty_field(identity, &["privateaddress", "privateAddress"]),
        owned_by_primary_address,
        minimum_signatures,
        tokenized_control,
        profile_editable: profile_editability_reason.is_none(),
        profile_editability_reason,
        warnings,
    })
}

pub(crate) fn linked_identity_from_details(details: &IdentityDetails) -> LinkedIdentity {
    LinkedIdentity {
        identity_address: details.identity_address.clone(),
        name: details.name.clone(),
        fully_qualified_name: details.fully_qualified_name.clone(),
        status: details.status.clone(),
        system_id: details.system.clone(),
        favorite: false,
    }
}

pub(crate) fn normalize_linked_identities(records: Vec<LinkedIdentity>) -> Vec<LinkedIdentity> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::<LinkedIdentity>::new();
    let mut favorite_count = 0usize;

    for record in records {
        let Some(identity_address) = normalize_non_empty(&record.identity_address) else {
            continue;
        };

        let key = identity_address.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }

        let favorite = record.favorite && favorite_count < MAX_FAVORITE_LINKED_IDENTITIES;
        if favorite {
            favorite_count += 1;
        }

        out.push(LinkedIdentity {
            identity_address,
            name: record
                .name
                .as_ref()
                .and_then(|value| normalize_non_empty(value)),
            fully_qualified_name: record
                .fully_qualified_name
                .as_ref()
                .and_then(|value| normalize_non_empty(value)),
            status: record
                .status
                .as_ref()
                .and_then(|value| normalize_non_empty(value)),
            system_id: record
                .system_id
                .as_ref()
                .and_then(|value| normalize_non_empty(value)),
            favorite,
        });

        if out.len() >= MAX_LINKED_IDENTITIES {
            break;
        }
    }

    out
}

pub(crate) fn upsert_linked_identity(
    mut records: Vec<LinkedIdentity>,
    incoming: LinkedIdentity,
) -> Vec<LinkedIdentity> {
    if let Some(existing) = records.iter_mut().find(|record| {
        record
            .identity_address
            .eq_ignore_ascii_case(&incoming.identity_address)
    }) {
        let mut merged = incoming;
        if existing.favorite {
            merged.favorite = true;
        }
        *existing = merged;
        return normalize_linked_identities(records);
    }

    records.insert(0, incoming);
    normalize_linked_identities(records)
}

fn apply_linked_identity_favorite(
    records: Vec<LinkedIdentity>,
    identity_address: &str,
    favorite: bool,
) -> Result<Vec<LinkedIdentity>, WalletError> {
    let mut records = normalize_linked_identities(records);
    let Some(index) = records.iter().position(|record| {
        record
            .identity_address
            .eq_ignore_ascii_case(identity_address)
    }) else {
        return Err(WalletError::IdentityNotFound);
    };

    if favorite {
        let other_favorite_count = records
            .iter()
            .enumerate()
            .filter(|(item_index, record)| *item_index != index && record.favorite)
            .count();
        if other_favorite_count >= MAX_FAVORITE_LINKED_IDENTITIES {
            return Err(WalletError::IdentityFavoriteLimitReached);
        }
    }

    records[index].favorite = favorite;
    Ok(normalize_linked_identities(records))
}

fn remove_linked_identity(
    records: Vec<LinkedIdentity>,
    identity_address: &str,
) -> Vec<LinkedIdentity> {
    normalize_linked_identities(
        records
            .into_iter()
            .filter(|record| {
                !record
                    .identity_address
                    .eq_ignore_ascii_case(identity_address)
            })
            .collect(),
    )
}

fn parse_candidate_from_value(value: &Value) -> Option<DiscoveryCandidate> {
    if let Some(address) = value.as_str().and_then(normalize_non_empty) {
        return Some(DiscoveryCandidate {
            identity_address: address,
            name: None,
            fully_qualified_name: None,
            status: None,
        });
    }

    let identity_object = value
        .get("identity")
        .filter(|nested| nested.is_object())
        .unwrap_or(value);
    let identity_address = extract_identity_address(identity_object, None)?;

    let status = first_non_empty_field(value, &["status"])
        .or_else(|| first_non_empty_field(identity_object, &["status"]));

    Some(DiscoveryCandidate {
        identity_address,
        name: first_non_empty_field(identity_object, &["name"]),
        fully_qualified_name: first_non_empty_field(
            identity_object,
            &["fullyqualifiedname", "fullyQualifiedName"],
        ),
        status,
    })
}

fn collect_discovery_candidates(value: &Value, out: &mut Vec<DiscoveryCandidate>) {
    match value {
        Value::Array(entries) => {
            for entry in entries {
                collect_discovery_candidates(entry, out);
            }
        }
        Value::Object(map) => {
            if let Some(candidate) = parse_candidate_from_value(value) {
                out.push(candidate);
                return;
            }

            for (key, nested) in map {
                if let Some(mut candidate) = parse_candidate_from_value(nested) {
                    if candidate.identity_address.is_empty() {
                        if let Some(fallback_address) = normalize_non_empty(key) {
                            candidate.identity_address = fallback_address;
                        }
                    }
                    out.push(candidate);
                } else if matches!(nested, Value::Array(_) | Value::Object(_)) {
                    collect_discovery_candidates(nested, out);
                }
            }
        }
        Value::String(_) => {
            if let Some(candidate) = parse_candidate_from_value(value) {
                out.push(candidate);
            }
        }
        _ => {}
    }
}

fn dedupe_discovery_candidates(candidates: Vec<DiscoveryCandidate>) -> Vec<DiscoveryCandidate> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::<DiscoveryCandidate>::new();

    for mut candidate in candidates {
        let Some(identity_address) = normalize_non_empty(&candidate.identity_address) else {
            continue;
        };

        let key = identity_address.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }

        candidate.identity_address = identity_address;
        out.push(candidate);
    }

    out
}

fn linkable_sort_key(candidate: &LinkableIdentity) -> String {
    candidate
        .fully_qualified_name
        .as_ref()
        .or(candidate.name.as_ref())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| candidate.identity_address.to_ascii_lowercase())
}

fn parse_discovery_candidates(raw: Value) -> Vec<DiscoveryCandidate> {
    let mut collected = Vec::<DiscoveryCandidate>::new();
    collect_discovery_candidates(&raw, &mut collected);
    dedupe_discovery_candidates(collected)
}

pub(crate) async fn identity_session_context(
    session_manager: &Arc<Mutex<SessionManager>>,
    account_state_store: &AccountStateStore,
) -> Result<IdentitySessionContext, WalletError> {
    let context = capture_active_wallet_access_context(session_manager).await?;

    Ok(IdentitySessionContext {
        session_id: context.session_id.clone(),
        account_id: context.account_id.clone(),
        network: context.wallet_network,
        primary_address: context.vrsc_address.clone(),
        access: context,
        account_state_store: account_state_store.clone(),
    })
}

pub(crate) async fn load_linked_for_context(
    context: &IdentitySessionContext,
) -> Result<Vec<LinkedIdentity>, WalletError> {
    context.access.load_linked_identities_cached().await
}

pub(crate) async fn store_linked_for_context(
    context: &IdentitySessionContext,
    records: &[LinkedIdentity],
) -> Result<Vec<LinkedIdentity>, WalletError> {
    let sanitized = normalize_linked_identities(records.to_vec());
    context
        .access
        .store_linked_identities_cached(&sanitized)
        .await?;
    Ok(sanitized)
}

/// Preflight identity operation on VRPC channel.
#[tauri::command(rename_all = "snake_case")]
pub async fn preflight_identity_update(
    params: IdentityPreflightParams,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<IdentityPreflightResult, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let account_id = session
        .active_account_id()
        .ok_or(WalletError::WalletLocked)?
        .to_string();
    let session_id = session
        .active_session_id()
        .ok_or(WalletError::WalletLocked)?
        .to_string();
    let (session_vrpc_address, _, _) = session.get_addresses()?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    drop(session);

    let resolved = vrpc::parse_vrpc_channel_id(&params.channel_id, Some(&session_vrpc_address))?;
    if resolved.address != session_vrpc_address {
        return Err(WalletError::InvalidAddress);
    }

    let is_testnet = matches!(network, WalletNetwork::Testnet);
    if coin_registry
        .find_by_system_id(&resolved.system_id, is_testnet)
        .is_none()
    {
        return Err(WalletError::UnsupportedChannel);
    }

    let canonical_channel_id = resolved.canonical_channel_id();

    vrpc_identity::preflight(
        params,
        &preflight_store,
        &account_id,
        &session_id,
        &resolved.address,
        &canonical_channel_id,
        vrpc_provider_pool.for_network(network),
    )
    .await
}

/// Broadcast identity operation by preflight id.
#[tauri::command(rename_all = "snake_case")]
pub async fn send_identity_update(
    request: IdentitySendRequest,
    preflight_store: State<'_, PreflightStore>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<IdentitySendResult, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    drop(session);

    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;
    let result = vrpc_identity::send(
        &request.preflight_id,
        &preflight_store,
        &session_manager,
        vrpc_provider_pool.inner().as_ref(),
    )
    .await?;

    if let Some(profile_update) = result.profile_update.as_ref() {
        if let Ok(mut pending) =
            account_state_store.load_pending_identity_profiles(&context.account_id, context.network)
        {
            pending.retain(|record| {
                !record
                    .identity_address
                    .eq_ignore_ascii_case(&profile_update.identity_address)
            });
            pending.push(profile_update.clone());
            // Broadcast already succeeded. A local persistence failure must not turn a
            // submitted transaction into a reported send failure; the UI also keeps
            // this record for the current wallet session.
            let _ = account_state_store.store_pending_identity_profiles(
                &context.account_id,
                context.network,
                &pending,
            );
        }
    }

    Ok(result)
}

/// Discover linkable identities for the active wallet primary VRSC address.
#[tauri::command(rename_all = "snake_case")]
pub async fn discover_linkable_identities(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<Vec<LinkableIdentity>, WalletError> {
    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;

    let discovery_raw = vrpc_provider_pool
        .for_network(context.network)
        .getidentitieswithaddress(&context.primary_address, false)
        .await?;

    let linked_records = load_linked_for_context(&context).await?;
    let linked_set = linked_records
        .iter()
        .map(|record| record.identity_address.to_ascii_lowercase())
        .collect::<HashSet<_>>();

    let mut candidates = parse_discovery_candidates(discovery_raw);

    for candidate in &mut candidates {
        let enriched = vrpc_provider_pool
            .for_network(context.network)
            .getidentity(&candidate.identity_address)
            .await;

        let Ok(raw_identity) = enriched else {
            continue;
        };

        let Ok(parsed) = parse_getidentity_payload(raw_identity) else {
            continue;
        };

        if let Ok(details) = build_identity_details_from_payload(
            &parsed.identity,
            parsed.status,
            &context.primary_address,
            Some(&candidate.identity_address),
            parsed.fully_qualified_name.as_deref(),
            parsed.friendly_name.as_deref(),
        ) {
            candidate.identity_address = details.identity_address;
            if details.name.is_some() {
                candidate.name = details.name;
            }
            if details.fully_qualified_name.is_some() {
                candidate.fully_qualified_name = details.fully_qualified_name;
            }
            if details.status.is_some() {
                candidate.status = details.status;
            }
        }
    }

    let deduped = dedupe_discovery_candidates(candidates);

    let mut output = deduped
        .into_iter()
        .map(|candidate| {
            let linked = linked_set.contains(&candidate.identity_address.to_ascii_lowercase());
            LinkableIdentity {
                identity_address: candidate.identity_address,
                name: candidate.name,
                fully_qualified_name: candidate.fully_qualified_name,
                status: candidate.status,
                linked,
            }
        })
        .collect::<Vec<_>>();

    output.sort_by(|left, right| {
        left.linked
            .cmp(&right.linked)
            .then(linkable_sort_key(left).cmp(&linkable_sort_key(right)))
    });

    Ok(output)
}

/// Return linked identities stored for the active wallet and active network.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_linked_identities(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
) -> Result<Vec<LinkedIdentity>, WalletError> {
    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;
    load_linked_for_context(&context).await
}

/// Link an identity to the active wallet after ownership validation.
#[tauri::command(rename_all = "snake_case")]
pub async fn link_identity(
    request: LinkIdentityRequest,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<Vec<LinkedIdentity>, WalletError> {
    let requested_identity_address =
        normalize_non_empty(&request.identity_address).ok_or(WalletError::InvalidAddress)?;

    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;

    let raw_identity = vrpc_provider_pool
        .for_network(context.network)
        .getidentity(&requested_identity_address)
        .await
        .map_err(map_identity_lookup_error)?;

    let parsed = parse_getidentity_payload(raw_identity)?;
    let details = build_identity_details_from_payload(
        &parsed.identity,
        parsed.status,
        &context.primary_address,
        Some(&requested_identity_address),
        parsed.fully_qualified_name.as_deref(),
        parsed.friendly_name.as_deref(),
    )?;

    if !details.owned_by_primary_address {
        return Err(WalletError::IdentityOwnershipMismatch);
    }

    let next_record = linked_identity_from_details(&details);
    let current = load_linked_for_context(&context).await?;
    let updated = upsert_linked_identity(current, next_record);
    store_linked_for_context(&context, &updated).await
}

/// Unlink an identity from the active wallet.
#[tauri::command(rename_all = "snake_case")]
pub async fn unlink_identity(
    request: UnlinkIdentityRequest,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
) -> Result<Vec<LinkedIdentity>, WalletError> {
    let requested_identity_address =
        normalize_non_empty(&request.identity_address).ok_or(WalletError::InvalidAddress)?;

    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;
    let current = load_linked_for_context(&context).await?;
    let updated = remove_linked_identity(current, &requested_identity_address);

    store_linked_for_context(&context, &updated).await
}

/// Set favorite state for a linked identity with max-2 favorites enforced.
#[tauri::command(rename_all = "snake_case")]
pub async fn set_linked_identity_favorite(
    request: SetLinkedIdentityFavoriteRequest,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
) -> Result<Vec<LinkedIdentity>, WalletError> {
    let requested_identity_address =
        normalize_non_empty(&request.identity_address).ok_or(WalletError::InvalidAddress)?;

    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;
    let current = load_linked_for_context(&context).await?;
    let updated =
        apply_linked_identity_favorite(current, &requested_identity_address, request.favorite)?;

    store_linked_for_context(&context, &updated).await
}

/// Return live identity details for an i-address or identity handle.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_identity_details(
    identity_address: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<IdentityDetails, WalletError> {
    let requested_identity_address =
        normalize_non_empty(&identity_address).ok_or(WalletError::InvalidAddress)?;

    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;

    let provider = vrpc_provider_pool.for_network(context.network);
    let raw_identity = provider
        .getidentity(&requested_identity_address)
        .await
        .map_err(map_identity_lookup_error)?;

    let parsed = parse_getidentity_payload(raw_identity)?;

    let mut details = build_identity_details_from_payload(
        &parsed.identity,
        parsed.status,
        &context.primary_address,
        Some(&requested_identity_address),
        parsed.fully_qualified_name.as_deref(),
        parsed.friendly_name.as_deref(),
    )?;

    let current_identity_address = details.identity_address.clone();
    let current_identity_display_name = details.fully_qualified_name.clone();
    let revocation_authority = details.revocation_authority.clone();
    let recovery_authority = details.recovery_authority.clone();
    let system_id = details.system.clone();
    let registered_system_name = registered_system_display_name(
        coin_registry.inner().as_ref(),
        system_id.as_deref(),
        matches!(context.network, WalletNetwork::Testnet),
    );
    let system_lookup_id = if registered_system_name.is_none() {
        system_id.as_deref()
    } else {
        None
    };

    let (revocation_authority_name, recovery_authority_name, system_identity_name) = tokio::join!(
        resolve_identity_reference_display_name(
            provider,
            revocation_authority.as_deref(),
            &current_identity_address,
            current_identity_display_name.as_deref(),
        ),
        resolve_identity_reference_display_name(
            provider,
            recovery_authority.as_deref(),
            &current_identity_address,
            current_identity_display_name.as_deref(),
        ),
        resolve_identity_reference_display_name(provider, system_lookup_id, "", None),
    );

    details.system_display_name =
        registered_system_name.or_else(|| identity_label_as_system_name(system_identity_name));
    details.revocation_authority_name = revocation_authority_name;
    details.recovery_authority_name = recovery_authority_name;

    if matches!(context.network, WalletNetwork::Mainnet) && details.profile_editable {
        details.profile_editable = false;
        details.profile_editability_reason = Some("testnet_only".to_string());
    }
    Ok(details)
}

/// Load an optional, independently verified public profile for a VerusID.
/// Profile failures remain isolated from the core identity details command.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_identity_profile(
    identity_address: String,
    expected_session_id: Option<String>,
    chain_id: Option<String>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<IdentityProfileLoadResult, WalletError> {
    let requested_identity_address =
        normalize_non_empty(&identity_address).ok_or(WalletError::InvalidAddress)?;
    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;
    if expected_session_id
        .as_deref()
        .is_some_and(|id| id != context.session_id)
    {
        return Err(WalletError::WalletLocked);
    }
    if chain_id
        .as_deref()
        .is_some_and(|id| id != super::address_book::contact_chain_id(context.network))
    {
        return Err(WalletError::InvalidAddress);
    }
    let signature_network = match context.network {
        WalletNetwork::Mainnet => Network::Mainnet,
        WalletNetwork::Testnet => Network::Testnet,
    };
    let profile = vrpc_identity::profile::load(
        vrpc_provider_pool.for_network(context.network),
        &requested_identity_address,
        signature_network,
    )
    .await?;
    if !session_manager
        .lock()
        .await
        .is_current_session(&context.session_id)
    {
        return Err(WalletError::WalletLocked);
    }
    Ok(profile)
}

/// Return account- and network-scoped public pending profile records.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_pending_identity_profile_updates(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
) -> Result<Vec<PendingIdentityProfileUpdate>, WalletError> {
    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;
    let mut pending =
        account_state_store.load_pending_identity_profiles(&context.account_id, context.network)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let oldest = now.saturating_sub(30 * 24 * 60 * 60);
    let initial_len = pending.len();
    pending.retain(|record| record.submitted_at >= oldest);
    if pending.len() != initial_len {
        account_state_store.store_pending_identity_profiles(
            &context.account_id,
            context.network,
            &pending,
        )?;
    }
    Ok(pending)
}

/// Clear a pending record only after the verified profile reader observes the
/// expected confirmed field digests.
#[tauri::command(rename_all = "snake_case")]
pub async fn clear_pending_identity_profile_update(
    identity_address: String,
    txid: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
) -> Result<bool, WalletError> {
    let identity_address =
        normalize_non_empty(&identity_address).ok_or(WalletError::InvalidAddress)?;
    if txid.len() != 64 || !txid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(WalletError::InvalidPreflight);
    }
    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;
    let mut pending =
        account_state_store.load_pending_identity_profiles(&context.account_id, context.network)?;
    let initial_len = pending.len();
    pending.retain(|record| {
        !(record
            .identity_address
            .eq_ignore_ascii_case(&identity_address)
            && record.txid.eq_ignore_ascii_case(&txid))
    });
    if pending.len() == initial_len {
        return Ok(false);
    }
    account_state_store.store_pending_identity_profiles(
        &context.account_id,
        context.network,
        &pending,
    )?;
    Ok(true)
}

/// Encode bounded crop pixels locally. No network, persistence or signing access.
#[tauri::command(rename_all = "snake_case")]
pub async fn encode_identity_profile_image(
    kind: String,
    png_base64: String,
) -> Result<vrpc_identity::profile::images::ProfileImageCandidates, WalletError> {
    tokio::task::spawn_blocking(move || vrpc_identity::profile::images::encode(&kind, &png_base64))
        .await
        .map_err(|_| WalletError::OperationFailed)?
}

/// Review a profile update. The backend owns all profile serialization,
/// transaction construction, fee calculation, and signing boundaries.
#[tauri::command(rename_all = "snake_case")]
pub async fn preflight_identity_profile_update(
    expected_session_id: Option<String>,
    request: IdentityProfilePreflightRequest,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    preflight_store: State<'_, PreflightStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<IdentityProfilePreflightResult, WalletError> {
    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;
    if expected_session_id
        .as_deref()
        .is_some_and(|expected| expected != context.session_id)
    {
        return Err(WalletError::WalletLocked);
    }
    if !matches!(context.network, WalletNetwork::Testnet) {
        return Err(WalletError::IdentityProfileWriteUnsupported);
    }
    let resolved =
        vrpc::parse_vrpc_channel_id(&request.channel_id, Some(&context.primary_address))?;
    if resolved.address != context.primary_address
        || resolved.system_id != "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq"
        || !coin_registry
            .find_by_system_id(&resolved.system_id, true)
            .map(|coin| coin.id == request.coin_id)
            .unwrap_or(false)
    {
        return Err(WalletError::UnsupportedChannel);
    }
    let private_key = load_primary_private_scalar_for_context(&context.access).await?;
    let mut request = request;
    request.channel_id = resolved.canonical_channel_id();
    vrpc_identity::profile::publication::prepare(
        request,
        None,
        None,
        &context.access,
        &preflight_store,
        &private_key,
        vrpc_provider_pool.for_network(context.network),
    )
    .await
}

/// Return the durable continuation after checking canonical confirmations.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_identity_profile_publication(
    identity_address: String,
    expected_session_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<Option<crate::types::ProfilePublicationState>, WalletError> {
    let access = capture_active_wallet_access_context(session_manager.inner()).await?;
    if access.session_id != expected_session_id {
        return Err(WalletError::WalletLocked);
    }
    if access.wallet_network != WalletNetwork::Testnet {
        return Ok(None);
    }
    let result = vrpc_identity::profile::publication::reconcile(
        &access,
        &identity_address,
        vrpc_provider_pool.for_network(access.wallet_network),
    )
    .await?;
    crate::core::auth::ensure_active_wallet_session(session_manager.inner(), &access.session_id)
        .await?;
    Ok(result)
}

/// Confirm a known submitted revision without depending on a retained publication plan.
#[tauri::command(rename_all = "snake_case")]
pub async fn confirm_identity_profile_update(
    identity_address: String,
    txid: String,
    expected_session_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<Option<crate::types::IdentityProfileLoadResult>, WalletError> {
    let access = capture_active_wallet_access_context(session_manager.inner()).await?;
    if access.session_id != expected_session_id {
        return Err(WalletError::WalletLocked);
    }
    if access.wallet_network != WalletNetwork::Testnet {
        return Err(WalletError::IdentityProfileWriteUnsupported);
    }
    let result = vrpc_identity::profile::publication::confirm_profile_revision(
        vrpc_provider_pool.for_network(access.wallet_network),
        &identity_address,
        &txid,
    )
    .await?;
    crate::core::auth::ensure_active_wallet_session(session_manager.inner(), &access.session_id)
        .await?;
    Ok(result)
}

/// Explicit fresh review, never automatic submission. No persisted preflight is restored.
#[tauri::command(rename_all = "snake_case")]
pub async fn review_identity_profile_publication(
    identity_address: String,
    smaller_field: Option<String>,
    plan_id: String,
    expected_session_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<IdentityProfilePreflightResult, WalletError> {
    let access = capture_active_wallet_access_context(session_manager.inner()).await?;
    if access.session_id != expected_session_id {
        return Err(WalletError::WalletLocked);
    }
    if access.wallet_network != WalletNetwork::Testnet {
        return Err(WalletError::IdentityProfileWriteUnsupported);
    }
    let provider = vrpc_provider_pool.for_network(access.wallet_network);
    let plan = vrpc_identity::profile::publication::reconcile(&access, &identity_address, provider)
        .await?
        .filter(|plan| plan.plan_id == plan_id)
        .ok_or(WalletError::InvalidPreflight)?;
    let private_key = load_primary_private_scalar_for_context(&access).await?;
    vrpc_identity::profile::publication::prepare(
        plan.request,
        Some(&plan_id),
        smaller_field.as_deref(),
        &access,
        &preflight_store,
        &private_key,
        provider,
    )
    .await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn discard_identity_profile_publication(
    plan_id: String,
    expected_session_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<(), WalletError> {
    let access = capture_active_wallet_access_context(session_manager.inner()).await?;
    if access.session_id != expected_session_id {
        return Err(WalletError::WalletLocked);
    }
    vrpc_identity::profile::publication::discard(&access, plan_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn discovery_parsing_dedupes_duplicate_identities_and_preserves_first_casing() {
        let raw = json!([
            {"identityaddress": "iAlpha", "name": "alpha"},
            {"identity": {"identityaddress": "iALPHA", "name": "alpha-duplicate"}},
            {"identity": {"identityaddress": "iBeta", "name": "beta"}}
        ]);

        let parsed = parse_discovery_candidates(raw);

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].identity_address, "iAlpha");
        assert_eq!(parsed[1].identity_address, "iBeta");
    }

    #[test]
    fn upsert_linked_identity_is_idempotent_case_insensitive() {
        let existing = vec![LinkedIdentity {
            identity_address: "iAlpha".to_string(),
            name: Some("alpha".to_string()),
            fully_qualified_name: Some("alpha@".to_string()),
            status: Some("active".to_string()),
            system_id: None,
            favorite: true,
        }];

        let updated = upsert_linked_identity(
            existing,
            LinkedIdentity {
                identity_address: "iALPHA".to_string(),
                name: Some("alpha-new".to_string()),
                fully_qualified_name: Some("alpha@".to_string()),
                status: Some("active".to_string()),
                system_id: None,
                favorite: false,
            },
        );

        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].identity_address, "iALPHA");
        assert_eq!(updated[0].name.as_deref(), Some("alpha-new"));
        assert!(updated[0].favorite);
    }

    #[test]
    fn remove_linked_identity_removes_matching_record_only() {
        let records = vec![
            LinkedIdentity {
                identity_address: "iAlpha".to_string(),
                name: None,
                fully_qualified_name: None,
                status: None,
                system_id: None,
                favorite: true,
            },
            LinkedIdentity {
                identity_address: "iBeta".to_string(),
                name: None,
                fully_qualified_name: None,
                status: None,
                system_id: None,
                favorite: false,
            },
        ];

        let updated = remove_linked_identity(records, "ialpha");
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].identity_address, "iBeta");
    }

    #[test]
    fn parse_getidentity_payload_returns_not_found_when_identity_is_missing() {
        let raw = json!({"status": "active"});

        let result = parse_getidentity_payload(raw);
        assert!(matches!(result, Err(WalletError::IdentityNotFound)));
    }

    #[test]
    fn normalize_linked_identities_caps_favorites_to_two() {
        let normalized = normalize_linked_identities(vec![
            LinkedIdentity {
                identity_address: "iAlpha".to_string(),
                name: None,
                fully_qualified_name: None,
                status: None,
                system_id: None,
                favorite: true,
            },
            LinkedIdentity {
                identity_address: "iBeta".to_string(),
                name: None,
                fully_qualified_name: None,
                status: None,
                system_id: None,
                favorite: true,
            },
            LinkedIdentity {
                identity_address: "iGamma".to_string(),
                name: None,
                fully_qualified_name: None,
                status: None,
                system_id: None,
                favorite: true,
            },
        ]);

        assert_eq!(normalized.len(), 3);
        assert!(normalized[0].favorite);
        assert!(normalized[1].favorite);
        assert!(!normalized[2].favorite);
    }

    #[test]
    fn apply_linked_identity_favorite_rejects_third_favorite() {
        let result = apply_linked_identity_favorite(
            vec![
                LinkedIdentity {
                    identity_address: "iAlpha".to_string(),
                    name: None,
                    fully_qualified_name: None,
                    status: None,
                    system_id: None,
                    favorite: true,
                },
                LinkedIdentity {
                    identity_address: "iBeta".to_string(),
                    name: None,
                    fully_qualified_name: None,
                    status: None,
                    system_id: None,
                    favorite: true,
                },
                LinkedIdentity {
                    identity_address: "iGamma".to_string(),
                    name: None,
                    fully_qualified_name: None,
                    status: None,
                    system_id: None,
                    favorite: false,
                },
            ],
            "igamma",
            true,
        );

        assert!(matches!(
            result,
            Err(WalletError::IdentityFavoriteLimitReached)
        ));
    }

    #[test]
    fn parse_getidentity_payload_extracts_top_level_names() {
        let raw = json!({
            "status": "active",
            "friendlyname": "shoes.valuid@",
            "fullyqualifiedname": "shoes.valuid.VRSC@",
            "identity": {
                "identityaddress": "iShoes",
                "name": "shoes"
            }
        });

        let parsed = parse_getidentity_payload(raw).expect("parsed payload");

        assert_eq!(parsed.status.as_deref(), Some("active"));
        assert_eq!(parsed.friendly_name.as_deref(), Some("shoes.valuid@"));
        assert_eq!(
            parsed.fully_qualified_name.as_deref(),
            Some("shoes.valuid.VRSC@")
        );
    }

    #[test]
    fn build_identity_details_formats_subid_display_name_from_fqn() {
        let identity = json!({
            "identityaddress": "iShoes",
            "name": "shoes",
            "primaryaddresses": ["RWalletPrimary"],
            "revocationauthority": "iShoes",
            "recoveryauthority": "iShoes"
        });

        let details = build_identity_details_from_payload(
            &identity,
            Some("active".to_string()),
            "RWalletPrimary",
            None,
            Some("shoes.valuid.VRSC@"),
            None,
        )
        .expect("details");

        assert_eq!(
            details.fully_qualified_name.as_deref(),
            Some("shoes.valuid@")
        );
    }

    #[test]
    fn build_identity_details_marks_unowned_primary_address() {
        let identity = json!({
            "identityaddress": "iAlpha",
            "name": "alpha",
            "primaryaddresses": ["RSomeoneElse"],
            "revocationauthority": "iAlpha",
            "recoveryauthority": "iAlpha"
        });

        let details = build_identity_details_from_payload(
            &identity,
            Some("active".to_string()),
            "RWalletPrimary",
            None,
            None,
            None,
        )
        .expect("details");

        assert!(!details.owned_by_primary_address);
        assert!(details
            .warnings
            .iter()
            .any(|warning| warning.warning_type == "spend_and_sign"));
    }

    #[test]
    fn build_identity_details_warns_for_additional_primary_address() {
        let identity = json!({
            "identityaddress": "iAlpha",
            "name": "alpha",
            "primaryaddresses": ["RWalletPrimary", "RSomeoneElse"],
            "revocationauthority": "iAlpha",
            "recoveryauthority": "iAlpha"
        });

        let details = build_identity_details_from_payload(
            &identity,
            Some("active".to_string()),
            "RWalletPrimary",
            None,
            None,
            None,
        )
        .expect("details");

        assert!(details.owned_by_primary_address);
        assert!(details
            .warnings
            .iter()
            .any(|warning| warning.warning_type == "spend_and_sign"));
    }

    #[test]
    fn resolves_registered_system_id_to_blockchain_name() {
        let registry = CoinRegistry::new();

        assert_eq!(
            registered_system_display_name(
                &registry,
                Some("iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq"),
                true,
            )
            .as_deref(),
            Some("Verus Testnet")
        );
    }

    #[test]
    fn formats_identity_label_as_blockchain_name_without_handle_suffix() {
        assert_eq!(
            identity_label_as_system_name(Some("vDEX@".to_string())).as_deref(),
            Some("vDEX")
        );
        assert_eq!(identity_label_as_system_name(Some(" @ ".to_string())), None);
    }
}
