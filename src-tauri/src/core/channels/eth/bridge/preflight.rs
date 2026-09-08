use std::sync::Arc;

use ethers::abi::Abi;
use ethers::contract::Contract;
use ethers::providers::Middleware;
use ethers::types::{Address, U256};
use ethers::utils::{format_units, parse_units};
use serde_json::Value;
use uuid::Uuid;

use crate::core::channels::eth::bridge::delegator::{
    delegator_contract_for_chain_id, VerusBridgeDelegatorContract,
};
use crate::core::channels::eth::bridge::fees::{
    add_fraction, approval_estimate_skip, approval_zero_out_required, base_bridge_fee_wei,
    current_bridge_max_fee_per_gas, import_fee_wei, reserve_transfer_fee_sats,
    transfer_gas_limit_for_token, FALLBACK_APPROVAL_GAS_COST, FALLBACK_GAS_BRIDGE_TRANSFER,
    GAS_PRICE_MODIFIER_DELEGATOR_CONTRACT, INITIAL_GAS_LIMIT,
};
use crate::core::channels::eth::bridge::reserve_transfer::{
    build_gateway_eth_destination, normalize_eth_destination, normalize_verus_destination,
    to_eth_address_from_iaddress, ReserveTransferDestination, ReserveTransferPayload,
    RESERVE_TRANSFER_CONVERT, RESERVE_TRANSFER_IMPORT_TO_SOURCE,
    RESERVE_TRANSFER_RESERVE_TO_RESERVE, RESERVE_TRANSFER_VALID,
};
use crate::core::channels::eth::bridge::token_mapping::get_currencies_mapped_to_eth;
use crate::core::channels::eth::preflight::EthPreflightPayload;
use crate::core::channels::eth::provider::EthNetworkProvider;
use crate::core::channels::store::{PreflightRecord, PreflightStore};
use crate::core::channels::vrpc::VrpcProvider;
use crate::core::coins::CoinDefinition;
use crate::types::bridge::{
    BridgeExecutionHint, BridgeTransferPreflightParams, BridgeTransferPreflightResult,
    BridgeTransferRoute,
};
use crate::types::transaction::PreflightWarning;
use crate::types::WalletError;

const APPROVE_ABI: &str = r#"[
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

