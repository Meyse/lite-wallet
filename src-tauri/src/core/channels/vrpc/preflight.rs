//
// Module 5: VRPC preflight — validate address, build/fund tx, store record (incl. to/from/value/fee for send), return PreflightResult.

use bitcoin::consensus::Decodable;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::io::Cursor;
use uuid::Uuid;

use crate::core::channels::store::{PreflightRecord, PreflightStore};
use crate::core::channels::vrpc::identity::verus_tx::codec::decode_hex as decode_verus_tx;
use crate::core::channels::vrpc::identity::verus_tx::model::txid_le_bytes_to_hex;
use crate::core::channels::vrpc::provider::VrpcProvider;
use crate::types::transaction::{PreflightParams, PreflightResult};
use crate::types::WalletError;

const SATOSHIS_PER_COIN: i64 = 100_000_000;
const DEFAULT_FEE_ESTIMATE_SAT: i64 = 10_000;

/// Payload stored in PreflightStore for VRPC send. Not sent to frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrpcPreflightPayload {
    pub hex: String,
    #[serde(rename = "inputs")]
    pub inputs: Vec<VrpcInputRef>,
    pub to_address: String,
    pub from_address: String,
    pub value: String,
    pub fee: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrpcInputRef {
    pub txid: String,
    pub vout: u32,
    pub satoshis: i64,
    #[serde(default)]
    pub script_pub_key: Option<String>,
}

