//
// Advanced VRPC transfer preflight (reserve-transfer/sendcurrency family).
// Builds tx templates server-side and stores signing payload by preflight_id.

use std::io::Cursor;
use std::time::Duration;

use bitcoin::consensus::Decodable;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::core::channels::store::{PreflightRecord, PreflightStore};
use crate::core::channels::vrpc::identity::verus_tx::codec::decode_hex as decode_verus_tx;
use crate::core::channels::vrpc::identity::verus_tx::model::txid_le_bytes_to_hex;
use crate::core::channels::vrpc::preflight::{VrpcInputRef, VrpcPreflightPayload};
use crate::core::channels::vrpc::provider::VrpcProvider;
use crate::types::transaction::PreflightWarning;
use crate::types::{VrpcTransferPreflightParams, VrpcTransferPreflightResult, WalletError};

const SATOSHIS_PER_COIN: i64 = 100_000_000;
const DEFAULT_PARENT_FEE_LOW: f64 = 0.0001;
const DEFAULT_PARENT_FEE_LOW_SAT: i64 = 10_000;
const DEFAULT_PARENT_FEE_HIGH: f64 = 0.0002;
const DEFAULT_PARENT_FEE_HIGH_SAT: i64 = 20_000;
const DEFAULT_NATIVE_CONVERSION_FEE_SAT: i64 = 25_000;
const TRANSFER_PREFLIGHT_TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone)]
struct FundingUtxo {
    txid: String,
    vout: u32,
    satoshis: i64,
    script_pub_key: String,
}

fn parse_i64(value: Option<&Value>) -> Option<i64> {
    let v = value?;
    if let Some(x) = v.as_i64() {
        return Some(x);
    }
    if let Some(x) = v.as_u64() {
        return i64::try_from(x).ok();
    }
    if let Some(x) = v.as_f64() {
        return Some(x as i64);
    }
    if let Some(x) = v.as_str() {
        if let Ok(parsed) = x.parse::<i64>() {
            return Some(parsed);
        }
    }
    None
}

fn parse_u32(value: Option<&Value>) -> Option<u32> {
    parse_i64(value).and_then(|x| (x >= 0).then_some(x as u32))
}

fn parse_string(value: Option<&Value>) -> Option<String> {
    value?.as_str().map(ToString::to_string)
}

fn parse_sendcurrency_hex(raw: Value) -> Result<String, WalletError> {
    if let Some(hex) = raw.as_str() {
        return Ok(hex.to_string());
    }
    if let Some(hex) = parse_string(raw.get("hextx").or(raw.get("hex")).or(raw.get("txhex"))) {
        return Ok(hex);
    }
    Err(WalletError::OperationFailed)
}

fn parse_fee_sat(value: Option<&Value>, fallback_sat: i64) -> i64 {
    let fallback = fallback_sat.max(1);
    let normalize = |candidate: i64| if candidate > 0 { candidate } else { fallback };

    let Some(v) = value else {
        return fallback;
    };

    if let Some(raw_sat) = v.as_i64() {
        return normalize(raw_sat);
    }
    if let Some(raw_sat) = v.as_u64() {
        return i64::try_from(raw_sat).map(normalize).unwrap_or(fallback);
    }
    if let Some(raw_coin) = v.as_f64() {
        let raw_sat = ((raw_coin.max(0.0)) * SATOSHIS_PER_COIN as f64).round() as i64;
        return normalize(raw_sat);
    }
    if let Some(raw_str) = v.as_str() {
        let trimmed = raw_str.trim();
        if trimmed.contains('.') || trimmed.contains('e') || trimmed.contains('E') {
            if let Ok(raw_coin) = trimmed.parse::<f64>() {
                let raw_sat = ((raw_coin.max(0.0)) * SATOSHIS_PER_COIN as f64).round() as i64;
                return normalize(raw_sat);
            }
            return fallback;
        }
        if let Ok(raw_sat) = trimmed.parse::<i64>() {
            return normalize(raw_sat);
        }
    }

    fallback
}

fn parse_fund_result(raw: Value, fallback_fee_sat: i64) -> Result<(String, i64), WalletError> {
    let hex = parse_string(raw.get("hex")).ok_or(WalletError::OperationFailed)?;
    let fee_sat = parse_fee_sat(raw.get("fee"), fallback_fee_sat);
    Ok((hex, fee_sat))
}

