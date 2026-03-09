//
// Identity preflight flow for update/revoke/recover.

use std::collections::HashSet;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::core::channels::store::{PreflightRecord, PreflightStore};
use crate::core::channels::vrpc::identity::validate::{
    apply_identity_operation, classify_high_risk_changes, validate_operation_authority,
    validate_target_state,
};
use crate::core::channels::vrpc::identity::verus_tx::codec::{
    decode_hex as decode_verus_tx, encode_hex as encode_verus_tx_hex,
};
use crate::core::channels::vrpc::identity::verus_tx::model::{
    txid_hex_to_le_bytes, txid_le_bytes_to_hex, InputSignMode, VerusTx, VerusTxIn, VerusTxOut,
};
use crate::core::channels::vrpc::identity::verus_tx::script::classify_prevout_script;
use crate::core::channels::vrpc::provider::VrpcProvider;
use crate::types::{
    IdentityOperation, IdentityPreflightParams, IdentityPreflightResult, IdentityWarning,
    WalletError,
};

const SATOSHIS_PER_COIN: i64 = 100_000_000;
pub(crate) const DEFAULT_FEE_SAT: i64 = 10_000;
const DUST_SAT: i64 = 546;
pub(crate) const IDENTITY_PREFLIGHT_TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentitySignMode {
    P2pkh,
    SmartTransaction,
}

