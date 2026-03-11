use std::sync::Arc;

use ethers::abi::Abi;
use ethers::contract::Contract;
use ethers::middleware::SignerMiddleware;
use ethers::providers::Middleware;
use ethers::signers::{LocalWallet, Signer};
use ethers::types::Bytes;
use ethers::types::{Address, Eip1559TransactionRequest, U256};
use tokio::sync::Mutex;

use crate::core::auth::{
    capture_active_wallet_access_context, load_primary_private_scalar_for_context, SessionManager,
};
use crate::core::channels::eth::bridge::delegator::{
    CcurrencyValueMap, CreserveTransfer, CtransferDestination, VerusBridgeDelegatorContract,
};
use crate::core::channels::eth::preflight::EthPreflightPayload;
use crate::core::channels::eth::provider::EthProviderPool;
use crate::core::channels::store::PreflightStore;
use crate::types::transaction::SendResult;
use crate::types::WalletError;

const ERC20_TRANSFER_ABI: &str = r#"[
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

const ERC20_APPROVE_ABI: &str = r#"[
  {
    "constant": false,
    "inputs": [
      {"name": "_spender", "type": "address"},
      {"name": "_value", "type": "uint256"}
    ],
    "name": "approve",
    "outputs": [{"name": "", "type": "bool"}],
    "type": "function"
  }
]"#;

