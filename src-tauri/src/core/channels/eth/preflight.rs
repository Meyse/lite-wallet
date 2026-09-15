use std::sync::Arc;

use ethers::abi::Abi;
use ethers::contract::Contract;
use ethers::providers::Middleware;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::{Address, Eip1559TransactionRequest, U256};
use ethers::utils::format_units;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::channels::direct_send_fee::{resolve_direct_evm_fee, ResolvedEvmFee};
use crate::core::channels::eth::provider::EthNetworkProvider;
use crate::core::channels::store::{PreflightRecord, PreflightStore};
use crate::core::coins::CoinDefinition;
use crate::types::transaction::{DirectSendFeeMode, PreflightParams, PreflightResult};
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

const ETH_TRANSFER_GAS_BASELINE: u64 = 21_000;
const EVM_ZERO_CALLDATA_GAS: u64 = 4;
const EVM_NONZERO_CALLDATA_GAS: u64 = 16;
const ETH_GAS_MARGIN_DIVISOR: u64 = 5;
const ERC20_GAS_MARGIN_DIVISOR: u64 = 3;

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
        #[serde(default)]
        fee_mode: DirectSendFeeMode,
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
        #[serde(default)]
        fee_mode: DirectSendFeeMode,
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

    let fee_mode = params.fee_mode.unwrap_or_default();
    let fee_params = current_fee_params(provider, fee_mode).await?;

    let balance = provider
        .rpc_provider
        .get_balance(parsed_from, None)
        .await
        .map_err(|_| WalletError::NetworkError)?;

    let (value, gas_limit, max_fee, fee_taken_from_amount, fee_taken_message) =
        resolve_eth_preflight_quote(
            provider,
            parsed_from,
            parsed_to,
            submitted_value,
            balance,
            &fee_params,
        )
        .await?;

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
        fee_mode,
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
        fee_mode: Some(fee_mode),
        fee_rate_sats_per_vbyte: None,
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

    let fee_mode = params.fee_mode.unwrap_or_default();
    let fee_params = current_fee_params(provider, fee_mode).await?;
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

    let transfer_call = contract
        .method::<_, bool>("transfer", (parsed_to, amount_raw))
        .map_err(|_| WalletError::OperationFailed)?
        .from(parsed_from);
    let calldata = transfer_call
        .calldata()
        .ok_or(WalletError::OperationFailed)?;
    let gas_estimate = transfer_call
        .estimate_gas()
        .await
        .map_err(|_| WalletError::GasEstimationFailed)?;
    let gas_estimate = validate_gas_estimate(gas_estimate, minimum_intrinsic_gas(&calldata))?;

    let gas_limit = add_fraction(gas_estimate, ERC20_GAS_MARGIN_DIVISOR);
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
        fee_mode,
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
        fee_currency: if coin.is_testnet { "GETH" } else { "ETH" }.to_string(),
        value: value_display,
        amount_submitted: params.amount,
        to_address: format_address(parsed_to),
        from_address: from_address.to_string(),
        fee_taken_from_amount: false,
        fee_taken_message: None,
        warnings: vec![],
        memo: params.memo,
        fee_mode: Some(fee_mode),
        fee_rate_sats_per_vbyte: None,
    })
}

async fn current_fee_params(
    provider: &EthNetworkProvider,
    mode: DirectSendFeeMode,
) -> Result<ResolvedEvmFee, WalletError> {
    let fee_data = provider
        .rpc_provider
        .estimate_eip1559_fees(None)
        .await
        .map_err(|_| WalletError::NetworkError)?;

    Ok(resolve_direct_evm_fee(mode, fee_data.0, fee_data.1))
}

