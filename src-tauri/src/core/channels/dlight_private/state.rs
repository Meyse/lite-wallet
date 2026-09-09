//! One authoritative generation for history, witnesses, and pending spends.
use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use zcash_client_backend::{
    data_api::BlockMetadata,
    keys::UnifiedFullViewingKey,
    proto::compact_formats::CompactBlock,
    scanning::{scan_block, Nullifiers, ScanningKeys},
};
use zcash_protocol::consensus::BlockHeight;
use zip32::Scope;

use super::{
    consensus,
    spend_db::{StoredSpendNote, StoredSpendScope, StoredSpendTree},
    spend_sync::{
        decode_note, decode_tree, decode_witness, encode_rseed, encode_tree, encode_witness,
        SpendableNote,
    },
    store::{unix_timestamp_secs, StoredTransaction},
};
use crate::types::{wallet::WalletNetwork, WalletError};

const SCHEMA_VERSION: u32 = 1;
const CHECKPOINT_LIMIT: usize = 32;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Checkpoint {
    pub height: u64,
    pub block_hash_hex: Option<String>,
    pub tree: StoredSpendTree,
    pub notes: Vec<StoredSpendNote>,
}

impl Checkpoint {
    pub fn empty(network: WalletNetwork) -> Self {
        Self {
            height: consensus::birthday_floor(network),
            block_hash_hex: None,
            tree: StoredSpendTree::default(),
            notes: vec![],
        }
    }
    pub fn metadata(&self) -> Result<Option<BlockMetadata>, WalletError> {
        let Some(hash) = &self.block_hash_hex else {
            return Ok(None);
        };
        let bytes = hex::decode(hash).map_err(|_| WalletError::OperationFailed)?;
        if bytes.len() != 32 {
            return Err(WalletError::OperationFailed);
        }
        let tree = decode_tree(&self.tree).ok_or(WalletError::OperationFailed)?;
        let block = CompactBlock {
            height: self.height,
            hash: bytes,
            ..Default::default()
        };
        Ok(Some(BlockMetadata::from_parts(
            block.height(),
            block.hash(),
            Some(u32::try_from(tree.size()).map_err(|_| WalletError::OperationFailed)?),
        )))
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PendingSend {
    pub id: String,
    pub nullifiers: Vec<String>,
    pub value_sats: u64,
    pub fee_sats: u64,
    pub change_sats: u64,
    pub to_address: String,
    pub created_at: u64,
    pub txid: Option<String>,
    pub raw_tx_hex: Option<String>,
    pub expiry_height: Option<u64>,
    pub mined_height: Option<u64>,
    pub conflict_height: Option<u64>,
}

impl PendingSend {
    pub fn is_active(&self, scanned_height: u64) -> bool {
        self.mined_height.is_none()
            && self.conflict_height.is_none()
            && self
                .expiry_height
                .is_none_or(|expiry| scanned_height <= expiry)
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WalletState {
    pub schema_version: u32,
    pub chain: Checkpoint,
    pub birthday: Checkpoint,
    pub checkpoints: Vec<Checkpoint>,
    pub transactions: BTreeMap<String, StoredTransaction>,
    pub pending: BTreeMap<String, PendingSend>,
    pub last_updated: u64,
}

impl WalletState {
    pub fn new(network: WalletNetwork) -> Self {
        let birthday = Checkpoint::empty(network);
        Self {
            schema_version: SCHEMA_VERSION,
            chain: birthday.clone(),
            birthday,
            checkpoints: vec![],
            transactions: BTreeMap::new(),
            pending: BTreeMap::new(),
            last_updated: unix_timestamp_secs(),
        }
    }

    pub fn validate(&self) -> Result<(), WalletError> {
        if self.schema_version != SCHEMA_VERSION || self.chain.height < self.birthday.height {
            return Err(WalletError::OperationFailed);
        }
        self.chain.metadata()?;
        decode_notes(&self.chain)?;
        Ok(())
    }

    /// A Building reservation has never reached the transport: a serialized tx
    /// is persisted before submission is admitted. Only those reservations may
    /// be released after process/session interruption without consulting chain.
    pub fn recover_interrupted_builds(&mut self) -> bool {
        let before = self.pending.len();
        self.pending.retain(|_, pending| pending.txid.is_some());
        before != self.pending.len()
    }

    pub fn reserved_nullifiers(&self) -> HashSet<&str> {
        self.pending
            .values()
            .filter(|p| p.is_active(self.chain.height))
            .flat_map(|p| p.nullifiers.iter().map(String::as_str))
            .collect()
    }

    pub fn available_notes(&self) -> Result<Vec<SpendableNote>, WalletError> {
        let reserved = self.reserved_nullifiers();
        let mut notes = decode_notes(&self.chain)?
            .into_values()
            .filter(|n| {
                n.note.value().inner() > 0 && !reserved.contains(n.stored.nullifier_hex.as_str())
            })
            .map(|n| {
                Ok(SpendableNote {
                    nullifier_hex: n.stored.nullifier_hex,
                    value_sats: n.note.value().inner(),
                    received_height: n.stored.received_height,
                    txid: n.stored.txid,
                    scope: match n.stored.scope {
                        StoredSpendScope::External => Scope::External,
                        StoredSpendScope::Internal => Scope::Internal,
                    },
                    note: n.note,
                    merkle_path: n
                        .witness
                        .path()
                        .ok_or(WalletError::DlightSpendCacheNotReady)?,
                })
            })
            .collect::<Result<Vec<_>, WalletError>>()?;
        notes.sort_by(|left, right| {
            left.received_height
                .cmp(&right.received_height)
                .then(left.txid.cmp(&right.txid))
                .then(left.nullifier_hex.cmp(&right.nullifier_hex))
        });
        Ok(notes)
    }

    pub fn balances(&self) -> (u64, u64) {
        let reserved = self.reserved_nullifiers();
        let available = self
            .chain
            .notes
            .iter()
            .filter(|n| !reserved.contains(n.nullifier_hex.as_str()))
            .map(|n| n.value_sats)
            .sum();
        let pending_change = self
            .pending
            .values()
            .filter(|p| p.is_active(self.chain.height))
            .map(|p| p.change_sats)
            .sum();
        (available, pending_change)
    }

    pub fn reserve(&mut self, pending: PendingSend) -> Result<(), WalletError> {
        let reserved = self.reserved_nullifiers();
        let available: HashSet<_> = self
            .chain
            .notes
            .iter()
            .map(|n| n.nullifier_hex.as_str())
            .collect();
        let distinct: HashSet<_> = pending.nullifiers.iter().collect();
        if pending.nullifiers.is_empty()
            || distinct.len() != pending.nullifiers.len()
            || self.pending.contains_key(&pending.id)
            || pending
                .nullifiers
                .iter()
                .any(|nf| reserved.contains(nf.as_str()) || !available.contains(nf.as_str()))
        {
            return Err(WalletError::InvalidPreflight);
        }
        self.pending.insert(pending.id.clone(), pending);
        Ok(())
    }

    pub fn rewind(&mut self, checkpoint: Checkpoint) {
        let height = checkpoint.height;
        self.chain = checkpoint;
        self.checkpoints.retain(|cp| cp.height <= height);
        self.transactions.retain(|_, tx| tx.block_height <= height);
        for pending in self.pending.values_mut() {
            if pending.mined_height.is_some_and(|h| h > height) {
                pending.mined_height = None;
            }
            if pending.conflict_height.is_some_and(|h| h > height) {
                pending.conflict_height = None;
            }
        }
        self.last_updated = unix_timestamp_secs();
    }

    /// Called on a blocking worker. Preparation is once per batch, not per block;
    /// seed derivation is performed once when the runtime's viewing key is made.
    pub fn scan_batch(
        &mut self,
        blocks: Vec<CompactBlock>,
        network: WalletNetwork,
        viewing_key: UnifiedFullViewingKey,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<(), WalletError> {
        if blocks.is_empty() {
            return Err(WalletError::NetworkError);
        }
        let params = consensus::parameters(network);
        let keys = ScanningKeys::from_account_ufvks(vec![(0u32, viewing_key)]);
        let mut tree = decode_tree(&self.chain.tree).ok_or(WalletError::OperationFailed)?;
        let mut notes = decode_notes(&self.chain)?;
        let mut prior = self.chain.metadata()?;
        for mut block in blocks {
            if cancel.is_cancelled() {
                return Err(WalletError::WalletLocked);
            }
            validate_compact_block(&block, self.chain.height + 1)?;
            // Verus lightwalletd can emit an all-zero ChainMetadata placeholder
            // even after Sapling outputs exist. Derive the size from our known
            // prior tree; the runtime checks the full tree at each batch end.
            if block.chain_metadata.as_ref().is_some_and(|meta| {
                meta.sapling_commitment_tree_size == 0 && meta.orchard_commitment_tree_size == 0
            }) {
                block.chain_metadata = None;
            }
            if prior.is_none() {
                // Only a known empty pre-activation tree may be seeded this way.
                // Never splice a cached witness tree onto an unverified new hash.
                if self.chain.height != consensus::birthday_floor(network) || tree.size() != 0 {
                    return Err(WalletError::DlightSpendCacheNotReady);
                }
                prior = Some(BlockMetadata::from_parts(
                    BlockHeight::from_u32(self.chain.height as u32),
                    block.prev_hash(),
                    Some(0),
                ));
            }
            // 0.21's public low-level scanner cannot accept a custom nullifier
            // set. Detect all spends ourselves from EVERY compact transaction,
            // including transactions without an output back to this wallet.
            let scanned = scan_block(
                &params,
                block.clone(),
                &keys,
                &Nullifiers::empty(),
                prior.as_ref(),
            )
            .map_err(|_| WalletError::DlightSpendCacheNotReady)?;
            let mut received = HashMap::new();
            for tx in scanned.transactions() {
                for output in tx.sapling_outputs() {
                    let nf = output.nf().ok_or(WalletError::OperationFailed)?;
                    received.insert(
                        (tx.txid().to_string(), output.index()),
                        (
                            hex::encode(nf.as_ref()),
                            output.note().clone(),
                            output.recipient_key_scope().unwrap_or(Scope::External),
                        ),
                    );
                }
            }
            for tx in &block.vtx {
                let txid = tx.txid().to_string();
                let mut spent = 0u64;
                let mut incoming = 0u64;
                let tx_nullifiers: HashSet<_> =
                    tx.spends.iter().map(|s| hex::encode(&s.nf)).collect();
                for nf in &tx_nullifiers {
                    if let Some(note) = notes.remove(nf) {
                        spent += note.note.value().inner();
                    }
                }
                for (index, output) in tx.outputs.iter().enumerate() {
                    let cmu = output.cmu().map_err(|_| WalletError::OperationFailed)?;
                    let node = sapling::Node::from_cmu(&cmu);
                    tree.append(node)
                        .map_err(|_| WalletError::OperationFailed)?;
                    for tracked in notes.values_mut() {
                        tracked
                            .witness
                            .append(node)
                            .map_err(|_| WalletError::OperationFailed)?;
                    }
                    if let Some((nf, note, scope)) = received.remove(&(txid.clone(), index)) {
                        incoming += note.value().inner();
                        let witness = sapling::IncrementalWitness::from_tree(tree.clone())
                            .ok_or(WalletError::OperationFailed)?;
                        let stored = StoredSpendNote {
                            nullifier_hex: nf.clone(),
                            value_sats: note.value().inner(),
                            received_height: block.height,
                            spent_height: None,
                            note_position: u64::from(witness.witnessed_position()),
                            txid: txid.clone(),
                            scope: match scope {
                                Scope::External => StoredSpendScope::External,
                                Scope::Internal => StoredSpendScope::Internal,
                            },
                            recipient_bytes_hex: hex::encode(note.recipient().to_bytes()),
                            rseed: encode_rseed(note.rseed()),
                            witness: encode_witness(&witness),
                        };
                        notes.insert(
                            nf,
                            TrackedNote {
                                stored,
                                note,
                                witness,
                            },
                        );
                    }
                }
                let mut to_address = None;
                for pending in self.pending.values_mut() {
                    if pending.txid.as_deref() == Some(txid.as_str()) {
                        pending.mined_height = Some(block.height);
                        pending.conflict_height = None;
                        to_address = Some(pending.to_address.clone());
                    } else if pending
                        .nullifiers
                        .iter()
                        .any(|nf| tx_nullifiers.contains(nf))
                    {
                        pending.conflict_height = Some(block.height);
                    }
                }
                if spent > 0 || incoming > 0 {
                    self.transactions.insert(
                        txid.clone(),
                        StoredTransaction {
                            txid,
                            net_sats: i128::from(incoming) - i128::from(spent),
                            block_height: block.height,
                            block_time: u64::from(block.time),
                            to_address,
                        },
                    );
                }
            }
            if Some(tree.size() as u32) != scanned.to_block_metadata().sapling_tree_size() {
                return Err(WalletError::DlightSpendCacheNotReady);
            }
            prior = Some(scanned.to_block_metadata());
            self.chain.height = block.height;
            self.chain.block_hash_hex = Some(hex::encode(&block.hash));
        }
        self.chain.tree = encode_tree(&tree);
        self.chain.notes = notes
            .into_values()
            .map(|mut n| {
                n.stored.witness = encode_witness(&n.witness);
                n.stored
            })
            .collect();
        self.chain
            .notes
            .sort_by(|a, b| a.nullifier_hex.cmp(&b.nullifier_hex));
        self.checkpoints.push(self.chain.clone());
        if self.checkpoints.len() > CHECKPOINT_LIMIT {
            self.checkpoints.remove(0);
        }
        self.last_updated = unix_timestamp_secs();
        Ok(())
    }
}

struct TrackedNote {
    stored: StoredSpendNote,
    note: sapling::Note,
    witness: sapling::IncrementalWitness,
}

fn decode_notes(checkpoint: &Checkpoint) -> Result<HashMap<String, TrackedNote>, WalletError> {
    let tree = decode_tree(&checkpoint.tree).ok_or(WalletError::OperationFailed)?;
    let mut notes = HashMap::new();
    for stored in &checkpoint.notes {
        if stored.spent_height.is_some() {
            return Err(WalletError::OperationFailed);
        }
        let note = decode_note(stored).ok_or(WalletError::OperationFailed)?;
        let witness = decode_witness(&stored.witness).ok_or(WalletError::OperationFailed)?;
        let path = witness.path().ok_or(WalletError::OperationFailed)?;
        if path.root(sapling::Node::from_cmu(&note.cmu())) != tree.root()
            || u64::from(witness.witnessed_position()) != stored.note_position
            || notes.contains_key(&stored.nullifier_hex)
        {
            return Err(WalletError::OperationFailed);
        }
        notes.insert(
            stored.nullifier_hex.clone(),
            TrackedNote {
                stored: stored.clone(),
                note,
                witness,
            },
        );
    }
    Ok(notes)
}

fn validate_compact_block(block: &CompactBlock, expected: u64) -> Result<(), WalletError> {
    if block.height != expected
        || block.height > u64::from(u32::MAX)
        || block.hash.len() != 32
        || block.prev_hash.len() != 32
        || block.vtx.iter().any(|tx| {
            !tx.actions.is_empty()
                || tx.hash.len() != 32
                || tx.index > u64::from(u16::MAX)
                || tx.spends.iter().any(|s| s.nf.len() != 32)
                || tx.outputs.iter().any(|o| {
                    o.cmu.len() != 32 || o.ephemeral_key.len() != 32 || o.ciphertext.len() != 52
                })
        })
    {
        return Err(WalletError::NetworkError);
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod testing {
    use super::super::spend_keys::DlightSpendKeyMaterial;
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};
    use zcash_client_backend::proto::compact_formats::{
        ChainMetadata, CompactSaplingOutput, CompactTx,
    };
    use zcash_note_encryption::Domain;

    pub fn keys() -> DlightSpendKeyMaterial {
        let extsk = sapling::zip32::ExtendedSpendingKey::master(&[7; 32]);
        let key = zcash_keys::encoding::encode_extended_spending_key(
            zcash_protocol::constants::mainnet::HRP_SAPLING_EXTENDED_SPENDING_KEY,
            &extsk,
        );
        let address = super::super::derive_scope_address(&key, WalletNetwork::Testnet).unwrap();
        DlightSpendKeyMaterial::from_seed_material(&key, WalletNetwork::Testnet, &address).unwrap()
    }

    pub fn incoming(
        keys: &DlightSpendKeyMaterial,
        id: u8,
        value: u64,
        position: u64,
        scope: Scope,
    ) -> (CompactTx, String) {
        let address = match scope {
            Scope::External => keys.external_payment_address(),
            Scope::Internal => keys.change_payment_address(),
        };
        let note = sapling::Note::from_parts(
            address,
            sapling::value::NoteValue::from_raw(value),
            sapling::Rseed::BeforeZip212(jubjub::Fr::from(u64::from(id))),
        );
        let nf = note.nf(&keys.sapling_fvk_for_scope(scope).vk.nk, position);
        let encryption = sapling::note_encryption::sapling_note_encryption(
            Some(keys.sapling_ovk_for_scope(scope)),
            note.clone(),
            [0; 512],
            &mut StdRng::from_seed([id; 32]),
        );
        let output = CompactSaplingOutput {
            cmu: note.cmu().to_bytes().to_vec(),
            ephemeral_key: sapling::note_encryption::SaplingDomain::epk_bytes(encryption.epk())
                .0
                .to_vec(),
            ciphertext: encryption.encrypt_note_plaintext()[..52].to_vec(),
        };
        (
            CompactTx {
                hash: vec![id; 32],
                outputs: vec![output],
                ..Default::default()
            },
            hex::encode(nf.0),
        )
    }

    pub fn block(
        height: u64,
        id: u8,
        prev: u8,
        mut transactions: Vec<CompactTx>,
        tree_size: u32,
    ) -> CompactBlock {
        for (i, tx) in transactions.iter_mut().enumerate() {
            tx.index = i as u64;
        }
        CompactBlock {
            height,
            hash: vec![id; 32],
            prev_hash: vec![prev; 32],
            time: 1_700_000_000 + height as u32,
            vtx: transactions,
            chain_metadata: Some(ChainMetadata {
                sapling_commitment_tree_size: tree_size,
                orchard_commitment_tree_size: 0,
            }),
            ..Default::default()
        }
    }

    pub fn funded_state() -> (WalletState, DlightSpendKeyMaterial, String) {
        let keys = keys();
        let (incoming, nf) = incoming(&keys, 20, 100_000, 0, Scope::External);
        let mut state = WalletState::new(WalletNetwork::Testnet);
        scan(&mut state, &keys, vec![block(1, 1, 0, vec![incoming], 1)]).unwrap();
        (state, keys, nf)
    }

    pub fn scan(
        state: &mut WalletState,
        keys: &DlightSpendKeyMaterial,
        blocks: Vec<CompactBlock>,
    ) -> Result<(), WalletError> {
        state.scan_batch(
            blocks,
            WalletNetwork::Testnet,
            keys.viewing_key().unwrap(),
            &tokio_util::sync::CancellationToken::new(),
        )
    }

    pub fn pending(id: &str, nf: &str) -> PendingSend {
        PendingSend {
            id: id.into(),
            nullifiers: vec![nf.into()],
            value_sats: 90_000,
            fee_sats: 10_000,
            change_sats: 0,
            to_address: "private-destination".into(),
            created_at: 123,
            txid: None,
            raw_tx_hex: None,
            expiry_height: None,
            mined_height: None,
            conflict_height: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;
    use zcash_client_backend::proto::compact_formats::{CompactSaplingSpend, CompactTx};

    fn spend(id: u8, nf: &str) -> CompactTx {
        CompactTx {
            hash: vec![id; 32],
            spends: vec![CompactSaplingSpend {
                nf: hex::decode(nf).unwrap(),
            }],
            ..Default::default()
        }
    }

    #[test]
    fn decrypts_pre_zip212_receipts_and_validates_live_witnesses() {
        let (state, _, nf) = funded_state();
        assert_eq!(state.balances(), (100_000, 0));
        let notes = state.available_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].nullifier_hex, nf);
        assert!(matches!(
            notes[0].note.rseed(),
            sapling::Rseed::BeforeZip212(_)
        ));
        state.validate().unwrap();
    }

    #[test]
    fn outgoing_without_change_is_recorded_and_reorg_restores_reservation() {
        let (mut state, keys, nf) = funded_state();
        let before = state.chain.clone();
        let outgoing = spend(30, &nf);
        let txid = outgoing.txid().to_string();
        let mut pending = pending("send", &nf);
        pending.txid = Some(txid.clone());
        pending.raw_tx_hex = Some("abcd".into());
        pending.expiry_height = Some(5);
        state.reserve(pending).unwrap();
        scan(&mut state, &keys, vec![block(2, 2, 1, vec![outgoing], 1)]).unwrap();
        assert_eq!(state.transactions[&txid].net_sats, -100_000);
        assert_eq!(
            state.transactions[&txid].to_address.as_deref(),
            Some("private-destination")
        );
        assert_eq!(state.pending["send"].mined_height, Some(2));
        assert!(state.chain.notes.is_empty());
        state.rewind(before);
        assert!(!state.transactions.contains_key(&txid));
        assert_eq!(state.pending["send"].mined_height, None);
        assert_eq!(state.chain.notes.len(), 1);
        assert!(state.available_notes().unwrap().is_empty());
        scan(&mut state, &keys, vec![block(2, 3, 1, vec![], 1)]).unwrap();
        assert_eq!(state.chain.block_hash_hex, Some("03".repeat(32)));
        state.validate().unwrap();
    }

    #[test]
    fn receives_and_spends_within_one_block_in_transaction_order() {
        let keys = keys();
        let (incoming, nf) = incoming(&keys, 40, 80_000, 0, Scope::External);
        let outgoing = spend(41, &nf);
        let outgoing_id = outgoing.txid().to_string();
        let mut state = WalletState::new(WalletNetwork::Testnet);
        scan(
            &mut state,
            &keys,
            vec![block(1, 1, 0, vec![incoming, outgoing], 1)],
        )
        .unwrap();
        assert_eq!(state.balances(), (0, 0));
        assert_eq!(state.transactions.len(), 2);
        assert_eq!(state.transactions[&outgoing_id].net_sats, -80_000);
        assert!(state.chain.notes.is_empty());
    }

    #[test]
    fn pending_survives_restart_until_scanned_past_expiry() {
        let (mut state, keys, nf) = funded_state();
        let mut pending = pending("send", &nf);
        pending.txid = Some("ab".repeat(32));
        pending.raw_tx_hex = Some("abcd".into());
        pending.expiry_height = Some(3);
        state.reserve(pending).unwrap();
        let mut restored: WalletState =
            serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
        assert!(!restored.recover_interrupted_builds());
        scan(
            &mut restored,
            &keys,
            vec![block(2, 2, 1, vec![], 1), block(3, 3, 2, vec![], 1)],
        )
        .unwrap();
        assert!(restored.available_notes().unwrap().is_empty());
        scan(&mut restored, &keys, vec![block(4, 4, 3, vec![], 1)]).unwrap();
        assert_eq!(restored.available_notes().unwrap().len(), 1);
        // A rewind before expiry must reserve the inputs again.
        restored.rewind(state.chain);
        assert!(restored.available_notes().unwrap().is_empty());
    }

    #[test]
    fn overlapping_or_unknown_reservations_fail_and_unsubmitted_builds_recover() {
        let (mut state, _, nf) = funded_state();
        state.reserve(pending("first", &nf)).unwrap();
        assert!(state.reserve(pending("second", &nf)).is_err());
        assert!(state.reserve(pending("unknown", "unknown")).is_err());
        assert_eq!(state.balances(), (0, 0));
        assert!(state.recover_interrupted_builds());
        assert_eq!(state.available_notes().unwrap().len(), 1);
    }

    #[test]
    fn conflicting_spend_is_reconciled_and_rolled_back() {
        let (mut state, keys, nf) = funded_state();
        let before = state.chain.clone();
        state.reserve(pending("pending", &nf)).unwrap();
        scan(
            &mut state,
            &keys,
            vec![block(2, 2, 1, vec![spend(50, &nf)], 1)],
        )
        .unwrap();
        assert_eq!(state.pending["pending"].conflict_height, Some(2));
        assert!(!state.pending["pending"].is_active(2));
        state.rewind(before);
        assert!(state.pending["pending"].is_active(1));
        assert!(state.available_notes().unwrap().is_empty());
    }

    #[test]
    fn internal_change_is_scanned_and_witnesses_follow_external_outputs() {
        let (mut state, keys, nf) = funded_state();
        let (mut change, change_nf) = incoming(&keys, 60, 20_000, 1, Scope::Internal);
        change.spends.push(CompactSaplingSpend {
            nf: hex::decode(nf).unwrap(),
        });
        scan(&mut state, &keys, vec![block(2, 2, 1, vec![change], 2)]).unwrap();
        let (external, _) = incoming(&testing::keys(), 61, 50_000, 2, Scope::External);
        scan(&mut state, &keys, vec![block(3, 3, 2, vec![external], 3)]).unwrap();
        let notes = state.available_notes().unwrap();
        let note = notes.iter().find(|n| n.nullifier_hex == change_nf).unwrap();
        assert_eq!(note.scope, Scope::Internal);
        state.validate().unwrap();
    }

    #[test]
    fn rejects_malformed_blocks_wrong_continuity_and_damaged_witnesses() {
        let (state, keys, _) = funded_state();
        let mut malformed = block(2, 2, 1, vec![spend(70, &"00".repeat(32))], 1);
        malformed.vtx[0].hash.clear();
        assert!(scan(&mut state.clone(), &keys, vec![malformed]).is_err());
        assert!(scan(&mut state.clone(), &keys, vec![block(2, 2, 99, vec![], 1)]).is_err());
        assert!(scan(&mut state.clone(), &keys, vec![block(2, 2, 1, vec![], 9)]).is_err());
        let mut damaged = state.clone();
        damaged.chain.notes[0].value_sats += 1;
        assert!(damaged.validate().is_err());
        let mut cancelled = state.clone();
        let token = tokio_util::sync::CancellationToken::new();
        token.cancel();
        assert!(matches!(
            cancelled.scan_batch(
                vec![block(2, 2, 1, vec![], 1)],
                WalletNetwork::Testnet,
                keys.viewing_key().unwrap(),
                &token
            ),
            Err(WalletError::WalletLocked)
        ));
        assert_eq!(cancelled.chain.height, state.chain.height);
    }
    #[test]
    fn spendable_notes_keep_oldest_first_order_after_reload() {
        let (mut state, keys, _) = funded_state();
        let (second, _) = incoming(&keys, 80, 50_000, 1, Scope::External);
        scan(&mut state, &keys, vec![block(2, 2, 1, vec![second], 2)]).unwrap();
        state.chain.notes.reverse();
        let restored: WalletState =
            serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
        assert_eq!(
            restored
                .available_notes()
                .unwrap()
                .iter()
                .map(|note| note.received_height)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }
    #[test]
    fn zero_server_tree_metadata_uses_known_tree_and_output_count() {
        let (mut state, keys, _) = funded_state();
        let (received, _) = incoming(&keys, 90, 30_000, 1, Scope::External);
        scan(&mut state, &keys, vec![block(2, 2, 1, vec![received], 0)]).unwrap();
        assert_eq!(decode_tree(&state.chain.tree).unwrap().size(), 2);
        assert_eq!(state.balances(), (130_000, 0));
        state.validate().unwrap();
    }
}