/// Minimal address check: non-empty, reasonable length.
fn validate_address(addr: &str) -> Result<(), WalletError> {
    let trimmed = addr.trim();
    if trimmed.is_empty() || trimmed.len() > 200 {
        return Err(WalletError::InvalidAddress);
    }
    // R-address, i-address, or VerusID: allow alphanumeric and @.
    if !trimmed
        .chars()
        .all(|c| c.is_alphanumeric() || c == '@' || c == '.' || c == '_' || c == '-')
    {
        return Err(WalletError::InvalidAddress);
    }
    Ok(())
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

fn resolve_send_value(
    submitted_sat: i64,
    total_input_sat: i64,
    fee_sat: i64,
) -> Result<(i64, bool, Option<String>), WalletError> {
    if total_input_sat <= fee_sat {
        return Err(WalletError::InsufficientFunds);
    }

    let needed = submitted_sat.saturating_add(fee_sat);
    if total_input_sat >= needed {
        return Ok((submitted_sat, false, None));
    }

    let adjusted = total_input_sat.saturating_sub(fee_sat);
    if adjusted <= 0 {
        return Err(WalletError::InsufficientFunds);
    }

    Ok((
        adjusted,
        true,
        Some("Fee was deducted from the submitted amount due to available balance.".to_string()),
    ))
}

fn collect_payload_inputs_from_funded_tx(
    funded_hex: &str,
    candidates: &[(String, u32, i64, Option<String>)],
) -> Result<Vec<VrpcInputRef>, WalletError> {
    let funded_inputs = parse_funded_input_refs(funded_hex)?;

    let mut seen = HashSet::<(String, u32)>::new();
    let mut out = Vec::with_capacity(funded_inputs.len());
    for (txid, vout) in funded_inputs {
        if !seen.insert((txid.clone(), vout)) {
            return Err(WalletError::OperationFailed);
        }
        let Some((_, _, satoshis, script_pub_key)) = candidates
            .iter()
            .find(|(c_txid, c_vout, _, _)| c_txid == &txid && *c_vout == vout)
        else {
            println!(
                "[VRPC][preflight] funded input missing from candidate utxos: txid={} vout={} candidate_count={}",
                txid,
                vout,
                candidates.len()
            );
            return Err(WalletError::OperationFailed);
        };

        out.push(VrpcInputRef {
            txid,
            vout,
            satoshis: *satoshis,
            script_pub_key: script_pub_key.clone(),
        });
    }

    if out.is_empty() {
        return Err(WalletError::OperationFailed);
    }
    Ok(out)
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
        .map_err(|err| {
            println!(
                "[VRPC][preflight] failed to decode funded hex bytes for input extraction: {}",
                err
            );
            WalletError::OperationFailed
        })?;
    let mut cursor = Cursor::new(&raw[..]);
    let funded_tx: bitcoin::Transaction = bitcoin::Transaction::consensus_decode(&mut cursor)
        .map_err(|err| {
            println!(
                "[VRPC][preflight] failed to decode funded transaction with bitcoin parser for input extraction: {}",
                err
            );
            WalletError::OperationFailed
        })?;
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

/// Run VRPC preflight: validate, build/fund tx, store record, return UI result.
pub async fn preflight(
    params: PreflightParams,
    preflight_store: &PreflightStore,
    account_id: &str,
    session_id: &str,
    from_address: &str,
    channel_id: &str,
    provider: &VrpcProvider,
) -> Result<PreflightResult, WalletError> {
    validate_address(&params.to_address)?;
    validate_address(from_address)?;

    let submitted_sat = parse_amount_sat(&params.amount)?;

    let addresses = vec![from_address.to_string()];
    let utxos_raw = provider.getaddressutxos(&addresses).await?;

    let utxos: Vec<(String, u32, i64, Option<String>)> = parse_utxos(&utxos_raw)?;
    if utxos.is_empty() {
        return Err(WalletError::InsufficientFunds);
    }

    let fee_estimate = DEFAULT_FEE_ESTIMATE_SAT;
    let needed = submitted_sat.saturating_add(fee_estimate);
    let mut selected: Vec<(String, u32, i64, Option<String>)> = Vec::new();
    let mut total: i64 = 0;
    for (txid, vout, satoshis, script_pub_key) in utxos {
        selected.push((txid.clone(), vout, satoshis, script_pub_key));
        total = total.saturating_add(satoshis);
        if total >= needed {
            break;
        }
    }

    let (send_value_sat, fee_taken_from_amount, fee_taken_message) =
        resolve_send_value(submitted_sat, total, fee_estimate)?;

    let amount_vrsc = send_value_sat as f64 / SATOSHIS_PER_COIN as f64;
    let outputs = serde_json::json!({ params.to_address.clone(): amount_vrsc });

    // Keep the tx unfunded at this stage. Funding (with chosen UTXOs + fee) is done via fundrawtransaction.
    let raw_inputs: Vec<Value> = Vec::new();
    let hex_unfunded = provider
        .createrawtransaction(&raw_inputs, &outputs)
        .await?
        .as_str()
        .ok_or(WalletError::OperationFailed)?
        .to_string();

    let funding_utxos: Vec<Value> = selected
        .iter()
        .map(|(txid, vout, _, _)| serde_json::json!({"txid": txid, "voutnum": *vout}))
        .collect();
    let explicit_fee = fee_estimate as f64 / SATOSHIS_PER_COIN as f64;
    let funded = provider
        .fundrawtransaction_with_options(
            &hex_unfunded,
            Some(&funding_utxos),
            Some(from_address),
            Some(explicit_fee),
        )
        .await?;
    let funded_hex = funded
        .get("hex")
        .and_then(|v| v.as_str())
        .ok_or(WalletError::OperationFailed)?
        .to_string();
    let fee_sat = parse_fee_sat(funded.get("fee"), fee_estimate);
    let payload_inputs = collect_payload_inputs_from_funded_tx(&funded_hex, &selected)?;

    let preflight_id = Uuid::new_v4().to_string();
    let fee_str = format!("{:.8}", fee_sat as f64 / SATOSHIS_PER_COIN as f64);
    let value_str = format!("{:.8}", send_value_sat as f64 / SATOSHIS_PER_COIN as f64);
    let payload = VrpcPreflightPayload {
        hex: funded_hex,
        inputs: payload_inputs,
        to_address: params.to_address.clone(),
        from_address: from_address.to_string(),
        value: value_str.clone(),
        fee: fee_str.clone(),
    };
    let payload_value = serde_json::to_value(&payload).map_err(|_| WalletError::OperationFailed)?;

    let record = PreflightRecord {
        session_id: session_id.to_string(),
        channel_id: channel_id.to_string(),
        account_id: account_id.to_string(),
        payload: payload_value,
    };
    if !preflight_store.put(preflight_id.clone(), record) {
        return Err(WalletError::WalletLocked);
    }

    Ok(PreflightResult {
        preflight_id,
        fee: fee_str,
        fee_currency: params.coin_id.clone(),
        value: value_str.clone(),
        amount_submitted: params.amount,
        to_address: params.to_address.clone(),
        from_address: from_address.to_string(),
        fee_taken_from_amount,
        fee_taken_message,
        warnings: vec![],
        memo: params.memo,
    })
}

fn parse_utxos(raw: &Value) -> Result<Vec<(String, u32, i64, Option<String>)>, WalletError> {
    let arr = raw.as_array().ok_or(WalletError::OperationFailed)?;
    let mut out = Vec::new();
    for entry in arr {
        let obj = entry.as_object().ok_or(WalletError::OperationFailed)?;
        let txid = obj
            .get("txid")
            .or(obj.get("outputTxId"))
            .and_then(|v| v.as_str())
            .ok_or(WalletError::OperationFailed)?
            .to_string();
        let vout = obj
            .get("vout")
            .or(obj.get("outputIndex"))
            .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)))
            .ok_or(WalletError::OperationFailed)? as u32;
        let satoshis = obj
            .get("satoshis")
            .or(obj.get("amount"))
            .and_then(|v| parse_i64(Some(v)))
            .unwrap_or(0);
        if satoshis <= 0 {
            continue;
        }
        let script_pub_key = obj
            .get("script")
            .or(obj.get("scriptPubKey"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string);
        out.push((txid, vout, satoshis, script_pub_key));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolve_send_value_keeps_submitted_when_funds_cover_fee() {
        let submitted = 100_000;
        let fee = 10_000;
        let (send_value, adjusted, message) =
            resolve_send_value(submitted, submitted + fee + 1, fee).expect("resolve");
        assert_eq!(send_value, submitted);
        assert!(!adjusted);
        assert!(message.is_none());
    }

    #[test]
    fn resolve_send_value_adjusts_when_fee_must_be_deducted() {
        let submitted = 100_000;
        let fee = 10_000;
        let (send_value, adjusted, message) =
            resolve_send_value(submitted, submitted, fee).expect("resolve");
        assert_eq!(send_value, 90_000);
        assert!(adjusted);
        assert!(message.is_some());
    }

    #[test]
    fn parse_utxos_accepts_scriptpubkey_and_skips_zero_value() {
        let raw = json!([
            {
                "txid": "abc",
                "vout": 0,
                "satoshis": 0,
                "scriptPubKey": "76a914000000000000000000000000000000000000000088ac"
            },
            {
                "txid": "def",
                "vout": 1,
                "satoshis": 12000,
                "scriptPubKey": "76a914111111111111111111111111111111111111111188ac"
            }
        ]);
        let parsed = parse_utxos(&raw).expect("parse utxos");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, "def");
        assert_eq!(parsed[0].1, 1);
        assert_eq!(parsed[0].2, 12000);
        assert_eq!(
            parsed[0].3.as_deref(),
            Some("76a914111111111111111111111111111111111111111188ac")
        );
    }

    #[test]
    fn parse_fee_sat_uses_fallback_when_rpc_reports_non_positive_fee() {
        assert_eq!(
            parse_fee_sat(Some(&json!(0)), DEFAULT_FEE_ESTIMATE_SAT),
            DEFAULT_FEE_ESTIMATE_SAT
        );
        assert_eq!(
            parse_fee_sat(Some(&json!("0.00000000")), DEFAULT_FEE_ESTIMATE_SAT),
            DEFAULT_FEE_ESTIMATE_SAT
        );
    }

    #[test]
    fn parse_fee_sat_parses_decimal_coin_fee() {
        assert_eq!(
            parse_fee_sat(Some(&json!(0.0001)), DEFAULT_FEE_ESTIMATE_SAT),
            10_000
        );
    }
}
