use std::sync::Arc;

use tokio::sync::Mutex;

use crate::core::channels::{dlight_private, PreflightStore};
use crate::core::coins::CoinRegistry;

use super::{GuardSessionManager, ProvisioningSignatureStore, SessionManager};

/// Clear all state that is valid only for one unlocked wallet session.
///
/// When `expected_session_id` is supplied, a cleanup started by an older update
/// task cannot clear a newly unlocked session. The session lock stays held while
/// runtimes stop so a new unlock cannot install state midway through cleanup.
pub async fn clear_wallet_session_if_current(
    expected_session_id: Option<&str>,
    session_manager: &Arc<Mutex<SessionManager>>,
    guard_session_manager: &Arc<Mutex<GuardSessionManager>>,
    preflight_store: &PreflightStore,
    provisioning_signature_store: &ProvisioningSignatureStore,
    coin_registry: &Arc<CoinRegistry>,
) -> bool {
    let mut session = session_manager.lock().await;
    if expected_session_id.is_some() && session.active_session_id() != expected_session_id {
        return false;
    }

    session.lock();
    dlight_private::stop_all_runtimes().await;
    preflight_store.clear();
    provisioning_signature_store.clear();
    coin_registry.set_active_account(None);
    guard_session_manager.lock().await.clear();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::channels::PreflightRecord;
    use crate::core::crypto::{derive_keys_v1, derive_public_profile_from_material, Network};
    use crate::core::StrongholdStore;
    use crate::types::wallet::{WalletNetwork, WalletSecretKind};
    use crate::types::WalletError;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::Poll;
    use std::time::Duration;
    use tokio::time::timeout;
    use zeroize::Zeroizing;

    fn test_session() -> Arc<Mutex<SessionManager>> {
        let path = std::env::temp_dir().join(format!(
            "lite_wallet_lifecycle_store_{}",
            uuid::Uuid::new_v4()
        ));
        Arc::new(Mutex::new(SessionManager::new(
            StrongholdStore::new_for_tests(path),
        )))
    }

    async fn unlock(session: &Arc<Mutex<SessionManager>>) -> String {
        let profile = derive_public_profile_from_material(
            "lifecycle test seed",
            WalletSecretKind::SeedText,
            Network::Mainnet,
        )
        .expect("public profile");
        session.lock().await.unlock_with_profile(
            "account-1".to_string(),
            WalletNetwork::Mainnet,
            WalletSecretKind::SeedText,
            profile,
            Zeroizing::new(vec![1, 2, 3]),
        )
    }

    fn record(session_id: &str) -> PreflightRecord {
        PreflightRecord {
            session_id: session_id.to_string(),
            channel_id: "vrpc.test".to_string(),
            account_id: "account-1".to_string(),
            payload: json!({"hex": "00"}),
        }
    }

    #[tokio::test]
    async fn expiry_cleanup_clears_preflights_and_guard_before_same_account_reunlock() {
        let session = test_session();
        let guard = Arc::new(Mutex::new(GuardSessionManager::new()));
        let preflights = PreflightStore::new();
        let provisioning = ProvisioningSignatureStore::new();
        let coins = Arc::new(CoinRegistry::new());
        let first_id = unlock(&session).await;
        preflights.activate_wallet_session(&first_id);
        assert!(preflights.put("old".to_string(), record(&first_id)));

        let guard_keys =
            derive_keys_v1("guard lifecycle seed", Network::Mainnet).expect("guard keys");
        let guard_id = guard
            .lock()
            .await
            .begin_session(guard_keys, WalletNetwork::Mainnet)
            .id;

        assert!(
            clear_wallet_session_if_current(
                Some(&first_id),
                &session,
                &guard,
                &preflights,
                &provisioning,
                &coins,
            )
            .await
        );
        assert!(!session.lock().await.is_unlocked());
        assert!(guard.lock().await.get_session(&guard_id).is_err());

        let second_id = unlock(&session).await;
        preflights.activate_wallet_session(&second_id);
        assert_ne!(first_id, second_id);
        assert!(preflights.take("old", &second_id).is_none());
    }

    #[tokio::test]
    async fn late_cleanup_from_old_session_cannot_clear_new_session() {
        let session = test_session();
        let guard = Arc::new(Mutex::new(GuardSessionManager::new()));
        let preflights = PreflightStore::new();
        let provisioning = ProvisioningSignatureStore::new();
        let coins = Arc::new(CoinRegistry::new());
        let first_id = unlock(&session).await;
        let second_id = unlock(&session).await;
        preflights.activate_wallet_session(&second_id);
        assert!(preflights.put("new".to_string(), record(&second_id)));

        assert!(
            !clear_wallet_session_if_current(
                Some(&first_id),
                &session,
                &guard,
                &preflights,
                &provisioning,
                &coins,
            )
            .await
        );
        assert!(session.lock().await.is_current_session(&second_id));
        assert!(preflights.get("new", &second_id).is_some());
    }

    #[tokio::test]
    async fn cleanup_invalidates_submission_before_waiting_for_guard_cleanup() {
        let session = test_session();
        let guard = Arc::new(Mutex::new(GuardSessionManager::new()));
        let preflights = PreflightStore::new();
        let provisioning = ProvisioningSignatureStore::new();
        let coins = Arc::new(CoinRegistry::new());
        let first_id = unlock(&session).await;
        preflights.activate_wallet_session(&first_id);
        assert!(preflights.put("old".to_string(), record(&first_id)));

        let submission_guard = session
            .lock()
            .await
            .active_wallet_access_context()
            .expect("active context")
            .session_submission_guard();
        let provider_polls = AtomicUsize::new(0);
        let first_poll = submission_guard.poll_admitted(|| {
            provider_polls.fetch_add(1, Ordering::SeqCst);
            Poll::<Result<(), WalletError>>::Pending
        });
        assert!(first_poll.is_pending());

        let held_guard_cleanup = guard.lock().await;
        let cleanup = tokio::spawn({
            let expected_id = first_id.clone();
            let session = Arc::clone(&session);
            let guard = Arc::clone(&guard);
            let preflights = preflights.clone();
            let provisioning = provisioning.clone();
            let coins = Arc::clone(&coins);
            async move {
                clear_wallet_session_if_current(
                    Some(&expected_id),
                    &session,
                    &guard,
                    &preflights,
                    &provisioning,
                    &coins,
                )
                .await
            }
        });

        timeout(
            Duration::from_secs(5),
            submission_guard.cancellation().cancelled(),
        )
        .await
        .expect("cleanup invalidates the old submission before blocking");
        timeout(Duration::from_secs(5), async {
            while preflights.get("old", &first_id).is_some() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("cleanup reaches the held guard-session mutex");
        assert!(!cleanup.is_finished());

        let next_poll = submission_guard.poll_admitted(|| {
            provider_polls.fetch_add(1, Ordering::SeqCst);
            Poll::Ready(Ok(()))
        });
        assert!(matches!(
            next_poll,
            Poll::Ready(Err(WalletError::WalletLocked))
        ));
        assert_eq!(provider_polls.load(Ordering::SeqCst), 1);

        drop(held_guard_cleanup);
        assert!(cleanup.await.expect("cleanup task"));

        let second_id = unlock(&session).await;
        preflights.activate_wallet_session(&second_id);
        assert!(preflights.put("new".to_string(), record(&second_id)));
        assert!(
            !clear_wallet_session_if_current(
                Some(&first_id),
                &session,
                &guard,
                &preflights,
                &provisioning,
                &coins,
            )
            .await
        );
        assert!(session.lock().await.is_current_session(&second_id));
        assert!(preflights.get("new", &second_id).is_some());
    }
}
