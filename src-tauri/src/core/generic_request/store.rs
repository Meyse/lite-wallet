//! The one open request. A request is held in memory only, for the wallet
//! session that verified it (spec: "One request at a time, in memory only").
//! Later commands act only on the request this store holds, so they never act
//! on bytes that skipped verification.

use std::sync::Mutex;

use crate::types::WalletError;

use super::details::DetailKind;
use super::request::{parse_request_hex, ParsedRequest};

#[derive(Debug, Clone)]
pub(crate) struct OpenRequest {
    pub session_id: String,
    pub request_hex: String,
    pub parsed: ParsedRequest,
    pub kinds: Vec<DetailKind>,
    /// The transaction broadcast for this request. Once set, a retry reuses it
    /// and never broadcasts again (ADR 0001).
    pub broadcast_txid: Option<String>,
}

#[derive(Default)]
pub struct GenericRequestStore {
    open: Mutex<Option<OpenRequest>>,
}

impl GenericRequestStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn open(&self, request: OpenRequest) {
        *self.open.lock().unwrap_or_else(|error| error.into_inner()) = Some(request);
    }

    pub fn close(&self) {
        *self.open.lock().unwrap_or_else(|error| error.into_inner()) = None;
    }

    /// The open request, if `request_hex` is the request this session verified.
    pub(crate) fn get(
        &self,
        session_id: &str,
        request_hex: &str,
    ) -> Result<OpenRequest, WalletError> {
        let guard = self.open.lock().unwrap_or_else(|error| error.into_inner());
        let open = guard
            .as_ref()
            .filter(|open| open.session_id == session_id)
            .ok_or(WalletError::GenericRequestRejected("notOpen".to_string()))?;
        let requested = parse_request_hex(request_hex)?;
        if requested.request_hash != open.parsed.request_hash
            || !open.request_hex.eq_ignore_ascii_case(request_hex.trim())
        {
            return Err(WalletError::GenericRequestRejected("notOpen".to_string()));
        }
        Ok(open.clone())
    }

    pub(crate) fn record_broadcast(&self, session_id: &str, request_hash: &[u8; 32], txid: &str) {
        let mut guard = self.open.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(open) = guard.as_mut().filter(|open| {
            open.session_id == session_id && &open.parsed.request_hash == request_hash
        }) {
            open.broadcast_txid = Some(txid.to_string());
        }
    }
}
