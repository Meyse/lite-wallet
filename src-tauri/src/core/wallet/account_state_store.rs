use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::types::generic_request::ProvisioningJobRecord;
use crate::types::wallet::WalletNetwork;
use crate::types::{PendingIdentityProfileUpdate, WalletError, WatchlistEntry};

const ACCOUNT_STATE_SCHEMA_VERSION: u8 = 1;
const ACTIVE_ASSETS_SCHEMA_VERSION: u8 = 1;
static ACCOUNT_STATE_WRITE_GATE: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[derive(Clone, Default)]
pub(crate) struct LegacyWatchlistNetwork {
    pub entries: Vec<WatchlistEntry>,
    pub addresses: Vec<String>,
}

#[derive(Clone, Default)]
pub(crate) struct LegacyWatchlistSources {
    pub mainnet: LegacyWatchlistNetwork,
    pub testnet: LegacyWatchlistNetwork,
    pub encrypted_version: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AccountStateActiveAssets {
    #[serde(default)]
    initialized: bool,
    #[serde(default)]
    profile_version: u8,
    #[serde(default)]
    coin_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AccountStateNetwork {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    watched_vrpc_addresses: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    watchlist_entries: Vec<WatchlistEntry>,
    #[serde(default)]
    active_assets_schema_version: u8,
    #[serde(default)]
    active_assets: AccountStateActiveAssets,
    #[serde(default)]
    hidden_asset_keys: Vec<String>,
    #[serde(default)]
    provisioning_jobs: Vec<ProvisioningJobRecord>,
    #[serde(default)]
    pending_identity_profiles: Vec<PendingIdentityProfileUpdate>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountStateSnapshot {
    schema_version: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    watchlist_encrypted_version: Option<u8>,
    #[serde(default)]
    mainnet: AccountStateNetwork,
    #[serde(default)]
    testnet: AccountStateNetwork,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

impl Default for AccountStateSnapshot {
    fn default() -> Self {
        Self {
            schema_version: ACCOUNT_STATE_SCHEMA_VERSION,
            watchlist_encrypted_version: None,
            mainnet: AccountStateNetwork {
                active_assets_schema_version: ACTIVE_ASSETS_SCHEMA_VERSION,
                ..AccountStateNetwork::default()
            },
            testnet: AccountStateNetwork {
                active_assets_schema_version: ACTIVE_ASSETS_SCHEMA_VERSION,
                ..AccountStateNetwork::default()
            },
            extra: Map::new(),
        }
    }
}

#[derive(Clone)]
pub struct AccountStateStore {
    base_path: PathBuf,
}

impl AccountStateStore {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub fn account_state_path(&self, account_id: &str) -> PathBuf {
        self.base_path
            .join("accounts")
            .join(account_id)
            .join("account_state.json")
    }

    #[cfg(test)]
    pub fn load_watched_vrpc_addresses(
        &self,
        account_id: &str,
        network: WalletNetwork,
    ) -> Result<Vec<String>, WalletError> {
        let state = self.load_snapshot(account_id)?;
        Ok(self
            .network_ref(&state, network)
            .watched_vrpc_addresses
            .clone())
    }

    #[cfg(test)]
    pub fn store_watched_vrpc_addresses(
        &self,
        account_id: &str,
        network: WalletNetwork,
        addresses: &[String],
    ) -> Result<(), WalletError> {
        self.update_snapshot(account_id, |state| {
            if state.watchlist_encrypted_version.is_some() {
                return Err(WalletError::OperationFailed);
            }
            self.network_mut(state, network).watched_vrpc_addresses = addresses.to_vec();
            Ok(())
        })
    }

    #[cfg(test)]
    pub fn load_watchlist_entries(
        &self,
        account_id: &str,
        network: WalletNetwork,
    ) -> Result<Vec<WatchlistEntry>, WalletError> {
        let state = self.load_snapshot(account_id)?;
        Ok(self.network_ref(&state, network).watchlist_entries.clone())
    }

    #[cfg(test)]
    pub fn store_watchlist_entries(
        &self,
        account_id: &str,
        network: WalletNetwork,
        entries: &[WatchlistEntry],
    ) -> Result<(), WalletError> {
        self.update_snapshot(account_id, |state| {
            if state.watchlist_encrypted_version.is_some() {
                return Err(WalletError::OperationFailed);
            }
            let network_state = self.network_mut(state, network);
            network_state.watchlist_entries = entries.to_vec();
            network_state.watched_vrpc_addresses =
                entries.iter().map(|entry| entry.address.clone()).collect();
            Ok(())
        })
    }

    pub fn load_active_assets(
        &self,
        account_id: &str,
        network: WalletNetwork,
    ) -> Result<(bool, Vec<String>, u8), WalletError> {
        let state = self.load_snapshot(account_id)?;
        let network_state = self.network_ref(&state, network);
        Ok((
            network_state.active_assets.initialized,
            network_state.active_assets.coin_ids.clone(),
            network_state.active_assets.profile_version,
        ))
    }

    pub fn store_active_assets(
        &self,
        account_id: &str,
        network: WalletNetwork,
        initialized: bool,
        profile_version: u8,
        coin_ids: &[String],
    ) -> Result<(), WalletError> {
        self.update_snapshot(account_id, |state| {
            let network_state = self.network_mut(state, network);
            network_state.active_assets_schema_version = ACTIVE_ASSETS_SCHEMA_VERSION;
            network_state.active_assets = AccountStateActiveAssets {
                initialized,
                profile_version,
                coin_ids: coin_ids.to_vec(),
            };
            Ok(())
        })
    }

    pub fn load_asset_preferences(
        &self,
        account_id: &str,
        network: WalletNetwork,
    ) -> Result<(bool, Vec<String>, u8, Vec<String>), WalletError> {
        let state = self.load_snapshot(account_id)?;
        let network_state = self.network_ref(&state, network);
        Ok((
            network_state.active_assets.initialized,
            network_state.active_assets.coin_ids.clone(),
            network_state.active_assets.profile_version,
            network_state.hidden_asset_keys.clone(),
        ))
    }

    pub fn store_asset_preferences(
        &self,
        account_id: &str,
        network: WalletNetwork,
        profile_version: u8,
        portfolio_coin_ids: &[String],
        hidden_asset_keys: &[String],
    ) -> Result<(), WalletError> {
        self.update_snapshot(account_id, |state| {
            let network_state = self.network_mut(state, network);
            network_state.active_assets_schema_version = ACTIVE_ASSETS_SCHEMA_VERSION;
            network_state.active_assets = AccountStateActiveAssets {
                initialized: true,
                profile_version,
                coin_ids: portfolio_coin_ids.to_vec(),
            };
            network_state.hidden_asset_keys = hidden_asset_keys.to_vec();
            Ok(())
        })
    }

    pub fn load_provisioning_jobs(
        &self,
        account_id: &str,
        network: WalletNetwork,
    ) -> Result<Vec<ProvisioningJobRecord>, WalletError> {
        let state = self.load_snapshot(account_id)?;
        Ok(self.network_ref(&state, network).provisioning_jobs.clone())
    }

    pub fn store_provisioning_jobs(
        &self,
        account_id: &str,
        network: WalletNetwork,
        jobs: &[ProvisioningJobRecord],
    ) -> Result<(), WalletError> {
        self.update_snapshot(account_id, |state| {
            self.network_mut(state, network).provisioning_jobs = jobs.to_vec();
            Ok(())
        })
    }

    pub fn load_pending_identity_profiles(
        &self,
        account_id: &str,
        network: WalletNetwork,
    ) -> Result<Vec<PendingIdentityProfileUpdate>, WalletError> {
        let state = self.load_snapshot(account_id)?;
        Ok(self
            .network_ref(&state, network)
            .pending_identity_profiles
            .clone())
    }

    pub fn store_pending_identity_profiles(
        &self,
        account_id: &str,
        network: WalletNetwork,
        records: &[PendingIdentityProfileUpdate],
    ) -> Result<(), WalletError> {
        let mut normalized = Vec::new();
        for record in records.iter().rev() {
            if record.identity_address.trim().is_empty()
                || record.txid.len() != 64
                || normalized
                    .iter()
                    .any(|existing: &PendingIdentityProfileUpdate| {
                        existing
                            .identity_address
                            .eq_ignore_ascii_case(&record.identity_address)
                    })
            {
                continue;
            }
            normalized.push(record.clone());
            if normalized.len() == 20 {
                break;
            }
        }
        normalized.reverse();

        self.update_snapshot(account_id, |state| {
            self.network_mut(state, network).pending_identity_profiles = normalized;
            Ok(())
        })
    }

    pub(crate) fn legacy_watchlist_sources(
        &self,
        account_id: &str,
    ) -> Result<LegacyWatchlistSources, WalletError> {
        let state = self.load_snapshot(account_id)?;
        Ok(LegacyWatchlistSources {
            mainnet: LegacyWatchlistNetwork {
                entries: state.mainnet.watchlist_entries,
                addresses: state.mainnet.watched_vrpc_addresses,
            },
            testnet: LegacyWatchlistNetwork {
                entries: state.testnet.watchlist_entries,
                addresses: state.testnet.watched_vrpc_addresses,
            },
            encrypted_version: state.watchlist_encrypted_version,
        })
    }

    pub(crate) fn clear_legacy_watchlist_fields(
        &self,
        account_id: &str,
    ) -> Result<(), WalletError> {
        let _gate = ACCOUNT_STATE_WRITE_GATE
            .lock()
            .map_err(|_| WalletError::OperationFailed)?;
        let mut state = self.load_snapshot(account_id)?;
        state.mainnet.watchlist_entries.clear();
        state.mainnet.watched_vrpc_addresses.clear();
        state.testnet.watchlist_entries.clear();
        state.testnet.watched_vrpc_addresses.clear();
        state.watchlist_encrypted_version = Some(1);
        self.save_snapshot(account_id, &state)?;
        // Interrupted writes before migration may have left plaintext temp
        // files. They were never committed and are safe to retire here.
        let path = self.account_state_path(account_id);
        if let Some(parent) = path.parent() {
            for item in fs::read_dir(parent).map_err(|_| WalletError::OperationFailed)? {
                let item = item.map_err(|_| WalletError::OperationFailed)?;
                let name = item.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("account_state.") && name.ends_with(".tmp") {
                    fs::remove_file(item.path()).map_err(|_| WalletError::OperationFailed)?;
                }
            }
        }
        Ok(())
    }

    fn update_snapshot<T>(
        &self,
        account_id: &str,
        update: impl FnOnce(&mut AccountStateSnapshot) -> Result<T, WalletError>,
    ) -> Result<T, WalletError> {
        let _gate = ACCOUNT_STATE_WRITE_GATE
            .lock()
            .map_err(|_| WalletError::OperationFailed)?;
        let mut state = self.load_snapshot(account_id)?;
        let result = update(&mut state)?;
        self.save_snapshot(account_id, &state)?;
        Ok(result)
    }

    fn load_snapshot(&self, account_id: &str) -> Result<AccountStateSnapshot, WalletError> {
        let path = self.account_state_path(account_id);
        if !path.exists() {
            return Ok(AccountStateSnapshot::default());
        }

        let bytes = fs::read(path).map_err(|_| WalletError::OperationFailed)?;
        let snapshot = serde_json::from_slice::<AccountStateSnapshot>(&bytes)
            .map_err(|_| WalletError::OperationFailed)?;
        if snapshot.schema_version != ACCOUNT_STATE_SCHEMA_VERSION {
            return Err(WalletError::OperationFailed);
        }
        Ok(snapshot)
    }

    fn save_snapshot(
        &self,
        account_id: &str,
        snapshot: &AccountStateSnapshot,
    ) -> Result<(), WalletError> {
        let path = self.account_state_path(account_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|_| WalletError::OperationFailed)?;
        }
        let bytes =
            serde_json::to_vec_pretty(snapshot).map_err(|_| WalletError::OperationFailed)?;
        let temp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| {
            let mut file = fs::File::create(&temp).map_err(|_| WalletError::OperationFailed)?;
            file.write_all(&bytes)
                .map_err(|_| WalletError::OperationFailed)?;
            file.sync_all().map_err(|_| WalletError::OperationFailed)?;
            fs::rename(&temp, &path).map_err(|_| WalletError::OperationFailed)?;
            #[cfg(unix)]
            if let Some(parent) = path.parent() {
                fs::File::open(parent)
                    .and_then(|file| file.sync_all())
                    .map_err(|_| WalletError::OperationFailed)?;
            }
            Ok(())
        })();
        if temp.exists() {
            let _ = fs::remove_file(temp);
        }
        result
    }

    fn network_ref<'a>(
        &self,
        snapshot: &'a AccountStateSnapshot,
        network: WalletNetwork,
    ) -> &'a AccountStateNetwork {
        match network {
            WalletNetwork::Mainnet => &snapshot.mainnet,
            WalletNetwork::Testnet => &snapshot.testnet,
        }
    }

