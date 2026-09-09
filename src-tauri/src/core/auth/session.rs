//
// Session management with timeout and zeroization
// Security: Keeps only public wallet state and the Stronghold unlock hash in memory
// Last Updated: Signing secrets now load from Stronghold on demand instead of living in session

use crate::core::auth::stronghold_store::StrongholdStore;
use crate::core::channels::dlight_private;
use crate::core::crypto::{derive_private_scalar_from_material, DerivedPublicProfile};
use crate::types::errors::WalletError;
use crate::types::wallet::{WalletNetwork, WalletSecretKind};
use crate::types::LinkedIdentity;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Duration;
use tokio::sync::{watch, Mutex};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use zeroize::Zeroizing;

pub const ALLOWED_SESSION_TIMEOUT_MINUTES: [u64; 4] = [5, 15, 30, 60];
pub const DEFAULT_SESSION_TIMEOUT_MINUTES: u64 = 15;

#[derive(Clone)]
struct SessionAddresses {
    vrsc_address: String,
    eth_address: String,
    btc_address: String,
}

struct SessionSubmissionState {
    admission: StdMutex<()>,
    invalidated: AtomicBool,
    cancellation: CancellationToken,
}

#[derive(Clone, Debug)]
pub(crate) struct DlightPublicMetadata {
    pub(crate) configured: bool,
    pub(crate) shielded_address: Option<String>,
}

#[derive(Default)]
struct SessionPublicMetadataCache {
    linked_identities: Mutex<Option<Vec<LinkedIdentity>>>,
    dlight: Mutex<Option<DlightPublicMetadata>>,
}

/// Coordinates transaction-future admission with session invalidation.
///
/// The short synchronous admission mutex is held for each poll of a submission
/// future and released whenever that poll returns. It establishes a total order:
/// invalidation first prevents any further polling, while a poll admitted first
/// may reach its transport and can no longer be guaranteed reversible.
#[derive(Clone)]
pub(crate) struct SessionSubmissionGuard {
    state: Arc<SessionSubmissionState>,
}

pub(crate) struct SessionExpiryRegistration {
    pub(crate) session_id: String,
    pub(crate) cancellation: CancellationToken,
    pub(crate) changes: watch::Receiver<u64>,
}

impl SessionSubmissionGuard {
    fn new() -> Self {
        Self {
            state: Arc::new(SessionSubmissionState {
                admission: StdMutex::new(()),
                invalidated: AtomicBool::new(false),
                cancellation: CancellationToken::new(),
            }),
        }
    }

    fn invalidate(&self) {
        let _admission = self
            .state
            .admission
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.state.invalidated.store(true, Ordering::Release);
        self.state.cancellation.cancel();
    }

    pub(crate) fn cancellation(&self) -> CancellationToken {
        self.state.cancellation.clone()
    }

    /// Stop polling as soon as lock/logout invalidates this session. A poll
    /// admitted before invalidation may already have reached the transport.
    pub(crate) async fn run<T>(
        &self,
        future: impl std::future::Future<Output = Result<T, WalletError>>,
    ) -> Result<T, WalletError> {
        let mut future = std::pin::pin!(future);
        let cancellation = self.cancellation();
        tokio::select! {
            biased;
            _ = cancellation.cancelled() => Err(WalletError::WalletLocked),
            result = std::future::poll_fn(|cx| self.poll_admitted(|| future.as_mut().poll(cx))) => result,
        }
    }

    pub(crate) fn poll_admitted<T>(
        &self,
        poll: impl FnOnce() -> std::task::Poll<Result<T, WalletError>>,
    ) -> std::task::Poll<Result<T, WalletError>> {
        let _admission = self
            .state
            .admission
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if self.state.invalidated.load(Ordering::Acquire) {
            return std::task::Poll::Ready(Err(WalletError::WalletLocked));
        }
        poll()
    }
}

#[derive(Clone)]
pub struct ActiveWalletAccessContext {
    pub session_id: String,
    pub account_id: String,
    pub wallet_network: WalletNetwork,
    pub wallet_secret_kind: WalletSecretKind,
    pub vrsc_address: String,
    pub eth_address: String,
    pub btc_address: String,
    session_submission_guard: SessionSubmissionGuard,
    stronghold_password_hash: Zeroizing<Vec<u8>>,
    pub stronghold_store: StrongholdStore,
    public_metadata_cache: Arc<SessionPublicMetadataCache>,
}

