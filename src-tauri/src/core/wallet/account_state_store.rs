use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::types::generic_request::ProvisioningJobRecord;
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

const ACCOUNT_STATE_SCHEMA_VERSION: u8 = 1;
const ACTIVE_ASSETS_SCHEMA_VERSION: u8 = 1;

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
    #[serde(default)]
    watched_vrpc_addresses: Vec<String>,
    #[serde(default)]
    active_assets_schema_version: u8,
    #[serde(default)]
    active_assets: AccountStateActiveAssets,
    #[serde(default)]
    provisioning_jobs: Vec<ProvisioningJobRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountStateSnapshot {
    schema_version: u8,
    #[serde(default)]
    mainnet: AccountStateNetwork,
    #[serde(default)]
    testnet: AccountStateNetwork,
}

impl Default for AccountStateSnapshot {
    fn default() -> Self {
        Self {
            schema_version: ACCOUNT_STATE_SCHEMA_VERSION,
            mainnet: AccountStateNetwork {
                active_assets_schema_version: ACTIVE_ASSETS_SCHEMA_VERSION,
                ..AccountStateNetwork::default()
            },
            testnet: AccountStateNetwork {
                active_assets_schema_version: ACTIVE_ASSETS_SCHEMA_VERSION,
                ..AccountStateNetwork::default()
            },
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

    pub fn store_watched_vrpc_addresses(
        &self,
        account_id: &str,
        network: WalletNetwork,
        addresses: &[String],
    ) -> Result<(), WalletError> {
        let mut state = self.load_snapshot(account_id)?;
        self.network_mut(&mut state, network).watched_vrpc_addresses = addresses.to_vec();
        self.save_snapshot(account_id, &state)
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
        let mut state = self.load_snapshot(account_id)?;
        let network_state = self.network_mut(&mut state, network);
        network_state.active_assets_schema_version = ACTIVE_ASSETS_SCHEMA_VERSION;
        network_state.active_assets = AccountStateActiveAssets {
            initialized,
            profile_version,
            coin_ids: coin_ids.to_vec(),
        };
        self.save_snapshot(account_id, &state)
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
        let mut state = self.load_snapshot(account_id)?;
        self.network_mut(&mut state, network).provisioning_jobs = jobs.to_vec();
        self.save_snapshot(account_id, &state)
    }

    fn load_snapshot(&self, account_id: &str) -> Result<AccountStateSnapshot, WalletError> {
        let path = self.account_state_path(account_id);
        if !path.exists() {
            return Ok(AccountStateSnapshot::default());
        }

        let bytes = fs::read(path).map_err(|_| WalletError::OperationFailed)?;
        let mut snapshot = serde_json::from_slice::<AccountStateSnapshot>(&bytes)
            .map_err(|_| WalletError::OperationFailed)?;
        if snapshot.schema_version != ACCOUNT_STATE_SCHEMA_VERSION {
            snapshot = AccountStateSnapshot::default();
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
        fs::write(path, bytes).map_err(|_| WalletError::OperationFailed)
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
}