fn parse_coin_value_sat(value: &Value) -> Option<i64> {
    if let Some(raw_sat) = value.as_i64() {
        return Some(raw_sat.max(0));
    }
    if let Some(raw_sat) = value.as_u64() {
        return i64::try_from(raw_sat).ok();
    }
    if let Some(raw_coin) = value.as_f64() {
        let sat = (raw_coin.max(0.0) * SATOSHIS_PER_COIN as f64).round() as i64;
        return Some(sat);
    }
    value.as_str().and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        if trimmed.contains('.') || trimmed.contains('e') || trimmed.contains('E') {
            let coin = trimmed.parse::<f64>().ok()?;
            let sat = (coin.max(0.0) * SATOSHIS_PER_COIN as f64).round() as i64;
            Some(sat)
        } else {
            trimmed.parse::<i64>().ok().map(|sat| sat.max(0))
        }
    })
}

fn parse_funding_utxos(raw: &Value) -> Vec<FundingUtxo> {
    let Some(arr) = raw.as_array() else {
        return vec![];
    };

    arr.iter()
        .filter_map(|entry| {
            let txid = parse_string(entry.get("txid").or(entry.get("outputTxId")))?;
            let vout = parse_u32(entry.get("outputIndex").or(entry.get("vout")))?;
            let satoshis = parse_i64(entry.get("satoshis").or(entry.get("amount"))).unwrap_or(0);
            let script_pub_key = parse_string(entry.get("script").or(entry.get("scriptPubKey")))?;
            let is_spendable = parse_i64(entry.get("isspendable")).unwrap_or(1) != 0;
            if !is_spendable {
                return None;
            }
            Some(FundingUtxo {
                txid,
                vout,
                satoshis,
                script_pub_key,
            })
        })
        .collect()
}

fn sat_to_decimal_string(sat: i64) -> String {
    format!("{:.8}", sat as f64 / SATOSHIS_PER_COIN as f64)
}

fn parse_amount_sat(amount: &str) -> Result<i64, WalletError> {
    let parsed = amount
        .trim()
        .parse::<f64>()
        .map_err(|_| WalletError::OperationFailed)?;
    let sat = (parsed * SATOSHIS_PER_COIN as f64).round() as i64;
    if sat <= 0 {
        return Err(WalletError::OperationFailed);
    }
    Ok(sat)
}

fn parse_system_id_from_channel_id(channel_id: &str) -> Option<String> {
    let (_, system_id) = channel_id.rsplit_once('.')?;
    if system_id.trim().is_empty() {
        return None;
    }
    Some(system_id.to_string())
}

async fn resolve_currency_id(provider: &VrpcProvider, currency: &str) -> Option<String> {
    let resolved = provider.getcurrency(currency).await.ok()?;
    parse_string(resolved.get("currencyid").or(resolved.get("currencyId")))
}

fn is_known_native_symbol(currency: &str) -> bool {
    matches!(
        currency.trim().to_ascii_uppercase().as_str(),
        "VRSC" | "VRSCTEST"
    )
}

fn total_available_satoshis(utxos: &[FundingUtxo]) -> i64 {
    utxos
        .iter()
        .fold(0i64, |acc, utxo| acc.saturating_add(utxo.satoshis))
}

fn resolve_send_value_for_native_fee(
    submitted_sat: i64,
    available_sat: i64,
    fee_sat: i64,
) -> Result<(i64, bool), WalletError> {
    if available_sat <= fee_sat {
        return Err(WalletError::InsufficientFunds);
    }
    let max_sendable = available_sat.saturating_sub(fee_sat);
    if max_sendable <= 0 {
        return Err(WalletError::InsufficientFunds);
    }
    if submitted_sat <= max_sendable {
        return Ok((submitted_sat, false));
    }
    Ok((max_sendable, true))
}

fn is_same_currency_ref(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}

fn parent_fee_for_route(is_conversion_or_export: bool, source_is_native: bool) -> (f64, i64) {
    let use_low_fee = is_conversion_or_export || source_is_native;
    if use_low_fee {
        (DEFAULT_PARENT_FEE_LOW, DEFAULT_PARENT_FEE_LOW_SAT)
    } else {
        (DEFAULT_PARENT_FEE_HIGH, DEFAULT_PARENT_FEE_HIGH_SAT)
    }
}

