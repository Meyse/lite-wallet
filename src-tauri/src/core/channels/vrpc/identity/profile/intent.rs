use serde::{Deserialize, Serialize};

use crate::core::channels::vrpc::identity::verus_tx::codec::decode_hex;
use crate::core::channels::vrpc::identity::verus_tx::model::txid_hex_to_le_bytes;
use crate::core::channels::vrpc::intent::decode_base58_destination;
use crate::types::{IdentityProfileSnapshot, WalletError};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProfileTransactionIntent {
    pub identity_txid: String,
    pub identity_vout: u32,
    pub output_scripts: Vec<String>,
    pub output_values: Vec<u64>,
    pub avatar_digest: Option<String>,
    pub description_digest: Option<String>,
    pub previous_profile: IdentityProfileSnapshot,
    pub proposed_profile: IdentityProfileSnapshot,
}

fn p2pkh_script(address: &str) -> Result<Vec<u8>, WalletError> {
    let destination = decode_base58_destination(address)?;
    if destination.destination_type != 2 || destination.destination_bytes.len() != 20 {
        return Err(WalletError::InvalidPreflight);
    }
    let mut script = vec![0x76, 0xa9, 0x14];
    script.extend_from_slice(&destination.destination_bytes);
    script.extend_from_slice(&[0x88, 0xac]);
    Ok(script)
}

pub(crate) fn validate(
    tx_hex: &str,
    intent: &ProfileTransactionIntent,
    change_address: &str,
    input_total_sats: i64,
    expected_fee_sats: i64,
) -> Result<(), WalletError> {
    let tx = decode_hex(tx_hex).map_err(|_| WalletError::InvalidPreflight)?;
    if intent.output_scripts.len() != intent.output_values.len()
        || tx.outputs.len() < intent.output_scripts.len()
        || tx.outputs.len() > intent.output_scripts.len() + 1
    {
        return Err(WalletError::InvalidPreflight);
    }

    let identity_txid =
        txid_hex_to_le_bytes(&intent.identity_txid).map_err(|_| WalletError::InvalidPreflight)?;
    if tx
        .inputs
        .iter()
        .filter(|input| {
            input.prevout_txid_le == identity_txid && input.prevout_vout == intent.identity_vout
        })
        .count()
        != 1
    {
        return Err(WalletError::InvalidPreflight);
    }

    let mut output_total = 0i64;
    for (index, expected_script) in intent.output_scripts.iter().enumerate() {
        let output = tx.outputs.get(index).ok_or(WalletError::InvalidPreflight)?;
        let expected = hex::decode(expected_script).map_err(|_| WalletError::InvalidPreflight)?;
        if output.script_pub_key != expected || output.value != intent.output_values[index] {
            return Err(WalletError::InvalidPreflight);
        }
        output_total = output_total
            .checked_add(i64::try_from(output.value).map_err(|_| WalletError::InvalidPreflight)?)
            .ok_or(WalletError::InvalidPreflight)?;
    }

    if let Some(change) = tx.outputs.get(intent.output_scripts.len()) {
        if change.script_pub_key != p2pkh_script(change_address)? || change.value == 0 {
            return Err(WalletError::InvalidPreflight);
        }
        output_total = output_total
            .checked_add(i64::try_from(change.value).map_err(|_| WalletError::InvalidPreflight)?)
            .ok_or(WalletError::InvalidPreflight)?;
    }
    let fee = input_total_sats
        .checked_sub(output_total)
        .ok_or(WalletError::InvalidPreflight)?;
    if fee != expected_fee_sats || fee <= 0 {
        return Err(WalletError::InvalidPreflight);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::channels::vrpc::identity::verus_tx::codec::encode_hex;
    use crate::core::channels::vrpc::identity::verus_tx::model::{
        VerusTx, VerusTxIn, VerusTxOut, SAPLING_VERSION_GROUP_ID,
    };

    fn empty_snapshot() -> IdentityProfileSnapshot {
        IdentityProfileSnapshot {
            avatar_base64: None,
            avatar_digest: None,
            description: None,
            description_digest: None,
        }
    }

    fn fixture() -> (VerusTx, ProfileTransactionIntent, String) {
        let change_address = "RLcoqsCLBQJPvciM1EvFzXH9p42Y61AtiB".to_string();
        let identity_txid = "11".repeat(32);
        let identity_script = vec![0x51, 0x21];
        let evidence_script = vec![0x52, 0x22];
        let tx = VerusTx {
            version: 4,
            overwintered: true,
            version_group_id: SAPLING_VERSION_GROUP_ID,
            inputs: vec![VerusTxIn {
                prevout_txid_le: txid_hex_to_le_bytes(&identity_txid).expect("txid"),
                prevout_vout: 3,
                script_sig: Vec::new(),
                sequence: u32::MAX,
            }],
            outputs: vec![
                VerusTxOut {
                    value: 0,
                    script_pub_key: identity_script.clone(),
                },
                VerusTxOut {
                    value: 0,
                    script_pub_key: evidence_script.clone(),
                },
                VerusTxOut {
                    value: 950,
                    script_pub_key: p2pkh_script(&change_address).expect("change script"),
                },
            ],
            lock_time: 0,
            expiry_height: 1,
            value_balance: 0,
        };
        let intent = ProfileTransactionIntent {
            identity_txid,
            identity_vout: 3,
            output_scripts: vec![hex::encode(identity_script), hex::encode(evidence_script)],
            output_values: vec![0, 0],
            avatar_digest: Some("aa".repeat(32)),
            description_digest: None,
            previous_profile: empty_snapshot(),
            proposed_profile: empty_snapshot(),
        };
        (tx, intent, change_address)
    }

    #[test]
    fn accepts_only_bound_profile_outputs_identity_outpoint_change_and_fee() {
        let (tx, intent, change_address) = fixture();
        let tx_hex = encode_hex(&tx).expect("encode");
        validate(&tx_hex, &intent, &change_address, 1_000, 50).expect("valid intent");

        let mut changed_output = tx.clone();
        changed_output.outputs[1].script_pub_key.push(0xff);
        assert!(validate(
            &encode_hex(&changed_output).expect("encode"),
            &intent,
            &change_address,
            1_000,
            50
        )
        .is_err());

        let mut extra_output = tx.clone();
        extra_output.outputs.push(VerusTxOut {
            value: 0,
            script_pub_key: vec![0x53],
        });
        assert!(validate(
            &encode_hex(&extra_output).expect("encode"),
            &intent,
            &change_address,
            1_000,
            50
        )
        .is_err());

        let mut changed_outpoint = tx;
        changed_outpoint.inputs[0].prevout_vout = 4;
        assert!(validate(
            &encode_hex(&changed_outpoint).expect("encode"),
            &intent,
            &change_address,
            1_000,
            50
        )
        .is_err());
        assert!(validate(&tx_hex, &intent, &change_address, 1_000, 51).is_err());
    }
}
