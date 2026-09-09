use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoredSpendScope {
    External,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoredRseedKind {
    BeforeZip212,
    AfterZip212,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredRseed {
    pub kind: StoredRseedKind,
    pub bytes_hex: String,
}

impl Default for StoredRseed {
    fn default() -> Self {
        Self {
            kind: StoredRseedKind::AfterZip212,
            bytes_hex: "00".repeat(32),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StoredSpendTree {
    pub left: Option<String>,
    pub right: Option<String>,
    pub parents: Vec<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StoredSpendWitness {
    pub tree: StoredSpendTree,
    pub filled: Vec<String>,
    pub cursor: Option<StoredSpendTree>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredSpendNote {
    pub nullifier_hex: String,
    pub value_sats: u64,
    pub received_height: u64,
    pub spent_height: Option<u64>,
    pub note_position: u64,
    pub txid: String,
    pub scope: StoredSpendScope,
    #[serde(default)]
    pub recipient_bytes_hex: String,
    #[serde(default)]
    pub rseed: StoredRseed,
    #[serde(default)]
    pub witness: StoredSpendWitness,
}
