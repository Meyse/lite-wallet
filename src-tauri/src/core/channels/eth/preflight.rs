use std::sync::Arc;

use ethers::abi::Abi;
use ethers::contract::Contract;
use ethers::providers::Middleware;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::{Address, Eip1559TransactionRequest, U256};
use ethers::utils::{format_units, parse_units};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::channels::eth::provider::EthNetworkProvider;
use crate::core::channels::store::{PreflightRecord, PreflightStore};
use crate::core::coins::CoinDefinition;
use crate::types::transaction::{PreflightParams, PreflightResult};
use crate::types::WalletError;

const ERC20_SEND_ABI: &str = r#"[
  {
    "constant": true,
    "inputs": [{"name": "_owner", "type": "address"}],
    "name": "balanceOf",
    "outputs": [{"name": "balance", "type": "uint256"}],
    "type": "function"
  },
  {
    "constant": false,
    "inputs": [
      {"name": "_to", "type": "address"},
      {"name": "_value", "type": "uint256"}
    ],
    "name": "transfer",
    "outputs": [{"name": "", "type": "bool"}],
    "type": "function"
  }
]"#;

const MIN_GAS_PRICE_GWEI_WEI: u64 = 1_000_000_000;
const ETH_GAS_LIMIT_FALLBACK: u64 = 21_000;
const ERC20_GAS_LIMIT_FALLBACK: u64 = 120_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EthPreflightPayload {
    Eth {
        chain_id: u64,
        coin_id: String,
        from_address: String,
        to_address: String,
        value_wei: String,
        gas_limit: String,
        max_fee_per_gas: String,
        max_priority_fee_per_gas: String,
        fee: String,
        value: String,
    },
    Erc20 {
        chain_id: u64,
        coin_id: String,
        token_address: String,
        token_decimals: u8,
        from_address: String,
        to_address: String,
        token_value_raw: String,
        gas_limit: String,
        max_fee_per_gas: String,
        max_priority_fee_per_gas: String,
        max_fee_cap: String,
        fee: String,
        value: String,
    },
    Bridge {
        chain_id: u64,
        coin_id: String,
        channel_id: String,
        from_address: String,
        refund_vrpc_address: String,
        to_address: String,
        source_contract: String,
        source_decimals: u8,
        source_amount_sats: String,
        source_amount_token_raw: Option<String>,
        mapped_currency_iaddress: String,
        mapped_currency_eth_address: String,
        reserve_transfer_version: u32,
        reserve_transfer_currency: String,
        reserve_transfer_amount: String,
        reserve_transfer_flags: u32,
        reserve_transfer_fee_currency_id: String,
        reserve_transfer_fees: u64,
        reserve_transfer_destination_type: u8,
        reserve_transfer_destination_address: String,
        reserve_transfer_dest_currency_id: String,
        reserve_transfer_dest_system_id: String,
        reserve_transfer_second_reserve_id: String,
        bridge_contract: String,
        transfer_value_wei: String,
        gas_limit: String,
        transfer_gas_limit: String,
        approval_gas_limit: String,
        max_fee_per_gas: String,
        max_priority_fee_per_gas: String,
        max_fee_cap: String,
        approval_zero_out: bool,
        fee: String,
        value: String,
    },
}

pub async fn preflight_eth(
    params: PreflightParams,
    preflight_store: &PreflightStore,
    account_id: &str,
    session_id: &str,
    from_address: &str,
    channel_id: &str,
    provider: &EthNetworkProvider,
) -> Result<PreflightResult, WalletError> {
    let parsed_from = parse_eth_address(from_address)?;
    let parsed_to = parse_eth_address(&params.to_address)?;

    let submitted_value = parse_token_amount(&params.amount, 18)?;

    let fee_params = current_fee_params(provider).await?;

    let tx = Eip1559TransactionRequest::new()
        .from(parsed_from)
        .to(parsed_to)
        .value(submitted_value);
    let typed_tx: TypedTransaction = tx.into();

    let gas_estimate = provider
        .rpc_provider
        .estimate_gas(&typed_tx, None)
        .await
        .unwrap_or_else(|_| U256::from(ETH_GAS_LIMIT_FALLBACK));
    let gas_limit = add_fraction(gas_estimate, 5).max(U256::from(ETH_GAS_LIMIT_FALLBACK));

    let max_fee = gas_limit.saturating_mul(fee_params.max_fee_per_gas);

    let balance = provider
        .rpc_provider
        .get_balance(parsed_from, None)
        .await
        .map_err(|_| WalletError::NetworkError)?;

    let (value, fee_taken_from_amount, fee_taken_message) =
        resolve_eth_value_after_fee(submitted_value, balance, max_fee)?;

    let fee_display = format_units(max_fee, 18).map_err(|_| WalletError::OperationFailed)?;
    let value_display = format_units(value, 18).map_err(|_| WalletError::OperationFailed)?;

    let payload = EthPreflightPayload::Eth {
        chain_id: provider.chain_id,
        coin_id: params.coin_id.clone(),
        from_address: from_address.to_string(),
        to_address: format_address(parsed_to),
        value_wei: value.to_string(),
        gas_limit: gas_limit.to_string(),
        max_fee_per_gas: fee_params.max_fee_per_gas.to_string(),
        max_priority_fee_per_gas: fee_params.max_priority_fee_per_gas.to_string(),
        fee: fee_display.clone(),
        value: value_display.clone(),
    };

    let preflight_id = Uuid::new_v4().to_string();
    if !preflight_store.put(
        preflight_id.clone(),
        PreflightRecord {
            session_id: session_id.to_string(),
            channel_id: channel_id.to_string(),
            account_id: account_id.to_string(),
            payload: serde_json::to_value(payload).map_err(|_| WalletError::OperationFailed)?,
        },
    ) {
        return Err(WalletError::WalletLocked);
    }

    Ok(PreflightResult {
        preflight_id,
        fee: fee_display,
        fee_currency: params.coin_id.clone(),
        value: value_display,
        amount_submitted: params.amount,
        to_address: format_address(parsed_to),
        from_address: from_address.to_string(),
        fee_taken_from_amount,
        fee_taken_message,
        warnings: vec![],
        memo: params.memo,
    })
}