const BRIDGE_NAME: &str = "Bridge.vETH";
const WEI_PER_SAT_U64: u64 = 10_000_000_000;

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
    let from = from_address
        .trim()
        .parse::<Address>()
        .map_err(|_| WalletError::InvalidAddress)?;

    let convert_to = normalized_optional(&params.convert_to);
    let export_to = normalized_optional(&params.export_to);
    let via = normalized_optional(&params.via);
    let map_to = normalized_optional(&params.map_to);
    let is_conversion = convert_to.is_some();

    let raw_destination =
        normalize_identity_destination(vrpc_provider, &params.destination).await?;
    let destination_eth_parse = normalize_eth_destination(&raw_destination);
    let destination_is_eth = destination_eth_parse.is_ok();

    if destination_is_eth && !is_conversion {
        return Err(WalletError::BridgeUnsupportedDestinationCombination);
    }
    if destination_is_eth && export_to.is_some() {
        return Err(WalletError::BridgeUnsupportedDestinationCombination);
    }

    let source_contract = source_coin
        .currency_id
        .trim()
        .parse::<Address>()
        .map_err(|_| WalletError::InvalidAddress)?;
    let source_contract_str = format!("{:#x}", source_contract);
    let source_is_eth = source_contract == Address::zero();

    let system_id = resolve_system_id(&params, vrpc_provider).await?;
    let system_hex = to_eth_address_from_iaddress(&system_id)?;

    let bridge_definition = match vrpc_provider.getcurrency(BRIDGE_NAME).await {
        Ok(value) => value,
        Err(_) => vrpc_provider.getcurrency("vETH").await?,
    };
    let bridge_iaddress =
        extract_currency_id(&bridge_definition).ok_or(WalletError::BridgeRouteInvalid)?;
    let bridge_hex = to_eth_address_from_iaddress(&bridge_iaddress)?;
    let veth_definition = match vrpc_provider.getcurrency("vETH").await {
        Ok(value) => value,
        Err(_) => bridge_definition.clone(),
    };
    let veth_iaddress =
        extract_currency_id(&veth_definition).ok_or(WalletError::BridgeRouteInvalid)?;
    let veth_hex = to_eth_address_from_iaddress(&veth_iaddress)?;

    let delegator_address = delegator_contract_for_chain_id(provider.chain_id)?;
    let delegator = VerusBridgeDelegatorContract::new(
        delegator_address,
        Arc::new(provider.rpc_provider.clone()),
    );
    let past_prelaunch = delegator
        .bridge_converter_active()
        .call()
        .await
        .map_err(|_| WalletError::NetworkError)?;

    if is_conversion && !past_prelaunch {
        return Err(WalletError::BridgeRouteInvalid);
    }

    let mapped_currency_iaddress = resolve_mapped_currency(
        &map_to,
        is_conversion,
        source_contract,
        &bridge_definition,
        &system_id,
        vrpc_provider,
        provider,
        past_prelaunch,
    )
    .await?;
    let mapped_currency_hex = to_eth_address_from_iaddress(&mapped_currency_iaddress)?;
    let mapped_is_bridge = mapped_currency_iaddress.eq_ignore_ascii_case(&bridge_iaddress);

    let mut flags = RESERVE_TRANSFER_VALID;
    let mut second_reserve_id = Address::zero();
    let destination_currency;
    if is_conversion {
        flags |= RESERVE_TRANSFER_CONVERT;
        if mapped_is_bridge {
            flags |= RESERVE_TRANSFER_IMPORT_TO_SOURCE;
        }
        if via.is_some() {
            flags |= RESERVE_TRANSFER_RESERVE_TO_RESERVE;
        }

        let convert_to_id = convert_to.clone().ok_or(WalletError::BridgeRouteInvalid)?;
        let convert_to_definition = vrpc_provider
            .getcurrency(&convert_to_id)
            .await
            .map_err(|_| WalletError::BridgeRouteInvalid)?;
        let convert_to_currency_id =
            extract_currency_id(&convert_to_definition).ok_or(WalletError::BridgeRouteInvalid)?;
        let final_destination_currency = to_eth_address_from_iaddress(&convert_to_currency_id)?;

        if via.is_some() {
            second_reserve_id = final_destination_currency;
        }

        destination_currency = if mapped_is_bridge {
            final_destination_currency
        } else {
            bridge_hex
        };
    } else if !past_prelaunch {
        destination_currency = system_hex;
    } else {
        destination_currency = bridge_hex;
    }

    let max_fee_per_gas = current_bridge_max_fee_per_gas(provider).await?;
    let max_priority_fee_per_gas = max_fee_per_gas;
    let reserve_transfer_fee = reserve_transfer_fee_sats(past_prelaunch);
    let base_fee_wei = base_bridge_fee_wei(past_prelaunch);
    let import_fee = import_fee_wei(max_fee_per_gas);
    let import_fee_sats = u256_to_u64(wei_to_sats(import_fee)?)?;
    let fee_currency_id = if past_prelaunch { veth_hex } else { system_hex };

    let mut submitted_sats = parse_amount_sats(&params.amount)?;
    let mut adjusted_amount: Option<String> = None;
    let mut final_payload: Option<(
        ReserveTransferPayload,
        ReserveTransferDestination,
        U256,
        U256,
        U256,
        U256,
        U256,
        Option<U256>,
        bool,
    )> = None;

    for attempt in 0..=1 {
        let destination = if let Ok((eth_destination, normalized)) = &destination_eth_parse {
            let mut built = build_gateway_eth_destination(
                *eth_destination,
                &veth_iaddress,
                import_fee_sats,
                refund_vrpc_address,
            )?;
            built.normalized_destination = normalized.clone();
            built
        } else {
            normalize_verus_destination(&raw_destination)?
        };

        let reserve_transfer = ReserveTransferPayload {
            version: 1,
            currency_value_currency: mapped_currency_hex,
            currency_value_amount: u256_to_u64(submitted_sats)?,
            flags,
            fee_currency_id,
            fees: reserve_transfer_fee,
            destination_type: destination.destination_type,
            destination_address: destination.destination_address.clone(),
            dest_currency_id: destination_currency,
            dest_system_id: Address::zero(),
            second_reserve_id,
        };

        let mut transfer_value_wei = base_fee_wei;
        if destination.is_gateway_destination {
            transfer_value_wei = transfer_value_wei.saturating_add(import_fee);
        }
        if source_is_eth {
            transfer_value_wei = transfer_value_wei.saturating_add(sats_to_wei(submitted_sats));
        }

        let transfer_gas_limit = if source_is_eth {
            let transfer = reserve_transfer.clone().into_contract_struct();
            let estimated_gas = match delegator
                .send_transfer(transfer)
                .from(from)
                .gas(U256::from(INITIAL_GAS_LIMIT))
                .gas_price(max_fee_per_gas)
                .value(transfer_value_wei)
                .estimate_gas()
                .await
            {
                Ok(gas) => gas,
                Err(_) => U256::from(FALLBACK_GAS_BRIDGE_TRANSFER),
            };

            add_fraction(estimated_gas, GAS_PRICE_MODIFIER_DELEGATOR_CONTRACT)
        } else {
            transfer_gas_limit_for_token(&source_contract_str)
        };

        let mut approval_amount_raw: Option<U256> = None;
        let approval_gas_limit = if source_is_eth {
            U256::zero()
        } else {
            let amount_raw = sats_to_token_raw(submitted_sats, source_coin.decimals)?;
            approval_amount_raw = Some(amount_raw);

            let estimate = if approval_estimate_skip(&source_contract_str) {
                U256::from(FALLBACK_APPROVAL_GAS_COST)
            } else {
                let abi: Abi =
                    serde_json::from_str(APPROVE_ABI).map_err(|_| WalletError::OperationFailed)?;
                let contract = Contract::new(
                    source_contract,
                    abi,
                    Arc::new(provider.rpc_provider.clone()),
                );
                contract
                    .method::<_, bool>("approve", (delegator_address, amount_raw))
                    .map_err(|_| WalletError::OperationFailed)?
                    .from(from)
                    .gas(U256::from(INITIAL_GAS_LIMIT))
                    .gas_price(max_fee_per_gas)
                    .estimate_gas()
                    .await
                    .unwrap_or_else(|_| U256::from(FALLBACK_APPROVAL_GAS_COST))
            };

            add_fraction(estimate, GAS_PRICE_MODIFIER_DELEGATOR_CONTRACT)
        };

        let gas_limit = transfer_gas_limit.saturating_add(approval_gas_limit);
        let gas_fee = gas_limit.saturating_mul(max_fee_per_gas);
        let max_total_fee = if destination.is_gateway_destination {
            gas_fee
                .saturating_add(import_fee)
                .saturating_add(base_fee_wei)
        } else {
            gas_fee.saturating_add(base_fee_wei)
        };

        let balance = provider
            .rpc_provider
            .get_balance(from, None)
            .await
            .map_err(|_| WalletError::NetworkError)?;

        let insufficient = if source_is_eth {
            let max_total_fee_sats = wei_to_sats_round(max_total_fee);
            let balance_sats = wei_to_sats_round(balance);
            max_total_fee_sats.saturating_add(submitted_sats) > balance_sats
        } else {
            max_total_fee > balance
        };

        if insufficient {
            if source_is_eth && attempt == 0 {
                if let Some(adjusted_sats) =
                    adjust_submitted_sats_for_fee_envelope(submitted_sats, max_total_fee)
                {
                    submitted_sats = adjusted_sats;
                    adjusted_amount = Some(sats_to_decimal_string(submitted_sats));
                    continue;
                }
            }
            return Err(WalletError::BridgeInsufficientEthFeeEnvelope);
        }

        if source_is_eth {
            let transfer = reserve_transfer.clone().into_contract_struct();
            let call_result = delegator
                .send_transfer(transfer)
                .from(from)
                .gas(transfer_gas_limit)
                .gas_price(max_fee_per_gas)
                .value(transfer_value_wei)
                .call()
                .await;
            if call_result.is_err() {
                if adjusted_amount.is_some() {
                    return Err(WalletError::BridgeInsufficientEthFeeEnvelope);
                }
                return Err(WalletError::BridgeRouteInvalid);
            }
        }

        final_payload = Some((
            reserve_transfer,
            destination,
            transfer_value_wei,
            gas_limit,
            transfer_gas_limit,
            approval_gas_limit,
            max_total_fee,
            approval_amount_raw,
            approval_zero_out_required(&source_contract_str),
        ));
        break;
    }

    let Some((
        reserve_transfer,
        destination,
        transfer_value_wei,
        gas_limit,
        transfer_gas_limit,
        approval_gas_limit,
        max_total_fee,
        approval_amount_raw,
        approval_zero_out,
    )) = final_payload
    else {
        return Err(WalletError::OperationFailed);
    };

    let fee_display = format_units(max_total_fee, 18).map_err(|_| WalletError::OperationFailed)?;
    let value_display = if source_is_eth {
        sats_to_decimal_string(submitted_sats)
    } else {
        params.amount.trim().to_string()
    };

    let payload = EthPreflightPayload::Bridge {
        chain_id: provider.chain_id,
        coin_id: source_coin.id.clone(),
        channel_id: channel_id.to_string(),
        from_address: from_address.to_string(),
        refund_vrpc_address: refund_vrpc_address.to_string(),
        to_address: destination.normalized_destination.clone(),
        source_contract: source_contract_str.clone(),
        source_decimals: source_coin.decimals,
        source_amount_sats: submitted_sats.to_string(),
        source_amount_token_raw: approval_amount_raw.map(|value| value.to_string()),
        mapped_currency_iaddress: mapped_currency_iaddress.clone(),
        mapped_currency_eth_address: format!("{:#x}", reserve_transfer.currency_value_currency),
        reserve_transfer_version: reserve_transfer.version,
        reserve_transfer_currency: format!("{:#x}", reserve_transfer.currency_value_currency),
        reserve_transfer_amount: reserve_transfer.currency_value_amount.to_string(),
        reserve_transfer_flags: reserve_transfer.flags,
        reserve_transfer_fee_currency_id: format!("{:#x}", reserve_transfer.fee_currency_id),
        reserve_transfer_fees: reserve_transfer.fees,
        reserve_transfer_destination_type: reserve_transfer.destination_type,
        reserve_transfer_destination_address: format!(
            "0x{}",
            hex::encode(reserve_transfer.destination_address.as_ref())
        ),
        reserve_transfer_dest_currency_id: format!("{:#x}", reserve_transfer.dest_currency_id),
        reserve_transfer_dest_system_id: format!("{:#x}", reserve_transfer.dest_system_id),
        reserve_transfer_second_reserve_id: format!("{:#x}", reserve_transfer.second_reserve_id),
        bridge_contract: format!("{:#x}", delegator_address),
        transfer_value_wei: transfer_value_wei.to_string(),
        gas_limit: gas_limit.to_string(),
        transfer_gas_limit: transfer_gas_limit.to_string(),
        approval_gas_limit: approval_gas_limit.to_string(),
        max_fee_per_gas: max_fee_per_gas.to_string(),
        max_priority_fee_per_gas: max_priority_fee_per_gas.to_string(),
        max_fee_cap: max_total_fee.to_string(),
        approval_zero_out,
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

    let mut warnings = Vec::<PreflightWarning>::new();
    if is_conversion {
        warnings.push(PreflightWarning {
            warning_type: "estimated_fee".to_string(),
            message: "Final amount you receive may vary slightly.".to_string(),
        });
    }

    Ok(BridgeTransferPreflightResult {
        preflight_id,
        fee: fee_display,
        fee_currency: "ETH".to_string(),
        value: value_display,
        amount_submitted: params.amount,
        amount_adjusted: adjusted_amount,
        to_address: destination.normalized_destination,
        from_address: from_address.to_string(),
        warnings,
        memo: params.memo.clone(),
        route: BridgeTransferRoute {
            convert_to,
            export_to,
            via,
            map_to,
        },
        execution: BridgeExecutionHint {
            engine: "eth_delegator_bridge".to_string(),
            requires_token_approval: !source_is_eth,
            bridge_contract: Some(format!("{:#x}", delegator_address)),
        },
    })
}