impl ActiveWalletAccessContext {
    pub fn password_hash(&self) -> &[u8] {
        self.stronghold_password_hash.as_ref()
    }

    pub(crate) fn session_submission_guard(&self) -> SessionSubmissionGuard {
        self.session_submission_guard.clone()
    }

    /// Load non-secret linked identity metadata once per unlock session.
    /// Holding this mutex across the storage read coalesces concurrent callers.
    pub(crate) async fn load_linked_identities_cached(
        &self,
    ) -> Result<Vec<LinkedIdentity>, WalletError> {
        let mut cached = self.public_metadata_cache.linked_identities.lock().await;
        if let Some(records) = cached.as_ref() {
            return Ok(records.clone());
        }

        let records = self
            .stronghold_store
            .load_linked_identities(&self.account_id, self.password_hash(), self.wallet_network)
            .await?;
        *cached = Some(records.clone());
        Ok(records)
    }

    pub(crate) async fn store_linked_identities_cached(
        &self,
        records: &[LinkedIdentity],
    ) -> Result<(), WalletError> {
        let mut cached = self.public_metadata_cache.linked_identities.lock().await;
        self.stronghold_store
            .store_linked_identities(
                &self.account_id,
                self.password_hash(),
                self.wallet_network,
                records,
            )
            .await?;
        *cached = Some(records.to_vec());
        Ok(())
    }

    /// Cache only public dlight metadata. Secret material is dropped immediately.
    pub(crate) async fn load_dlight_public_metadata_cached(
        &self,
    ) -> Result<DlightPublicMetadata, WalletError> {
        let mut cached = self.public_metadata_cache.dlight.lock().await;
        if let Some(metadata) = cached.as_ref() {
            return Ok(metadata.clone());
        }

        let seed = self
            .stronghold_store
            .load_dlight_runtime_material(
                &self.account_id,
                self.password_hash(),
                self.wallet_network,
            )
            .await?;
        let shielded_address = seed.as_ref().and_then(|(value, _)| {
            dlight_private::derive_scope_address(value, self.wallet_network)
                .map_err(|error| {
                    println!(
                        "[SESSION] Failed to derive cached dlight scope address: {:?}",
                        error
                    );
                    error
                })
                .ok()
        });
        let metadata = DlightPublicMetadata {
            configured: seed.is_some() && shielded_address.is_some(),
            shielded_address,
        };
        *cached = Some(metadata.clone());
        Ok(metadata)
    }
}

pub async fn capture_active_wallet_access_context(
    session_manager: &Arc<Mutex<SessionManager>>,
) -> Result<ActiveWalletAccessContext, WalletError> {
    let session = session_manager.lock().await;
    session.active_wallet_access_context()
}

pub async fn ensure_active_wallet_session(
    session_manager: &Arc<Mutex<SessionManager>>,
    session_id: &str,
) -> Result<(), WalletError> {
    if session_manager.lock().await.is_current_session(session_id) {
        Ok(())
    } else {
        Err(WalletError::WalletLocked)
    }
}

pub async fn load_primary_secret_material_for_context(
    context: &ActiveWalletAccessContext,
) -> Result<Zeroizing<String>, WalletError> {
    let primary_secret = context
        .stronghold_store
        .load_seed(&context.account_id, context.password_hash())
        .await?;
    Ok(Zeroizing::new(primary_secret))
}

pub async fn load_primary_private_scalar_for_context(
    context: &ActiveWalletAccessContext,
) -> Result<Zeroizing<[u8; 32]>, WalletError> {
    let primary_secret = load_primary_secret_material_for_context(context).await?;
    derive_private_scalar_from_material(primary_secret.as_str(), context.wallet_secret_kind)
}

pub fn normalize_session_timeout_minutes(minutes: u64) -> u64 {
    if ALLOWED_SESSION_TIMEOUT_MINUTES.contains(&minutes) {
        minutes
    } else {
        DEFAULT_SESSION_TIMEOUT_MINUTES
    }
}

