//
// Advanced VRPC transfer preflight (reserve-transfer/sendcurrency family).
// Builds tx templates server-side and stores signing payload by preflight_id.

use std::time::Duration;

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::core::channels::store::{PreflightRecord, PreflightStore};
use crate::core::channels::vrpc::common::{
    collect_payload_inputs, parse_coin_value_sat, parse_fee_sat, parse_positive_amount_sat,
    parse_result_string, parse_string, parse_utxo_entry, sat_to_decimal_string,
    VrpcPreflightPayload, VrpcUtxo, SATOSHIS_PER_COIN,
};
use crate::core::channels::vrpc::provider::VrpcProvider;
use crate::types::transaction::PreflightWarning;
use crate::types::{VrpcTransferPreflightParams, VrpcTransferPreflightResult, WalletError};

const DEFAULT_PARENT_FEE_LOW: f64 = 0.0001;
const DEFAULT_PARENT_FEE_LOW_SAT: i64 = 10_000;
const DEFAULT_PARENT_FEE_HIGH: f64 = 0.0002;
const DEFAULT_PARENT_FEE_HIGH_SAT: i64 = 20_000;
const DEFAULT_NATIVE_CONVERSION_FEE_SAT: i64 = 25_000;
const TRANSFER_PREFLIGHT_TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone)]
struct TransferRouteContext {
    source_is_native: bool,
    effective_fee_currency_id: Option<String>,
    parent_fee_coin: f64,
    parent_fee_sat: i64,
    native_required_fee_sat: i64,
}

#[derive(Debug, Clone)]
struct TransferAmountPlan {
    send_value_sat: i64,
    amount_adjusted: Option<String>,
}

#[derive(Debug, Clone)]
struct FundedTransfer {
    funded_hex: String,
    fee_sat: i64,
    payload_inputs: Vec<crate::core::channels::vrpc::common::VrpcInputRef>,
}

fn parse_sendcurrency_hex(raw: Value) -> Result<String, WalletError> {
    parse_result_string(&raw, &["hextx", "hex", "txhex"]).ok_or(WalletError::OperationFailed)
}

fn parse_fund_result(raw: Value, fallback_fee_sat: i64) -> Result<(String, i64), WalletError> {
    let hex = parse_result_string(&raw, &["hex"]).ok_or(WalletError::OperationFailed)?;
    let fee_sat = parse_fee_sat(raw.get("fee"), fallback_fee_sat);
    Ok((hex, fee_sat))
}

fn parse_funding_utxos(raw: &Value) -> Vec<VrpcUtxo> {
    let Some(arr) = raw.as_array() else {
        return vec![];
    };

    arr.iter()
        .filter_map(|entry| {
            let utxo = parse_utxo_entry(entry)?;
            (utxo.is_spendable && utxo.script_pub_key.is_some()).then_some(utxo)
        })
        .collect()
}

fn total_available_satoshis(utxos: &[VrpcUtxo]) -> i64 {
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
    if is_conversion_or_export || source_is_native {
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

async fn resolve_effective_fee_currency_id(
    provider: &VrpcProvider,
    params: &VrpcTransferPreflightParams,
    system_id: &str,
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

    is_conversion_or_export.then(|| system_id.to_string())
}

async fn estimate_transfer_fee_satoshis(
    provider: &VrpcProvider,
    from_address: &str,
    params: &VrpcTransferPreflightParams,
    normalized_destination: &str,
    parent_fee_coin: f64,
    effective_fee_currency_id: Option<&str>,
    system_id: &str,
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
        .unwrap_or_else(|| system_id.to_string());
    let mut fee_currency_refs = vec![fee_currency_id.clone()];
    if let Some(raw_fee_currency) = params
        .fee_currency
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !fee_currency_refs
            .iter()
            .any(|existing| is_same_currency_ref(existing, raw_fee_currency))
        {
            fee_currency_refs.push(raw_fee_currency.to_string());
        }
    }

    if params.export_to.is_none() && is_same_currency_ref(system_id, &fee_currency_id) {
        return Ok(DEFAULT_NATIVE_CONVERSION_FEE_SAT);
    }

    let probe_output = build_sendcurrency_output(
        params,
        normalized_destination,
        0.0,
        Some(fee_currency_id.as_str()),
    )?;
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

        return Ok((
            identity_addr.clone(),
            vec![PreflightWarning {
                warning_type: "resolved_destination".to_string(),
                message: format!(
                    "Destination handle {} resolved to {}.",
                    trimmed, identity_addr
                ),
            }],
        ));
    }

    Ok((trimmed.to_string(), Vec::new()))
}