async fn resolve_mapped_currency(
    map_to: &Option<String>,
    is_conversion: bool,
    source_contract: Address,
    bridge_definition: &Value,
    system_id: &str,
    vrpc_provider: &VrpcProvider,
    provider: &EthNetworkProvider,
    past_prelaunch: bool,
) -> Result<String, WalletError> {
    if let Some(map_to_ref) = map_to {
        let mapped = vrpc_provider
            .getcurrency(map_to_ref)
            .await
            .map_err(|_| WalletError::BridgeRouteInvalid)?;
        return extract_currency_id(&mapped).ok_or(WalletError::BridgeRouteInvalid);
    }

    if !is_conversion {
        return Err(WalletError::BridgeRouteInvalid);
    }
    if !past_prelaunch {
        return Err(WalletError::BridgeRouteInvalid);
    }

    let mappings = get_currencies_mapped_to_eth(vrpc_provider, Some(provider)).await?;
    let source_contract_key = format!("{:#x}", source_contract);
    let Some(mapped_to_source) = mappings.contract_to_currencies.get(&source_contract_key) else {
        return Err(WalletError::BridgeRouteInvalid);
    };

    let bridge_id =
        extract_currency_id(bridge_definition).ok_or(WalletError::BridgeRouteInvalid)?;
    let mut convertable = vec![system_id.to_string(), bridge_id.clone()];
    if let Some(currencies) = bridge_definition
        .get("currencies")
        .and_then(Value::as_array)
    {
        for currency in currencies {
            if let Some(currency_id) = extract_currency_ref(currency) {
                if !convertable
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(&currency_id))
                {
                    convertable.push(currency_id);
                }
            }
        }
    }

    for currency in mapped_to_source {
        if convertable
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(&currency.currency_id))
        {
            return Ok(currency.currency_id.clone());
        }
    }

    Err(WalletError::BridgeRouteInvalid)
}