pub async fn send(
    preflight_id: &str,
    preflight_store: &PreflightStore,
    session_manager: &Arc<Mutex<SessionManager>>,
    provider_pool: &EthProviderPool,
) -> Result<SendResult, WalletError> {
    let record = preflight_store
        .take(preflight_id)
        .ok_or(WalletError::InvalidPreflight)?;

    let payload: EthPreflightPayload =
        serde_json::from_value(record.payload).map_err(|_| WalletError::InvalidPreflight)?;

    let context = capture_active_wallet_access_context(session_manager).await?;
    if context.account_id != record.account_id {
        return Err(WalletError::InvalidPreflight);
    }

    let wallet_network = context.wallet_network;
    let private_key = load_primary_private_scalar_for_context(&context).await?;

    let network_provider = provider_pool.for_network(wallet_network)?;
    let wallet = LocalWallet::from_bytes(&*private_key)
        .map_err(|_| WalletError::OperationFailed)?
        .with_chain_id(network_provider.chain_id);

    let signer = Arc::new(SignerMiddleware::new(
        network_provider.rpc_provider.clone(),
        wallet,
    ));

    match payload {
        EthPreflightPayload::Eth {
            from_address,
            to_address,
            value_wei,
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            fee,
            value,
            ..
        } => {
            let parsed_from: Address = from_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let parsed_to: Address = to_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let value_wei = parse_u256(&value_wei)?;
            let gas_limit = parse_u256(&gas_limit)?;
            let max_fee_per_gas = parse_u256(&max_fee_per_gas)?;
            let max_priority_fee_per_gas = parse_u256(&max_priority_fee_per_gas)?;

            let tx = Eip1559TransactionRequest::new()
                .from(parsed_from)
                .to(parsed_to)
                .value(value_wei)
                .gas(gas_limit)
                .max_fee_per_gas(max_fee_per_gas)
                .max_priority_fee_per_gas(max_priority_fee_per_gas)
                .chain_id(network_provider.chain_id);

            let pending = signer
                .send_transaction(tx, None)
                .await
                .map_err(|_| WalletError::NetworkError)?;

            Ok(SendResult {
                txid: format!("{:#x}", pending.tx_hash()),
                fee,
                value,
                to_address,
                from_address,
            })
        }
        EthPreflightPayload::Erc20 {
            from_address,
            to_address,
            token_address,
            token_value_raw,
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            max_fee_cap,
            fee,
            value,
            ..
        } => {
            let parsed_from: Address = from_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let parsed_to: Address = to_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let token_address: Address = token_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let amount_raw = parse_u256(&token_value_raw)?;
            let gas_limit = parse_u256(&gas_limit)?;
            let max_fee_per_gas = parse_u256(&max_fee_per_gas)?;
            let max_priority_fee_per_gas = parse_u256(&max_priority_fee_per_gas)?;
            let max_fee_cap = parse_u256(&max_fee_cap)?;

            let fee_data = network_provider
                .rpc_provider
                .estimate_eip1559_fees(None)
                .await
                .map_err(|_| WalletError::NetworkError)?;
            let current_max_fee = fee_data.0;

            if fee_drift_exceeds_cap(gas_limit, current_max_fee, max_fee_cap) {
                return Err(WalletError::OperationFailed);
            }

            let abi: Abi = serde_json::from_str(ERC20_TRANSFER_ABI)
                .map_err(|_| WalletError::OperationFailed)?;
            let contract = Contract::new(token_address, abi, signer.clone());

            let transfer_call = contract
                .method::<_, bool>("transfer", (parsed_to, amount_raw))
                .map_err(|_| WalletError::OperationFailed)?;

            let configured_call = transfer_call
                .from(parsed_from)
                .gas(gas_limit)
                .gas_price(max_fee_per_gas.max(max_priority_fee_per_gas));

            let pending = configured_call
                .send()
                .await
                .map_err(|_| WalletError::NetworkError)?;

            Ok(SendResult {
                txid: format!("{:#x}", pending.tx_hash()),
                fee,
                value,
                to_address,
                from_address,
            })
        }
        EthPreflightPayload::Bridge {
            from_address,
            to_address,
            source_contract,
            source_amount_token_raw,
            reserve_transfer_version,
            reserve_transfer_currency,
            reserve_transfer_amount,
            reserve_transfer_flags,
            reserve_transfer_fee_currency_id,
            reserve_transfer_fees,
            reserve_transfer_destination_type,
            reserve_transfer_destination_address,
            reserve_transfer_dest_currency_id,
            reserve_transfer_dest_system_id,
            reserve_transfer_second_reserve_id,
            bridge_contract,
            transfer_value_wei,
            gas_limit,
            transfer_gas_limit,
            approval_gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            max_fee_cap,
            approval_zero_out,
            fee,
            value,
            ..
        } => {
            let parsed_from: Address = from_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let source_contract: Address = source_contract
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let bridge_contract: Address = bridge_contract
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let reserve_currency: Address = reserve_transfer_currency
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let reserve_fee_currency: Address = reserve_transfer_fee_currency_id
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let reserve_dest_currency: Address = reserve_transfer_dest_currency_id
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let reserve_dest_system: Address = reserve_transfer_dest_system_id
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let reserve_second_reserve: Address = reserve_transfer_second_reserve_id
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;

            let reserve_amount = parse_u64(&reserve_transfer_amount)?;
            let reserve_destination_address =
                parse_hex_bytes(&reserve_transfer_destination_address)?;
            let transfer_value_wei = parse_u256(&transfer_value_wei)?;
            let gas_limit = parse_u256(&gas_limit)?;
            let transfer_gas_limit = parse_u256(&transfer_gas_limit)?;
            let approval_gas_limit = parse_u256(&approval_gas_limit)?;
            let max_fee_per_gas = parse_u256(&max_fee_per_gas)?;
            let max_priority_fee_per_gas = parse_u256(&max_priority_fee_per_gas)?;
            let max_fee_cap = parse_u256(&max_fee_cap)?;

            let fee_data = network_provider
                .rpc_provider
                .estimate_eip1559_fees(None)
                .await
                .map_err(|_| WalletError::NetworkError)?;
            let current_max_fee = fee_data.0;
            if fee_drift_exceeds_cap(gas_limit, current_max_fee, max_fee_cap) {
                return Err(WalletError::BridgeGasDriftExceeded);
            }

            if source_contract != Address::zero() {
                let approval_amount_raw = source_amount_token_raw
                    .as_deref()
                    .ok_or(WalletError::InvalidPreflight)
                    .and_then(parse_u256)?;

                let abi: Abi = serde_json::from_str(ERC20_APPROVE_ABI)
                    .map_err(|_| WalletError::OperationFailed)?;
                let token_contract = Contract::new(source_contract, abi, signer.clone());

                let approval_gas_price = max_fee_per_gas.max(max_priority_fee_per_gas);

                if approval_zero_out {
                    let zero_call = token_contract
                        .method::<_, bool>("approve", (bridge_contract, U256::zero()))
                        .map_err(|_| WalletError::OperationFailed)?;
                    let zero_call = zero_call
                        .from(parsed_from)
                        .gas(approval_gas_limit)
                        .gas_price(approval_gas_price);
                    let zero_pending = zero_call
                        .send()
                        .await
                        .map_err(|_| WalletError::NetworkError)?;
                    let zero_receipt = zero_pending.await.map_err(|_| WalletError::NetworkError)?;
                    let zero_ok = zero_receipt
                        .and_then(|receipt| receipt.status)
                        .map(|status| status.as_u64() == 1)
                        .unwrap_or(false);
                    if !zero_ok {
                        return Err(WalletError::BridgeApprovalFailed);
                    }
                }

                let approval_call = token_contract
                    .method::<_, bool>("approve", (bridge_contract, approval_amount_raw))
                    .map_err(|_| WalletError::OperationFailed)?;
                let approval_call = approval_call
                    .from(parsed_from)
                    .gas(approval_gas_limit)
                    .gas_price(approval_gas_price);
                let approval_pending = approval_call
                    .send()
                    .await
                    .map_err(|_| WalletError::NetworkError)?;
                let approval_receipt = approval_pending
                    .await
                    .map_err(|_| WalletError::NetworkError)?;
                let approval_ok = approval_receipt
                    .and_then(|receipt| receipt.status)
                    .map(|status| status.as_u64() == 1)
                    .unwrap_or(false);
                if !approval_ok {
                    return Err(WalletError::BridgeApprovalFailed);
                }
            }

            let reserve_transfer = CreserveTransfer {
                version: reserve_transfer_version,
                currencyvalue: CcurrencyValueMap {
                    currency: reserve_currency,
                    amount: reserve_amount,
                },
                flags: reserve_transfer_flags,
                feecurrencyid: reserve_fee_currency,
                fees: reserve_transfer_fees,
                destination: CtransferDestination {
                    destinationtype: reserve_transfer_destination_type,
                    destinationaddress: reserve_destination_address,
                },
                destcurrencyid: reserve_dest_currency,
                destsystemid: reserve_dest_system,
                secondreserveid: reserve_second_reserve,
            };

            let delegator = VerusBridgeDelegatorContract::new(bridge_contract, signer.clone());
            let send_transfer_call = delegator
                .send_transfer(reserve_transfer)
                .from(parsed_from)
                .gas(transfer_gas_limit)
                .gas_price(max_fee_per_gas.max(max_priority_fee_per_gas))
                .value(transfer_value_wei);
            let pending = send_transfer_call
                .send()
                .await
                .map_err(|_| WalletError::NetworkError)?;

            Ok(SendResult {
                txid: format!("{:#x}", pending.tx_hash()),
                fee,
                value,
                to_address,
                from_address,
            })
        }
    }
}

