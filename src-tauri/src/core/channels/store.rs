//
// Module 4: Session-scoped preflight records. Cleared on lock; do not send to frontend.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Internal preflight record keyed by preflight_id. Used by router to dispatch send; payload is channel-specific.
#[derive(Clone)]
pub struct PreflightRecord {
    pub session_id: String,
    pub channel_id: String,
    pub account_id: String,
    pub payload: Value,
}

#[derive(Clone)]
struct StoredPreflightRecord {
    record: PreflightRecord,
    expires_at: Option<Instant>,
}

/// In-memory store of preflight records. Must be cleared when the wallet is locked (session-scoped).
#[derive(Clone)]
pub struct PreflightStore {
    inner: Arc<Mutex<HashMap<String, StoredPreflightRecord>>>,
    active_wallet_session_id: Arc<Mutex<Option<String>>>,
}

impl PreflightStore {
    pub const DEFAULT_TTL: Duration = Duration::from_secs(20 * 60);

    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            active_wallet_session_id: Arc::new(Mutex::new(None)),
        }
    }

    fn prune_expired(inner: &mut HashMap<String, StoredPreflightRecord>) {
        let now = Instant::now();
        inner.retain(|_, stored| stored.expires_at.map(|expiry| expiry > now).unwrap_or(true));
    }

    pub fn get(&self, id: &str, session_id: &str) -> Option<PreflightRecord> {
        let mut inner = self.inner.lock().expect("preflight store lock");
        Self::prune_expired(&mut inner);
        inner
            .get(id)
            .filter(|stored| stored.record.session_id == session_id)
            .map(|stored| stored.record.clone())
    }

    /// Get and consume a preflight record in one step (single-use send semantics).
    pub fn take(&self, id: &str, session_id: &str) -> Option<PreflightRecord> {
        let mut inner = self.inner.lock().expect("preflight store lock");
        Self::prune_expired(&mut inner);
        if inner
            .get(id)
            .is_some_and(|stored| stored.record.session_id == session_id)
        {
            inner.remove(id).map(|stored| stored.record)
        } else {
            None
        }
    }

    pub fn activate_wallet_session(&self, session_id: &str) {
        *self
            .active_wallet_session_id
            .lock()
            .expect("preflight session lock") = Some(session_id.to_string());
        self.inner.lock().expect("preflight store lock").clear();
    }

    pub fn put(&self, id: String, record: PreflightRecord) -> bool {
        self.put_with_ttl(id, record, Some(Self::DEFAULT_TTL))
    }

    pub fn put_with_ttl(&self, id: String, record: PreflightRecord, ttl: Option<Duration>) -> bool {
        let is_guard_record = record.account_id.starts_with("guard:");
        let active_session = self
            .active_wallet_session_id
            .lock()
            .expect("preflight session lock");
        if !is_guard_record && active_session.as_deref() != Some(record.session_id.as_str()) {
            return false;
        }
        let expires_at = ttl.map(|dur| Instant::now() + dur);
        self.inner
            .lock()
            .expect("preflight store lock")
            .insert(id, StoredPreflightRecord { record, expires_at });
        true
    }

    /// Clear all records. Must be called when the user locks the wallet.
    pub fn clear(&self) {
        *self
            .active_wallet_session_id
            .lock()
            .expect("preflight session lock") = None;
        self.inner.lock().expect("preflight store lock").clear();
    }
}

impl Default for PreflightStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_record() -> PreflightRecord {
        PreflightRecord {
            session_id: "session-1".to_string(),
            channel_id: "vrpc.Rtest.i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            account_id: "acct".to_string(),
            payload: json!({"hex": "00"}),
        }
    }

    #[test]
    fn take_consumes_record() {
        let store = PreflightStore::new();
        store.activate_wallet_session("session-1");
        store.put("id".to_string(), make_record());
        assert!(store.get("id", "session-1").is_some());
        assert!(store.take("id", "session-1").is_some());
        assert!(store.get("id", "session-1").is_none());
    }

    #[test]
    fn ttl_expiry_removes_record() {
        let store = PreflightStore::new();
        store.activate_wallet_session("session-1");
        store.put_with_ttl(
            "id".to_string(),
            make_record(),
            Some(Duration::from_millis(1)),
        );
        std::thread::sleep(Duration::from_millis(5));
        assert!(store.get("id", "session-1").is_none());
    }

    #[test]
    fn records_are_bound_to_the_unlock_session() {
        let store = PreflightStore::new();
        store.activate_wallet_session("session-1");
        store.put("id".to_string(), make_record());

        assert!(store.get("id", "session-2").is_none());
        assert!(store.take("id", "session-2").is_none());
        assert!(store.take("id", "session-1").is_some());
    }

    #[test]
    fn late_record_from_an_old_session_is_rejected_after_reunlock() {
        let store = PreflightStore::new();
        store.activate_wallet_session("session-1");
        store.activate_wallet_session("session-2");

        assert!(!store.put("old".to_string(), make_record()));
        assert!(store.get("old", "session-1").is_none());
    }
}
