//
// Module 5d: BTC preflight — validate address, select UTXOs, build unsigned tx, store payload (preflight_id only to UI).

use std::str::FromStr;

use bitcoin::absolute::LockTime;
use bitcoin::address::NetworkUnchecked;
use bitcoin::blockdata::script::Builder;
use bitcoin::blockdata::transaction::{OutPoint, Transaction, TxIn, TxOut};
use bitcoin::consensus::Encodable;
use bitcoin::{Amount, Network as BtcNetwork, ScriptBuf, Sequence, Txid};
use bs58;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::channels::btc::provider::{BtcProvider, UtxoEntry};
use crate::core::channels::store::{PreflightRecord, PreflightStore};
use crate::types::transaction::{PreflightParams, PreflightResult, PreflightWarning};
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

const SATOSHI_PER_COIN: f64 = 100_000_000.0;
const DUST_SATOSHI: u64 = 1000;
const DEFAULT_FEE_SAT: u64 = 2000;

/// Payload stored in PreflightStore for BTC send. Not sent to frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtcPreflightPayload {
    pub unsigned_hex: String,
    pub to_address: String,
    pub from_address: String,
    pub value: String,
    pub fee: String,
    pub inputs: Vec<BtcInputRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtcInputRef {
    pub txid: String,
    pub vout: u32,
    pub value: u64,
}

fn expected_btc_network(network: WalletNetwork) -> BtcNetwork {
    match network {
        WalletNetwork::Mainnet => BtcNetwork::Bitcoin,
        WalletNetwork::Testnet => BtcNetwork::Testnet,
    }
}

/// Validate source Bitcoin address as legacy P2PKH for the selected network.
///
/// The current signer implementation is P2PKH-only for inputs, so source must remain P2PKH.
fn validate_btc_source_address(
    addr: &str,
    network: WalletNetwork,
) -> Result<[u8; 20], WalletError> {
    let trimmed = addr.trim();
    if trimmed.is_empty() || trimmed.len() > 35 {
        return Err(WalletError::InvalidAddress);
    }
    let expected_version = match network {
        WalletNetwork::Mainnet => 0x00u8,
        WalletNetwork::Testnet => 0x6Fu8,
    };
    let decoded = bs58::decode(trimmed)
        .with_check(None)
        .into_vec()
        .map_err(|_| WalletError::InvalidAddress)?;
    if decoded.len() != 21 || decoded[0] != expected_version {
        return Err(WalletError::InvalidAddress);
    }
    let mut hash = [0u8; 20];
    hash.copy_from_slice(&decoded[1..21]);
    Ok(hash)
}

/// Parse destination address (P2PKH/P2SH/Bech32) and return its scriptPubKey.
fn parse_btc_destination_script(
    addr: &str,
    network: WalletNetwork,
) -> Result<ScriptBuf, WalletError> {
    let trimmed = addr.trim();
    if trimmed.is_empty() {
        return Err(WalletError::InvalidAddress);
    }

    let parsed = trimmed
        .parse::<bitcoin::Address<NetworkUnchecked>>()
        .map_err(|_| WalletError::InvalidAddress)?;
    let checked = parsed
        .require_network(expected_btc_network(network))
        .map_err(|_| WalletError::InvalidAddress)?;
    Ok(checked.script_pubkey())
}

fn p2pkh_script_from_hash160(hash: &[u8; 20]) -> ScriptBuf {
    use bitcoin::blockdata::opcodes::all::*;
    Builder::new()
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(hash)
        .push_opcode(OP_EQUALVERIFY)
        .push_opcode(OP_CHECKSIG)
        .into_script()
}

fn resolve_send_value_after_fee(
    submitted_sat: u64,
    total_sat: u64,
    fee_sat: u64,
) -> Result<(u64, bool, Option<String>), WalletError> {
    if submitted_sat == 0 || fee_sat == 0 || total_sat == 0 {
        return Err(WalletError::InsufficientFunds);
    }

    if total_sat < submitted_sat {
        return Err(WalletError::InsufficientFunds);
    }

    let needed = submitted_sat.saturating_add(fee_sat);
    if total_sat >= needed {
        return Ok((submitted_sat, false, None));
    }

    let adjusted = total_sat.saturating_sub(fee_sat);
    if adjusted == 0 {
        return Err(WalletError::InsufficientFunds);
    }

    Ok((
        adjusted,
        true,
        Some("Fee was deducted from the submitted amount due to available balance.".to_string()),
    ))
}

