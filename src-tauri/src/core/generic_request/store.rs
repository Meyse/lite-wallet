//! The one open request. A request is held in memory only, for the wallet
//! session that verified it (spec: "One request at a time, in memory only").
//! Later commands act only on the request this store holds, so they never act
//! on bytes that skipped verification.

use std::sync::{Arc, Mutex};

use crate::types::WalletError;

use super::credentials::Credential;
use super::details::DetailKind;
use super::legacy::{is_legacy_link, LoginConsentRequest};
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
    /// The credentials the user reviewed, with the identity they belong to.
    /// The response shares exactly these.
    pub reviewed_credentials: Option<(String, Vec<Credential>)>,
    /// A legacy login consent request, which has its own response. A legacy
    /// invoice needs nothing here: it is held as the request's invoice.
    pub legacy_login: Option<LoginConsentRequest>,
    /// A gift card's claim: its key once reviewed, then the signed
    /// transactions and the IDs of those broadcast so far.
    pub claim: Option<GiftCardClaim>,
}

/// The key of a gift card being claimed. It stays in memory, like the open
/// request, and never prints.
#[derive(Clone)]
pub(crate) struct GiftCardClaim {
    pub address: String,
    pub private_key: zeroize::Zeroizing<[u8; 32]>,
    /// The claim's transactions, signed in broadcast order. A retry sends the
    /// same ones, never new ones.
    pub signed: Vec<String>,
    pub broadcast: Vec<String>,
}

impl std::fmt::Debug for GiftCardClaim {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GiftCardClaim")
            .field("address", &self.address)
            .field("broadcast", &self.broadcast)
            .finish_non_exhaustive()
    }
}

/// Clones share one store, so the session expiry task can clear it on lock.
#[derive(Clone, Default)]
pub struct GenericRequestStore {
    open: Arc<Mutex<Option<OpenRequest>>>,
    /// Invoices paid in a wallet session, by session and request hash. The
    /// wallet refuses to pay the same request twice in one session.
    paid: Arc<Mutex<Vec<(String, [u8; 32])>>>,
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
        // A legacy link is held as given: base64url is case sensitive.
        let same = if is_legacy_link(request_hex) {
            open.request_hex == request_hex.trim()
        } else {
            let requested = parse_request_hex(request_hex)?;
            requested.request_hash == open.parsed.request_hash
                && open.request_hex.eq_ignore_ascii_case(request_hex.trim())
        };
        if !same {
            return Err(WalletError::GenericRequestRejected("notOpen".to_string()));
        }
        Ok(open.clone())
    }

    pub(crate) fn is_paid(&self, session_id: &str, request_hash: &[u8; 32]) -> bool {
        self.paid
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
            .any(|(session, hash)| session == session_id && hash == request_hash)
    }

    pub(crate) fn record_payment(&self, session_id: &str, request_hash: &[u8; 32]) {
        let mut paid = self.paid.lock().unwrap_or_else(|error| error.into_inner());
        // Only the current session's payments matter; drop older sessions.
        paid.retain(|(session, _)| session == session_id);
        paid.push((session_id.to_string(), *request_hash));
    }

    pub(crate) fn record_broadcast(&self, session_id: &str, request_hash: &[u8; 32], txid: &str) {
        let mut guard = self.open.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(open) = guard.as_mut().filter(|open| {
            open.session_id == session_id && &open.parsed.request_hash == request_hash
        }) {
            open.broadcast_txid = Some(txid.to_string());
        }
    }

    pub(crate) fn record_credentials(
        &self,
        session_id: &str,
        request_hash: &[u8; 32],
        identity: &str,
        credentials: Vec<Credential>,
    ) {
        let mut guard = self.open.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(open) = guard.as_mut().filter(|open| {
            open.session_id == session_id && &open.parsed.request_hash == request_hash
        }) {
            open.reviewed_credentials = Some((identity.to_string(), credentials));
        }
    }

    /// Updates the open request's gift card claim.
    pub(crate) fn update_claim(
        &self,
        session_id: &str,
        request_hash: &[u8; 32],
        update: impl FnOnce(&mut Option<GiftCardClaim>),
    ) {
        let mut guard = self.open.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(open) = guard.as_mut().filter(|open| {
            open.session_id == session_id && &open.parsed.request_hash == request_hash
        }) {
            update(&mut open.claim);
        }
    }
}
