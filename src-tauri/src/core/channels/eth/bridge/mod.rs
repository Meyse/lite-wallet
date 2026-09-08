//
// ETH bridge channel scaffolding.
// Phase-1 scope: typed boundaries and deterministic not-implemented behavior.

pub mod delegator;
pub mod fees;
pub mod paths;
pub mod preflight;
pub mod reserve_transfer;
pub mod send;
pub mod token_mapping;

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::core::auth::SessionManager;
use crate::core::channels::eth::provider::EthNetworkProvider;
use crate::core::channels::eth::EthProviderPool;
use crate::core::channels::vrpc::VrpcProvider;
use crate::core::channels::PreflightStore;
use crate::core::coins::CoinDefinition;
use crate::types::bridge::{
    BridgeConversionEstimateRequest, BridgeConversionEstimateResult, BridgeConversionPathRequest,
    BridgeConversionPathsResult, BridgeTransferPreflightParams, BridgeTransferPreflightResult,
};
use crate::types::transaction::SendResult;
use crate::types::WalletError;

pub fn parity_feature_enabled() -> bool {
    std::env::var("ETH_ERC20_BRIDGE_PARITY_ENABLED")
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}

pub async fn get_conversion_paths(
    request: &BridgeConversionPathRequest,
    vrpc_provider: &VrpcProvider,
    eth_provider: Option<&EthNetworkProvider>,
) -> Result<BridgeConversionPathsResult, WalletError> {
    paths::get_conversion_paths(request, vrpc_provider, eth_provider).await
}

pub async fn estimate_conversion(
    request: &BridgeConversionEstimateRequest,
    vrpc_provider: &VrpcProvider,
    eth_provider: Option<&EthNetworkProvider>,
) -> Result<BridgeConversionEstimateResult, WalletError> {
    paths::estimate_conversion(request, vrpc_provider, eth_provider).await
}

pub async fn preflight(
    params: BridgeTransferPreflightParams,
    preflight_store: &PreflightStore,
    account_id: &str,
    session_id: &str,
    source_coin: &CoinDefinition,
    from_address: &str,
    refund_vrpc_address: &str,
    channel_id: &str,
    vrpc_provider: &VrpcProvider,
    provider: &EthNetworkProvider,
) -> Result<BridgeTransferPreflightResult, WalletError> {
    preflight::preflight(
        params,
        preflight_store,
        account_id,
        session_id,
        source_coin,
        from_address,
        refund_vrpc_address,
        channel_id,
        vrpc_provider,
        provider,
    )
    .await
}

pub async fn send(
    preflight_id: &str,
    preflight_store: &PreflightStore,
    session_manager: &Arc<Mutex<SessionManager>>,
    provider_pool: &EthProviderPool,
) -> Result<SendResult, WalletError> {
    send::send(
        preflight_id,
        preflight_store,
        session_manager,
        provider_pool,
    )
    .await
}