fn parse_token_amount(amount: &str, decimals: usize) -> Result<U256, WalletError> {
    let trimmed = amount.trim();
    if trimmed.is_empty() || trimmed.starts_with('+') || trimmed.starts_with('-') {
        return Err(WalletError::InvalidAmount);
    }

    let (whole, fraction) = match trimmed.split_once('.') {
        Some((whole, fraction)) => {
            if fraction.is_empty() || fraction.contains('.') {
                return Err(WalletError::InvalidAmount);
            }
            (whole, fraction)
        }
        None => (trimmed, ""),
    };
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > decimals
    {
        return Err(WalletError::InvalidAmount);
    }

    let whole = whole.trim_start_matches('0');
    let whole = if whole.is_empty() { "0" } else { whole };
    let mut exact = String::with_capacity(whole.len().saturating_add(decimals));
    exact.push_str(whole);
    exact.push_str(fraction);
    exact.extend(std::iter::repeat_n('0', decimals - fraction.len()));

    let parsed = U256::from_dec_str(&exact).map_err(|_| WalletError::InvalidAmount)?;
    if parsed.is_zero() {
        return Err(WalletError::InvalidAmount);
    }
    Ok(parsed)
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
    if submitted_value > balance {
        return Err(WalletError::InsufficientFunds);
    }
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

async fn estimate_eth_transfer_gas(
    provider: &EthNetworkProvider,
    from: Address,
    to: Address,
    value: U256,
) -> Result<U256, WalletError> {
    let tx = Eip1559TransactionRequest::new()
        .from(from)
        .to(to)
        .value(value);
    let typed_tx: TypedTransaction = tx.into();
    let estimate = provider
        .rpc_provider
        .estimate_gas(&typed_tx, None)
        .await
        .map_err(|_| WalletError::GasEstimationFailed)?;
    validate_gas_estimate(estimate, U256::from(ETH_TRANSFER_GAS_BASELINE))
}

fn minimum_intrinsic_gas(calldata: &[u8]) -> U256 {
    calldata
        .iter()
        .fold(U256::from(ETH_TRANSFER_GAS_BASELINE), |total, byte| {
            total.saturating_add(U256::from(if *byte == 0 {
                EVM_ZERO_CALLDATA_GAS
            } else {
                EVM_NONZERO_CALLDATA_GAS
            }))
        })
}

fn validate_gas_estimate(estimate: U256, minimum: U256) -> Result<U256, WalletError> {
    if estimate < minimum {
        return Err(WalletError::GasEstimationFailed);
    }
    Ok(estimate)
}

async fn resolve_eth_preflight_quote(
    provider: &EthNetworkProvider,
    from: Address,
    to: Address,
    submitted_value: U256,
    balance: U256,
    fee_params: &ResolvedEvmFee,
) -> Result<(U256, U256, U256, bool, Option<String>), WalletError> {
    if submitted_value > balance {
        return Err(WalletError::InsufficientFunds);
    }

    let mut candidate = submitted_value;
    let mut seeded_max_send = false;
    for _ in 0..4 {
        let estimate = match estimate_eth_transfer_gas(provider, from, to, candidate).await {
            Ok(estimate) => estimate,
            Err(WalletError::GasEstimationFailed)
                if !seeded_max_send
                    && submitted_value == balance
                    && candidate == submitted_value =>
            {
                seeded_max_send = true;
                let baseline_limit = add_fraction(
                    U256::from(ETH_TRANSFER_GAS_BASELINE),
                    ETH_GAS_MARGIN_DIVISOR,
                );
                let baseline_fee = baseline_limit.saturating_mul(fee_params.max_fee_per_gas);
                candidate = balance
                    .checked_sub(baseline_fee)
                    .filter(|value| !value.is_zero())
                    .ok_or(WalletError::InsufficientFunds)?;
                continue;
            }
            Err(error) => return Err(error),
        };
        let gas_limit = add_fraction(estimate, ETH_GAS_MARGIN_DIVISOR);
        let max_fee = gas_limit.saturating_mul(fee_params.max_fee_per_gas);
        let (value, adjusted, message) =
            resolve_eth_value_after_fee(submitted_value, balance, max_fee)?;
        if value == candidate {
            return Ok((value, gas_limit, max_fee, adjusted, message));
        }
        candidate = value;
    }

    Err(WalletError::GasEstimationFailed)
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
    use super::{
        minimum_intrinsic_gas, parse_eth_address, parse_token_amount, resolve_eth_value_after_fee,
        validate_gas_estimate,
    };
    use crate::types::WalletError;
    use ethers::types::U256;

    #[test]
    fn rejects_zero_and_below_intrinsic_gas_estimates() {
        let calldata = [0u8, 1u8];
        let minimum = minimum_intrinsic_gas(&calldata);
        assert_eq!(minimum, U256::from(21_020u64));
        assert!(matches!(
            validate_gas_estimate(U256::zero(), minimum),
            Err(WalletError::GasEstimationFailed)
        ));
        assert!(matches!(
            validate_gas_estimate(U256::from(21_019u64), minimum),
            Err(WalletError::GasEstimationFailed)
        ));
        assert_eq!(
            validate_gas_estimate(minimum, minimum).expect("minimum valid"),
            minimum
        );
    }

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
    fn resolve_eth_value_after_fee_never_turns_an_overbalance_amount_into_max() {
        let result = resolve_eth_value_after_fee(
            U256::from(1_000_001u64),
            U256::from(1_000_000u64),
            U256::from(21_000u64),
        );
        assert!(matches!(result, Err(WalletError::InsufficientFunds)));
    }

    #[test]
    fn token_amount_parser_is_exact_and_unsigned() {
        assert_eq!(
            parse_token_amount("1.234567", 6).expect("six decimals"),
            U256::from(1_234_567u64)
        );
        assert_eq!(
            parse_token_amount("0001.2", 6).expect("padding"),
            U256::from(1_200_000u64)
        );
        for invalid in ["", "0", "-1", "+1", "1e2", "1.2345678", "1.", ".1"] {
            assert!(matches!(
                parse_token_amount(invalid, 6),
                Err(WalletError::InvalidAmount)
            ));
        }
        assert!(matches!(
            parse_token_amount(
                "999999999999999999999999999999999999999999999999999999999999999999999999999999",
                18,
            ),
            Err(WalletError::InvalidAmount)
        ));
    }

    #[test]
    fn parse_eth_address_rejects_invalid_destination() {
        let result = parse_eth_address("not-an-eth-address");
        assert!(matches!(result, Err(WalletError::InvalidAddress)));
    }

    #[test]
    fn parse_token_amount_rejects_zero() {
        let result = parse_token_amount("0", 18);
        assert!(matches!(result, Err(WalletError::InvalidAmount)));
    }
}
