use ethers::abi::Abi;
use ethers::contract::Contract;
use ethers::providers::Middleware;
use ethers::types::Address;
use ethers::utils::format_units;
use std::sync::Arc;

use crate::core::channels::eth::provider::EthNetworkProvider;
use crate::core::coins::CoinDefinition;
use crate::types::transaction::BalanceResult;
use crate::types::WalletError;

const ERC20_BALANCE_ABI: &str = r#"[
  {
    "constant": true,
    "inputs": [{"name": "_owner", "type": "address"}],
    "name": "balanceOf",
    "outputs": [{"name": "balance", "type": "uint256"}],
    "type": "function"
  }
]"#;

pub async fn get_eth_balance(
    provider: &EthNetworkProvider,
    address: &str,
) -> Result<BalanceResult, WalletError> {
    provider.validate_rpc_identity().await?;
    let parsed_address: Address = address.parse().map_err(|_| WalletError::InvalidAddress)?;

    let balance_wei = provider
        .rpc_provider
        .get_balance(parsed_address, None)
        .await
        .map_err(|_| WalletError::NetworkError)?;

    let balance = format_units(balance_wei, 18).map_err(|_| WalletError::OperationFailed)?;

    Ok(BalanceResult {
        confirmed: balance.clone(),
        pending: "0".to_string(),
        total: balance,
    })
}

pub async fn get_erc20_balance(
    provider: &EthNetworkProvider,
    from_address: &str,
    coin: &CoinDefinition,
) -> Result<BalanceResult, WalletError> {
    provider.validate_rpc_identity().await?;
    let parsed_from: Address = from_address
        .parse()
        .map_err(|_| WalletError::InvalidAddress)?;
    let token_address: Address = coin
        .currency_id
        .parse()
        .map_err(|_| WalletError::InvalidAddress)?;

    let abi: Abi =
        serde_json::from_str(ERC20_BALANCE_ABI).map_err(|_| WalletError::OperationFailed)?;
    let rpc = Arc::new(provider.rpc_provider.clone());
    let contract = Contract::new(token_address, abi, rpc);

    let balance_raw = contract
        .method::<_, ethers::types::U256>("balanceOf", parsed_from)
        .map_err(|_| WalletError::OperationFailed)?
        .call()
        .await
        .map_err(|_| WalletError::NetworkError)?;

    let balance = format_units(balance_raw, coin.decimals as usize)
        .map_err(|_| WalletError::OperationFailed)?;

    Ok(BalanceResult {
        confirmed: balance.clone(),
        pending: "0".to_string(),
        total: balance,
    })
}