/// Run BTC preflight: validate, fetch UTXOs, build unsigned tx, store record, return PreflightResult.
pub async fn preflight(
    params: PreflightParams,
    preflight_store: &PreflightStore,
    account_id: &str,
    session_id: &str,
    from_address: &str,
    channel_id: &str,
    provider: &BtcProvider,
    network: WalletNetwork,
) -> Result<PreflightResult, WalletError> {
    let to_script = parse_btc_destination_script(&params.to_address, network)?;
    validate_btc_source_address(from_address, network)?;

    let amount_sat = params
        .amount
        .trim()
        .parse::<f64>()
        .map_err(|_| WalletError::OperationFailed)?
        * SATOSHI_PER_COIN;
    if amount_sat <= 0.0 {
        return Err(WalletError::OperationFailed);
    }
    let amount_sat = amount_sat as u64;

    let utxos = provider.get_utxos(from_address).await?;
    if utxos.is_empty() {
        return Err(WalletError::InsufficientFunds);
    }

    let fee_sat = DEFAULT_FEE_SAT;
    let needed = amount_sat.saturating_add(fee_sat);
    let mut selected: Vec<&UtxoEntry> = Vec::new();
    let mut total: u64 = 0;
    for u in &utxos {
        selected.push(u);
        total = total.saturating_add(u.value);
        if total >= needed {
            break;
        }
    }
    let (send_value_sat, fee_taken_from_amount, fee_taken_message) =
        resolve_send_value_after_fee(amount_sat, total, fee_sat)?;

    let change = total.saturating_sub(send_value_sat).saturating_sub(fee_sat);
    let from_hash = validate_btc_source_address(from_address, network)?;
    let from_script = p2pkh_script_from_hash160(&from_hash);

    let mut inputs: Vec<TxIn> = Vec::new();
    let mut payload_inputs: Vec<BtcInputRef> = Vec::new();
    for u in &selected {
        let txid = Txid::from_str(&u.txid).map_err(|_| WalletError::OperationFailed)?;
        inputs.push(TxIn {
            previous_output: OutPoint { txid, vout: u.vout },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Default::default(),
        });
        payload_inputs.push(BtcInputRef {
            txid: u.txid.clone(),
            vout: u.vout,
            value: u.value,
        });
    }

    let mut outputs: Vec<TxOut> = Vec::new();
    outputs.push(TxOut {
        value: Amount::from_sat(send_value_sat),
        script_pubkey: to_script,
    });
    if change >= DUST_SATOSHI {
        outputs.push(TxOut {
            value: Amount::from_sat(change),
            script_pubkey: from_script,
        });
    }

    let tx = Transaction {
        version: bitcoin::blockdata::transaction::Version::TWO,
        lock_time: LockTime::ZERO,
        input: inputs,
        output: outputs,
    };
    let mut raw = Vec::new();
    tx.consensus_encode(&mut raw)
        .map_err(|_| WalletError::OperationFailed)?;
    let unsigned_hex = hex::encode(&raw);

    let preflight_id = Uuid::new_v4().to_string();
    let value_str = format!("{:.8}", send_value_sat as f64 / SATOSHI_PER_COIN);
    let fee_str = format!("{:.8}", fee_sat as f64 / SATOSHI_PER_COIN);
    let payload = BtcPreflightPayload {
        unsigned_hex: unsigned_hex.clone(),
        to_address: params.to_address.clone(),
        from_address: from_address.to_string(),
        value: value_str.clone(),
        fee: fee_str.clone(),
        inputs: payload_inputs,
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

    let warnings: Vec<PreflightWarning> = if change > 0 && change < DUST_SATOSHI {
        vec![PreflightWarning {
            warning_type: "dust_change".to_string(),
            message: "Change below dust threshold is added to fee.".to_string(),
        }]
    } else {
        vec![]
    };

    Ok(PreflightResult {
        preflight_id,
        fee: fee_str,
        fee_currency: params.coin_id.clone(),
        value: value_str,
        amount_submitted: params.amount,
        to_address: params.to_address,
        from_address: from_address.to_string(),
        fee_taken_from_amount,
        fee_taken_message,
        warnings,
        memo: params.memo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_btc_destination_script_accepts_mainnet_bech32() {
        let script = parse_btc_destination_script(
            "bc1qggqzj0uzun238nhzzs5wdz2en05s0d9ncwhxcf",
            WalletNetwork::Mainnet,
        )
        .expect("mainnet bech32 address should be valid");
        assert!(script.is_witness_program());
    }

    #[test]
    fn parse_btc_destination_script_rejects_network_mismatch() {
        let result = parse_btc_destination_script(
            "bc1qggqzj0uzun238nhzzs5wdz2en05s0d9ncwhxcf",
            WalletNetwork::Testnet,
        );
        assert!(matches!(result, Err(WalletError::InvalidAddress)));
    }

    #[test]
    fn parse_btc_destination_script_accepts_testnet_bech32() {
        let script = parse_btc_destination_script(
            "tb1qggqzj0uzun238nhzzs5wdz2en05s0d9njgv4r6",
            WalletNetwork::Testnet,
        )
        .expect("testnet bech32 address should be valid");
        assert!(script.is_witness_program());
    }

    #[test]
    fn resolve_send_value_after_fee_keeps_submitted_when_total_covers_fee() {
        let (value, adjusted, message) =
            resolve_send_value_after_fee(100_000, 102_000, 2_000).expect("should resolve");
        assert_eq!(value, 100_000);
        assert!(!adjusted);
        assert!(message.is_none());
    }

    #[test]
    fn resolve_send_value_after_fee_adjusts_when_fee_must_be_deducted() {
        let (value, adjusted, message) =
            resolve_send_value_after_fee(100_000, 100_000, 2_000).expect("should resolve");
        assert_eq!(value, 98_000);
        assert!(adjusted);
        assert!(message.is_some());
    }

    #[test]
    fn resolve_send_value_after_fee_fails_when_total_below_submitted() {
        let result = resolve_send_value_after_fee(100_000, 99_000, 2_000);
        assert!(matches!(result, Err(WalletError::InsufficientFunds)));
    }
}
