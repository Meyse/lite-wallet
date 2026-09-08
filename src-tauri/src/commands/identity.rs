//
// Identity transaction commands (update/revoke/recover) and identity linking/discovery commands.
// Security: preflight stores backend-owned payload; send accepts preflight_id only.

use std::collections::HashSet;
use std::sync::Arc;

use serde_json::Value;
use tauri::State;
use tokio::sync::Mutex;
use zeroize::Zeroizing;

use crate::core::auth::{capture_active_wallet_access_context, SessionManager};
use crate::core::channels::vrpc::identity as vrpc_identity;
use crate::core::channels::vrpc::{self, VrpcProviderPool};
use crate::core::channels::PreflightStore;
use crate::core::coins::CoinRegistry;
use crate::core::identity_display::{ensure_identity_handle_suffix, format_identity_display_name};
use crate::core::wallet::AccountStateStore;
use crate::core::StrongholdStore;
use crate::types::wallet::WalletNetwork;
use crate::types::{
    IdentityDetailWarning, IdentityDetails, IdentityPreflightParams, IdentityPreflightResult,
    IdentitySendRequest, IdentitySendResult, LinkIdentityRequest, LinkableIdentity, LinkedIdentity,
    SetLinkedIdentityFavoriteRequest, UnlinkIdentityRequest, WalletError,
};

const MAX_LINKED_IDENTITIES: usize = 100;
const MAX_FAVORITE_LINKED_IDENTITIES: usize = 2;

pub(crate) struct IdentitySessionContext {
    pub(crate) session_id: String,
    pub(crate) account_id: String,
    pub(crate) network: WalletNetwork,
    pub(crate) primary_address: String,
    pub(crate) password_hash: Zeroizing<Vec<u8>>,
    pub(crate) stronghold_store: StrongholdStore,
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
        parent,
        revocation_authority,
        recovery_authority,
        primary_addresses,
        private_address: first_non_empty_field(identity, &["privateaddress", "privateAddress"]),
        owned_by_primary_address,
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
    let password_hash = Zeroizing::new(context.password_hash().to_vec());

    Ok(IdentitySessionContext {
        session_id: context.session_id,
        account_id: context.account_id,
        network: context.wallet_network,
        primary_address: context.vrsc_address,
        password_hash,
        stronghold_store: context.stronghold_store,
        account_state_store: account_state_store.clone(),
    })
}

pub(crate) async fn load_linked_for_context(
    context: &IdentitySessionContext,
) -> Result<Vec<LinkedIdentity>, WalletError> {
    context
        .stronghold_store
        .load_linked_identities(
            &context.account_id,
            context.password_hash.as_ref(),
            context.network,
        )
        .await
}

pub(crate) async fn store_linked_for_context(
    context: &IdentitySessionContext,
    records: &[LinkedIdentity],
) -> Result<Vec<LinkedIdentity>, WalletError> {
    let sanitized = normalize_linked_identities(records.to_vec());
    context
        .stronghold_store
        .store_linked_identities(
            &context.account_id,
            context.password_hash.as_ref(),
            context.network,
            &sanitized,
        )
        .await?;
    load_linked_for_context(context).await
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
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<IdentitySendResult, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    drop(session);

    vrpc_identity::send(
        &request.preflight_id,
        &preflight_store,
        &session_manager,
        vrpc_provider_pool.inner().as_ref(),
    )
    .await
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
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<IdentityDetails, WalletError> {
    let requested_identity_address =
        normalize_non_empty(&identity_address).ok_or(WalletError::InvalidAddress)?;

    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;

    let raw_identity = vrpc_provider_pool
        .for_network(context.network)
        .getidentity(&requested_identity_address)
        .await
        .map_err(map_identity_lookup_error)?;

    let parsed = parse_getidentity_payload(raw_identity)?;

    build_identity_details_from_payload(
        &parsed.identity,
        parsed.status,
        &context.primary_address,
        Some(&requested_identity_address),
        parsed.fully_qualified_name.as_deref(),
        parsed.friendly_name.as_deref(),
    )
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
}
