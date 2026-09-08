//
// Module 5: VRPC send — load preflight record, fetch key from Stronghold on demand, sign, broadcast.

use std::io::Cursor;
use std::sync::Arc;

use bitcoin::consensus::{Decodable, Encodable};
use bitcoin::hashes::Hash;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::ScriptBuf;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::core::auth::{
    capture_active_wallet_access_context, ensure_active_wallet_session,
    load_primary_private_scalar_for_context, SessionManager,
};
use crate::core::channels::store::PreflightStore;
use crate::core::channels::vrpc::identity::verus_tx::codec::{
    decode_hex as decode_verus_tx, encode_hex as encode_verus_tx_hex,
};
use crate::core::channels::vrpc::identity::verus_tx::model::{txid_le_bytes_to_hex, InputSignMode};
use crate::core::channels::vrpc::identity::verus_tx::script::{
    build_p2pkh_script_sig, build_single_push_script_sig, classify_prevout_script,
};
use crate::core::channels::vrpc::identity::verus_tx::sighash::{
    signature_hash as zcash_signature_hash, SIGHASH_ALL as ZCASH_SIGHASH_ALL,
};
use crate::core::channels::vrpc::identity::verus_tx::smart_sig::build_single_signature_chunk;
use crate::core::channels::vrpc::parse_vrpc_channel_id;
use crate::core::channels::vrpc::preflight::VrpcPreflightPayload;
use crate::core::channels::vrpc::provider::VrpcProviderPool;
use crate::types::transaction::SendResult;
use crate::types::WalletError;

const SIGHASH_ALL: u32 = 1u32;

/// Sign and broadcast a VRPC preflight. Never logs transaction hex or keys.
pub async fn send(
    preflight_id: &str,
    preflight_store: &PreflightStore,
    session_manager: &Arc<Mutex<SessionManager>>,
    provider_pool: &VrpcProviderPool,
) -> Result<SendResult, WalletError> {
    let context = capture_active_wallet_access_context(session_manager).await?;
    let record = preflight_store
        .take(preflight_id, &context.session_id)
        .ok_or(WalletError::InvalidPreflight)?;

    if context.account_id != record.account_id {
        return Err(WalletError::InvalidPreflight);
    }
    let wallet_network = context.wallet_network;
    let private_key = load_primary_private_scalar_for_context(&context).await?;

    let payload: VrpcPreflightPayload = serde_json::from_value(record.payload.clone())
        .map_err(|_| WalletError::InvalidPreflight)?;

    let secp = Secp256k1::new();
    let secret_key = bitcoin::secp256k1::SecretKey::from_slice(&*private_key)
        .map_err(|_| WalletError::OperationFailed)?;
    let public_key = bitcoin::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);

    let signed_hex = sign_payload(&payload, &secp, &secret_key, &public_key)?;
    ensure_active_wallet_session(session_manager, &context.session_id).await?;

    let resolved_channel = parse_vrpc_channel_id(&record.channel_id, Some(&payload.from_address))?;
    let provider = provider_pool.for_system(wallet_network, &resolved_channel.system_id);
    let txid_value = provider.sendrawtransaction(&signed_hex).await?;
    let txid = parse_txid_from_result(&txid_value).unwrap_or_default();
    if txid.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    Ok(SendResult {
        txid,
        fee: payload.fee,
        value: payload.value,
        to_address: payload.to_address,
        from_address: payload.from_address,
    })
}

fn sign_payload(
    payload: &VrpcPreflightPayload,
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    secret_key: &bitcoin::secp256k1::SecretKey,
    public_key: &bitcoin::secp256k1::PublicKey,
) -> Result<String, WalletError> {
    if decode_verus_tx(&payload.hex).is_ok() {
        return sign_payload_overwinter(payload, secp, secret_key, public_key);
    }
    sign_payload_legacy(payload, secp, secret_key, public_key)
}

fn sign_payload_legacy(
    payload: &VrpcPreflightPayload,
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    secret_key: &bitcoin::secp256k1::SecretKey,
    public_key: &bitcoin::secp256k1::PublicKey,
) -> Result<String, WalletError> {
    let tx_bytes = hex::decode(payload.hex.trim_start_matches("0x"))
        .or_else(|_| hex::decode(&payload.hex))
        .map_err(|_| WalletError::OperationFailed)?;
    let mut cursor = Cursor::new(&tx_bytes[..]);
    let mut tx: bitcoin::Transaction = bitcoin::Transaction::consensus_decode(&mut cursor)
        .map_err(|_| WalletError::OperationFailed)?;

    let fallback_script_pubkey = p2pkh_script(&public_key.serialize());
    for i in 0..tx.input.len() {
        let prevout = tx
            .input
            .get(i)
            .ok_or(WalletError::OperationFailed)?
            .previous_output;
        let txid = prevout.txid.to_string();
        let vout = prevout.vout;
        let script_pubkey = find_payload_input(&payload.inputs, &txid, vout)
            .and_then(|input| input.script_pub_key.clone())
            .and_then(|raw| hex::decode(raw).ok())
            .map(ScriptBuf::from_bytes)
            .unwrap_or_else(|| fallback_script_pubkey.clone());
        let cache = bitcoin::sighash::SighashCache::new(&tx);
        let sighash = cache
            .legacy_signature_hash(i, &script_pubkey, SIGHASH_ALL)
            .map_err(|_| WalletError::OperationFailed)?;
        let msg = bitcoin::secp256k1::Message::from_digest(sighash.to_byte_array());
        let sig = secp.sign_ecdsa(&msg, secret_key);
        let sig_der = sig.serialize_der();
        let mut sig_bytes = sig_der.to_vec();
        sig_bytes.push(SIGHASH_ALL as u8);

        let mut script_sig = ScriptBuf::new();
        push_slice_from_vec(&mut script_sig, &sig_bytes)?;
        script_sig.push_slice(&public_key.serialize());

        if let Some(txin) = tx.input.get_mut(i) {
            txin.script_sig = script_sig;
        }
    }

    let mut signed = Vec::new();
    tx.consensus_encode(&mut signed)
        .map_err(|_| WalletError::OperationFailed)?;
    Ok(hex::encode(&signed))
}