pub struct SessionManager {
    is_unlocked: bool,
    active_session_id: Option<String>,
    active_account_id: Option<String>,
    last_activity_at: Option<Instant>,
    timeout_duration: Duration,
    active_network: Option<WalletNetwork>,
    active_secret_kind: Option<WalletSecretKind>,
    active_addresses: Option<SessionAddresses>,
    active_submission_guard: Option<SessionSubmissionGuard>,
    active_expiry_changes: Option<watch::Sender<u64>>,
    active_public_metadata_cache: Option<Arc<SessionPublicMetadataCache>>,
    stronghold_password_hash: Option<Zeroizing<Vec<u8>>>,
    stronghold_store: StrongholdStore,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new(stronghold_store: StrongholdStore) -> Self {
        let default_timeout_minutes =
            normalize_session_timeout_minutes(DEFAULT_SESSION_TIMEOUT_MINUTES);
        Self {
            is_unlocked: false,
            active_session_id: None,
            active_account_id: None,
            last_activity_at: None,
            timeout_duration: Duration::from_secs(default_timeout_minutes * 60),
            active_network: None,
            active_secret_kind: None,
            active_addresses: None,
            active_submission_guard: None,
            active_expiry_changes: None,
            active_public_metadata_cache: None,
            stronghold_password_hash: None,
            stronghold_store,
        }
    }

    /// Unlock wallet session by caching only non-secret wallet state.
    pub fn unlock_with_profile(
        &mut self,
        account_id: String,
        wallet_network: WalletNetwork,
        wallet_secret_kind: WalletSecretKind,
        public_profile: DerivedPublicProfile,
        stronghold_password_hash: Zeroizing<Vec<u8>>,
    ) -> String {
        println!("[SESSION] Unlock requested for account: {}", account_id);

        if let Some(guard) = self.active_submission_guard.take() {
            guard.invalidate();
        }
        let session_id = Uuid::new_v4().to_string();
        let (expiry_changes, _) = watch::channel(0);
        self.active_submission_guard = Some(SessionSubmissionGuard::new());
        self.active_expiry_changes = Some(expiry_changes);
        self.active_public_metadata_cache = Some(Arc::new(SessionPublicMetadataCache::default()));
        self.active_session_id = Some(session_id.clone());
        self.active_account_id = Some(account_id);
        self.active_network = Some(wallet_network);
        self.active_secret_kind = Some(wallet_secret_kind);
        self.active_addresses = Some(SessionAddresses {
            vrsc_address: public_profile.address,
            eth_address: public_profile.eth_address,
            btc_address: public_profile.btc_address,
        });
        self.stronghold_password_hash = Some(stronghold_password_hash);
        self.is_unlocked = true;
        self.last_activity_at = Some(Instant::now());

        println!("[SESSION] Unlock successful");
        session_id
    }

    /// Lock wallet session and zeroize all session-scoped secrets.
    pub fn lock(&mut self) {
        println!("[SESSION] Locking wallet");

        if let Some(guard) = self.active_submission_guard.take() {
            guard.invalidate();
        }
        self.active_expiry_changes = None;
        self.active_public_metadata_cache = None;
        self.active_account_id = None;
        self.active_session_id = None;
        self.active_network = None;
        self.active_secret_kind = None;
        self.active_addresses = None;
        self.stronghold_password_hash = None;
        self.is_unlocked = false;
        self.last_activity_at = None;

        println!("[SESSION] Wallet locked, session secrets zeroized");
    }

    /// Check if session has expired.
    pub fn is_expired(&self) -> bool {
        if !self.is_unlocked {
            return true;
        }

        if let Some(last_activity_at) = self.last_activity_at {
            Instant::now().duration_since(last_activity_at) >= self.timeout_duration
        } else {
            true
        }
    }

    /// Get derived addresses for active account.
    pub fn get_addresses(&self) -> Result<(String, String, String), WalletError> {
        if !self.is_unlocked || self.is_expired() {
            return Err(WalletError::WalletLocked);
        }

        let addresses = self
            .active_addresses
            .as_ref()
            .ok_or(WalletError::WalletLocked)?;

        Ok((
            addresses.vrsc_address.clone(),
            addresses.eth_address.clone(),
            addresses.btc_address.clone(),
        ))
    }

    /// Check if wallet is currently unlocked and not expired.
    pub fn is_unlocked(&self) -> bool {
        self.is_unlocked && !self.is_expired()
    }