pub async fn preflight_erc20(
    params: PreflightParams,
    preflight_store: &PreflightStore,
    account_id: &str,
    session_id: &str,
    from_address: &str,
    channel_id: &str,
    coin: &CoinDefinition,
    provider: &EthNetworkProvider,
) -> Result<PreflightResult, WalletError> {
    let parsed_from = parse_eth_address(from_address)?;
    let parsed_to = parse_eth_address(&params.to_address)?;

    let token_address: Address = coin
        .currency_id
        .parse()
        .map_err(|_| WalletError::InvalidAddress)?;

    let amount_raw = parse_token_amount(&params.amount, coin.decimals as usize)?;

    let fee_params = current_fee_params(provider).await?;
    let abi: Abi =
        serde_json::from_str(ERC20_SEND_ABI).map_err(|_| WalletError::OperationFailed)?;
    let rpc = Arc::new(provider.rpc_provider.clone());
    let contract = Contract::new(token_address, abi, rpc.clone());

    let token_balance = contract
        .method::<_, U256>("balanceOf", parsed_from)
        .map_err(|_| WalletError::OperationFailed)?
        .call()
        .await
        .map_err(|_| WalletError::NetworkError)?;

    if token_balance < amount_raw {
        return Err(WalletError::InsufficientFunds);
    }

    let gas_estimate = contract
        .method::<_, bool>("transfer", (parsed_to, amount_raw))
        .map_err(|_| WalletError::OperationFailed)?
        .from(parsed_from)
        .estimate_gas()
        .await
        .unwrap_or_else(|_| U256::from(ERC20_GAS_LIMIT_FALLBACK));

    let gas_limit = add_fraction(gas_estimate, 3).max(U256::from(ERC20_GAS_LIMIT_FALLBACK));
    let max_fee_cap = gas_limit.saturating_mul(fee_params.max_fee_per_gas);

    let eth_balance = provider
        .rpc_provider
        .get_balance(parsed_from, None)
        .await
        .map_err(|_| WalletError::NetworkError)?;

    if eth_balance < max_fee_cap {
        return Err(WalletError::InsufficientEthForGas);
    }

    let fee_display = format_units(max_fee_cap, 18).map_err(|_| WalletError::OperationFailed)?;
    let value_display = format_units(amount_raw, coin.decimals as usize)
        .map_err(|_| WalletError::OperationFailed)?;

    let payload = EthPreflightPayload::Erc20 {
        chain_id: provider.chain_id,
        coin_id: coin.id.clone(),
        token_address: coin.currency_id.clone(),
        token_decimals: coin.decimals,
        from_address: from_address.to_string(),
        to_address: format_address(parsed_to),
        token_value_raw: amount_raw.to_string(),
        gas_limit: gas_limit.to_string(),
        max_fee_per_gas: fee_params.max_fee_per_gas.to_string(),
        max_priority_fee_per_gas: fee_params.max_priority_fee_per_gas.to_string(),
        max_fee_cap: max_fee_cap.to_string(),
        fee: fee_display.clone(),
        value: value_display.clone(),
    };

    let preflight_id = Uuid::new_v4().to_string();
    if !preflight_store.put(
        preflight_id.clone(),
        PreflightRecord {
            session_id: session_id.to_string(),
            channel_id: channel_id.to_string(),
            account_id: account_id.to_string(),
            payload: serde_json::to_value(payload).map_err(|_| WalletError::OperationFailed)?,
        },
    ) {
        return Err(WalletError::WalletLocked);
    }

    Ok(PreflightResult {
        preflight_id,
        fee: fee_display,
        fee_currency: "ETH".to_string(),
        value: value_display,
        amount_submitted: params.amount,
        to_address: format_address(parsed_to),
        from_address: from_address.to_string(),
        fee_taken_from_amount: false,
        fee_taken_message: None,
        warnings: vec![],
        memo: params.memo,
    })
}

