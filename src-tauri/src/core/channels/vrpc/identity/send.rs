//
// Identity send flow: sign all signable inputs from preflight payload and broadcast.

use std::sync::Arc;

use bitcoin::secp256k1::{Message, Secp256k1};
use tokio::sync::Mutex;

use crate::core::auth::{
    capture_active_wallet_access_context, ensure_active_wallet_session,
    load_primary_private_scalar_for_context, SessionManager,
};
use crate::core::channels::store::PreflightStore;
use crate::core::channels::vrpc::common::parse_txid_from_result;
use crate::core::channels::vrpc::identity::preflight::{
    IdentityPreflightPayload, IdentitySignMode,
};
use crate::core::channels::vrpc::identity::profile::intent::validate as validate_profile_intent;
use crate::core::channels::vrpc::identity::verus_tx::codec::{
    decode_hex as decode_verus_tx, encode_hex as encode_verus_tx_hex,
};
use crate::core::channels::vrpc::identity::verus_tx::script::{
    build_p2pkh_script_sig, build_single_push_script_sig,
};
use crate::core::channels::vrpc::identity::verus_tx::sighash::{
    signature_hash as zcash_signature_hash, SIGHASH_ALL,
};
use crate::core::channels::vrpc::identity::verus_tx::smart_sig::build_single_signature_chunk;
use crate::core::channels::vrpc::intent::validate_identity_transaction_intent;
use crate::core::channels::vrpc::VrpcProviderPool;
use crate::core::crypto::wif_encoding::{decode_wif, Network};
use crate::types::wallet::WalletNetwork;
use crate::types::{IdentitySendResult, PendingIdentityProfileUpdate, WalletError};

pub async fn send(
    preflight_id: &str,
    preflight_store: &PreflightStore,
    session_manager: &Arc<Mutex<SessionManager>>,
    provider_pool: &VrpcProviderPool,
) -> Result<IdentitySendResult, WalletError> {
    let context = capture_active_wallet_access_context(session_manager).await?;
    let account_id = context.account_id.clone();
    let wallet_network = context.wallet_network;
    let private_key = load_primary_private_scalar_for_context(&context).await?;

    send_with_private_key_material(
        preflight_id,
        preflight_store,
        &account_id,
        &context.session_id,
        &private_key,
        wallet_network,
        Some(session_manager),
        provider_pool,
    )
    .await
}

pub async fn send_with_signing_material(
    preflight_id: &str,
    preflight_store: &PreflightStore,
    expected_account_id: &str,
    expected_session_id: &str,
    wif: &str,
    wallet_network: WalletNetwork,
    provider_pool: &VrpcProviderPool,
) -> Result<IdentitySendResult, WalletError> {
    let wif_network = match wallet_network {
        WalletNetwork::Mainnet => Network::Mainnet,
        WalletNetwork::Testnet => Network::Testnet,
    };
    let private_key = decode_wif(wif, wif_network)?;

    send_with_private_key_material(
        preflight_id,
        preflight_store,
        expected_account_id,
        expected_session_id,
        &private_key,
        wallet_network,
        None,
        provider_pool,
    )
    .await
}

