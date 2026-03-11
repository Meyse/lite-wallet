//
// Module 5d: BTC send — load preflight record, fetch key from Stronghold on demand, sign, broadcast.

use std::io::Cursor;
use std::sync::Arc;

use bitcoin::consensus::{Decodable, Encodable};
use bitcoin::hashes::Hash;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::ScriptBuf;
use tokio::sync::Mutex;

use crate::core::auth::{
    capture_active_wallet_access_context, load_primary_private_scalar_for_context, SessionManager,
};
use crate::core::channels::btc::preflight::BtcPreflightPayload;
use crate::core::channels::btc::provider::BtcProviderPool;
use crate::core::channels::store::PreflightStore;
use crate::types::transaction::SendResult;
use crate::types::WalletError;

const SIGHASH_ALL: u32 = 1u32;

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

/// Sign and broadcast a BTC preflight using on-demand Stronghold key access.
pub async fn send(
    preflight_id: &str,
    preflight_store: &PreflightStore,
    session_manager: &Arc<Mutex<SessionManager>>,
    provider_pool: &BtcProviderPool,
) -> Result<SendResult, WalletError> {
    let record = preflight_store
        .take(preflight_id)
        .ok_or(WalletError::InvalidPreflight)?;

    let context = capture_active_wallet_access_context(session_manager).await?;
    if context.account_id != record.account_id {
        return Err(WalletError::InvalidPreflight);
    }
    let wallet_network = context.wallet_network;
    let private_key = load_primary_private_scalar_for_context(&context).await?;

    let payload: BtcPreflightPayload = serde_json::from_value(record.payload.clone())
        .map_err(|_| WalletError::InvalidPreflight)?;

    let secp = Secp256k1::new();
    let secret_key = bitcoin::secp256k1::SecretKey::from_slice(&*private_key)
        .map_err(|_| WalletError::OperationFailed)?;
    let public_key = bitcoin::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
    let script_pubkey = p2pkh_script(&public_key.serialize());

    let tx_bytes = hex::decode(payload.unsigned_hex.trim_start_matches("0x"))
        .or_else(|_| hex::decode(&payload.unsigned_hex))
        .map_err(|_| WalletError::OperationFailed)?;

    let mut cursor = Cursor::new(&tx_bytes[..]);
    let mut tx: bitcoin::Transaction = bitcoin::Transaction::consensus_decode(&mut cursor)
        .map_err(|_| WalletError::OperationFailed)?;

    for i in 0..tx.input.len() {
        let cache = bitcoin::sighash::SighashCache::new(&tx);
        let sighash = cache
            .legacy_signature_hash(i, &script_pubkey, SIGHASH_ALL)
            .map_err(|_| WalletError::OperationFailed)?;
        let msg = bitcoin::secp256k1::Message::from_digest(sighash.to_byte_array());
        let sig = secp.sign_ecdsa(&msg, &secret_key);
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
    let signed_hex = hex::encode(&signed);

    let provider = provider_pool.for_network(wallet_network);
    let txid = provider.broadcast(&signed_hex).await?;
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
