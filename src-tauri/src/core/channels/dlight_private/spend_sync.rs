//! Spend views and Sapling serialization shared by the single sync worker.
use super::spend_db::{
    StoredRseed, StoredRseedKind, StoredSpendNote, StoredSpendTree, StoredSpendWitness,
};
use super::DlightRuntimeRequest;
use crate::types::WalletError;
use zip32::Scope;

#[derive(Debug, Clone)]
pub struct SpendableNote {
    pub nullifier_hex: String,
    pub value_sats: u64,
    pub received_height: u64,
    pub txid: String,
    pub scope: Scope,
    pub note: sapling::Note,
    pub merkle_path: sapling::MerklePath,
}

#[derive(Debug, Clone)]
pub struct SpendSyncSnapshot {
    pub scanned_height: u64,
    pub chain_tip_height: u64,
    pub confirmed_balance_sats: u64,
    pub spendable_notes: Vec<SpendableNote>,
}

#[derive(Debug, Clone)]
pub struct SpendCacheStatus {
    pub ready: bool,
    pub scanned_height: u64,
    pub chain_tip_height: u64,
    pub effective_tip_height: u64,
    pub lag_blocks: u64,
    pub status_kind: String,
    pub percent: Option<f64>,
    pub last_updated: u64,
    pub note_count: u64,
    pub last_error: Option<String>,
}

pub fn load_spend_snapshot(
    request: &DlightRuntimeRequest,
    _tip: Option<u64>,
) -> Result<SpendSyncSnapshot, WalletError> {
    super::runtime::spend_snapshot(&request.runtime_key)
}

pub fn get_spend_cache_status(
    request: &DlightRuntimeRequest,
    _tip: Option<u64>,
) -> Option<SpendCacheStatus> {
    super::runtime::spend_cache_status(&request.runtime_key)
}

pub(super) fn encode_rseed(rseed: &sapling::Rseed) -> StoredRseed {
    match rseed {
        sapling::Rseed::BeforeZip212(rcm) => StoredRseed {
            kind: StoredRseedKind::BeforeZip212,
            bytes_hex: hex::encode(rcm.to_bytes()),
        },
        sapling::Rseed::AfterZip212(bytes) => StoredRseed {
            kind: StoredRseedKind::AfterZip212,
            bytes_hex: hex::encode(bytes),
        },
    }
}

fn decode_rseed(stored: &StoredRseed) -> Option<sapling::Rseed> {
    match stored.kind {
        StoredRseedKind::AfterZip212 => {
            let bytes = decode_fixed_hex::<32>(&stored.bytes_hex)?;
            Some(sapling::Rseed::AfterZip212(bytes))
        }
        StoredRseedKind::BeforeZip212 => {
            let bytes = decode_fixed_hex::<32>(&stored.bytes_hex)?;
            let rcm = Option::<jubjub::Fr>::from(jubjub::Fr::from_bytes(&bytes))?;
            Some(sapling::Rseed::BeforeZip212(rcm))
        }
    }
}

pub(super) fn decode_note(stored_note: &StoredSpendNote) -> Option<sapling::Note> {
    let recipient_bytes = decode_fixed_hex::<43>(&stored_note.recipient_bytes_hex)?;
    let recipient = sapling::PaymentAddress::from_bytes(&recipient_bytes)?;
    let rseed = decode_rseed(&stored_note.rseed)?;
    Some(sapling::Note::from_parts(
        recipient,
        sapling::value::NoteValue::from_raw(stored_note.value_sats),
        rseed,
    ))
}

pub(super) fn encode_witness(witness: &sapling::IncrementalWitness) -> StoredSpendWitness {
    StoredSpendWitness {
        tree: encode_tree(witness.tree()),
        filled: witness
            .filled()
            .iter()
            .map(|node| hex::encode(node.to_bytes()))
            .collect::<Vec<_>>(),
        cursor: witness.cursor().as_ref().map(encode_tree),
    }
}

pub(super) fn decode_witness(stored: &StoredSpendWitness) -> Option<sapling::IncrementalWitness> {
    let tree = decode_tree(&stored.tree)?;
    let filled = stored
        .filled
        .iter()
        .map(|encoded| decode_node_hex(encoded))
        .collect::<Option<Vec<_>>>()?;
    let cursor = match stored.cursor.as_ref() {
        Some(value) => Some(decode_tree(value)?),
        None => None,
    };

    sapling::IncrementalWitness::from_parts(tree, filled, cursor)
}

pub(super) fn encode_tree(tree: &sapling::CommitmentTree) -> StoredSpendTree {
    StoredSpendTree {
        left: tree
            .left()
            .as_ref()
            .map(|node| hex::encode(node.to_bytes())),
        right: tree
            .right()
            .as_ref()
            .map(|node| hex::encode(node.to_bytes())),
        parents: tree
            .parents()
            .iter()
            .map(|parent| parent.as_ref().map(|node| hex::encode(node.to_bytes())))
            .collect::<Vec<_>>(),
    }
}