pub async fn send_with_private_key_material(
    preflight_id: &str,
    preflight_store: &PreflightStore,
    expected_account_id: &str,
    expected_session_id: &str,
    private_key: &[u8; 32],
    wallet_network: WalletNetwork,
    session_manager: Option<&Arc<Mutex<SessionManager>>>,
    provider_pool: &VrpcProviderPool,
) -> Result<IdentitySendResult, WalletError> {
    let record = preflight_store
        .take(preflight_id, expected_session_id)
        .ok_or(WalletError::InvalidPreflight)?;
    if !record.channel_id.starts_with("vrpc.") || record.account_id != expected_account_id {
        return Err(WalletError::InvalidPreflight);
    }
    let payload: IdentityPreflightPayload =
        serde_json::from_value(record.payload).map_err(|_| WalletError::InvalidPreflight)?;

    let input_total = payload
        .signable_inputs
        .iter()
        .try_fold(0i64, |total, input| total.checked_add(input.satoshis))
        .ok_or(WalletError::InvalidPreflight)?;
    let fee_sats = crate::core::channels::vrpc::common::parse_positive_amount_sat(&payload.fee)
        .map_err(|_| WalletError::InvalidPreflight)?;
    if let Some(profile_intent) = payload.profile_intent.as_ref() {
        validate_profile_intent(
            &payload.unsigned_hex,
            profile_intent,
            &payload.from_address,
            input_total,
            fee_sats,
        )
        .map_err(|_| WalletError::InvalidPreflight)?;
    } else {
        validate_identity_transaction_intent(
            &payload.unsigned_hex,
            &payload.control_intent,
            &payload.from_address,
            input_total,
            fee_sats,
        )
        .map_err(|_| WalletError::InvalidPreflight)?;
    }

    let signed_hex = sign_payload(&payload, private_key)?;
    let provider = provider_pool.for_network(wallet_network);
    if let Some(profile_intent) = payload.profile_intent.as_ref() {
        let current = provider
            .getidentity_for_profile(&payload.target_identity)
            .await
            .map_err(|_| WalletError::InvalidPreflight)?;
        let current_txid = current.get("txid").and_then(serde_json::Value::as_str);
        let current_vout = current
            .get("vout")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u32::try_from(value).ok());
        if current_txid != Some(profile_intent.identity_txid.as_str())
            || current_vout != Some(profile_intent.identity_vout)
        {
            return Err(WalletError::InvalidPreflight);
        }
    }
    // The profile outpoint lookup can outlive the wallet session. Recheck only
    // after it completes, immediately before crossing the broadcast boundary.
    if let Some(session_manager) = session_manager {
        ensure_active_wallet_session(session_manager, expected_session_id).await?;
    }
    let expected_txid = super::profile::publication::signed_txid(&signed_hex)?;
    let profile_update =
        payload
            .profile_intent
            .as_ref()
            .map(|intent| PendingIdentityProfileUpdate {
                identity_address: payload.target_identity.clone(),
                txid: expected_txid.clone(),
                submitted_at: super::profile::publication::now(),
                previous_profile: intent.previous_profile.clone(),
                proposed_profile: intent.proposed_profile.clone(),
            });
    let access = if let Some(manager) = session_manager {
        let access = capture_active_wallet_access_context(manager).await?;
        if access.session_id != expected_session_id || access.account_id != expected_account_id {
            return Err(WalletError::WalletLocked);
        }
        Some(access)
    } else {
        None
    };
    if let Some(intent) = payload.profile_intent.as_ref() {
        let binding = intent
            .publication
            .as_ref()
            .ok_or(WalletError::InvalidPreflight)?;
        let access = access.as_ref().ok_or(WalletError::InvalidPreflight)?;
        super::profile::publication::admit_submission(
            access,
            binding,
            profile_update
                .clone()
                .ok_or(WalletError::InvalidPreflight)?,
            decode_verus_tx(&signed_hex)?.expiry_height,
        )
        .await?;
    }
    let txid_raw = if let Some(access) = access.as_ref() {
        access
            .session_submission_guard()
            .run(provider.sendrawtransaction(&signed_hex))
            .await?
    } else {
        provider.sendrawtransaction(&signed_hex).await?
    };
    let txid = parse_txid_from_result(&txid_raw).ok_or(WalletError::IdentityBuildFailed)?;
    if txid != expected_txid {
        return Err(WalletError::IdentityBuildFailed);
    }

    Ok(IdentitySendResult {
        txid,
        operation: payload.operation,
        target_identity: payload.target_identity,
        fee: payload.fee,
        from_address: payload.from_address,
        profile_update,
    })
}

fn sign_payload(
    payload: &IdentityPreflightPayload,
    private_key: &[u8; 32],
) -> Result<String, WalletError> {
    let secp = Secp256k1::new();
    let secret_key = bitcoin::secp256k1::SecretKey::from_slice(private_key)
        .map_err(|_| WalletError::IdentitySignFailed)?;
    let public_key = bitcoin::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);

    let mut tx =
        decode_verus_tx(&payload.unsigned_hex).map_err(|_| WalletError::IdentitySignFailed)?;

    for signable in &payload.signable_inputs {
        if signable.input_index >= tx.inputs.len() || signable.satoshis < 0 {
            return Err(WalletError::IdentitySignFailed);
        }
        let prevout_script =
            hex::decode(&signable.script_pub_key).map_err(|_| WalletError::IdentitySignFailed)?;
        let value = signable.satoshis as u64;
        let sighash = zcash_signature_hash(
            &tx,
            signable.input_index,
            &prevout_script,
            value,
            SIGHASH_ALL,
        )?;
        let msg = Message::from_digest(sighash);
        let sig = secp.sign_ecdsa(&msg, &secret_key);

        let script_sig = match signable.sign_mode {
            IdentitySignMode::P2pkh => {
                let mut der_plus_hashtype = sig.serialize_der().to_vec();
                der_plus_hashtype.push(SIGHASH_ALL as u8);
                build_p2pkh_script_sig(&der_plus_hashtype, &public_key.serialize())?
            }
            IdentitySignMode::SmartTransaction => {
                let compact = sig.serialize_compact();
                let chunk = build_single_signature_chunk(
                    &public_key.serialize(),
                    &compact,
                    SIGHASH_ALL as u8,
                )?;
                build_single_push_script_sig(&chunk)?
            }
        };

        if let Some(input) = tx.inputs.get_mut(signable.input_index) {
            input.script_sig = script_sig;
        }
    }

    encode_verus_tx_hex(&tx).map_err(|_| WalletError::IdentitySignFailed)
}