#[derive(Debug, Clone)]
struct FeeParams {
    max_fee_per_gas: U256,
    max_priority_fee_per_gas: U256,
}

async fn current_fee_params(provider: &EthNetworkProvider) -> Result<FeeParams, WalletError> {
    let fee_data = provider
        .rpc_provider
        .estimate_eip1559_fees(None)
        .await
        .map_err(|_| WalletError::NetworkError)?;

    let market = fee_data.0;

    let mut max_fee = add_fraction(market, 3);
    let min_gas = U256::from(MIN_GAS_PRICE_GWEI_WEI);
    if max_fee < min_gas {
        max_fee = min_gas;
    }

    let mut max_priority = fee_data.1;
    if max_priority > max_fee {
        max_priority = max_fee;
    }

    Ok(FeeParams {
        max_fee_per_gas: max_fee,
        max_priority_fee_per_gas: max_priority,
    })
}

fn parse_token_amount(amount: &str, decimals: usize) -> Result<U256, WalletError> {
    let parsed = parse_units(amount.trim(), decimals).map_err(|_| WalletError::OperationFailed)?;
    let as_u256: U256 = parsed.into();
    if as_u256.is_zero() {
        return Err(WalletError::OperationFailed);
    }
    Ok(as_u256)
}

fn parse_eth_address(address: &str) -> Result<Address, WalletError> {
    address
        .trim()
        .parse()
        .map_err(|_| WalletError::InvalidAddress)
}

fn resolve_eth_value_after_fee(
    submitted_value: U256,
    balance: U256,
    max_fee: U256,
) -> Result<(U256, bool, Option<String>), WalletError> {
    if balance <= max_fee {
        return Err(WalletError::InsufficientFunds);
    }

    let total_cost = submitted_value.saturating_add(max_fee);
    if total_cost <= balance {
        return Ok((submitted_value, false, None));
    }

    let adjusted = balance.saturating_sub(max_fee);
    if adjusted.is_zero() {
        return Err(WalletError::InsufficientFunds);
    }

    Ok((
        adjusted,
        true,
        Some("Fee was deducted from the submitted amount due to available balance.".to_string()),
    ))
}

fn add_fraction(value: U256, divisor: u64) -> U256 {
    if divisor == 0 {
        return value;
    }

    value.saturating_add(value / U256::from(divisor))
}

fn format_address(address: Address) -> String {
    format!("{:#x}", address)
}

#[cfg(test)]
mod tests {
    use super::{parse_eth_address, parse_token_amount, resolve_eth_value_after_fee};
    use crate::types::WalletError;
    use ethers::types::U256;

    #[test]
    fn resolve_eth_value_after_fee_keeps_submitted_when_balance_covers() {
        let submitted = U256::from(1_000_000u64);
        let fee = U256::from(21_000u64);
        let balance = submitted + fee + U256::from(1u64);

        let (value, adjusted, message) =
            resolve_eth_value_after_fee(submitted, balance, fee).expect("resolve value");

        assert_eq!(value, submitted);
        assert!(!adjusted);
        assert!(message.is_none());
    }

    #[test]
    fn resolve_eth_value_after_fee_adjusts_when_total_exceeds_balance() {
        let submitted = U256::from(1_000_000u64);
        let fee = U256::from(21_000u64);
        let balance = submitted;

        let (value, adjusted, message) =
            resolve_eth_value_after_fee(submitted, balance, fee).expect("resolve value");

        assert_eq!(value, U256::from(979_000u64));
        assert!(adjusted);
        assert!(message.is_some());
    }

    #[test]
    fn resolve_eth_value_after_fee_fails_when_balance_cannot_cover_fee() {
        let submitted = U256::from(1_000u64);
        let fee = U256::from(2_000u64);
        let balance = U256::from(2_000u64);

        let result = resolve_eth_value_after_fee(submitted, balance, fee);
        assert!(matches!(result, Err(WalletError::InsufficientFunds)));
    }

    #[test]
    fn parse_eth_address_rejects_invalid_destination() {
        let result = parse_eth_address("not-an-eth-address");
        assert!(matches!(result, Err(WalletError::InvalidAddress)));
    }

    #[test]
    fn parse_token_amount_rejects_zero() {
        let result = parse_token_amount("0", 18);
        assert!(matches!(result, Err(WalletError::OperationFailed)));
    }
}