fn parse_u256(input: &str) -> Result<U256, WalletError> {
    U256::from_dec_str(input.trim()).map_err(|_| WalletError::OperationFailed)
}

fn parse_u64(input: &str) -> Result<u64, WalletError> {
    input
        .trim()
        .parse::<u64>()
        .map_err(|_| WalletError::OperationFailed)
}

fn parse_hex_bytes(input: &str) -> Result<Bytes, WalletError> {
    let trimmed = input.trim().trim_start_matches("0x");
    let raw = hex::decode(trimmed).map_err(|_| WalletError::OperationFailed)?;
    Ok(Bytes::from(raw))
}

fn fee_drift_exceeds_cap(
    gas_limit: U256,
    current_max_fee_per_gas: U256,
    max_fee_cap: U256,
) -> bool {
    gas_limit.saturating_mul(current_max_fee_per_gas) > max_fee_cap
}

#[cfg(test)]
mod tests {
    use super::fee_drift_exceeds_cap;
    use ethers::types::U256;

    #[test]
    fn fee_drift_exceeds_cap_returns_true_when_current_fee_is_higher_than_preflight_cap() {
        let gas_limit = U256::from(100_000u64);
        let current_fee = U256::from(40u64);
        let cap = U256::from(3_900_000u64);

        assert!(fee_drift_exceeds_cap(gas_limit, current_fee, cap));
    }

    #[test]
    fn fee_drift_exceeds_cap_returns_false_when_within_cap() {
        let gas_limit = U256::from(100_000u64);
        let current_fee = U256::from(39u64);
        let cap = U256::from(3_900_000u64);

        assert!(!fee_drift_exceeds_cap(gas_limit, current_fee, cap));
    }
}
