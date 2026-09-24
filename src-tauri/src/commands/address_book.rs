use std::sync::Arc;
use std::time::Instant;
use tauri::State;
use tokio::sync::Mutex;

use crate::commands::identity::{
    build_identity_details_from_payload, map_identity_lookup_error, parse_getidentity_payload,
};
use crate::core::address_book::manager;
use crate::core::auth::session::ActiveWalletAccessContext;
use crate::core::auth::stronghold_store::AddressBookPerf;
use crate::core::auth::{capture_active_wallet_access_context, SessionManager};
use crate::core::channels::vrpc::{VrpcProvider, VrpcProviderPool};
use crate::types::address_book::{ContactIdentity, ResolvedContactIdentity};
use crate::types::wallet::WalletNetwork;
use crate::types::{
    AddressBookContact, SaveAddressBookContactRequest, ValidateDestinationAddressRequest,
    ValidateDestinationAddressResult, WalletError,
};

pub(crate) fn contact_chain_id(network: WalletNetwork) -> &'static str {
    match network {
        WalletNetwork::Mainnet => "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV",
        WalletNetwork::Testnet => "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq",
    }
}

async fn context(
    session: &Arc<Mutex<SessionManager>>,
    expected: Option<&str>,
) -> Result<ActiveWalletAccessContext, WalletError> {
    let context = capture_active_wallet_access_context(session).await?;
    if expected.is_some_and(|id| id != context.session_id) {
        return Err(WalletError::WalletLocked);
    }
    Ok(context)
}

async fn update<T: Send + 'static>(
    context: ActiveWalletAccessContext,
    perf: AddressBookPerf,
    mutation: impl FnOnce(&mut crate::types::AddressBookSnapshot) -> Result<T, WalletError>
        + Send
        + 'static,
) -> Result<T, WalletError> {
    let cancellation = context.session_submission_guard().cancellation();
    let worker_cancellation = cancellation.clone();
    let network = context.wallet_network;
    let storage_started = Instant::now();
    let result = context
        .stronghold_store
        .update_address_book_with_perf(
            &context.account_id,
            context.password_hash(),
            Some(context.session_submission_guard()),
            perf,
            move |snapshot| {
                if worker_cancellation.is_cancelled() {
                    return Err(WalletError::WalletLocked);
                }
                manager::migrate_legacy_zs_snapshot(snapshot, network);
                mutation(snapshot)
            },
        )
        .await;
    perf.mark("storage_total", storage_started);
    let result = result?;
    if cancellation.is_cancelled() {
        return Err(WalletError::WalletLocked);
    }
    Ok(result)
}