impl From<InputSignMode> for IdentitySignMode {
    fn from(value: InputSignMode) -> Self {
        match value {
            InputSignMode::P2pkh => Self::P2pkh,
            InputSignMode::SmartTransaction => Self::SmartTransaction,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentitySignableInput {
    pub txid: String,
    pub vout: u32,
    pub satoshis: i64,
    pub script_pub_key: String,
    pub input_index: usize,
    pub sign_mode: IdentitySignMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityPreflightPayload {
    pub unsigned_hex: String,
    pub signable_inputs: Vec<IdentitySignableInput>,
    pub operation: IdentityOperation,
    pub target_identity: String,
    pub from_address: String,
    pub fee: String,
    pub memo: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct TargetIdentityState {
    pub(crate) status: String,
    pub(crate) txid: String,
    pub(crate) vout: u32,
    pub(crate) identity: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct FundingUtxo {
    txid: String,
    vout: u32,
    satoshis: i64,
    script_pub_key: String,
}

pub(crate) fn sat_to_decimal_string(sat: i64) -> String {
    format!("{:.8}", sat as f64 / SATOSHIS_PER_COIN as f64)
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

pub(crate) fn parse_target_identity(raw: Value) -> Result<TargetIdentityState, WalletError> {
    let status = parse_string(raw.get("status")).ok_or(WalletError::IdentityNotFound)?;
    let txid = parse_string(raw.get("txid")).ok_or(WalletError::IdentityNotFound)?;
    let vout = parse_u32(raw.get("vout")).ok_or(WalletError::IdentityNotFound)?;
    let identity = raw
        .get("identity")
        .cloned()
        .ok_or(WalletError::IdentityNotFound)?;

    Ok(TargetIdentityState {
        status,
        txid,
        vout,
        identity,
    })
}

pub(crate) fn map_identity_lookup_error(err: WalletError) -> WalletError {
    match err {
        WalletError::IdentityRpcUnsupported => WalletError::IdentityRpcUnsupported,
        WalletError::NetworkError => WalletError::NetworkError,
        WalletError::IdentityNotFound
        | WalletError::InvalidAddress
        | WalletError::OperationFailed => WalletError::IdentityNotFound,
        _ => WalletError::IdentityNotFound,
    }
}

fn parse_raw_tx_hex(raw: Value) -> Result<String, WalletError> {
    if let Some(hex) = raw.as_str() {
        return Ok(hex.to_string());
    }
    if let Some(hex) = raw.get("hex").and_then(|v| v.as_str()) {
        return Ok(hex.to_string());
    }
    Err(WalletError::IdentityBuildFailed)
}

pub(crate) fn parse_updateidentity_hex(raw: Value) -> Result<String, WalletError> {
    parse_raw_tx_hex(raw).map_err(|_| WalletError::IdentityBuildFailed)
}

pub(crate) fn parse_funding_utxos(raw: &Value) -> Vec<FundingUtxo> {
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
            if !is_spendable || satoshis <= 0 {
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

pub(crate) fn total_satoshis(utxos: &[FundingUtxo]) -> i64 {
    utxos.iter().map(|utxo| utxo.satoshis).sum()
}

fn txid_vout_key(txid: &str, vout: u32) -> String {
    format!("{txid}:{vout}")
}

fn classify_sign_mode(script_hex: &str) -> Result<IdentitySignMode, WalletError> {
    let script = hex::decode(script_hex).map_err(|_| WalletError::IdentityBuildFailed)?;
    let mode = classify_prevout_script(&script)?;
    Ok(mode.into())
}

pub(crate) fn build_unsigned_identity_tx(
    template_tx: &mut VerusTx,
    identity_txid: &str,
    identity_vout: u32,
    identity_script_hex: &str,
    identity_satoshis: i64,
    funding_candidates: &[FundingUtxo],
    fee_sat: i64,
) -> Result<(String, Vec<IdentitySignableInput>, bool), WalletError> {
    let identity_txid_le = txid_hex_to_le_bytes(identity_txid)?;
    let identity_input_index = template_tx
        .inputs
        .iter()
        .position(|input| {
            input.prevout_vout == identity_vout && input.prevout_txid_le == identity_txid_le
        })
        .ok_or(WalletError::IdentityBuildFailed)?;

    let mut existing_inputs = HashSet::new();
    for input in &template_tx.inputs {
        existing_inputs.insert(txid_vout_key(
            &txid_le_bytes_to_hex(&input.prevout_txid_le),
            input.prevout_vout,
        ));
    }

    let outputs_total_sat = template_tx.outputs.iter().fold(0i64, |acc, o| {
        acc.saturating_add(i64::try_from(o.value).unwrap_or(i64::MAX))
    });
    let required_total = outputs_total_sat.saturating_add(fee_sat);

    let mut selected: Vec<&FundingUtxo> = Vec::new();
    let mut selected_total: i64 = 0;
    for utxo in funding_candidates {
        if existing_inputs.contains(&txid_vout_key(&utxo.txid, utxo.vout)) {
            continue;
        }
        selected.push(utxo);
        selected_total = selected_total.saturating_add(utxo.satoshis);
        if selected_total >= required_total {
            break;
        }
    }

    if selected_total < required_total {
        return Err(WalletError::InsufficientFunds);
    }

    let mut signable_inputs: Vec<IdentitySignableInput> = Vec::new();
    let template_inputs = template_tx.inputs.len();
    for (idx, utxo) in selected.iter().enumerate() {
        let prevout_txid_le = txid_hex_to_le_bytes(&utxo.txid)?;
        template_tx.inputs.push(VerusTxIn {
            prevout_txid_le,
            prevout_vout: utxo.vout,
            script_sig: vec![],
            sequence: 0xffff_ffff,
        });

        signable_inputs.push(IdentitySignableInput {
            txid: utxo.txid.clone(),
            vout: utxo.vout,
            satoshis: utxo.satoshis,
            script_pub_key: utxo.script_pub_key.clone(),
            input_index: template_inputs + idx,
            sign_mode: classify_sign_mode(&utxo.script_pub_key)?,
        });
    }

    signable_inputs.push(IdentitySignableInput {
        txid: identity_txid.to_string(),
        vout: identity_vout,
        satoshis: identity_satoshis,
        script_pub_key: identity_script_hex.to_string(),
        input_index: identity_input_index,
        sign_mode: classify_sign_mode(identity_script_hex)?,
    });

    let change_sat = selected_total.saturating_sub(required_total);
    let mut dropped_dust_change = false;
    if change_sat >= DUST_SAT {
        let change_script = hex::decode(&selected[0].script_pub_key)
            .map_err(|_| WalletError::IdentityBuildFailed)?;
        template_tx.outputs.push(VerusTxOut {
            value: change_sat as u64,
            script_pub_key: change_script,
        });
    } else if change_sat > 0 {
        dropped_dust_change = true;
    }

    signable_inputs.sort_by_key(|input| input.input_index);

    let unsigned_hex = encode_verus_tx_hex(template_tx)?;
    Ok((unsigned_hex, signable_inputs, dropped_dust_change))
}

fn add_operation_warnings(
    warnings: &mut Vec<IdentityWarning>,
    params: &IdentityPreflightParams,
    before_identity: &Value,
    after_identity: &Value,
    dropped_dust_change: bool,
) {
    if matches!(params.operation, IdentityOperation::Update) && params.patch.is_none() {
        warnings.push(IdentityWarning {
            warning_type: "empty_patch".to_string(),
            message: "No patch values were provided for update operation.".to_string(),
        });
    }

    if before_identity == after_identity {
        warnings.push(IdentityWarning {
            warning_type: "no_effect".to_string(),
            message: "Operation does not change identity fields.".to_string(),
        });
    }

    if dropped_dust_change {
        warnings.push(IdentityWarning {
            warning_type: "dust_change".to_string(),
            message: "Change below dust threshold is added to fee.".to_string(),
        });
    }
}

pub(crate) async fn fetch_identity_prevout(
    provider: &VrpcProvider,
    identity_txid: &str,
    identity_vout: u32,
) -> Result<(String, i64), WalletError> {
    let raw_identity_tx = provider
        .getrawtransaction(identity_txid, 0)
        .await
        .map_err(|_| WalletError::IdentityBuildFailed)?;
    let raw_identity_hex = parse_raw_tx_hex(raw_identity_tx)?;
    let identity_tx = decode_verus_tx(&raw_identity_hex)?;
    let output = identity_tx
        .outputs
        .get(identity_vout as usize)
        .ok_or(WalletError::IdentityBuildFailed)?;

    let satoshis = i64::try_from(output.value).map_err(|_| WalletError::IdentityBuildFailed)?;
    Ok((hex::encode(&output.script_pub_key), satoshis))
}

pub async fn preflight(
    params: IdentityPreflightParams,
    preflight_store: &PreflightStore,
    account_id: &str,
    from_address: &str,
    channel_id: &str,
    provider: &VrpcProvider,
) -> Result<IdentityPreflightResult, WalletError> {
    if params.target_identity.trim().is_empty() {
        return Err(WalletError::IdentityNotFound);
    }

    let raw_target = provider
        .getidentity(&params.target_identity)
        .await
        .map_err(map_identity_lookup_error)?;
    let target = parse_target_identity(raw_target)?;

    validate_target_state(&target.status, &params.operation)?;
    validate_operation_authority(
        provider,
        &params.operation,
        &target.identity,
        &target.status,
        from_address,
    )
    .await?;

    let before_identity = target.identity.clone();
    let mut after_identity = target.identity.clone();
    apply_identity_operation(
        &mut after_identity,
        &params.operation,
        params.patch.as_ref(),
    )?;
    let high_risk_changes = classify_high_risk_changes(&before_identity, &after_identity);

    // Fetch spendable funding UTXOs early so we can return a deterministic fee-funding error
    // before touching transaction-template parsing, which may fail for unsupported tx formats.
    let funding_candidates = parse_funding_utxos(
        &provider
            .getaddressutxos(&[from_address.to_string()])
            .await
            .map_err(|_| WalletError::IdentityBuildFailed)?,
    );

    if total_satoshis(&funding_candidates) < DEFAULT_FEE_SAT {
        return Err(WalletError::InsufficientFunds);
    }

    let update_tx_raw = provider
        .updateidentity(&after_identity, true)
        .await
        .map_err(|err| match err {
            WalletError::IdentityRpcUnsupported => WalletError::IdentityRpcUnsupported,
            _ => WalletError::IdentityBuildFailed,
        })?;
    let update_tx_hex = parse_updateidentity_hex(update_tx_raw)?;
    let mut template_tx = decode_verus_tx(&update_tx_hex).map_err(|err| {
        println!(
            "[IDENTITY_PREFLIGHT] Failed to decode updateidentity tx template (op={:?}, target={}, from={}, hex_prefix={})",
            params.operation,
            params.target_identity,
            from_address,
            update_tx_hex.chars().take(16).collect::<String>()
        );
        err
    })?;

    let (identity_input_script, identity_input_satoshis) =
        fetch_identity_prevout(provider, &target.txid, target.vout).await?;

    let (unsigned_hex, signable_inputs, dropped_dust_change) = build_unsigned_identity_tx(
        &mut template_tx,
        &target.txid,
        target.vout,
        &identity_input_script,
        identity_input_satoshis,
        &funding_candidates,
        DEFAULT_FEE_SAT,
    )?;

    let mut warnings = Vec::new();
    add_operation_warnings(
        &mut warnings,
        &params,
        &before_identity,
        &after_identity,
        dropped_dust_change,
    );

    let fee = sat_to_decimal_string(DEFAULT_FEE_SAT);
    let preflight_id = Uuid::new_v4().to_string();
    let payload = IdentityPreflightPayload {
        unsigned_hex,
        signable_inputs,
        operation: params.operation.clone(),
        target_identity: params.target_identity.clone(),
        from_address: from_address.to_string(),
        fee: fee.clone(),
        memo: params.memo.clone(),
    };
    let payload_value =
        serde_json::to_value(payload).map_err(|_| WalletError::IdentityBuildFailed)?;

    preflight_store.put_with_ttl(
        preflight_id.clone(),
        PreflightRecord {
            channel_id: channel_id.to_string(),
            account_id: account_id.to_string(),
            payload: payload_value,
        },
        Some(IDENTITY_PREFLIGHT_TTL),
    );

    Ok(IdentityPreflightResult {
        preflight_id,
        operation: params.operation,
        target_identity: params.target_identity,
        from_address: from_address.to_string(),
        fee,
        fee_currency: params.coin_id,
        high_risk_changes,
        warnings,
        memo: params.memo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_lookup_error_mapping_preserves_network_error() {
        assert!(matches!(
            map_identity_lookup_error(WalletError::NetworkError),
            WalletError::NetworkError
        ));
    }

    #[test]
    fn identity_lookup_error_mapping_preserves_rpc_unsupported() {
        assert!(matches!(
            map_identity_lookup_error(WalletError::IdentityRpcUnsupported),
            WalletError::IdentityRpcUnsupported
        ));
    }

    #[test]
    fn identity_lookup_error_mapping_maps_lookup_failures_to_not_found() {
        assert!(matches!(
            map_identity_lookup_error(WalletError::OperationFailed),
            WalletError::IdentityNotFound
        ));
        assert!(matches!(
            map_identity_lookup_error(WalletError::InvalidAddress),
            WalletError::IdentityNotFound
        ));
    }
}
