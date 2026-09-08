//
// Identity send flow: sign all signable inputs from preflight payload and broadcast.

use std::sync::Arc;

use bitcoin::secp256k1::{Message, Secp256k1};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::core::auth::{
    capture_active_wallet_access_context, ensure_active_wallet_session,
    load_primary_private_scalar_for_context, SessionManager,
};
use crate::core::channels::store::PreflightStore;
use crate::core::channels::vrpc::identity::preflight::{
    IdentityPreflightPayload, IdentitySignMode,
};
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
use crate::core::channels::vrpc::VrpcProviderPool;
use crate::core::crypto::wif_encoding::{decode_wif, Network};
use crate::types::wallet::WalletNetwork;
use crate::types::{IdentitySendResult, WalletError};

fn parse_txid_from_result(v: &Value) -> Option<String> {
    if let Some(s) = v.as_str() {
        return Some(s.to_string());
    }
    v.get("txid")?.as_str().map(ToString::to_string)
}

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

    let signed_hex = sign_payload(&payload, private_key)?;
    if let Some(session_manager) = session_manager {
        ensure_active_wallet_session(session_manager, expected_session_id).await?;
    }
    let provider = provider_pool.for_network(wallet_network);
    let txid_raw = provider.sendrawtransaction(&signed_hex).await?;
    let txid = parse_txid_from_result(&txid_raw).ok_or(WalletError::IdentityBuildFailed)?;

    Ok(IdentitySendResult {
        txid,
        operation: payload.operation,
        target_identity: payload.target_identity,
        fee: payload.fee,
        from_address: payload.from_address,
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
