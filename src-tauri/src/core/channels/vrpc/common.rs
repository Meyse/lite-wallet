//
// Shared VRPC parsing and payload helpers.

use std::collections::HashSet;
use std::io::Cursor;

use bitcoin::consensus::Decodable;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::channels::vrpc::identity::verus_tx::codec::decode_hex as decode_verus_tx;
use crate::core::channels::vrpc::identity::verus_tx::model::txid_le_bytes_to_hex;
use crate::types::WalletError;

pub(crate) const SATOSHIS_PER_COIN: i64 = 100_000_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct VrpcPreflightPayload {
    pub hex: String,
    #[serde(rename = "inputs")]
    pub inputs: Vec<VrpcInputRef>,
    pub system_id: String,
    pub to_address: String,
    pub from_address: String,
    pub value: String,
    pub fee: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct VrpcInputRef {
    pub txid: String,
    pub vout: u32,
    pub satoshis: i64,
    #[serde(default)]
    pub script_pub_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VrpcUtxo {
    pub txid: String,
    pub vout: u32,
    pub satoshis: i64,
    pub script_pub_key: Option<String>,
    pub is_spendable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TxOutpointRef {
    pub txid: String,
    pub vout: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct HistoryEntry {
    pub raw: Value,
    pub pending: bool,
}

fn integral_f64_to_i64(value: f64) -> Option<i64> {
    if !value.is_finite()
        || value.fract() != 0.0
        || value < i64::MIN as f64
        || value >= -(i64::MIN as f64)
    {
        return None;
    }
    Some(value as i64)
}

fn coin_value_to_sat(value: f64) -> Option<i64> {
    let satoshis = (value * SATOSHIS_PER_COIN as f64).round();
    if !satoshis.is_finite() || satoshis < i64::MIN as f64 || satoshis >= -(i64::MIN as f64) {
        return None;
    }
    Some(satoshis as i64)
}

pub(crate) fn parse_i64(value: Option<&Value>) -> Option<i64> {
    let value = value?;
    if let Some(raw) = value.as_i64() {
        return Some(raw);
    }
    if let Some(raw) = value.as_u64() {
        return i64::try_from(raw).ok();
    }
    if let Some(raw) = value.as_f64() {
        return integral_f64_to_i64(raw);
    }
    value.as_str()?.parse::<i64>().ok()
}

pub(crate) fn parse_u32(value: Option<&Value>) -> Option<u32> {
    u32::try_from(parse_i64(value)?).ok()
}

pub(crate) fn parse_string(value: Option<&Value>) -> Option<String> {
    value?.as_str().map(ToString::to_string)
}

pub(crate) fn value_as_f64(value: &Value) -> Option<f64> {
    let parsed = if let Some(raw) = value.as_f64() {
        raw
    } else if let Some(raw) = value.as_i64() {
        raw as f64
    } else if let Some(raw) = value.as_u64() {
        raw as f64
    } else {
        value.as_str()?.parse::<f64>().ok()?
    };
    if parsed.is_finite() {
        Some(parsed)
    } else {
        None
    }
}

pub(crate) fn parse_positive_amount_sat(amount: &str) -> Result<i64, WalletError> {
    let parsed = amount
        .trim()
        .parse::<f64>()
        .map_err(|_| WalletError::OperationFailed)?;
    let sat = coin_value_to_sat(parsed).ok_or(WalletError::OperationFailed)?;
    if sat <= 0 {
        return Err(WalletError::OperationFailed);
    }
    Ok(sat)
}

pub(crate) fn parse_fee_sat(value: Option<&Value>, fallback_sat: i64) -> i64 {
    let fallback = fallback_sat.max(1);
    let normalize = |candidate: i64| if candidate > 0 { candidate } else { fallback };

    let Some(value) = value else {
        return fallback;
    };

    if let Some(raw_sat) = value.as_i64() {
        return normalize(raw_sat);
    }
    if let Some(raw_sat) = value.as_u64() {
        return i64::try_from(raw_sat).map(normalize).unwrap_or(fallback);
    }
    if let Some(raw_coin) = value.as_f64() {
        return coin_value_to_sat(raw_coin)
            .map(normalize)
            .unwrap_or(fallback);
    }
    if let Some(raw_str) = value.as_str() {
        let trimmed = raw_str.trim();
        if trimmed.contains('.') || trimmed.contains('e') || trimmed.contains('E') {
            if let Ok(raw_coin) = trimmed.parse::<f64>() {
                return coin_value_to_sat(raw_coin)
                    .map(normalize)
                    .unwrap_or(fallback);
            }
            return fallback;
        }
        if let Ok(raw_sat) = trimmed.parse::<i64>() {
            return normalize(raw_sat);
        }
    }

    fallback
}

pub(crate) fn sat_to_decimal_string(sat: i64) -> String {
    format!("{:.8}", sat as f64 / SATOSHIS_PER_COIN as f64)
}

pub(crate) fn parse_coin_value_sat(value: &Value) -> Option<i64> {
    if let Some(raw_sat) = value.as_i64() {
        return Some(raw_sat.max(0));
    }
    if let Some(raw_sat) = value.as_u64() {
        return i64::try_from(raw_sat).ok();
    }
    if let Some(raw_coin) = value.as_f64() {
        if !raw_coin.is_finite() {
            return None;
        }
        return coin_value_to_sat(raw_coin.max(0.0));
    }

    value.as_str().and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        if trimmed.contains('.') || trimmed.contains('e') || trimmed.contains('E') {
            let coin = trimmed.parse::<f64>().ok()?;
            if !coin.is_finite() {
                return None;
            }
            coin_value_to_sat(coin.max(0.0))
        } else {
            trimmed.parse::<i64>().ok().map(|sat| sat.max(0))
        }
    })
}

pub(crate) fn parse_result_string(value: &Value, fields: &[&str]) -> Option<String> {
    if let Some(raw) = value.as_str() {
        return Some(raw.to_string());
    }

    fields
        .iter()
        .find_map(|field| parse_string(value.get(field)))
}

pub(crate) fn parse_txid_from_result(value: &Value) -> Option<String> {
    parse_result_string(value, &["txid"])
}

pub(crate) fn parse_utxo_entry(entry: &Value) -> Option<VrpcUtxo> {
    Some(VrpcUtxo {
        txid: parse_string(entry.get("txid").or(entry.get("outputTxId")))?,
        vout: parse_u32(entry.get("vout").or(entry.get("outputIndex")))?,
        satoshis: parse_i64(entry.get("satoshis").or(entry.get("amount"))).unwrap_or(0),
        script_pub_key: parse_string(entry.get("script").or(entry.get("scriptPubKey"))),
        is_spendable: parse_i64(entry.get("isspendable")).unwrap_or(1) != 0,
    })
}

pub(crate) fn parse_funded_input_refs(
    funded_hex: &str,
    log_context: &str,
) -> Result<Vec<TxOutpointRef>, WalletError> {
    if let Ok(verus_tx) = decode_verus_tx(funded_hex) {
        let refs = verus_tx
            .inputs
            .iter()
            .map(|input| TxOutpointRef {
                txid: txid_le_bytes_to_hex(&input.prevout_txid_le),
                vout: input.prevout_vout,
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
                "[VRPC][{}] failed to decode funded hex bytes for input extraction: {}",
                log_context, err
            );
            WalletError::OperationFailed
        })?;
    let mut cursor = Cursor::new(&raw[..]);
    let funded_tx: bitcoin::Transaction = bitcoin::Transaction::consensus_decode(&mut cursor)
        .map_err(|err| {
            println!(
                "[VRPC][{}] failed to decode funded transaction with bitcoin parser for input extraction: {}",
                log_context, err
            );
            WalletError::OperationFailed
        })?;
    Ok(funded_tx
        .input
        .into_iter()
        .map(|input| TxOutpointRef {
            txid: input.previous_output.txid.to_string(),
            vout: input.previous_output.vout,
        })
        .collect())
}

pub(crate) fn collect_payload_inputs(
    funded_hex: &str,
    candidates: &[VrpcUtxo],
    log_context: &str,
) -> Result<Vec<VrpcInputRef>, WalletError> {
    let funded_inputs = parse_funded_input_refs(funded_hex, log_context)?;

    let mut seen = HashSet::<(String, u32)>::new();
    let mut out = Vec::with_capacity(funded_inputs.len());
    for input in funded_inputs {
        if !seen.insert((input.txid.clone(), input.vout)) {
            return Err(WalletError::OperationFailed);
        }

        let Some(candidate) = candidates
            .iter()
            .find(|candidate| candidate.txid == input.txid && candidate.vout == input.vout)
        else {
            println!(
                "[VRPC][{}] funded input missing from candidate utxos: txid={} vout={} candidate_count={}",
                log_context,
                input.txid,
                input.vout,
                candidates.len()
            );
            return Err(WalletError::OperationFailed);
        };

        out.push(VrpcInputRef {
            txid: candidate.txid.clone(),
            vout: candidate.vout,
            satoshis: candidate.satoshis,
            script_pub_key: candidate.script_pub_key.clone(),
        });
    }

    if out.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bitcoin::consensus::Encodable;
    use bitcoin::{
        absolute::LockTime, transaction::Version, OutPoint, ScriptBuf, Sequence, Transaction, TxIn,
        TxOut, Txid, Witness,
    };
    use serde_json::json;

    use super::*;

    const SAMPLE_V4_HEX: &str = "0400008085202f8901ffeeddccbbaa99887766554433221100ffeeddccbbaa998877665544332211000300000000ffffffff0150c30000000000001976a914111111111111111111111111111111111111111188ac00000000000000000000000000000000000000";

    #[test]
    fn parse_positive_amount_sat_accepts_string_decimal() {
        assert_eq!(parse_positive_amount_sat("0.0001").expect("amount"), 10_000);
        assert_eq!(parse_positive_amount_sat("1e-8").expect("amount"), 1);
    }

    #[test]
    fn shared_integer_parsers_reject_fractional_negative_and_oversized_vout_values() {
        assert_eq!(parse_i64(Some(&json!(42.0))), Some(42));
        assert_eq!(parse_i64(Some(&json!(42.5))), None);
        assert_eq!(parse_u32(Some(&json!(u32::MAX))), Some(u32::MAX));
        assert_eq!(parse_u32(Some(&json!(-1))), None);
        assert_eq!(parse_u32(Some(&json!(1.5))), None);
        assert_eq!(parse_u32(Some(&json!(u64::from(u32::MAX) + 1))), None);
    }

    #[test]
    fn shared_amount_parsers_reject_non_finite_and_overflowing_values() {
        for amount in ["NaN", "inf", "-1", "0", "1e300"] {
            assert!(matches!(
                parse_positive_amount_sat(amount),
                Err(WalletError::OperationFailed)
            ));
        }

        assert_eq!(parse_fee_sat(Some(&json!("1e300")), 10_000), 10_000);
        assert_eq!(parse_coin_value_sat(&json!("NaN")), None);
        assert_eq!(parse_coin_value_sat(&json!("1e300")), None);
    }

    #[test]
    fn parse_fee_sat_accepts_integer_float_and_string_values() {
        assert_eq!(parse_fee_sat(Some(&json!(42)), 1), 42);
        assert_eq!(parse_fee_sat(Some(&json!(0.0001)), 1), 10_000);
        assert_eq!(parse_fee_sat(Some(&json!("0.0001")), 1), 10_000);
    }

    #[test]
    fn parse_txid_from_result_supports_string_and_object_shapes() {
        assert_eq!(
            parse_txid_from_result(&json!("abc123")).as_deref(),
            Some("abc123")
        );
        assert_eq!(
            parse_txid_from_result(&json!({ "txid": "def456" })).as_deref(),
            Some("def456")
        );
    }

    #[test]
    fn parse_result_string_reads_string_or_first_matching_field() {
        assert_eq!(
            parse_result_string(&json!({ "hextx": "beef" }), &["hex", "hextx"]).as_deref(),
            Some("beef")
        );
        assert_eq!(
            parse_result_string(&json!("cafe"), &["hex"]).as_deref(),
            Some("cafe")
        );
    }

    #[test]
    fn parse_utxo_entry_keeps_spendable_and_script_shape() {
        let utxo = parse_utxo_entry(&json!({
            "txid": "abc",
            "vout": 1,
            "satoshis": 12000,
            "scriptPubKey": "76a9",
            "isspendable": 0
        }))
        .expect("utxo");

        assert_eq!(utxo.txid, "abc");
        assert_eq!(utxo.vout, 1);
        assert_eq!(utxo.satoshis, 12_000);
        assert_eq!(utxo.script_pub_key.as_deref(), Some("76a9"));
        assert!(!utxo.is_spendable);
    }

    #[test]
    fn parse_utxo_entry_rejects_missing_or_invalid_outpoints() {
        assert!(parse_utxo_entry(&json!({ "vout": 1, "satoshis": 1 })).is_none());
        assert!(parse_utxo_entry(&json!({ "txid": "abc", "satoshis": 1 })).is_none());
        assert!(parse_utxo_entry(&json!({
            "txid": "abc",
            "vout": -1,
            "satoshis": 1
        }))
        .is_none());
        assert!(parse_utxo_entry(&json!({
            "txid": "abc",
            "vout": 1.5,
            "satoshis": 1
        }))
        .is_none());
        assert!(parse_utxo_entry(&json!({
            "txid": "abc",
            "vout": u64::from(u32::MAX) + 1,
            "satoshis": 1
        }))
        .is_none());
    }

    #[test]
    fn parse_funded_input_refs_supports_verus_transactions() {
        let refs = parse_funded_input_refs(SAMPLE_V4_HEX, "test").expect("refs");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].vout, 3);
        assert_eq!(
            refs[0].txid,
            "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
        );
    }

    #[test]
    fn parse_funded_input_refs_supports_bitcoin_transactions() {
        let tx = Transaction {
            version: Version(2),
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: Txid::from_str(
                        "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
                    )
                    .expect("txid"),
                    vout: 7,
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::default(),
            }],
            output: vec![TxOut {
                value: bitcoin::Amount::from_sat(1),
                script_pubkey: ScriptBuf::new(),
            }],
        };
        let mut encoded = Vec::new();
        tx.consensus_encode(&mut encoded).expect("encode");

        let refs = parse_funded_input_refs(&hex::encode(encoded), "test").expect("refs");
        assert_eq!(refs.len(), 1);
        assert_eq!(
            refs[0],
            TxOutpointRef {
                txid: "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
                    .to_string(),
                vout: 7,
            }
        );
    }
}
