//
// Module 5: VRPC preflight — validate address, build/fund tx, store record (incl. to/from/value/fee for send), return PreflightResult.

use serde_json::Value;
use uuid::Uuid;

use crate::core::channels::store::{PreflightRecord, PreflightStore};
use crate::core::channels::vrpc::common::{
    collect_payload_inputs, parse_fee_sat, parse_positive_amount_sat, parse_utxo_entry,
    sat_to_decimal_string, VrpcPreflightPayload, VrpcUtxo, SATOSHIS_PER_COIN,
};
use crate::core::channels::vrpc::provider::VrpcProvider;
use crate::types::transaction::{PreflightParams, PreflightResult};
use crate::types::WalletError;

const DEFAULT_FEE_ESTIMATE_SAT: i64 = 10_000;

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

/// Run VRPC preflight: validate, build/fund tx, store record, return UI result.
pub async fn preflight(
    params: PreflightParams,
    preflight_store: &PreflightStore,
    account_id: &str,
    session_id: &str,
    from_address: &str,
    channel_id: &str,
    system_id: &str,
    provider: &VrpcProvider,
) -> Result<PreflightResult, WalletError> {
    validate_address(&params.to_address)?;
    validate_address(from_address)?;

    let submitted_sat = parse_positive_amount_sat(&params.amount)?;

    let addresses = vec![from_address.to_string()];
    let utxos_raw = provider.getaddressutxos(&addresses).await?;

    let utxos = parse_utxos(&utxos_raw)?;
    if utxos.is_empty() {
        return Err(WalletError::InsufficientFunds);
    }

    let fee_estimate = DEFAULT_FEE_ESTIMATE_SAT;
    let needed = submitted_sat.saturating_add(fee_estimate);
    let mut selected = Vec::<VrpcUtxo>::new();
    let mut total: i64 = 0;
    for utxo in utxos {
        total = total.saturating_add(utxo.satoshis);
        selected.push(utxo);
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
        .map(|utxo| serde_json::json!({"txid": utxo.txid, "voutnum": utxo.vout}))
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
    let payload_inputs = collect_payload_inputs(&funded_hex, &selected, "preflight")?;

    let preflight_id = Uuid::new_v4().to_string();
    let fee_str = sat_to_decimal_string(fee_sat);
    let value_str = sat_to_decimal_string(send_value_sat);
    let payload = VrpcPreflightPayload {
        hex: funded_hex,
        inputs: payload_inputs,
        system_id: system_id.to_string(),
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

fn parse_utxos(raw: &Value) -> Result<Vec<VrpcUtxo>, WalletError> {
    let arr = raw.as_array().ok_or(WalletError::OperationFailed)?;
    let mut out = Vec::new();
    for entry in arr {
        let utxo = parse_utxo_entry(entry).ok_or(WalletError::OperationFailed)?;
        if utxo.satoshis <= 0 {
            continue;
        }
        out.push(utxo);
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
        assert_eq!(parsed[0].txid, "def");
        assert_eq!(parsed[0].vout, 1);
        assert_eq!(parsed[0].satoshis, 12000);
        assert_eq!(
            parsed[0].script_pub_key.as_deref(),
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

    #[test]
    fn build_payload_includes_system_id() {
        let payload = VrpcPreflightPayload {
            hex: "00".to_string(),
            inputs: vec![],
            system_id: "iSystem".to_string(),
            to_address: "Rto".to_string(),
            from_address: "Rfrom".to_string(),
            value: "1.00000000".to_string(),
            fee: "0.00010000".to_string(),
        };

        assert_eq!(payload.system_id, "iSystem");
    }
}