fn build_sendcurrency_output(
    params: &VrpcTransferPreflightParams,
    normalized_destination: &str,
    send_amount: f64,
    effective_fee_currency_id: Option<&str>,
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
    if let Some(v) = effective_fee_currency_id
        .map(ToString::to_string)
        .or_else(|| params.fee_currency.clone())
    {
        out.insert("feecurrency".to_string(), Value::String(v));
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

async fn derive_route_context(
    provider: &VrpcProvider,
    params: &VrpcTransferPreflightParams,
    from_address: &str,
    normalized_destination: &str,
    system_id: &str,
) -> Result<TransferRouteContext, WalletError> {
    let source_currency_id = resolve_currency_id(provider, &params.coin_id).await;
    let source_is_native = source_currency_id
        .as_deref()
        .map(|currency_id| currency_id == system_id)
        .unwrap_or(false)
        || (source_currency_id.is_none() && is_known_native_symbol(&params.coin_id));
    let is_conversion_or_export = params.convert_to.is_some() || params.export_to.is_some();
    let (parent_fee_coin, parent_fee_sat) =
        parent_fee_for_route(is_conversion_or_export, source_is_native);
    let effective_fee_currency_id =
        resolve_effective_fee_currency_id(provider, params, system_id, is_conversion_or_export)
            .await;
    let transfer_fee_sat = estimate_transfer_fee_satoshis(
        provider,
        from_address,
        params,
        normalized_destination,
        parent_fee_coin,
        effective_fee_currency_id.as_deref(),
        system_id,
        is_conversion_or_export,
    )
    .await?;
    let native_required_fee_sat = if source_is_native
        && effective_fee_currency_id
            .as_deref()
            .map(|fee_currency| is_same_currency_ref(fee_currency, system_id))
            .unwrap_or(false)
    {
        parent_fee_sat.saturating_add(transfer_fee_sat)
    } else {
        parent_fee_sat
    };

    Ok(TransferRouteContext {
        source_is_native,
        effective_fee_currency_id,
        parent_fee_coin,
        parent_fee_sat,
        native_required_fee_sat,
    })
}

fn derive_amount_plan(
    submitted_sat: i64,
    available_sat: i64,
    route_context: &TransferRouteContext,
) -> Result<TransferAmountPlan, WalletError> {
    let (send_value_sat, amount_was_adjusted) = if route_context.source_is_native {
        resolve_send_value_for_native_fee(
            submitted_sat,
            available_sat,
            route_context.native_required_fee_sat,
        )?
    } else {
        (submitted_sat, false)
    };

    Ok(TransferAmountPlan {
        send_value_sat,
        amount_adjusted: amount_was_adjusted.then(|| sat_to_decimal_string(send_value_sat)),
    })
}

async fn build_funded_transfer(
    provider: &VrpcProvider,
    from_address: &str,
    output: Value,
    available_utxos: &[VrpcUtxo],
    parent_fee_coin: f64,
    fallback_fee_sat: i64,
) -> Result<FundedTransfer, WalletError> {
    let sendcurrency_result = provider
        .sendcurrency(from_address, &[output], 1, parent_fee_coin, true)
        .await?;
    let unfunded_hex = parse_sendcurrency_hex(sendcurrency_result)?;

    let funding_utxos: Vec<Value> = available_utxos
        .iter()
        .map(|utxo| serde_json::json!({ "txid": utxo.txid, "voutnum": utxo.vout }))
        .collect();
    let funded_raw = provider
        .fundrawtransaction_with_options(
            &unfunded_hex,
            Some(&funding_utxos),
            Some(from_address),
            Some(parent_fee_coin),
        )
        .await?;
    let (funded_hex, fee_sat) = parse_fund_result(funded_raw, fallback_fee_sat)?;
    let payload_inputs = collect_payload_inputs(&funded_hex, available_utxos, "transfer")?;

    Ok(FundedTransfer {
        funded_hex,
        fee_sat,
        payload_inputs,
    })
}

fn add_transfer_warnings(
    warnings: &mut Vec<PreflightWarning>,
    params: &VrpcTransferPreflightParams,
) {
    if params.convert_to.is_some() {
        warnings.push(PreflightWarning {
            warning_type: "estimated_fee".to_string(),
            message: "Final amount you receive may vary slightly.".to_string(),
        });
    }
}

fn store_transfer_payload(
    preflight_store: &PreflightStore,
    preflight_id: &str,
    channel_id: &str,
    account_id: &str,
    session_id: &str,
    payload: &VrpcPreflightPayload,
) -> Result<(), WalletError> {
    let payload_value = serde_json::to_value(payload).map_err(|_| WalletError::OperationFailed)?;
    if !preflight_store.put_with_ttl(
        preflight_id.to_string(),
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
    Ok(())
}

pub async fn preflight_transfer(
    params: VrpcTransferPreflightParams,
    preflight_store: &PreflightStore,
    account_id: &str,
    session_id: &str,
    from_address: &str,
    channel_id: &str,
    system_id: &str,
    provider: &VrpcProvider,
) -> Result<VrpcTransferPreflightResult, WalletError> {
    let submitted_sat = parse_positive_amount_sat(&params.amount)?;
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

    let route_context = derive_route_context(
        provider,
        &params,
        from_address,
        &normalized_destination,
        system_id,
    )
    .await?;
    let amount_plan = derive_amount_plan(
        submitted_sat,
        total_available_satoshis(&available_utxos),
        &route_context,
    )?;
    let send_amount = amount_plan.send_value_sat as f64 / SATOSHIS_PER_COIN as f64;
    let output = build_sendcurrency_output(
        &params,
        &normalized_destination,
        send_amount,
        route_context.effective_fee_currency_id.as_deref(),
    )?;
    let funded_transfer = build_funded_transfer(
        provider,
        from_address,
        output,
        &available_utxos,
        route_context.parent_fee_coin,
        route_context.parent_fee_sat,
    )
    .await?;

    add_transfer_warnings(&mut warnings, &params);

    let preflight_id = Uuid::new_v4().to_string();
    let payload = VrpcPreflightPayload {
        hex: funded_transfer.funded_hex,
        inputs: funded_transfer.payload_inputs,
        system_id: system_id.to_string(),
        to_address: normalized_destination.clone(),
        from_address: from_address.to_string(),
        value: sat_to_decimal_string(amount_plan.send_value_sat),
        fee: sat_to_decimal_string(funded_transfer.fee_sat),
    };
    store_transfer_payload(
        preflight_store,
        &preflight_id,
        channel_id,
        account_id,
        session_id,
        &payload,
    )?;

    Ok(VrpcTransferPreflightResult {
        preflight_id,
        fee: sat_to_decimal_string(funded_transfer.fee_sat),
        fee_currency: params.coin_id.clone(),
        value: sat_to_decimal_string(amount_plan.send_value_sat),
        amount_submitted: params.amount,
        amount_adjusted: amount_plan.amount_adjusted,
        to_address: normalized_destination,
        from_address: from_address.to_string(),
        warnings,
        memo: params.memo,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

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

        let output = build_sendcurrency_output(
            &params,
            "Rdest",
            1.0,
            Some("i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV"),
        )
        .expect("output");
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

        let output = build_sendcurrency_output(
            &params,
            "0x8fda30a676fbc8f1406adeac7921998b1af4fd05",
            1.0,
            None,
        )
        .expect("output");
        assert!(output.get("mapto").is_none());
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

    #[test]
    fn parse_funding_utxos_requires_spendable_outputs_with_scripts() {
        let parsed = parse_funding_utxos(&json!([
            {
                "txid": "usable",
                "vout": 0,
                "satoshis": 20_000,
                "scriptPubKey": "76a9",
                "isspendable": 1
            },
            {
                "txid": "missing-script",
                "vout": 1,
                "satoshis": 20_000,
                "isspendable": 1
            },
            {
                "txid": "not-spendable",
                "vout": 2,
                "satoshis": 20_000,
                "scriptPubKey": "76a9",
                "isspendable": 0
            }
        ]));

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].txid, "usable");
    }

    #[test]
    fn store_transfer_payload_is_session_bound_and_keeps_route_system_id() {
        let store = PreflightStore::new();
        store.activate_wallet_session("session-1");
        let payload = VrpcPreflightPayload {
            hex: "00".to_string(),
            inputs: vec![],
            system_id: "iSystem".to_string(),
            to_address: "Rto".to_string(),
            from_address: "Rfrom".to_string(),
            value: "1.00000000".to_string(),
            fee: "0.00010000".to_string(),
        };

        store_transfer_payload(
            &store,
            "test-id",
            "vrpc.Rfrom.iSystem",
            "account-id",
            "session-1",
            &payload,
        )
        .expect("store");

        let record = store.take("test-id", "session-1").expect("record");
        let stored: VrpcPreflightPayload = serde_json::from_value(record.payload).expect("payload");
        assert_eq!(stored.system_id, "iSystem");

        store.clear();
        assert!(matches!(
            store_transfer_payload(
                &store,
                "locked-id",
                "vrpc.Rfrom.iSystem",
                "account-id",
                "session-1",
                &payload,
            ),
            Err(WalletError::WalletLocked)
        ));

        store.activate_wallet_session("session-2");
        assert!(matches!(
            store_transfer_payload(
                &store,
                "stale-id",
                "vrpc.Rfrom.iSystem",
                "account-id",
                "session-1",
                &payload,
            ),
            Err(WalletError::WalletLocked)
        ));
        store_transfer_payload(
            &store,
            "fresh-id",
            "vrpc.Rfrom.iSystem",
            "account-id",
            "session-2",
            &payload,
        )
        .expect("store after reunlock");
        assert!(store.take("fresh-id", "session-1").is_none());
        assert!(store.take("fresh-id", "session-2").is_some());
    }
}
