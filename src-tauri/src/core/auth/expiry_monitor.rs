use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tokio::time::Instant;

use crate::core::channels::PreflightStore;
use crate::core::coins::CoinRegistry;

use super::lifecycle::{clear_expired_wallet_session_if_current, ExpiredSessionCleanupResult};
use super::{
    GuardSessionManager, ProvisioningSignatureStore, SessionExpiryRegistration, SessionManager,
};

const EVENT_SESSION_EXPIRED: &str = "wallet://session-expired";
type ExpiryNotifier = Arc<dyn Fn(&str) + Send + Sync>;

/// Spawn the monitor for one unlock instance.
///
/// The task is intentionally generation-owned instead of stored in a shared
/// controller: locking or replacing the session cancels it, and a late task can
/// only observe that its captured session is stale. This avoids task replacement
/// races and prevents cleanup from being aborted after invalidation commits.
pub(crate) fn spawn_session_expiry_monitor(
    app_handle: AppHandle,
    registration: SessionExpiryRegistration,
    session_manager: Arc<Mutex<SessionManager>>,
    guard_session_manager: Arc<Mutex<GuardSessionManager>>,
    preflight_store: PreflightStore,
    provisioning_signature_store: ProvisioningSignatureStore,
    coin_registry: Arc<CoinRegistry>,
) {
    let notifier: ExpiryNotifier = Arc::new(move |_| {
        if let Err(err) = app_handle.emit(EVENT_SESSION_EXPIRED, ()) {
            println!("[SESSION] Emit session-expired failed: {:?}", err);
        }
    });

    drop(tokio::spawn(run_session_expiry_monitor(
        registration,
        session_manager,
        guard_session_manager,
        preflight_store,
        provisioning_signature_store,
        coin_registry,
        notifier,
    )));
}