pub(super) fn decode_tree(stored: &StoredSpendTree) -> Option<sapling::CommitmentTree> {
    let left = match stored.left.as_ref() {
        Some(value) => Some(decode_node_hex(value)?),
        None => None,
    };
    let right = match stored.right.as_ref() {
        Some(value) => Some(decode_node_hex(value)?),
        None => None,
    };
    let parents = stored
        .parents
        .iter()
        .map(|entry| match entry {
            Some(value) => Some(decode_node_hex(value)),
            None => Some(None),
        })
        .collect::<Option<Vec<_>>>()?;

    sapling::CommitmentTree::from_parts(left, right, parents).ok()
}

fn decode_node_hex(value: &str) -> Option<sapling::Node> {
    let bytes = decode_fixed_hex::<32>(value)?;
    Option::<sapling::Node>::from(sapling::Node::from_bytes(bytes))
}

fn decode_fixed_hex<const N: usize>(value: &str) -> Option<[u8; N]> {
    let bytes = hex::decode(value.trim()).ok()?;
    bytes.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::{
        decode_note, decode_tree, decode_witness, encode_rseed, encode_tree, encode_witness,
    };
    use crate::core::channels::dlight_private::spend_db::{
        StoredRseed, StoredRseedKind, StoredSpendTree, StoredSpendWitness,
    };

    #[test]
    fn encode_decode_tree_roundtrip_empty() {
        let tree = sapling::CommitmentTree::empty();
        let stored = encode_tree(&tree);
        let decoded = decode_tree(&stored).expect("decode tree");
        assert_eq!(decoded.size(), tree.size());
    }

    #[test]
    fn encode_decode_witness_roundtrip() {
        let mut tree = sapling::CommitmentTree::empty();
        let extsk = sapling::zip32::ExtendedSpendingKey::master(&[9u8; 32]);
        let recipient = extsk
            .to_diversifiable_full_viewing_key()
            .default_address()
            .1;
        let note = sapling::Note::from_parts(
            recipient,
            sapling::value::NoteValue::from_raw(1),
            sapling::Rseed::AfterZip212([5u8; 32]),
        );
        let node = sapling::Node::from_cmu(&note.cmu());
        tree.append(node).expect("append leaf");
        let witness = sapling::IncrementalWitness::from_tree(tree).expect("create witness");

        let stored = encode_witness(&witness);
        let decoded = decode_witness(&stored).expect("decode witness");
        assert_eq!(decoded.tree().size(), witness.tree().size());
    }

    #[test]
    fn decode_note_accepts_after_zip212() {
        let extsk = sapling::zip32::ExtendedSpendingKey::master(&[7u8; 32]);
        let recipient = extsk
            .to_diversifiable_full_viewing_key()
            .default_address()
            .1;
        let note = sapling::Note::from_parts(
            recipient,
            sapling::value::NoteValue::from_raw(123),
            sapling::Rseed::AfterZip212([7u8; 32]),
        );
        let stored = crate::core::channels::dlight_private::spend_db::StoredSpendNote {
            nullifier_hex: "00".repeat(32),
            value_sats: 123,
            received_height: 1,
            spent_height: None,
            note_position: 0,
            txid: "00".repeat(32),
            scope: crate::core::channels::dlight_private::spend_db::StoredSpendScope::External,
            recipient_bytes_hex: hex::encode(note.recipient().to_bytes()),
            rseed: encode_rseed(note.rseed()),
            witness: StoredSpendWitness {
                tree: StoredSpendTree::default(),
                filled: vec![],
                cursor: None,
            },
        };

        let decoded = decode_note(&stored).expect("decode note");
        assert_eq!(decoded.value().inner(), 123);
    }

    #[test]
    fn decode_note_rejects_invalid_rseed_hex() {
        let stored = crate::core::channels::dlight_private::spend_db::StoredSpendNote {
            nullifier_hex: "00".repeat(32),
            value_sats: 123,
            received_height: 1,
            spent_height: None,
            note_position: 0,
            txid: "00".repeat(32),
            scope: crate::core::channels::dlight_private::spend_db::StoredSpendScope::External,
            recipient_bytes_hex: "00".repeat(43),
            rseed: StoredRseed {
                kind: StoredRseedKind::AfterZip212,
                bytes_hex: "abc".to_string(),
            },
            witness: StoredSpendWitness {
                tree: StoredSpendTree::default(),
                filled: vec![],
                cursor: None,
            },
        };

        assert!(decode_note(&stored).is_none());
    }
}