    /// Get the active account ID.
    pub fn active_account_id(&self) -> Option<&String> {
        self.active_account_id.as_ref()
    }

    /// Opaque identifier for the current unlock instance. It changes even when
    /// the same account is unlocked again.
    pub fn active_session_id(&self) -> Option<&str> {
        self.active_session_id.as_deref()
    }

    pub fn is_current_session(&self, session_id: &str) -> bool {
        self.is_unlocked() && self.active_session_id() == Some(session_id)
    }

    /// Returns the selected wallet network for the active session.
    pub fn active_network(&self) -> Option<WalletNetwork> {
        self.active_network
    }

    /// Set session timeout duration.
    pub fn set_timeout(&mut self, duration: Duration) {
        self.timeout_duration = duration;
        self.notify_expiry_change();
    }

    /// Set timeout from minute granularity with strict allowlist normalization.
    pub fn set_timeout_minutes(&mut self, minutes: u64) -> u64 {
        let normalized_minutes = normalize_session_timeout_minutes(minutes);
        self.set_timeout(Duration::from_secs(normalized_minutes * 60));
        normalized_minutes
    }

    /// Refreshes the session inactivity timer for explicit user activity only.
    pub fn touch_activity(&mut self) -> Result<(), WalletError> {
        if !self.is_unlocked || self.is_expired() {
            return Err(WalletError::WalletLocked);
        }

        self.last_activity_at = Some(Instant::now());
        self.notify_expiry_change();
        Ok(())
    }

    /// Returns the active timeout in minutes.
    pub fn timeout_minutes(&self) -> u64 {
        let minutes = self.timeout_duration.as_secs() / 60;
        normalize_session_timeout_minutes(minutes)
    }

    /// Get reference to StrongholdStore (for use in commands).
    pub fn stronghold_store(&self) -> &StrongholdStore {
        &self.stronghold_store
    }

    /// Returns a copy of the current unlocked Stronghold password hash bytes for storage commands.
    pub fn stronghold_password_hash_for_storage(&self) -> Result<Zeroizing<Vec<u8>>, WalletError> {
        if !self.is_unlocked || self.is_expired() {
            return Err(WalletError::WalletLocked);
        }

        let hash = self
            .stronghold_password_hash
            .as_ref()
            .ok_or(WalletError::WalletLocked)?;
        Ok(Zeroizing::new(hash.to_vec()))
    }

    /// Snapshot the current active wallet context for on-demand Stronghold access.
    pub fn active_wallet_access_context(&self) -> Result<ActiveWalletAccessContext, WalletError> {
        if !self.is_unlocked || self.is_expired() {
            return Err(WalletError::WalletLocked);
        }

        let account_id = self
            .active_account_id
            .as_ref()
            .ok_or(WalletError::WalletLocked)?
            .clone();
        let session_id = self
            .active_session_id
            .as_ref()
            .ok_or(WalletError::WalletLocked)?
            .clone();
        let wallet_network = self.active_network.ok_or(WalletError::WalletLocked)?;
        let wallet_secret_kind = self.active_secret_kind.ok_or(WalletError::WalletLocked)?;
        let addresses = self
            .active_addresses
            .as_ref()
            .ok_or(WalletError::WalletLocked)?;
        let stronghold_password_hash = self
            .stronghold_password_hash
            .as_ref()
            .ok_or(WalletError::WalletLocked)?
            .to_vec();
        let session_submission_guard = self
            .active_submission_guard
            .as_ref()
            .ok_or(WalletError::WalletLocked)?
            .clone();
        let public_metadata_cache = self
            .active_public_metadata_cache
            .as_ref()
            .ok_or(WalletError::WalletLocked)?
            .clone();

        Ok(ActiveWalletAccessContext {
            session_id,
            account_id,
            wallet_network,
            wallet_secret_kind,
            vrsc_address: addresses.vrsc_address.clone(),
            eth_address: addresses.eth_address.clone(),
            btc_address: addresses.btc_address.clone(),
            session_submission_guard,
            stronghold_password_hash: Zeroizing::new(stronghold_password_hash),
            stronghold_store: self.stronghold_store.clone(),
            public_metadata_cache,
        })
    }