async fn normalize_identity_destination(
    vrpc_provider: &VrpcProvider,
    destination: &str,
) -> Result<String, WalletError> {
    let trimmed = destination.trim();
    if trimmed.is_empty() {
        return Err(WalletError::InvalidAddress);
    }
    if !trimmed.ends_with('@') {
        return Ok(trimmed.to_string());
    }

    let identity = vrpc_provider
        .getidentity(trimmed)
        .await
        .map_err(|_| WalletError::InvalidAddress)?;
    identity
        .get("identity")
        .and_then(|entry| entry.get("identityaddress"))
        .or(identity.get("identityaddress"))
        .and_then(extract_stringish)
        .ok_or(WalletError::InvalidAddress)
}

async fn resolve_system_id(
    params: &BridgeTransferPreflightParams,
    vrpc_provider: &VrpcProvider,
) -> Result<String, WalletError> {
    if let Some(export_to) = normalized_optional(&params.export_to) {
        if to_eth_address_from_iaddress(&export_to).is_ok() {
            return Ok(export_to);
        }

        if let Ok(currency_definition) = vrpc_provider.getcurrency(&export_to).await {
            if let Some(currency_id) = extract_currency_id(&currency_definition) {
                return Ok(currency_id);
            }
        }
    }

    let info = vrpc_provider.getinfo().await?;
    info.get("chainid")
        .and_then(extract_stringish)
        .ok_or(WalletError::BridgeRouteInvalid)
}