    fn network_mut<'a>(
        &self,
        snapshot: &'a mut AccountStateSnapshot,
        network: WalletNetwork,
    ) -> &'a mut AccountStateNetwork {
        match network {
            WalletNetwork::Mainnet => &mut snapshot.mainnet,
            WalletNetwork::Testnet => &mut snapshot.testnet,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::AccountStateStore;
    use crate::core::auth::stronghold_store::ACTIVE_ASSETS_PROFILE_VERSION;
    use crate::types::wallet::WalletNetwork;
    use crate::types::{
        IdentityProfileSnapshot, PendingIdentityProfileUpdate, WatchlistEntry, WatchlistTargetKind,
    };

    fn temp_store() -> AccountStateStore {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        AccountStateStore::new(
            std::env::temp_dir().join(format!("lite_wallet_account_state_store_{}", unique)),
        )
    }

    #[test]
    fn watched_addresses_round_trip_is_network_scoped() {
        let store = temp_store();
        store
            .store_watched_vrpc_addresses("account", WalletNetwork::Mainnet, &["RMain".to_string()])
            .expect("store mainnet");
        store
            .store_watched_vrpc_addresses("account", WalletNetwork::Testnet, &["RTest".to_string()])
            .expect("store testnet");

        assert_eq!(
            store
                .load_watched_vrpc_addresses("account", WalletNetwork::Mainnet)
                .expect("load mainnet"),
            vec!["RMain".to_string()]
        );
        assert_eq!(
            store
                .load_watched_vrpc_addresses("account", WalletNetwork::Testnet)
                .expect("load testnet"),
            vec!["RTest".to_string()]
        );
    }

    #[test]
    fn watchlist_entries_round_trip_and_keep_discovery_addresses_in_sync() {
        let store = temp_store();
        let entry = WatchlistEntry {
            id: "entry".to_string(),
            target_kind: WatchlistTargetKind::Identity,
            display_name: "Alice@".to_string(),
            address: "iAlice1111111111111111111111111111".to_string(),
            system_id: Some("iSystem11111111111111111111111111".to_string()),
            created_at: 10,
            updated_at: 11,
        };

        store
            .store_watchlist_entries(
                "account",
                WalletNetwork::Mainnet,
                std::slice::from_ref(&entry),
            )
            .expect("store watchlist");

        assert_eq!(
            store
                .load_watchlist_entries("account", WalletNetwork::Mainnet)
                .expect("load watchlist"),
            vec![entry.clone()]
        );
        assert_eq!(
            store
                .load_watched_vrpc_addresses("account", WalletNetwork::Mainnet)
                .expect("load discovery addresses"),
            vec![entry.address]
        );
        assert!(store
            .load_watchlist_entries("account", WalletNetwork::Testnet)
            .expect("load testnet")
            .is_empty());
    }

    #[test]
    fn watchlist_cleanup_preserves_unknown_fields_and_rejects_unknown_schema() {
        let store = temp_store();
        let path = store.account_state_path("account");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let old = serde_json::json!({
            "schemaVersion": 1,
            "futureTopLevel": { "keep": true },
            "mainnet": {
                "watchedVrpcAddresses": ["RPrivate"],
                "futureNetwork": { "keep": 7 }
            },
            "testnet": { "watchlistEntries": [] }
        });
        std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        store
            .clear_legacy_watchlist_fields("account")
            .expect("scrub");
        let cleaned: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(cleaned["mainnet"].get("watchedVrpcAddresses").is_none());
        assert_eq!(cleaned["futureTopLevel"]["keep"], true);
        assert_eq!(cleaned["mainnet"]["futureNetwork"]["keep"], 7);
        assert_eq!(cleaned["watchlistEncryptedVersion"], 1);
        let unsupported = serde_json::json!({ "schemaVersion": 2, "mainnet": {
            "watchedVrpcAddresses": ["RMustNotDisappear"]
        }});
        let bytes = serde_json::to_vec(&unsupported).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(store.clear_legacy_watchlist_fields("account").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }

    #[test]
    fn active_assets_round_trip_preserves_profile_version() {
        let store = temp_store();
        store
            .store_active_assets(
                "account",
                WalletNetwork::Mainnet,
                true,
                ACTIVE_ASSETS_PROFILE_VERSION,
                &["VRSC".to_string(), "BTC".to_string()],
            )
            .expect("store active assets");

        let loaded = store
            .load_active_assets("account", WalletNetwork::Mainnet)
            .expect("load active assets");

        assert_eq!(loaded.0, true);
        assert_eq!(loaded.1, vec!["VRSC".to_string(), "BTC".to_string()]);
        assert_eq!(loaded.2, ACTIVE_ASSETS_PROFILE_VERSION);
    }

    #[test]
    fn asset_preferences_are_wallet_and_network_scoped() {
        let store = temp_store();
        store
            .store_asset_preferences(
                "account-a",
                WalletNetwork::Mainnet,
                ACTIVE_ASSETS_PROFILE_VERSION,
                &["VRSC".to_string()],
                &["vrsc:ihidden".to_string()],
            )
            .expect("store mainnet preferences");
        store
            .store_asset_preferences(
                "account-a",
                WalletNetwork::Testnet,
                ACTIVE_ASSETS_PROFILE_VERSION,
                &["VRSCTEST".to_string()],
                &["erc20:0xtest".to_string()],
            )
            .expect("store testnet preferences");

        let mainnet = store
            .load_asset_preferences("account-a", WalletNetwork::Mainnet)
            .expect("load mainnet preferences");
        assert_eq!(mainnet.1, vec!["VRSC".to_string()]);
        assert_eq!(mainnet.3, vec!["vrsc:ihidden".to_string()]);

        let testnet = store
            .load_asset_preferences("account-a", WalletNetwork::Testnet)
            .expect("load testnet preferences");
        assert_eq!(testnet.1, vec!["VRSCTEST".to_string()]);
        assert_eq!(testnet.3, vec!["erc20:0xtest".to_string()]);

        let other_wallet = store
            .load_asset_preferences("account-b", WalletNetwork::Mainnet)
            .expect("load other wallet preferences");
        assert!(other_wallet.1.is_empty());
        assert!(other_wallet.3.is_empty());
    }

    #[test]
    fn pending_identity_profiles_are_network_scoped_and_replace_by_identity() {
        let store = temp_store();
        let snapshot = IdentityProfileSnapshot {
            avatar_mime_type: None,
            header_mime_type: None,
            header_base64: None,
            header_digest: None,
            avatar_base64: None,
            avatar_digest: None,
            description: Some("Profile".to_string()),
            description_digest: Some("aa".repeat(32)),
        };
        let record = |txid: &str, submitted_at: u64| PendingIdentityProfileUpdate {
            identity_address: "iSduGc7La416e3SfLD17tCe4Qvreg2i6br".to_string(),
            txid: txid.to_string(),
            submitted_at,
            previous_profile: IdentityProfileSnapshot {
                avatar_mime_type: None,
                header_mime_type: None,
                header_base64: None,
                header_digest: None,
                avatar_base64: None,
                avatar_digest: None,
                description: None,
                description_digest: None,
            },
            proposed_profile: snapshot.clone(),
        };

        store
            .store_pending_identity_profiles(
                "account",
                WalletNetwork::Testnet,
                &[record(&"11".repeat(32), 1), record(&"22".repeat(32), 2)],
            )
            .expect("store testnet pending");

        let testnet = store
            .load_pending_identity_profiles("account", WalletNetwork::Testnet)
            .expect("load testnet pending");
        assert_eq!(testnet.len(), 1);
        assert_eq!(testnet[0].txid, "22".repeat(32));
        assert!(store
            .load_pending_identity_profiles("account", WalletNetwork::Mainnet)
            .expect("load mainnet pending")
            .is_empty());
    }
}
