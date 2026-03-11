//
// Session management with timeout and zeroization
// Security: Keeps only public wallet state and the Stronghold unlock hash in memory
// Last Updated: Signing secrets now load from Stronghold on demand instead of living in session

use crate::core::auth::stronghold_store::StrongholdStore;
use crate::core::crypto::{derive_private_scalar_from_material, DerivedPublicProfile};
use crate::types::errors::WalletError;
use crate::types::wallet::{WalletNetwork, WalletSecretKind};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use zeroize::Zeroizing;

pub const ALLOWED_SESSION_TIMEOUT_MINUTES: [u64; 4] = [5, 15, 30, 60];
pub const DEFAULT_SESSION_TIMEOUT_MINUTES: u64 = 15;

#[derive(Clone)]
struct SessionAddresses {
    vrsc_address: String,
    eth_address: String,
    btc_address: String,
}

#[derive(Clone)]
pub struct ActiveWalletAccessContext {
    pub account_id: String,
    pub wallet_network: WalletNetwork,
    pub wallet_secret_kind: WalletSecretKind,
    pub vrsc_address: String,
    pub eth_address: String,
    pub btc_address: String,
    stronghold_password_hash: Zeroizing<Vec<u8>>,
    pub stronghold_store: StrongholdStore,
}

impl ActiveWalletAccessContext {
    pub fn password_hash(&self) -> &[u8] {
        self.stronghold_password_hash.as_ref()
    }
}

pub async fn capture_active_wallet_access_context(
    session_manager: &Arc<Mutex<SessionManager>>,
) -> Result<ActiveWalletAccessContext, WalletError> {
    let session = session_manager.lock().await;
    session.active_wallet_access_context()
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
    active_account_id: Option<String>,
    unlocked_at: Option<Instant>,
    timeout_duration: Duration,
    active_network: Option<WalletNetwork>,
    active_secret_kind: Option<WalletSecretKind>,
    active_addresses: Option<SessionAddresses>,
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
            active_account_id: None,
            unlocked_at: None,
            timeout_duration: Duration::from_secs(default_timeout_minutes * 60),
            active_network: None,
            active_secret_kind: None,
            active_addresses: None,
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
    ) {
        println!("[SESSION] Unlock requested for account: {}", account_id);

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
        self.unlocked_at = Some(Instant::now());

        println!("[SESSION] Unlock successful");
    }

    /// Lock wallet session and zeroize all session-scoped secrets.
    pub fn lock(&mut self) {
        println!("[SESSION] Locking wallet");

        self.active_account_id = None;
        self.active_network = None;
        self.active_secret_kind = None;
        self.active_addresses = None;
        self.stronghold_password_hash = None;
        self.is_unlocked = false;
        self.unlocked_at = None;

        println!("[SESSION] Wallet locked, session secrets zeroized");
    }

    /// Check if session has expired.
    pub fn is_expired(&self) -> bool {
        if !self.is_unlocked {
            return true;
        }

        if let Some(unlocked_at) = self.unlocked_at {
            unlocked_at.elapsed() > self.timeout_duration
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

    /// Returns the selected wallet network for the active session.
    pub fn active_network(&self) -> Option<WalletNetwork> {
        self.active_network
    }

    /// Set session timeout duration.
    pub fn set_timeout(&mut self, duration: Duration) {
        self.timeout_duration = duration;
    }

    /// Set timeout from minute granularity with strict allowlist normalization.
    pub fn set_timeout_minutes(&mut self, minutes: u64) -> u64 {
        let normalized_minutes = normalize_session_timeout_minutes(minutes);
        self.set_timeout(Duration::from_secs(normalized_minutes * 60));
        normalized_minutes
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

        Ok(ActiveWalletAccessContext {
            account_id,
            wallet_network,
            wallet_secret_kind,
            vrsc_address: addresses.vrsc_address.clone(),
            eth_address: addresses.eth_address.clone(),
            btc_address: addresses.btc_address.clone(),
            stronghold_password_hash: Zeroizing::new(stronghold_password_hash),
            stronghold_store: self.stronghold_store.clone(),
        })
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
    use crate::types::wallet::{WalletNetwork, WalletSecretKind};
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
}