fn parse_outputtotals_fee_sat(raw: &Value, fee_currency_refs: &[String]) -> Option<i64> {
    let totals = raw.get("outputtotals")?.as_object()?;
    for fee_ref in fee_currency_refs {
        if fee_ref.trim().is_empty() {
            continue;
        }
        if let Some((_, value)) = totals
            .iter()
            .find(|(key, _)| key.trim().eq_ignore_ascii_case(fee_ref))
        {
            let parsed = parse_coin_value_sat(value)?;
            if parsed > 0 {
                return Some(parsed);
            }
        }
    }
    None
}

async fn resolve_effective_fee_currency_id(
    provider: &VrpcProvider,
    params: &VrpcTransferPreflightParams,
    system_id: Option<&str>,
    is_conversion_or_export: bool,
) -> Option<String> {
    let user_selected = params
        .fee_currency
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if let Some(currency) = user_selected {
        return resolve_currency_id(provider, currency)
            .await
            .or_else(|| Some(currency.to_string()));
    }

    if is_conversion_or_export {
        return system_id.map(ToString::to_string);
    }

    None
}

async fn estimate_transfer_fee_satoshis(
    provider: &VrpcProvider,
    from_address: &str,
    params: &VrpcTransferPreflightParams,
    normalized_destination: &str,
    parent_fee_coin: f64,
    effective_fee_currency_id: Option<&str>,
    system_id: Option<&str>,
    is_conversion_or_export: bool,
) -> Result<i64, WalletError> {
    let explicit_fee_satoshis = params
        .fee_satoshis
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(raw) = explicit_fee_satoshis {
        let parsed = raw
            .parse::<i64>()
            .map_err(|_| WalletError::OperationFailed)?;
        if parsed <= 0 {
            return Err(WalletError::OperationFailed);
        }
        return Ok(parsed);
    }

    if !is_conversion_or_export {
        return Ok(0);
    }

    let fee_currency_id = effective_fee_currency_id
        .map(ToString::to_string)
        .or_else(|| system_id.map(ToString::to_string));
    let fee_currency_refs = {
        let mut refs = Vec::new();
        if let Some(ref_id) = fee_currency_id.as_deref() {
            refs.push(ref_id.to_string());
        }
        if let Some(raw_fee_currency) = params
            .fee_currency
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if !refs
                .iter()
                .any(|existing| is_same_currency_ref(existing, raw_fee_currency))
            {
                refs.push(raw_fee_currency.to_string());
            }
        }
        refs
    };

    if let (Some(system), Some(fee_currency)) = (system_id, fee_currency_id.as_deref()) {
        if params.export_to.is_none() && is_same_currency_ref(system, fee_currency) {
            return Ok(DEFAULT_NATIVE_CONVERSION_FEE_SAT);
        }
    }

    let mut probe_output = build_sendcurrency_output(params, normalized_destination, 0.0)?;
    if let Some(fee_currency) = fee_currency_id {
        if let Some(output_obj) = probe_output.as_object_mut() {
            output_obj.insert("feecurrency".to_string(), Value::String(fee_currency));
        }
    }

    let sendcurrency_probe = provider
        .sendcurrency(from_address, &[probe_output], 1, parent_fee_coin, true)
        .await?;

    if fee_currency_refs.is_empty() {
        return Ok(0);
    }

    parse_outputtotals_fee_sat(&sendcurrency_probe, &fee_currency_refs)
        .ok_or(WalletError::OperationFailed)
}

async fn normalize_destination(
    provider: &VrpcProvider,
    destination: &str,
) -> Result<(String, Vec<PreflightWarning>), WalletError> {
    let trimmed = destination.trim();
    if trimmed.is_empty() {
        return Err(WalletError::InvalidAddress);
    }
    if trimmed.ends_with('@') {
        let raw = provider.getidentity(trimmed).await?;
        let identity_addr = parse_string(
            raw.get("identity")
                .and_then(|id| id.get("identityaddress"))
                .or(raw.get("identityaddress")),
        )
        .ok_or(WalletError::InvalidAddress)?;

        let warning = PreflightWarning {
            warning_type: "resolved_destination".to_string(),
            message: format!(
                "Destination handle {} resolved to {}.",
                trimmed, identity_addr
            ),
        };
        return Ok((identity_addr, vec![warning]));
    }

    Ok((trimmed.to_string(), Vec::new()))
}

