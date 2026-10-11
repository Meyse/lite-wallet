//! Claiming a key's VerusIDs and funds into this wallet: gift cards, as Verus
//! Mobile claims them at `dev-asf-parent` (`spendableKey.js`).
//!
//! Each VerusID the key controls moves to the wallet's address, paid from a
//! plain native output of the key, or else from native funds the VerusID holds
//! itself, with the change to the wallet. Every other output of the key then
//! goes to the wallet's address in one sweep. The claim key signs both, and
//! nothing pays anyone but the wallet.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::types::{IdentityOperation, WalletError};

use super::common::{
    authenticate_payload_inputs, collect_payload_inputs, parse_utxo_entry, sat_to_decimal_string,
    VrpcInputRef, VrpcPreflightPayload, VrpcUtxo,
};
use super::identity::preflight::{
    build_unsigned_identity_tx, fetch_identity_prevout, parse_target_identity,
    parse_updateidentity_hex, IdentityPreflightPayload, DEFAULT_FEE_SAT,
};
use super::identity::validate::{validate_operation_authority, validate_target_state};
use super::identity::verus_tx::codec::decode_hex as decode_verus_tx;
use super::intent::{
    decode_base58_destination, identity_control_intent_from_json, is_native_identity_output,
    validate_identity_control_intent, validate_identity_transaction_intent,
    validate_sweep_transaction, VrpcOutputIntent,
};
use super::transfer::{parse_fund_result, parse_sendcurrency_hex};
use super::VrpcProvider;

const SATOSHIS_PER_COIN: f64 = 100_000_000.0;
/// The sweep fee Verus Mobile plans: more when other currencies move too.
pub(crate) const SWEEP_FEE_SAT: i64 = 10_000;
pub(crate) const SWEEP_WITH_CURRENCIES_FEE_SAT: i64 = 20_000;

/// One output the claim key can spend, with the other currencies it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClaimOutput {
    pub utxo: VrpcUtxo,
    /// Satoshis of each currency other than the chain's own.
    pub currencies: BTreeMap<String, i64>,
}

impl ClaimOutput {
    /// A plain output: native only, to the key's address. Only these pay
    /// identity fees, so no other currency is ever left out of a transaction.
    fn is_plain_native(&self) -> bool {
        self.currencies.is_empty()
            && self.utxo.satoshis > 0
            && self
                .utxo
                .script_pub_key
                .as_deref()
                .is_some_and(|script| script.starts_with("76a914") && script.len() == 50)
    }
}

fn coins(sats: i64) -> f64 {
    sats as f64 / SATOSHIS_PER_COIN
}

