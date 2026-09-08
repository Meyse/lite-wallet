use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::wallet::WalletNetwork;

#[derive(Clone)]
pub struct ProvisioningSignatureChallenge {
    pub session_id: String,
    pub account_id: String,
    pub network: WalletNetwork,
    pub system_id: String,
    pub challenge_hash: [u8; 32],
    pub expires_at: u64,
}

#[derive(Clone, Default)]
pub struct ProvisioningSignatureStore {
    records: Arc<Mutex<HashMap<String, ProvisioningSignatureChallenge>>>,
    active_session_id: Arc<Mutex<Option<String>>>,
}

impl ProvisioningSignatureStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn activate_wallet_session(&self, session_id: &str) {
        *self
            .active_session_id
            .lock()
            .expect("provisioning signature session lock") = Some(session_id.to_string());
        self.records
            .lock()
            .expect("provisioning signature store lock")
            .clear();
    }

    pub fn put(&self, id: String, record: ProvisioningSignatureChallenge) -> bool {
        let active_session = self
            .active_session_id
            .lock()
            .expect("provisioning signature session lock");
        if active_session.as_deref() != Some(record.session_id.as_str()) {
            return false;
        }
        self.records
            .lock()
            .expect("provisioning signature store lock")
            .insert(id, record);
        true
    }

    pub fn take(&self, id: &str, session_id: &str) -> Option<ProvisioningSignatureChallenge> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(u64::MAX);
        let mut records = self
            .records
            .lock()
            .expect("provisioning signature store lock");
        records.retain(|_, record| record.expires_at >= now);
        if records
            .get(id)
            .is_some_and(|record| record.session_id == session_id)
        {
            records.remove(id)
        } else {
            None
        }
    }

    pub fn clear(&self) {
        *self
            .active_session_id
            .lock()
            .expect("provisioning signature session lock") = None;
        self.records
            .lock()
            .expect("provisioning signature store lock")
            .clear();
    }
}