/// Typed public resolution, independent of profile content and transaction validation.
pub(crate) async fn resolve_contact_on_provider(
    provider: &VrpcProvider,
    address: &str,
    network: WalletNetwork,
) -> Result<ResolvedContactIdentity, WalletError> {
    let parsed = parse_getidentity_payload(
        provider
            .getidentity(address.trim())
            .await
            .map_err(map_identity_lookup_error)?,
    )?;
    let details = build_identity_details_from_payload(
        &parsed.identity,
        parsed.status,
        "",
        None,
        parsed.fully_qualified_name.as_deref(),
        parsed.friendly_name.as_deref(),
    )?;
    crate::core::crypto::verus_id_signature::encode_compact_i_address(&details.identity_address)?;
    if address.trim().starts_with('i')
        && !address.trim().ends_with('@')
        && address.trim() != details.identity_address
    {
        return Err(WalletError::IdentityNotFound);
    }
    let name = details
        .fully_qualified_name
        .ok_or(WalletError::IdentityNotFound)?;
    Ok(ResolvedContactIdentity {
        identity: ContactIdentity {
            identity_address: details.identity_address,
            fully_qualified_name: name,
            network,
            chain_id: contact_chain_id(network).to_string(),
        },
        status: details.status,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn resolve_contact_identity(
    identity: String,
    expected_session_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<ResolvedContactIdentity, WalletError> {
    let ctx = context(session_manager.inner(), Some(&expected_session_id)).await?;
    let result = resolve_contact_on_provider(
        vrpc_provider_pool.for_network(ctx.wallet_network),
        &identity,
        ctx.wallet_network,
    )
    .await?;
    if !session_manager
        .lock()
        .await
        .is_current_session(&ctx.session_id)
    {
        return Err(WalletError::WalletLocked);
    }
    Ok(result)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_address_book_contacts(
    expected_session_id: Option<String>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<Vec<AddressBookContact>, WalletError> {
    let perf = AddressBookPerf::new("list");
    let started = Instant::now();
    let result = update(
        context(session_manager.inner(), expected_session_id.as_deref()).await?,
        perf,
        |snapshot| Ok(manager::sorted_contacts(snapshot)),
    )
    .await;
    perf.mark("command_total", started);
    result
}

#[tauri::command(rename_all = "snake_case")]
pub async fn save_address_book_contact(
    mut request: SaveAddressBookContactRequest,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<AddressBookContact, WalletError> {
    let perf = AddressBookPerf::new("save");
    let started = Instant::now();
    let ctx = context(
        session_manager.inner(),
        request.expected_session_id.as_deref(),
    )
    .await?;
    let network = ctx.wallet_network;
    // Already verified associations may be edited offline. Recheck their presence
    // under the writer gate so an intervening edit cannot silently restore one.
    // An explicit empty list only removes associations. It needs no retained
    // identity lookup, so an ordinary address-only edit takes one encrypted pass.
    let retained = if request
        .identities
        .as_ref()
        .is_some_and(|identities| !identities.is_empty())
    {
        if let Some(id) = request.id.clone() {
            let read_started = Instant::now();
            let result = update(ctx.clone(), perf, move |snapshot| {
                Ok(snapshot
                    .contacts
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| c.identities.clone())
                    .unwrap_or_default())
            })
            .await;
            perf.mark("retained_read", read_started);
            result?
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    // New associations must be resolved here; the renderer cannot assert a name/address pairing.
    // Other-network associations may only be retained verbatim from the latest encrypted record.
    if let Some(identities) = request.identities.as_mut() {
        if request.expected_session_id.is_none() || identities.len() > 20 {
            return Err(WalletError::AddressBookInvalidInput);
        }
        let resolution_started = Instant::now();
        for identity in identities
            .iter_mut()
            .filter(|id| id.network == network && !retained.contains(id))
        {
            if identity.chain_id != contact_chain_id(network) {
                return Err(WalletError::AddressBookInvalidInput);
            }
            *identity = resolve_contact_on_provider(
                vrpc_provider_pool.for_network(network),
                &identity.identity_address,
                network,
            )
            .await?
            .identity;
        }
        perf.mark("identity_resolution", resolution_started);
    }
    let result = update(ctx, perf, move |snapshot| {
        if let Some(identities) = request.identities.as_ref() {
            for identity in identities
                .iter()
                .filter(|id| id.network != network || retained.contains(id))
            {
                let existing = request
                    .id
                    .as_ref()
                    .and_then(|id| snapshot.contacts.iter().find(|c| &c.id == id));
                if !existing.is_some_and(|c| c.identities.contains(identity)) {
                    return Err(WalletError::AddressBookInvalidInput);
                }
            }
        }
        if request.add_identity_if_missing {
            let identity = request
                .identities
                .as_ref()
                .filter(|ids| ids.len() == 1)
                .and_then(|ids| ids.first())
                .ok_or(WalletError::AddressBookInvalidInput)?;
            if identity.network != network {
                return Err(WalletError::AddressBookInvalidInput);
            }
            request.display_name = identity.fully_qualified_name.clone();
            request.note = None;
            request.endpoints = vec![crate::types::address_book::SaveAddressBookEndpointInput {
                id: None,
                kind: crate::types::AddressEndpointKind::Vrpc,
                address: identity.identity_address.clone(),
                label: "VerusID".to_string(),
            }];
        }
        manager::upsert_contact(snapshot, request, network)
    })
    .await;
    perf.mark("command_total", started);
    result
}

#[tauri::command(rename_all = "snake_case")]
pub async fn delete_address_book_contact(
    contact_id: String,
    expected_session_id: Option<String>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<bool, WalletError> {
    let perf = AddressBookPerf::new("delete");
    let started = Instant::now();
    let result = update(
        context(session_manager.inner(), expected_session_id.as_deref()).await?,
        perf,
        move |snapshot| Ok(manager::delete_contact(snapshot, &contact_id)),
    )
    .await;
    perf.mark("command_total", started);
    result
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mark_address_book_endpoint_used(
    endpoint_id: String,
    expected_session_id: Option<String>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<bool, WalletError> {
    let perf = AddressBookPerf::new("last_used");
    let started = Instant::now();
    let result = update(
        context(session_manager.inner(), expected_session_id.as_deref()).await?,
        perf,
        move |snapshot| Ok(manager::mark_endpoint_used(snapshot, &endpoint_id)),
    )
    .await;
    perf.mark("command_total", started);
    result
}

#[tauri::command(rename_all = "snake_case")]
pub async fn validate_destination_address(
    request: ValidateDestinationAddressRequest,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<ValidateDestinationAddressResult, WalletError> {
    let ctx = context(session_manager.inner(), None).await?;
    match manager::normalize_destination_address(request.kind, &request.address, ctx.wallet_network)
    {
        Ok(value) => Ok(ValidateDestinationAddressResult {
            valid: true,
            normalized_address: Some(value),
            reason: None,
        }),
        Err(
            WalletError::InvalidAddress
            | WalletError::AddressBookInvalidInput
            | WalletError::AddressBookDuplicate
            | WalletError::AddressBookContactNotFound,
        ) => Ok(ValidateDestinationAddressResult {
            valid: false,
            normalized_address: None,
            reason: Some("invalid_destination".to_string()),
        }),
        Err(error) => Err(error),
    }
}