/// The spendable outputs of the claim key (`getaddressutxos`), with the
/// currencies each holds besides `native_currency_id`.
pub(crate) fn parse_claim_outputs(raw: &Value, native_currency_id: &str) -> Vec<ClaimOutput> {
    raw.as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let utxo = parse_utxo_entry(entry)?;
                    if !utxo.is_spendable || utxo.script_pub_key.is_none() {
                        return None;
                    }
                    let currencies = entry
                        .get("currencyvalues")
                        .and_then(Value::as_object)
                        .map(|values| {
                            values
                                .iter()
                                .filter(|(currency, _)| {
                                    !currency.eq_ignore_ascii_case(native_currency_id)
                                })
                                .filter_map(|(currency, amount)| {
                                    let sats =
                                        (amount.as_f64()? * SATOSHIS_PER_COIN).round() as i64;
                                    (sats > 0).then(|| (currency.clone(), sats))
                                })
                                .collect::<BTreeMap<_, _>>()
                        })
                        .unwrap_or_default();
                    (utxo.satoshis > 0 || !currencies.is_empty())
                        .then_some(ClaimOutput { utxo, currencies })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The native satoshis, and the satoshis of each other currency.
pub(crate) fn claim_totals(outputs: &[ClaimOutput]) -> (i64, BTreeMap<String, i64>) {
    let mut native = 0i64;
    let mut currencies = BTreeMap::new();
    for output in outputs {
        native = native.saturating_add(output.utxo.satoshis);
        for (currency, sats) in &output.currencies {
            *currencies.entry(currency.clone()).or_insert(0i64) += sats;
        }
    }
    (native, currencies)
}

/// Picks one plain native output for each identity fee, largest first, as long
/// as they last. An identity without one pays from its own funds
/// ([`identity_fee_outputs`]). The rest of the outputs are swept.
pub(crate) fn split_fee_outputs(
    outputs: &[ClaimOutput],
    identity_count: usize,
) -> (Vec<VrpcUtxo>, Vec<ClaimOutput>) {
    let mut plain = outputs
        .iter()
        .filter(|output| output.is_plain_native() && output.utxo.satoshis >= DEFAULT_FEE_SAT)
        .collect::<Vec<_>>();
    plain.sort_by_key(|output| std::cmp::Reverse(output.utxo.satoshis));
    let fee_outputs = plain
        .into_iter()
        .take(identity_count)
        .map(|output| output.utxo.clone())
        .collect::<Vec<_>>();
    let rest = outputs
        .iter()
        .filter(|output| {
            !fee_outputs
                .iter()
                .any(|fee| fee.txid == output.utxo.txid && fee.vout == output.utxo.vout)
        })
        .cloned()
        .collect();
    (fee_outputs, rest)
}

/// Native funds a VerusID holds at its own i-address, largest first, for its
/// claim fee when the key has no plain native output for it (Verus Mobile's
/// `getIdentityFeeUtxos`). Only native outputs locked to the VerusID alone
/// qualify, so nothing else it holds moves. The claim key can sign them while
/// it is the VerusID's primary address.
pub(crate) fn identity_fee_outputs(
    raw: &Value,
    native_currency_id: &str,
    identity_address: &str,
) -> Vec<VrpcUtxo> {
    let mut fee_outputs = parse_claim_outputs(raw, native_currency_id)
        .into_iter()
        .filter(|output| {
            output.currencies.is_empty()
                && output.utxo.satoshis > 0
                && output
                    .utxo
                    .script_pub_key
                    .as_deref()
                    .is_some_and(|script| is_native_identity_output(script, identity_address))
        })
        .map(|output| output.utxo)
        .collect::<Vec<_>>();
    fee_outputs.sort_by_key(|utxo| std::cmp::Reverse(utxo.satoshis));
    fee_outputs
}

fn p2pkh_script(address: &str) -> Result<Vec<u8>, WalletError> {
    let destination = decode_base58_destination(address)?;
    if destination.destination_type != 2 || destination.destination_bytes.len() != 20 {
        return Err(WalletError::InvalidAddress);
    }
    let mut script = vec![0x76, 0xa9, 0x14];
    script.extend_from_slice(&destination.destination_bytes);
    script.extend_from_slice(&[0x88, 0xac]);
    Ok(script)
}

/// The update that moves `identity_address` to the wallet: the wallet's
/// address as its only primary address, one signature, and the wallet's
/// private address, or none (Verus Mobile clears it when the wallet has
/// none). Its revocation and recovery stay as they are. The fee comes from
/// `fee_outputs`, as many as it takes; the change goes to the wallet. (Verus
/// Mobile returns change from the VerusID's own funds to the VerusID; here it
/// goes to the wallet, so every output but the identity pays the wallet.)
pub(crate) async fn preflight_identity_claim(
    provider: &VrpcProvider,
    identity_address: &str,
    claim_address: &str,
    wallet_address: &str,
    wallet_private_address: Option<&str>,
    fee_outputs: &[VrpcUtxo],
) -> Result<IdentityPreflightPayload, WalletError> {
    let target = parse_target_identity(provider.getidentity(identity_address).await?)?;
    validate_target_state(&target.status, &IdentityOperation::Update)?;
    validate_operation_authority(
        provider,
        &IdentityOperation::Update,
        &target.identity,
        &target.status,
        claim_address,
    )
    .await?;

    let mut requested = target.identity.clone();
    let fields = requested
        .as_object_mut()
        .ok_or(WalletError::IdentityBuildFailed)?;
    fields.insert("primaryaddresses".into(), json!([wallet_address]));
    fields.insert("minimumsignatures".into(), json!(1));
    // Content is written per transaction; a claim writes none.
    fields.remove("contentmultimap");
    fields.remove("privateaddresses");
    match wallet_private_address {
        Some(address) => {
            fields.insert("privateaddress".into(), json!(address));
        }
        None => {
            fields.remove("privateaddress");
        }
    }

    let template_hex = parse_updateidentity_hex(provider.updateidentity(&requested, true).await?)?;
    let control_intent = identity_control_intent_from_json(&requested)?;
    validate_identity_control_intent(&template_hex, &control_intent)?;
    let mut template =
        decode_verus_tx(&template_hex).map_err(|_| WalletError::IdentityBuildFailed)?;
    let (identity_script, identity_sats) =
        fetch_identity_prevout(provider, &target.txid, target.vout).await?;
    let change_script = p2pkh_script(wallet_address)?;
    let (unsigned_hex, signable_inputs, _) = build_unsigned_identity_tx(
        &mut template,
        &target.txid,
        target.vout,
        &identity_script,
        identity_sats,
        fee_outputs,
        DEFAULT_FEE_SAT,
        Some(&change_script),
    )?;
    let inputs = signable_inputs
        .iter()
        .map(|input| VrpcInputRef {
            txid: input.txid.clone(),
            vout: input.vout,
            satoshis: input.satoshis,
            script_pub_key: Some(input.script_pub_key.clone()),
        })
        .collect::<Vec<_>>();
    authenticate_payload_inputs(provider, &inputs)
        .await
        .map_err(|_| WalletError::IdentityBuildFailed)?;
    let input_total = inputs
        .iter()
        .try_fold(0i64, |total, input| total.checked_add(input.satoshis))
        .ok_or(WalletError::IdentityBuildFailed)?;
    validate_identity_transaction_intent(
        &unsigned_hex,
        &control_intent,
        wallet_address,
        input_total,
        DEFAULT_FEE_SAT,
    )?;

    Ok(IdentityPreflightPayload {
        unsigned_hex,
        signable_inputs,
        operation: IdentityOperation::Update,
        target_identity: identity_address.to_string(),
        from_address: claim_address.to_string(),
        fee: sat_to_decimal_string(DEFAULT_FEE_SAT),
        memo: None,
        control_intent,
        profile_intent: None,
    })
}

/// The sweep of `outputs` to the wallet: every currency in full, the native
/// amount less the fee. `None` when there is nothing worth moving.
pub(crate) async fn preflight_sweep(
    provider: &VrpcProvider,
    claim_address: &str,
    wallet_address: &str,
    outputs: &[ClaimOutput],
    system_id: &str,
) -> Result<Option<VrpcPreflightPayload>, WalletError> {
    let (native, currencies) = claim_totals(outputs);
    let fee = if currencies.is_empty() {
        SWEEP_FEE_SAT
    } else {
        SWEEP_WITH_CURRENCIES_FEE_SAT
    };
    let send_native = native - fee;
    if currencies.is_empty() && send_native <= 0 {
        return Ok(None);
    }
    if send_native < 0 {
        // Other currencies need native funds for the fee.
        return Err(WalletError::InsufficientFunds);
    }

    let mut transfers = Vec::new();
    if send_native > 0 {
        transfers.push(json!({
            "currency": system_id,
            "amount": coins(send_native),
            "address": wallet_address,
        }));
    }
    for (currency, sats) in &currencies {
        transfers.push(json!({
            "currency": currency,
            "amount": coins(*sats),
            "address": wallet_address,
        }));
    }
    let unfunded = parse_sendcurrency_hex(
        provider
            .sendcurrency(claim_address, &transfers, 1, coins(fee), true)
            .await?,
    )?;
    let funding = outputs
        .iter()
        .map(|output| json!({ "txid": output.utxo.txid, "voutnum": output.utxo.vout }))
        .collect::<Vec<_>>();
    let (funded_hex, _) = parse_fund_result(
        provider
            .fundrawtransaction_with_options(
                &unfunded,
                Some(&funding),
                Some(wallet_address),
                Some(coins(fee)),
            )
            .await?,
        fee,
    )?;
    let candidates = outputs
        .iter()
        .map(|output| output.utxo.clone())
        .collect::<Vec<_>>();
    let inputs = collect_payload_inputs(&funded_hex, &candidates, "claim")?;
    // A sweep leaves nothing behind.
    if inputs.len() != candidates.len() {
        return Err(WalletError::OperationFailed);
    }
    authenticate_payload_inputs(provider, &inputs).await?;
    validate_sweep_transaction(&funded_hex, wallet_address, &inputs, fee)?;

    let destination = decode_base58_destination(wallet_address)?;
    Ok(Some(VrpcPreflightPayload {
        hex: funded_hex,
        inputs,
        system_id: system_id.to_string(),
        entered_to_address: None,
        to_address: wallet_address.to_string(),
        from_address: claim_address.to_string(),
        value: sat_to_decimal_string(send_native),
        fee: sat_to_decimal_string(fee),
        intent: VrpcOutputIntent::NativePayment {
            destination,
            amount_sats: u64::try_from(send_native).map_err(|_| WalletError::OperationFailed)?,
        },
    }))
}

/// Signs a claim's identity updates, then its sweep, with the claim key, in
/// the order they must be broadcast. Each is checked again first.
pub(crate) fn sign_claim(
    wallet_address: &str,
    identities: &[IdentityPreflightPayload],
    sweep: Option<&VrpcPreflightPayload>,
    claim_key: &[u8; 32],
) -> Result<Vec<String>, WalletError> {
    let mut signed = Vec::new();
    for payload in identities {
        let input_total = payload
            .signable_inputs
            .iter()
            .try_fold(0i64, |total, input| total.checked_add(input.satoshis))
            .ok_or(WalletError::InvalidPreflight)?;
        validate_identity_transaction_intent(
            &payload.unsigned_hex,
            &payload.control_intent,
            wallet_address,
            input_total,
            DEFAULT_FEE_SAT,
        )?;
        signed.push(super::identity::sign_identity_payload(payload, claim_key)?);
    }
    if let Some(sweep) = sweep {
        let fee = super::common::parse_positive_amount_sat(&sweep.fee)?;
        validate_sweep_transaction(&sweep.hex, wallet_address, &sweep.inputs, fee)?;
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let secret_key = bitcoin::secp256k1::SecretKey::from_slice(claim_key)
            .map_err(|_| WalletError::OperationFailed)?;
        let public_key = bitcoin::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        signed.push(super::send::sign_payload(
            sweep,
            &secp,
            &secret_key,
            &public_key,
        )?);
    }
    Ok(signed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TESTNET: &str = "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq";
    const PLAIN_SCRIPT: &str = "76a914000102030405060708090a0b0c0d0e0f1011121388ac";

    fn entry(txid: &str, sats: i64, script: &str, currencies: Value) -> Value {
        json!({
            "txid": txid,
            "outputIndex": 0,
            "satoshis": sats,
            "script": script,
            "isspendable": 1,
            "currencyvalues": currencies,
        })
    }

    #[test]
    fn claim_outputs_keep_other_currencies_apart_from_the_native_one() {
        let raw = json!([
            entry("aa", 5_000_000, PLAIN_SCRIPT, json!({ TESTNET: 0.05 })),
            entry(
                "bb",
                0,
                "cc0102",
                json!({ "iTokenCurrency1111111111111111111": 1.5 })
            ),
            entry("cc", 0, PLAIN_SCRIPT, json!({})),
        ]);
        let outputs = parse_claim_outputs(&raw, TESTNET);
        assert_eq!(outputs.len(), 2);
        let (native, currencies) = claim_totals(&outputs);
        assert_eq!(native, 5_000_000);
        assert_eq!(
            currencies.get("iTokenCurrency1111111111111111111"),
            Some(&150_000_000)
        );
    }

    #[test]
    fn identity_fees_come_only_from_plain_native_outputs() {
        let raw = json!([
            entry("aa", 2_000_000, PLAIN_SCRIPT, json!({})),
            entry(
                "bb",
                9_000_000,
                "cc0102",
                json!({ "iTokenCurrency1111111111111111111": 1 })
            ),
            entry("cc", 3_000_000, PLAIN_SCRIPT, json!({})),
        ]);
        let outputs = parse_claim_outputs(&raw, TESTNET);
        let (fees, rest) = split_fee_outputs(&outputs, 1);
        assert_eq!(fees.len(), 1);
        assert_eq!(fees[0].txid, "cc");
        assert_eq!(rest.len(), 2);
        // A third identity pays from its own funds; the token output is swept.
        let (fees, rest) = split_fee_outputs(&outputs, 3);
        assert_eq!(fees.len(), 2);
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].utxo.txid, "bb");
    }

    #[test]
    fn a_verusid_pays_its_fee_only_from_native_funds_locked_to_it_alone() {
        // `getaddressutxos` for dlsgiftqa1@ on VRSCTEST: 0.05 sent to its
        // i-address (`sendcurrency`), and the VerusID's own reservation output.
        const IDENTITY: &str = "i4HkuymMZBaF7oYqE2hxmdQVHrgKxH6imH";
        const NATIVE_TO_IDENTITY: &str =
            "050403000000cc1b0403000101150408eecdbbae7b3a704053efb295f68b95cd49d71d75";
        const IDENTITY_OWN_OUTPUT: &str = "1b0403000101150408eecdbbae7b3a704053efb295f68b95cd49d71dcc4c7404030a0101150408eecdbbae7b3a704053efb295f68b95cd49d71d4c57010000000a646c7367696674716131a6ef9ea235635e328124ff3429db9f9e91b64e2d00000000000000000000000000000000000000003ec71906776f6f07f9f959559a9933af76784195321a0eb834c1088e8b59119b75";
        let raw = json!([
            entry(
                "aa",
                5_000_000,
                NATIVE_TO_IDENTITY,
                json!({ TESTNET: 0.05 })
            ),
            entry("bb", 0, IDENTITY_OWN_OUTPUT, json!({})),
            entry(
                "cc",
                3_000_000,
                NATIVE_TO_IDENTITY,
                json!({ TESTNET: 0.03, "iTokenCurrency1111111111111111111": 1 })
            ),
            entry("dd", 4_000_000, PLAIN_SCRIPT, json!({})),
        ]);
        let fees = identity_fee_outputs(&raw, TESTNET, IDENTITY);
        assert_eq!(
            fees.iter()
                .map(|utxo| utxo.txid.as_str())
                .collect::<Vec<_>>(),
            ["aa"]
        );
        // The same output does not pay for another VerusID.
        assert!(
            identity_fee_outputs(&raw, TESTNET, "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq").is_empty()
        );
    }
}