async fn run_session_expiry_monitor(
    mut registration: SessionExpiryRegistration,
    session_manager: Arc<Mutex<SessionManager>>,
    guard_session_manager: Arc<Mutex<GuardSessionManager>>,
    preflight_store: PreflightStore,
    provisioning_signature_store: ProvisioningSignatureStore,
    coin_registry: Arc<CoinRegistry>,
    notifier: ExpiryNotifier,
) {
    loop {
        if registration.cancellation.is_cancelled() {
            return;
        }

        let deadline = {
            let session = session_manager.lock().await;
            session.expiry_deadline_for(&registration.session_id)
        };
        let Some(deadline) = deadline else {
            return;
        };

        if Instant::now() >= deadline {
            let expired_session_id = registration.session_id.clone();
            let expiry_notifier = Arc::clone(&notifier);
            match clear_expired_wallet_session_if_current(
                &registration.session_id,
                &session_manager,
                &guard_session_manager,
                &preflight_store,
                &provisioning_signature_store,
                &coin_registry,
                move || expiry_notifier(&expired_session_id),
            )
            .await
            {
                ExpiredSessionCleanupResult::Cleared
                | ExpiredSessionCleanupResult::SessionChanged => return,
                ExpiredSessionCleanupResult::Renewed => continue,
            }
        }

        tokio::select! {
            biased;
            _ = registration.cancellation.cancelled() => return,
            changed = registration.changes.changed() => {
                if changed.is_err() {
                    return;
                }
            }
            _ = tokio::time::sleep_until(deadline) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::auth::{clear_wallet_session_if_current, ProvisioningSignatureChallenge};
    use crate::core::channels::PreflightRecord;
    use crate::core::crypto::{derive_keys_v1, derive_public_profile_from_material, Network};
    use crate::core::StrongholdStore;
    use crate::types::wallet::{WalletNetwork, WalletSecretKind};
    use crate::types::WalletError;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::Poll;
    use std::time::Duration;
    use tokio::sync::mpsc;
    use zeroize::Zeroizing;

    struct TestWalletState {
        session: Arc<Mutex<SessionManager>>,
        guard: Arc<Mutex<GuardSessionManager>>,
        preflights: PreflightStore,
        provisioning: ProvisioningSignatureStore,
        coins: Arc<CoinRegistry>,
    }

    impl TestWalletState {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "lite_wallet_expiry_monitor_{}",
                uuid::Uuid::new_v4()
            ));
            Self {
                session: Arc::new(Mutex::new(SessionManager::new(
                    StrongholdStore::new_for_tests(path),
                ))),
                guard: Arc::new(Mutex::new(GuardSessionManager::new())),
                preflights: PreflightStore::new(),
                provisioning: ProvisioningSignatureStore::new(),
                coins: Arc::new(CoinRegistry::new()),
            }
        }

        async fn unlock(
            &self,
            material: &str,
            timeout: Duration,
        ) -> (
            String,
            SessionExpiryRegistration,
            super::super::SessionSubmissionGuard,
        ) {
            let profile = derive_public_profile_from_material(
                material,
                WalletSecretKind::SeedText,
                Network::Mainnet,
            )
            .expect("public profile");
            let mut session = self.session.lock().await;
            let session_id = session.unlock_with_profile(
                "account-1".to_string(),
                WalletNetwork::Mainnet,
                WalletSecretKind::SeedText,
                profile,
                Zeroizing::new(vec![1, 2, 3]),
            );
            session.set_timeout(timeout);
            self.preflights.activate_wallet_session(&session_id);
            self.provisioning.activate_wallet_session(&session_id);
            self.coins.set_active_account(Some("account-1".to_string()));
            let registration = session
                .expiry_monitor_registration()
                .expect("expiry registration");
            let submission_guard = session
                .active_wallet_access_context()
                .expect("active wallet context")
                .session_submission_guard();
            (session_id, registration, submission_guard)
        }

        fn spawn(
            &self,
            registration: SessionExpiryRegistration,
            notifier: ExpiryNotifier,
        ) -> tokio::task::JoinHandle<()> {
            tokio::spawn(run_session_expiry_monitor(
                registration,
                Arc::clone(&self.session),
                Arc::clone(&self.guard),
                self.preflights.clone(),
                self.provisioning.clone(),
                Arc::clone(&self.coins),
                notifier,
            ))
        }
    }

    fn notification_channel() -> (ExpiryNotifier, mpsc::UnboundedReceiver<String>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let notifier: ExpiryNotifier = Arc::new(move |session_id| {
            let _ = sender.send(session_id.to_string());
        });
        (notifier, receiver)
    }

    fn record(session_id: &str) -> PreflightRecord {
        PreflightRecord {
            session_id: session_id.to_string(),
            channel_id: "vrpc.test".to_string(),
            account_id: "account-1".to_string(),
            payload: json!({"hex": "00"}),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn monitor_expires_without_update_engine_and_clears_session_state_once() {
        let state = TestWalletState::new();
        let (session_id, registration, submission_guard) = state
            .unlock("expiry monitor full cleanup", Duration::from_secs(30))
            .await;
        assert!(state.preflights.put("old".to_string(), record(&session_id)));
        assert!(state.provisioning.put(
            "challenge".to_string(),
            ProvisioningSignatureChallenge {
                session_id: session_id.clone(),
                account_id: "account-1".to_string(),
                network: WalletNetwork::Mainnet,
                system_id: "system-1".to_string(),
                challenge_hash: [7; 32],
                expires_at: u64::MAX,
            },
        ));
        let guard_id = state
            .guard
            .lock()
            .await
            .begin_session(
                derive_keys_v1("expiry guard seed", Network::Mainnet).expect("guard keys"),
                WalletNetwork::Mainnet,
            )
            .id;

        let provider_polls = AtomicUsize::new(0);
        assert!(submission_guard
            .poll_admitted(|| {
                provider_polls.fetch_add(1, Ordering::SeqCst);
                Poll::<Result<(), WalletError>>::Pending
            })
            .is_pending());

        let (notifier, mut notifications) = notification_channel();
        let monitor = state.spawn(registration, notifier);
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(30)).await;

        assert_eq!(
            notifications.recv().await.as_deref(),
            Some(session_id.as_str())
        );
        monitor.await.expect("expiry monitor task");
        assert!(matches!(
            notifications.try_recv(),
            Err(mpsc::error::TryRecvError::Empty | mpsc::error::TryRecvError::Disconnected)
        ));
        assert!(state
            .session
            .lock()
            .await
            .active_wallet_access_context()
            .is_err());
        assert!(state.preflights.get("old", &session_id).is_none());
        assert!(state.provisioning.take("challenge", &session_id).is_none());
        assert!(state.guard.lock().await.get_session(&guard_id).is_err());
        assert!(state.coins.active_account_id_for_tests().is_none());

        let next_poll = submission_guard.poll_admitted(|| {
            provider_polls.fetch_add(1, Ordering::SeqCst);
            Poll::Ready(Ok(()))
        });
        assert!(matches!(
            next_poll,
            Poll::Ready(Err(WalletError::WalletLocked))
        ));
        assert_eq!(provider_polls.load(Ordering::SeqCst), 1);

        tokio::time::advance(Duration::from_secs(3_600)).await;
        assert!(matches!(
            notifications.try_recv(),
            Err(mpsc::error::TryRecvError::Empty | mpsc::error::TryRecvError::Disconnected)
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn monitor_reschedules_for_activity_and_timeout_changes() {
        let state = TestWalletState::new();
        let (session_id, registration, _) = state
            .unlock("expiry monitor reschedule", Duration::from_secs(10))
            .await;
        let (notifier, mut notifications) = notification_channel();
        let monitor = state.spawn(registration, notifier);
        tokio::task::yield_now().await;

        tokio::time::advance(Duration::from_secs(6)).await;
        state
            .session
            .lock()
            .await
            .touch_activity()
            .expect("activity extends session");
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(5)).await;
        assert!(state.session.lock().await.is_current_session(&session_id));
        assert!(matches!(
            notifications.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));

        state
            .session
            .lock()
            .await
            .set_timeout(Duration::from_secs(2));
        assert_eq!(
            notifications.recv().await.as_deref(),
            Some(session_id.as_str())
        );
        monitor.await.expect("rescheduled expiry monitor");
    }

    #[tokio::test(start_paused = true)]
    async fn late_old_monitor_cannot_replace_or_notify_for_new_session() {
        let state = TestWalletState::new();
        let (first_id, first_registration, _) = state
            .unlock("first expiry monitor", Duration::from_secs(5))
            .await;

        assert!(
            clear_wallet_session_if_current(
                None,
                &state.session,
                &state.guard,
                &state.preflights,
                &state.provisioning,
                &state.coins,
            )
            .await
        );
        let (second_id, second_registration, _) = state
            .unlock("second expiry monitor", Duration::from_secs(10))
            .await;
        assert_ne!(first_id, second_id);

        let (notifier, mut notifications) = notification_channel();
        let current_monitor = state.spawn(second_registration, Arc::clone(&notifier));
        let late_old_monitor = state.spawn(first_registration, notifier);
        late_old_monitor.await.expect("late stale monitor");

        tokio::time::advance(Duration::from_secs(6)).await;
        assert!(state.session.lock().await.is_current_session(&second_id));
        assert!(matches!(
            notifications.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));

        tokio::time::advance(Duration::from_secs(4)).await;
        assert_eq!(
            notifications.recv().await.as_deref(),
            Some(second_id.as_str())
        );
        current_monitor.await.expect("current monitor");

        let (third_id, third_registration, _) = state
            .unlock("third expiry monitor", Duration::from_secs(10))
            .await;
        let third_monitor = state.spawn(third_registration, Arc::new(|_| {}));
        assert!(
            clear_wallet_session_if_current(
                None,
                &state.session,
                &state.guard,
                &state.preflights,
                &state.provisioning,
                &state.coins,
            )
            .await
        );
        third_monitor.await.expect("manually stopped monitor");
        assert!(!state.session.lock().await.is_current_session(&third_id));
    }
}