fn build_sendcurrency_output(
    params: &VrpcTransferPreflightParams,
    normalized_destination: &str,
    send_amount: f64,
) -> Result<Value, WalletError> {
    let mut out = Map::<String, Value>::new();
    out.insert(
        "currency".to_string(),
        Value::String(params.coin_id.clone()),
    );
    out.insert("amount".to_string(), Value::from(send_amount));
    out.insert(
        "address".to_string(),
        Value::String(normalized_destination.to_string()),
    );

    if let Some(v) = &params.convert_to {
        out.insert("convertto".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = &params.export_to {
        out.insert("exportto".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = &params.via {
        out.insert("via".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = &params.fee_currency {
        out.insert("feecurrency".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = &params.fee_satoshis {
        out.insert("feesatoshis".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = params.preconvert {
        out.insert("preconvert".to_string(), Value::Bool(v));
    }
    if let Some(v) = &params.map_to {
        if destination_supports_map_to(normalized_destination) {
            out.insert("mapto".to_string(), Value::String(v.clone()));
        }
    }
    if let Some(v) = &params.vdxf_tag {
        out.insert("vdxftag".to_string(), Value::String(v.clone()));
    }

    Ok(Value::Object(out))
}

fn destination_supports_map_to(normalized_destination: &str) -> bool {
    !is_eth_hex_destination(normalized_destination)
}

fn is_eth_hex_destination(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.len() == 42
        && trimmed.starts_with("0x")
        && trimmed
            .as_bytes()
            .iter()
            .skip(2)
            .all(|byte| byte.is_ascii_hexdigit())
}

fn parse_funded_input_refs(funded_hex: &str) -> Result<Vec<(String, u32)>, WalletError> {
    if let Ok(verus_tx) = decode_verus_tx(funded_hex) {
        let refs = verus_tx
            .inputs
            .iter()
            .map(|input| {
                (
                    txid_le_bytes_to_hex(&input.prevout_txid_le),
                    input.prevout_vout,
                )
            })
            .collect::<Vec<_>>();
        if !refs.is_empty() {
            return Ok(refs);
        }
    }

    let raw = hex::decode(funded_hex.trim_start_matches("0x"))
        .or_else(|_| hex::decode(funded_hex))
        .map_err(|_| WalletError::OperationFailed)?;
    let mut cursor = Cursor::new(&raw[..]);
    let funded_tx: bitcoin::Transaction = bitcoin::Transaction::consensus_decode(&mut cursor)
        .map_err(|_| WalletError::OperationFailed)?;
    Ok(funded_tx
        .input
        .into_iter()
        .map(|input| {
            (
                input.previous_output.txid.to_string(),
                input.previous_output.vout,
            )
        })
        .collect())
}

fn collect_payload_inputs(
    funded_hex: &str,
    available_utxos: &[FundingUtxo],
) -> Result<Vec<VrpcInputRef>, WalletError> {
    let funded_inputs = parse_funded_input_refs(funded_hex)?;

    let mut payload_inputs = Vec::new();
    for (in_hash, in_vout) in funded_inputs {
        let utxo = available_utxos
            .iter()
            .find(|u| u.txid == in_hash && u.vout == in_vout)
            .ok_or(WalletError::OperationFailed)?;
        payload_inputs.push(VrpcInputRef {
            txid: utxo.txid.clone(),
            vout: utxo.vout,
            satoshis: utxo.satoshis,
            script_pub_key: Some(utxo.script_pub_key.clone()),
        });
    }
    Ok(payload_inputs)
}

pub async fn preflight_transfer(
    params: VrpcTransferPreflightParams,
    preflight_store: &PreflightStore,
    account_id: &str,
    session_id: &str,
    from_address: &str,
    channel_id: &str,
    provider: &VrpcProvider,
) -> Result<VrpcTransferPreflightResult, WalletError> {
    let submitted_sat = parse_amount_sat(&params.amount)?;
    let (normalized_destination, mut warnings) =
        normalize_destination(provider, &params.destination).await?;

    let available_utxos = parse_funding_utxos(
        &provider
            .getaddressutxos(&[from_address.to_string()])
            .await?,
    );
    if available_utxos.is_empty() {
        return Err(WalletError::InsufficientFunds);
    }

    let system_id = parse_system_id_from_channel_id(channel_id);
    let source_currency_id = resolve_currency_id(provider, &params.coin_id).await;
    let source_is_native = system_id
        .as_deref()
        .map(|system| {
            source_currency_id
                .as_deref()
                .map(|currency_id| currency_id == system)
                .unwrap_or(false)
                || (source_currency_id.is_none() && is_known_native_symbol(&params.coin_id))
        })
        .unwrap_or(false);

    let is_conversion_or_export = params.convert_to.is_some() || params.export_to.is_some();
    let (parent_fee_coin, parent_fee_sat) =
        parent_fee_for_route(is_conversion_or_export, source_is_native);
    let effective_fee_currency_id = resolve_effective_fee_currency_id(
        provider,
        &params,
        system_id.as_deref(),
        is_conversion_or_export,
    )
    .await;
    let transfer_fee_sat = estimate_transfer_fee_satoshis(
        provider,
        from_address,
        &params,
        &normalized_destination,
        parent_fee_coin,
        effective_fee_currency_id.as_deref(),
        system_id.as_deref(),
        is_conversion_or_export,
    )
    .await?;
    let native_required_fee_sat = if source_is_native
        && system_id.is_some()
        && effective_fee_currency_id
            .as_deref()
            .zip(system_id.as_deref())
            .map(|(fee_currency, system)| is_same_currency_ref(fee_currency, system))
            .unwrap_or(false)
    {
        parent_fee_sat.saturating_add(transfer_fee_sat)
    } else {
        parent_fee_sat
    };

    let available_sat = total_available_satoshis(&available_utxos);
    let (send_value_sat, amount_was_adjusted) = if source_is_native {
        resolve_send_value_for_native_fee(submitted_sat, available_sat, native_required_fee_sat)?
    } else {
        (submitted_sat, false)
    };
    let send_amount = send_value_sat as f64 / SATOSHIS_PER_COIN as f64;
    let amount_adjusted = amount_was_adjusted.then(|| sat_to_decimal_string(send_value_sat));

    let mut output = build_sendcurrency_output(&params, &normalized_destination, send_amount)?;
    if let Some(fee_currency) = effective_fee_currency_id.clone() {
        if let Some(output_obj) = output.as_object_mut() {
            output_obj.insert("feecurrency".to_string(), Value::String(fee_currency));
        }
    }

    let sendcurrency_result = provider
        .sendcurrency(from_address, &[output], 1, parent_fee_coin, true)
        .await?;
    let unfunded_hex = parse_sendcurrency_hex(sendcurrency_result)?;

    let funding_utxos: Vec<Value> = available_utxos
        .iter()
        .map(|utxo| serde_json::json!({"txid": utxo.txid, "voutnum": utxo.vout}))
        .collect();
    let funded_raw = provider
        .fundrawtransaction_with_options(
            &unfunded_hex,
            Some(&funding_utxos),
            Some(from_address),
            Some(parent_fee_coin),
        )
        .await?;
    let (funded_hex, fee_sat) = parse_fund_result(funded_raw, parent_fee_sat)?;
    let payload_inputs = collect_payload_inputs(&funded_hex, &available_utxos)?;
    if payload_inputs.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    if params.convert_to.is_some() {
        warnings.push(PreflightWarning {
            warning_type: "estimated_fee".to_string(),
            message: "Final amount you receive may vary slightly.".to_string(),
        });
    }

    let preflight_id = Uuid::new_v4().to_string();
    let payload = VrpcPreflightPayload {
        hex: funded_hex,
        inputs: payload_inputs,
        to_address: normalized_destination.clone(),
        from_address: from_address.to_string(),
        value: sat_to_decimal_string(send_value_sat),
        fee: sat_to_decimal_string(fee_sat),
    };
    let payload_value = serde_json::to_value(&payload).map_err(|_| WalletError::OperationFailed)?;

    if !preflight_store.put_with_ttl(
        preflight_id.clone(),
        PreflightRecord {
            session_id: session_id.to_string(),
            channel_id: channel_id.to_string(),
            account_id: account_id.to_string(),
            payload: payload_value,
        },
        Some(TRANSFER_PREFLIGHT_TTL),
    ) {
        return Err(WalletError::WalletLocked);
    }

    Ok(VrpcTransferPreflightResult {
        preflight_id,
        fee: sat_to_decimal_string(fee_sat),
        fee_currency: params.coin_id.clone(),
        value: sat_to_decimal_string(send_value_sat),
        amount_submitted: params.amount,
        amount_adjusted,
        to_address: normalized_destination,
        from_address: from_address.to_string(),
        warnings,
        memo: params.memo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn base_params() -> VrpcTransferPreflightParams {
        VrpcTransferPreflightParams {
            coin_id: "VRSC".to_string(),
            channel_id: "vrpc.Rabc.i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            source_address: None,
            destination: "Rdest".to_string(),
            amount: "1.0".to_string(),
            convert_to: None,
            export_to: None,
            via: None,
            fee_currency: None,
            fee_satoshis: None,
            preconvert: None,
            map_to: None,
            vdxf_tag: None,
            memo: None,
        }
    }

    #[test]
    fn build_sendcurrency_output_includes_optional_route_flags() {
        let mut params = base_params();
        params.convert_to = Some("Bridge.CHIPS".to_string());
        params.export_to = Some("CHIPS".to_string());
        params.via = Some("Bridge.vETH".to_string());
        params.fee_currency = Some("i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string());
        params.fee_satoshis = Some("20000".to_string());
        params.preconvert = Some(true);
        params.map_to = Some("Bridge.vETH".to_string());
        params.vdxf_tag = Some("iTag".to_string());

        let output = build_sendcurrency_output(&params, "Rdest", 1.0).expect("output");
        assert_eq!(
            output,
            json!({
                "currency": "VRSC",
                "amount": 1.0,
                "address": "Rdest",
                "convertto": "Bridge.CHIPS",
                "exportto": "CHIPS",
                "via": "Bridge.vETH",
                "feecurrency": "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV",
                "feesatoshis": "20000",
                "preconvert": true,
                "mapto": "Bridge.vETH",
                "vdxftag": "iTag"
            })
        );
    }

    #[test]
    fn build_sendcurrency_output_omits_mapto_for_eth_destination() {
        let mut params = base_params();
        params.export_to = Some("i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X".to_string());
        params.map_to = Some("i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd".to_string());

        let output =
            build_sendcurrency_output(&params, "0x8fda30a676fbc8f1406adeac7921998b1af4fd05", 1.0)
                .expect("output");
        let mapto = output.get("mapto");
        assert!(
            mapto.is_none(),
            "mapto must be omitted for ETH destination outputs"
        );
    }

    #[test]
    fn parse_sendcurrency_hex_supports_multiple_result_shapes() {
        assert_eq!(
            parse_sendcurrency_hex(json!("deadbeef")).expect("string shape"),
            "deadbeef"
        );
        assert_eq!(
            parse_sendcurrency_hex(json!({"hextx": "cafe"})).expect("hextx shape"),
            "cafe"
        );
        assert_eq!(
            parse_sendcurrency_hex(json!({"hex": "babe"})).expect("hex shape"),
            "babe"
        );
    }

    #[test]
    fn parse_fund_result_uses_default_fee_when_non_positive_reported() {
        let (hex, fee_sat) = parse_fund_result(
            json!({"hex": "deadbeef", "fee": 0}),
            DEFAULT_PARENT_FEE_LOW_SAT,
        )
        .expect("parse");
        assert_eq!(hex, "deadbeef");
        assert_eq!(fee_sat, DEFAULT_PARENT_FEE_LOW_SAT);
    }

    #[test]
    fn parse_fund_result_parses_decimal_coin_fee() {
        let (_, fee_sat) = parse_fund_result(
            json!({"hex": "deadbeef", "fee": 0.0001}),
            DEFAULT_PARENT_FEE_LOW_SAT,
        )
        .expect("parse");
        assert_eq!(fee_sat, DEFAULT_PARENT_FEE_LOW_SAT);
    }

    #[test]
    fn resolve_send_value_for_native_fee_keeps_submitted_when_fee_headroom_exists() {
        let (value, adjusted) =
            resolve_send_value_for_native_fee(100_000, 150_000, DEFAULT_PARENT_FEE_LOW_SAT)
                .expect("resolve");
        assert_eq!(value, 100_000);
        assert!(!adjusted);
    }

    #[test]
    fn resolve_send_value_for_native_fee_adjusts_when_submitted_equals_available() {
        let (value, adjusted) =
            resolve_send_value_for_native_fee(100_000, 100_000, 10_000).expect("resolve");
        assert_eq!(value, 90_000);
        assert!(adjusted);
    }

    #[test]
    fn resolve_send_value_for_native_fee_returns_insufficient_when_fee_unfundable() {
        let result = resolve_send_value_for_native_fee(100_000, 10_000, 10_000);
        assert!(matches!(result, Err(WalletError::InsufficientFunds)));
    }
}