fn normalized_optional(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|trimmed| !trimmed.is_empty())
        .map(ToString::to_string)
}

fn parse_amount_sats(amount: &str) -> Result<U256, WalletError> {
    let parsed = parse_units(amount.trim(), 8).map_err(|_| WalletError::OperationFailed)?;
    let value: U256 = parsed.into();
    if value.is_zero() {
        return Err(WalletError::OperationFailed);
    }
    Ok(value)
}

fn sats_to_token_raw(sats: U256, decimals: u8) -> Result<U256, WalletError> {
    let sats_u64 = u256_to_u64(sats)?;
    let amount_decimal = sats_to_decimal_string(U256::from(sats_u64));
    let parsed =
        parse_units(amount_decimal, decimals as usize).map_err(|_| WalletError::OperationFailed)?;
    Ok(parsed.into())
}

fn sats_to_wei(sats: U256) -> U256 {
    sats.saturating_mul(U256::from(WEI_PER_SAT_U64))
}

fn wei_to_sats(wei: U256) -> Result<U256, WalletError> {
    Ok(wei / U256::from(WEI_PER_SAT_U64))
}

fn wei_to_sats_round(wei: U256) -> U256 {
    let divisor = U256::from(WEI_PER_SAT_U64);
    let half_divisor = divisor / U256::from(2u64);
    wei.saturating_add(half_divisor) / divisor
}

