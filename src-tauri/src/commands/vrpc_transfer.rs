//
// Advanced VRPC transfer command handlers (reserve-transfer/sendcurrency family).

use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;

use crate::core::auth::SessionManager;
use crate::core::channels::vrpc::{self, VrpcProviderPool};
use crate::core::channels::PreflightStore;
use crate::core::coins::CoinRegistry;
use crate::types::wallet::WalletNetwork;
use crate::types::{VrpcTransferPreflightParams, VrpcTransferPreflightResult, WalletError};

/// Preflight advanced VRPC transfer (convert/export/sendcurrency) and return preflight_id.
#[tauri::command(rename_all = "snake_case")]
pub async fn preflight_vrpc_transfer(
    params: VrpcTransferPreflightParams,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<VrpcTransferPreflightResult, WalletError> {
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
    let effective_source = params
        .source_address
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or(resolved.address.as_str())
        .to_string();
    if effective_source != session_vrpc_address || resolved.address != session_vrpc_address {
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
    if !vrpc_provider_pool.has_system_provider(network, &resolved.system_id) {
        println!(
            "[VRPC] Missing system-specific endpoint for {}. Falling back to network default.",
            resolved.system_id
        );
    }

    vrpc::preflight_transfer(
        params,
        &preflight_store,
        &account_id,
        &session_id,
        &effective_source,
        &canonical_channel_id,
        &resolved.system_id,
        vrpc_provider_pool.for_system(network, &resolved.system_id),
    )
    .await
}