    pub(crate) fn expiry_monitor_registration(
        &self,
    ) -> Result<SessionExpiryRegistration, WalletError> {
        if !self.is_unlocked || self.is_expired() {
            return Err(WalletError::WalletLocked);
        }

        let session_id = self
            .active_session_id
            .as_ref()
            .ok_or(WalletError::WalletLocked)?
            .clone();
        let cancellation = self
            .active_submission_guard
            .as_ref()
            .ok_or(WalletError::WalletLocked)?
            .cancellation();
        let changes = self
            .active_expiry_changes
            .as_ref()
            .ok_or(WalletError::WalletLocked)?
            .subscribe();

        Ok(SessionExpiryRegistration {
            session_id,
            cancellation,
            changes,
        })
    }

    pub(crate) fn expiry_deadline_for(&self, session_id: &str) -> Option<Instant> {
        if !self.is_unlocked || self.active_session_id() != Some(session_id) {
            return None;
        }

        Some(
            self.last_activity_at
                .and_then(|last_activity| last_activity.checked_add(self.timeout_duration))
                .unwrap_or_else(Instant::now),
        )
    }

    fn notify_expiry_change(&self) {
        if let Some(changes) = &self.active_expiry_changes {
            changes.send_modify(|revision| *revision = revision.wrapping_add(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_session_timeout_minutes, SessionManager, ALLOWED_SESSION_TIMEOUT_MINUTES,
        DEFAULT_SESSION_TIMEOUT_MINUTES,
    };
    use crate::core::crypto::{derive_public_profile_from_material, Network};
    use crate::core::StrongholdStore;
    use crate::types::errors::WalletError;
    use crate::types::wallet::{WalletNetwork, WalletSecretKind};
    use std::time::Duration;
    use zeroize::Zeroizing;

    fn test_store() -> StrongholdStore {
        let base_path = std::env::temp_dir().join(format!(
            "lite_wallet_session_store_{}",
            uuid::Uuid::new_v4()
        ));
        StrongholdStore::new_for_tests(base_path)
    }

    #[test]
    fn normalize_session_timeout_accepts_allowlisted_values() {
        for value in ALLOWED_SESSION_TIMEOUT_MINUTES {
            assert_eq!(normalize_session_timeout_minutes(value), value);
        }
    }

    #[test]
    fn normalize_session_timeout_rejects_invalid_values() {
        assert_eq!(
            normalize_session_timeout_minutes(0),
            DEFAULT_SESSION_TIMEOUT_MINUTES
        );
        assert_eq!(
            normalize_session_timeout_minutes(10),
            DEFAULT_SESSION_TIMEOUT_MINUTES
        );
        assert_eq!(
            normalize_session_timeout_minutes(90),
            DEFAULT_SESSION_TIMEOUT_MINUTES
        );
    }

    #[test]
    fn unlock_with_profile_caches_only_public_wallet_state() {
        let store = test_store();
        let public_profile = derive_public_profile_from_material(
            "session public profile seed",
            WalletSecretKind::SeedText,
            Network::Mainnet,
        )
        .expect("public profile");
        let expected_vrsc_address = public_profile.address.clone();
        let expected_eth_address = public_profile.eth_address.clone();
        let expected_btc_address = public_profile.btc_address.clone();
        let mut session = SessionManager::new(store);

        session.unlock_with_profile(
            "account-1".to_string(),
            WalletNetwork::Mainnet,
            WalletSecretKind::SeedText,
            public_profile,
            Zeroizing::new(vec![1, 2, 3, 4]),
        );

        let context = session
            .active_wallet_access_context()
            .expect("active wallet access context");
        assert_eq!(context.account_id, "account-1");
        assert_eq!(context.wallet_network, WalletNetwork::Mainnet);
        assert_eq!(context.wallet_secret_kind, WalletSecretKind::SeedText);
        assert_eq!(context.vrsc_address, expected_vrsc_address);
        assert_eq!(context.eth_address, expected_eth_address);
        assert_eq!(context.btc_address, expected_btc_address);
        assert_eq!(context.password_hash(), &[1, 2, 3, 4]);
        assert_eq!(
            session.get_addresses().expect("session addresses"),
            (
                expected_vrsc_address,
                expected_eth_address,
                expected_btc_address
            )
        );
    }

    #[test]
    fn lock_clears_cached_session_state() {
        let store = test_store();
        let public_profile = derive_public_profile_from_material(
            "session lock clears state",
            WalletSecretKind::SeedText,
            Network::Mainnet,
        )
        .expect("public profile");
        let mut session = SessionManager::new(store);

        session.unlock_with_profile(
            "account-2".to_string(),
            WalletNetwork::Mainnet,
            WalletSecretKind::SeedText,
            public_profile,
            Zeroizing::new(vec![9, 9, 9, 9]),
        );
        session.lock();

        assert!(!session.is_unlocked());
        assert!(session.active_account_id().is_none());
        assert!(session.active_network().is_none());
        assert!(session.get_addresses().is_err());
        assert!(session.stronghold_password_hash_for_storage().is_err());
        assert!(session.active_wallet_access_context().is_err());
    }

    #[test]
    fn touch_activity_extends_session_from_last_user_interaction() {
        let store = test_store();
        let public_profile = derive_public_profile_from_material(
            "session activity extends timeout",
            WalletSecretKind::SeedText,
            Network::Mainnet,
        )
        .expect("public profile");
        let mut session = SessionManager::new(store);

        session.unlock_with_profile(
            "account-3".to_string(),
            WalletNetwork::Mainnet,
            WalletSecretKind::SeedText,
            public_profile,
            Zeroizing::new(vec![7, 7, 7, 7]),
        );
        session.set_timeout(Duration::from_millis(150));

        std::thread::sleep(Duration::from_millis(90));
        session.touch_activity().expect("session activity touch");
        std::thread::sleep(Duration::from_millis(90));
        assert!(!session.is_expired());

        std::thread::sleep(Duration::from_millis(90));
        assert!(session.is_expired());
    }

    #[test]
    fn touch_activity_rejects_locked_sessions() {
        let store = test_store();
        let mut session = SessionManager::new(store);

        assert!(matches!(
            session.touch_activity(),
            Err(WalletError::WalletLocked)
        ));
    }
    #[tokio::test]
    async fn cancelled_submission_guard_never_polls_transport() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        let guard = super::SessionSubmissionGuard::new();
        let polls = Arc::new(AtomicUsize::new(0));
        guard.invalidate();
        let result: Result<(), WalletError> = guard
            .run(std::future::poll_fn(|_| {
                polls.fetch_add(1, Ordering::SeqCst);
                std::task::Poll::Ready(Ok(()))
            }))
            .await;
        assert!(matches!(result, Err(WalletError::WalletLocked)));
        assert_eq!(polls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn lock_during_proof_discards_result_before_broadcast() {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        let guard = super::SessionSubmissionGuard::new();
        let broadcast = Arc::new(AtomicBool::new(false));
        let submitted = broadcast.clone();
        let operation_guard = guard.clone();
        let (proof_started, started) = tokio::sync::oneshot::channel();
        let (finish_proof, proof_finished) = tokio::sync::oneshot::channel();
        let operation = tokio::spawn(async move {
            operation_guard
                .run(async {
                    let _ = proof_started.send(());
                    proof_finished
                        .await
                        .map_err(|_| WalletError::OperationFailed)?;
                    submitted.store(true, Ordering::SeqCst);
                    Ok(())
                })
                .await
        });
        started.await.unwrap();
        guard.invalidate();
        let _ = finish_proof.send(());
        assert!(matches!(
            operation.await.unwrap(),
            Err(WalletError::WalletLocked)
        ));
        assert!(!broadcast.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn lock_after_first_transport_poll_prevents_further_polls() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        let guard = super::SessionSubmissionGuard::new();
        let polls = Arc::new(AtomicUsize::new(0));
        let transport_polls = polls.clone();
        let (started, first_poll) = tokio::sync::oneshot::channel();
        let mut started = Some(started);
        let operation_guard = guard.clone();
        let operation = tokio::spawn(async move {
            operation_guard
                .run(std::future::poll_fn(|_| {
                    transport_polls.fetch_add(1, Ordering::SeqCst);
                    if let Some(started) = started.take() {
                        let _ = started.send(());
                    }
                    std::task::Poll::<Result<(), WalletError>>::Pending
                }))
                .await
        });
        first_poll.await.unwrap();
        guard.invalidate();
        assert!(matches!(
            operation.await.unwrap(),
            Err(WalletError::WalletLocked)
        ));
        assert_eq!(polls.load(Ordering::SeqCst), 1);
    }
}
