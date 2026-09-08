mod balance;
pub mod bridge;
mod config;
mod preflight;
mod provider;
mod send;
mod transactions;

use crate::core::coins::CoinDefinition;
use crate::types::transaction::{BalanceResult, PreflightParams, PreflightResult, Transaction};
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

pub use provider::{EthNetworkProvider, EthProviderPool};

pub fn parse_coin_channel_id(
    channel_id: &str,
    expected_prefix: &str,
) -> Result<String, WalletError> {
    let mut parts = channel_id.split('.');
    let prefix = parts.next().unwrap_or_default();
    let coin_id = parts.next().unwrap_or_default();
    let extra = parts.next();

    if prefix != expected_prefix || coin_id.is_empty() || extra.is_some() {
        return Err(WalletError::UnsupportedChannel);
    }

    Ok(coin_id.to_string())
}

pub async fn preflight_eth(
    params: PreflightParams,
    preflight_store: &crate::core::channels::PreflightStore,
    account_id: &str,
    session_id: &str,
    from_address: &str,
    channel_id: &str,
    provider: &EthNetworkProvider,
) -> Result<PreflightResult, WalletError> {
    preflight::preflight_eth(
        params,
        preflight_store,
        account_id,
        session_id,
        from_address,
        channel_id,
        provider,
    )
    .await
}

pub async fn preflight_erc20(
    params: PreflightParams,
    preflight_store: &crate::core::channels::PreflightStore,
    account_id: &str,
    session_id: &str,
    from_address: &str,
    channel_id: &str,
    coin: &CoinDefinition,
    provider: &EthNetworkProvider,
) -> Result<PreflightResult, WalletError> {
    preflight::preflight_erc20(
        params,
        preflight_store,
        account_id,
        session_id,
        from_address,
        channel_id,
        coin,
        provider,
    )
    .await
}

pub async fn send(
    preflight_id: &str,
    preflight_store: &crate::core::channels::PreflightStore,
    session_manager: &std::sync::Arc<tokio::sync::Mutex<crate::core::auth::SessionManager>>,
    provider_pool: &EthProviderPool,
) -> Result<crate::types::transaction::SendResult, WalletError> {
    send::send(
        preflight_id,
        preflight_store,
        session_manager,
        provider_pool,
    )
    .await
}

pub async fn get_eth_balance(
    provider: &EthNetworkProvider,
    address: &str,
) -> Result<BalanceResult, WalletError> {
    balance::get_eth_balance(provider, address).await
}

pub async fn get_erc20_balance(
    provider: &EthNetworkProvider,
    address: &str,
    coin: &CoinDefinition,
) -> Result<BalanceResult, WalletError> {
    balance::get_erc20_balance(provider, address, coin).await
}

pub async fn get_eth_transactions(
    provider: &EthNetworkProvider,
    network: WalletNetwork,
    address: &str,
) -> Result<Vec<Transaction>, WalletError> {
    transactions::get_eth_transactions(provider, network, address).await
}

pub async fn get_eth_transactions_page(
    provider: &EthNetworkProvider,
    network: WalletNetwork,
    address: &str,
    page: u32,
    limit: usize,
) -> Result<transactions::EthTransactionsPage, WalletError> {
    transactions::get_eth_transactions_page(provider, network, address, page, limit).await
}

pub async fn get_erc20_transactions(
    provider: &EthNetworkProvider,
    network: WalletNetwork,
    address: &str,
    coin: &CoinDefinition,
) -> Result<Vec<Transaction>, WalletError> {
    transactions::get_erc20_transactions(provider, network, address, coin).await
}

pub async fn get_erc20_transactions_page(
    provider: &EthNetworkProvider,
    network: WalletNetwork,
    address: &str,
    coin: &CoinDefinition,
    page: u32,
    limit: usize,
) -> Result<transactions::EthTransactionsPage, WalletError> {
    transactions::get_erc20_transactions_page(provider, network, address, coin, page, limit).await
}

#[cfg(test)]
mod tests {
    use super::parse_coin_channel_id;
    use crate::types::WalletError;

    #[test]
    fn parse_coin_channel_id_accepts_eth_pattern() {
        let parsed = parse_coin_channel_id("eth.ETH", "eth").expect("parse channel id");
        assert_eq!(parsed, "ETH");
    }

    #[test]
    fn parse_coin_channel_id_rejects_wrong_prefix_or_shape() {
        assert!(matches!(
            parse_coin_channel_id("erc20.USDC", "eth"),
            Err(WalletError::UnsupportedChannel)
        ));
        assert!(matches!(
            parse_coin_channel_id("eth.ETH.extra", "eth"),
            Err(WalletError::UnsupportedChannel)
        ));
        assert!(matches!(
            parse_coin_channel_id("eth", "eth"),
            Err(WalletError::UnsupportedChannel)
        ));
    }
}