fn sign_payload_overwinter(
    payload: &VrpcPreflightPayload,
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    secret_key: &bitcoin::secp256k1::SecretKey,
    public_key: &bitcoin::secp256k1::PublicKey,
) -> Result<String, WalletError> {
    let mut tx = decode_verus_tx(&payload.hex).map_err(|_| WalletError::OperationFailed)?;

    for input_index in 0..tx.inputs.len() {
        let txid = txid_le_bytes_to_hex(&tx.inputs[input_index].prevout_txid_le);
        let vout = tx.inputs[input_index].prevout_vout;
        let payload_input =
            find_payload_input(&payload.inputs, &txid, vout).ok_or(WalletError::OperationFailed)?;
        let prevout_script = payload_input
            .script_pub_key
            .as_ref()
            .ok_or(WalletError::OperationFailed)
            .and_then(|raw| hex::decode(raw).map_err(|_| WalletError::OperationFailed))?;
        let prevout_value =
            u64::try_from(payload_input.satoshis).map_err(|_| WalletError::OperationFailed)?;
        let sighash = zcash_signature_hash(
            &tx,
            input_index,
            &prevout_script,
            prevout_value,
            ZCASH_SIGHASH_ALL,
        )
        .map_err(|_| WalletError::OperationFailed)?;
        let msg = bitcoin::secp256k1::Message::from_digest(sighash);
        let sig = secp.sign_ecdsa(&msg, secret_key);

        let script_sig = match classify_prevout_script(&prevout_script)
            .map_err(|_| WalletError::OperationFailed)?
        {
            InputSignMode::P2pkh => {
                let mut der_plus_hashtype = sig.serialize_der().to_vec();
                der_plus_hashtype.push(ZCASH_SIGHASH_ALL as u8);
                build_p2pkh_script_sig(&der_plus_hashtype, &public_key.serialize())
                    .map_err(|_| WalletError::OperationFailed)?
            }
            InputSignMode::SmartTransaction => {
                let compact_sig = sig.serialize_compact();
                let chunk = build_single_signature_chunk(
                    &public_key.serialize(),
                    &compact_sig,
                    ZCASH_SIGHASH_ALL as u8,
                )
                .map_err(|_| WalletError::OperationFailed)?;
                build_single_push_script_sig(&chunk).map_err(|_| WalletError::OperationFailed)?
            }
        };

        if let Some(input) = tx.inputs.get_mut(input_index) {
            input.script_sig = script_sig;
        }
    }

    encode_verus_tx_hex(&tx).map_err(|_| WalletError::OperationFailed)
}

/// Push bytes from a slice onto a script. Uses fixed-size array for PushBytesBuf (bitcoin crate requires &[u8; N]).
fn push_slice_from_vec(script: &mut ScriptBuf, bytes: &[u8]) -> Result<(), WalletError> {
    if bytes.is_empty() || bytes.len() > 80 {
        return Err(WalletError::OperationFailed);
    }
    let len = bytes.len();
    let push = match len {
        68 => {
            let mut a = [0u8; 68];
            a.copy_from_slice(bytes);
            bitcoin::script::PushBytesBuf::from(&a)
        }
        69 => {
            let mut a = [0u8; 69];
            a.copy_from_slice(bytes);
            bitcoin::script::PushBytesBuf::from(&a)
        }
        70 => {
            let mut a = [0u8; 70];
            a.copy_from_slice(bytes);
            bitcoin::script::PushBytesBuf::from(&a)
        }
        71 => {
            let mut a = [0u8; 71];
            a.copy_from_slice(bytes);
            bitcoin::script::PushBytesBuf::from(&a)
        }
        72 => {
            let mut a = [0u8; 72];
            a.copy_from_slice(bytes);
            bitcoin::script::PushBytesBuf::from(&a)
        }
        73 => {
            let mut a = [0u8; 73];
            a.copy_from_slice(bytes);
            bitcoin::script::PushBytesBuf::from(&a)
        }
        _ => return Err(WalletError::OperationFailed),
    };
    script.push_slice(&push);
    Ok(())
}

fn parse_txid_from_result(v: &Value) -> Option<String> {
    if let Some(s) = v.as_str() {
        return Some(s.to_string());
    }
    v.get("txid")?.as_str().map(String::from)
}

fn p2pkh_script(pubkey: &[u8]) -> ScriptBuf {
    use bitcoin::blockdata::opcodes::all::*;
    use bitcoin::blockdata::script::Builder;
    let hash = hash160(pubkey);
    Builder::new()
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(&hash)
        .push_opcode(OP_EQUALVERIFY)
        .push_opcode(OP_CHECKSIG)
        .into_script()
}

fn hash160(data: &[u8]) -> [u8; 20] {
    use ripemd::Digest;
    use sha2::Sha256;
    let sha = Sha256::digest(data);
    let ripemd = ripemd::Ripemd160::digest(&sha);
    let mut out = [0u8; 20];
    out.copy_from_slice(&ripemd[..]);
    out
}

fn find_payload_input<'a>(
    inputs: &'a [crate::core::channels::vrpc::preflight::VrpcInputRef],
    txid: &str,
    vout: u32,
) -> Option<&'a crate::core::channels::vrpc::preflight::VrpcInputRef> {
    inputs
        .iter()
        .find(|input| input.txid == txid && input.vout == vout)
}