fn adjust_submitted_sats_for_fee_envelope(
    submitted_sats: U256,
    max_total_fee_wei: U256,
) -> Option<U256> {
    let max_total_fee_sats = wei_to_sats_round(max_total_fee_wei);
    if submitted_sats <= max_total_fee_sats {
        return None;
    }
    Some(submitted_sats.saturating_sub(max_total_fee_sats))
}

fn u256_to_u64(value: U256) -> Result<u64, WalletError> {
    if value > U256::from(u64::MAX) {
        return Err(WalletError::OperationFailed);
    }
    Ok(value.as_u64())
}

fn sats_to_decimal_string(sats: U256) -> String {
    let mut raw = sats.to_string();
    while raw.len() < 9 {
        raw.insert(0, '0');
    }
    let split_index = raw.len() - 8;
    let (whole, fractional) = raw.split_at(split_index);
    let mut out = format!("{}.{}", whole, fractional);
    while out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.pop();
    }
    out
}

fn extract_currency_id(value: &Value) -> Option<String> {
    value
        .get("currencyid")
        .and_then(extract_stringish)
        .or_else(|| value.get("fullyqualifiedname").and_then(extract_stringish))
        .or_else(|| value.get("name").and_then(extract_stringish))
}

fn extract_currency_ref(value: &Value) -> Option<String> {
    if let Some(raw) = value.as_str() {
        return Some(raw.to_string());
    }

    value
        .get("currencyid")
        .and_then(extract_stringish)
        .or_else(|| value.get("address").and_then(extract_stringish))
        .or_else(|| value.get("fullyqualifiedname").and_then(extract_stringish))
        .or_else(|| value.get("name").and_then(extract_stringish))
}

fn extract_stringish(value: &Value) -> Option<String> {
    if let Some(raw) = value.as_str() {
        return Some(raw.to_string());
    }
    if let Some(raw) = value.as_i64() {
        return Some(raw.to_string());
    }
    if let Some(raw) = value.as_u64() {
        return Some(raw.to_string());
    }
    value.as_f64().map(|raw| raw.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        adjust_submitted_sats_for_fee_envelope, extract_currency_ref, sats_to_wei,
        wei_to_sats_round, WEI_PER_SAT_U64,
    };
    use ethers::types::U256;
    use serde_json::json;

    #[test]
    fn wei_to_sats_round_keeps_exact_division() {
        let sats = U256::from(123u64);
        let wei = sats_to_wei(sats);
        assert_eq!(wei_to_sats_round(wei), sats);
    }

    #[test]
    fn wei_to_sats_round_rounds_down_below_half_sat() {
        let divisor = U256::from(WEI_PER_SAT_U64);
        let wei = divisor
            .saturating_mul(U256::from(123u64))
            .saturating_add(U256::from(1u64));
        assert_eq!(wei_to_sats_round(wei), U256::from(123u64));
    }

    #[test]
    fn wei_to_sats_round_rounds_up_at_half_sat() {
        let divisor = U256::from(WEI_PER_SAT_U64);
        let half = divisor / U256::from(2u64);
        let wei = divisor
            .saturating_mul(U256::from(123u64))
            .saturating_add(half);
        assert_eq!(wei_to_sats_round(wei), U256::from(124u64));
    }

    #[test]
    fn adjust_submitted_sats_for_fee_envelope_returns_adjusted_amount() {
        let submitted = U256::from(1_000_000u64);
        let fee_wei = sats_to_wei(U256::from(900_000u64)).saturating_add(U256::from(1u64));
        let adjusted =
            adjust_submitted_sats_for_fee_envelope(submitted, fee_wei).expect("should adjust");
        assert_eq!(adjusted, U256::from(100_000u64));
    }

    #[test]
    fn adjust_submitted_sats_for_fee_envelope_rejects_non_positive_adjustment() {
        let submitted = U256::from(100_000u64);
        let fee_wei = sats_to_wei(U256::from(100_000u64)).saturating_add(U256::from(1u64));
        let adjusted = adjust_submitted_sats_for_fee_envelope(submitted, fee_wei);
        assert!(adjusted.is_none());
    }

    #[test]
    fn extract_currency_ref_accepts_string_entries() {
        assert_eq!(
            extract_currency_ref(&json!("i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X")).as_deref(),
            Some("i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X")
        );
    }
}
