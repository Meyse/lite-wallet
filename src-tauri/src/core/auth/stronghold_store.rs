//
// Stronghold-backed secure seed storage using IOTA Stronghold vault
// Security: Seeds encrypted at rest via Stronghold snapshot; one vault per account (per password)
// Last Updated: Replaced XOR with iota_stronghold; unified app data dir with WalletManager

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use zeroize::Zeroizing;

use crate::core::auth::kdf::{
    argon2_salt_path, derive_current_argon2id, derive_legacy_sha256,
    CURRENT_KEY_DERIVATION_VERSION, LEGACY_KEY_DERIVATION_VERSION,
};
use crate::core::wallet::{AccountStateStore, WalletManager};
use crate::types::errors::WalletError;
use crate::types::generic_request::ProvisioningJobRecord;
use crate::types::identity::LinkedIdentity;
use crate::types::wallet::{AccountRecord, WalletNetwork};
use iota_stronghold::{KeyProvider, SnapshotPath, Stronghold};

const SEED_RECORD_KEY: &[u8] = b"seed";
const ADDRESS_BOOK_RECORD_KEY: &[u8] = b"address_book_v1";
const WATCHED_VRPC_ADDRESSES_RECORD_KEY: &[u8] = b"watched_vrpc_addresses_v1";
const WATCHED_VRPC_ADDRESSES_SCHEMA_VERSION: u8 = 1;
const ACTIVE_ASSETS_RECORD_KEY: &[u8] = b"active_assets_v1";
const ACTIVE_ASSETS_SCHEMA_VERSION: u8 = 1;
pub const ACTIVE_ASSETS_PROFILE_VERSION: u8 = 2;
const LINKED_IDENTITIES_RECORD_KEY: &[u8] = b"linked_identities_v1";
const LINKED_IDENTITIES_SCHEMA_VERSION: u8 = 1;
const PROVISIONING_JOBS_RECORD_KEY: &[u8] = b"provisioning_jobs_v1";
const PROVISIONING_JOBS_SCHEMA_VERSION: u8 = 1;
const DLIGHT_SEED_RECORD_KEY: &[u8] = b"dlight_seed_v1";
const DLIGHT_SEED_SCHEMA_VERSION: u8 = 1;
const MAX_LINKED_IDENTITIES: usize = 100;
const MAX_FAVORITE_LINKED_IDENTITIES: usize = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WatchedVrpcAddressesSnapshot {
    schema_version: u8,
    mainnet: Vec<String>,
    testnet: Vec<String>,
}

impl Default for WatchedVrpcAddressesSnapshot {
    fn default() -> Self {
        Self {
            schema_version: WATCHED_VRPC_ADDRESSES_SCHEMA_VERSION,
            mainnet: vec![],
            testnet: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActiveAssetsNetworkSnapshot {
    #[serde(default)]
    initialized: bool,
    #[serde(default)]
    profile_version: u8,
    #[serde(default)]
    coin_ids: Vec<String>,
}

impl Default for ActiveAssetsNetworkSnapshot {
    fn default() -> Self {
        Self {
            initialized: false,
            profile_version: 0,
            coin_ids: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActiveAssetsSnapshot {
    schema_version: u8,
    #[serde(default)]
    mainnet: ActiveAssetsNetworkSnapshot,
    #[serde(default)]
    testnet: ActiveAssetsNetworkSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LinkedIdentitiesSnapshot {
    schema_version: u8,
    #[serde(default)]
    mainnet: Vec<LinkedIdentity>,
    #[serde(default)]
    testnet: Vec<LinkedIdentity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProvisioningJobsSnapshot {
    schema_version: u8,
    #[serde(default)]
    mainnet: Vec<ProvisioningJobRecord>,
    #[serde(default)]
    testnet: Vec<ProvisioningJobRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DlightSeedSnapshot {
    schema_version: u8,
    #[serde(default)]
    mainnet_metadata: crate::core::channels::dlight_private::DlightSeedMetadata,
    #[serde(default)]
    testnet_metadata: crate::core::channels::dlight_private::DlightSeedMetadata,
    #[serde(default)]
    mainnet: Option<String>,
    #[serde(default)]
    testnet: Option<String>,
}

impl Default for LinkedIdentitiesSnapshot {
    fn default() -> Self {
        Self {
            schema_version: LINKED_IDENTITIES_SCHEMA_VERSION,
            mainnet: vec![],
            testnet: vec![],
        }
    }
}

impl Default for ProvisioningJobsSnapshot {
    fn default() -> Self {
        Self {
            schema_version: PROVISIONING_JOBS_SCHEMA_VERSION,
            mainnet: vec![],
            testnet: vec![],
        }
    }
}

impl Default for DlightSeedSnapshot {
    fn default() -> Self {
        Self {
            schema_version: DLIGHT_SEED_SCHEMA_VERSION,
            mainnet_metadata: Default::default(),
            testnet_metadata: Default::default(),
            mainnet: None,
            testnet: None,
        }
    }
}

impl Default for ActiveAssetsSnapshot {
    fn default() -> Self {
        Self {
            schema_version: ACTIVE_ASSETS_SCHEMA_VERSION,
            mainnet: ActiveAssetsNetworkSnapshot::default(),
            testnet: ActiveAssetsNetworkSnapshot::default(),
        }
    }
}

struct LegacyMigrationBundle {
    seed: String,
    address_book: Option<Vec<u8>>,
    linked_identities: Option<LinkedIdentitiesSnapshot>,
    dlight_seed: DlightSeedSnapshot,
    watched_vrpc_addresses: WatchedVrpcAddressesSnapshot,
    active_assets: ActiveAssetsSnapshot,
    provisioning_jobs: ProvisioningJobsSnapshot,
}

fn network_mut<'a, T>(mainnet: &'a mut T, testnet: &'a mut T, network: WalletNetwork) -> &'a mut T {
    match network {
        WalletNetwork::Mainnet => mainnet,
        WalletNetwork::Testnet => testnet,
    }
}

fn network_value<T>(mainnet: T, testnet: T, network: WalletNetwork) -> T {
    match network {
        WalletNetwork::Mainnet => mainnet,
        WalletNetwork::Testnet => testnet,
    }
}

#[derive(Clone)]
pub struct StrongholdStore {
    /// Base directory for Stronghold (app data dir / stronghold); accounts go under accounts/<id>/
    base_path: PathBuf,
    salt_path: PathBuf,
}

impl StrongholdStore {
    /// Initialize with app data directory (same root as wallet metadata for unified storage).
    pub fn new(app_handle: &AppHandle) -> Result<Self, WalletError> {
        let app_dir = app_handle.path().app_data_dir().map_err(|e| {
            println!("[AUTH] Failed to get app data directory: {}", e);
            WalletError::OperationFailed
        })?;
        let base_path = app_dir.join("stronghold");
        std::fs::create_dir_all(&base_path).map_err(|e| {
            println!("[AUTH] Failed to create Stronghold directory: {}", e);
            WalletError::OperationFailed
        })?;
        let app_local_dir = app_handle.path().app_local_data_dir().map_err(|e| {
            println!("[AUTH] Failed to get app local data directory: {}", e);
            WalletError::OperationFailed
        })?;
        let salt_path = argon2_salt_path(&app_local_dir);
        Ok(Self {
            base_path,
            salt_path,
        })
    }

    #[cfg(test)]
    pub(crate) fn new_for_tests(base_path: PathBuf) -> Self {
        let salt_path = base_path.join("argon2_salt.bin");
        Self {
            base_path,
            salt_path,
        }
    }

    /// Returns the app data directory root used by this store.
    pub fn app_data_dir(&self) -> PathBuf {
        self.base_path
            .parent()
            .map(|path| path.to_path_buf())
            .unwrap_or_else(|| self.base_path.clone())
    }

    fn account_dir(&self, account_id: &str) -> PathBuf {
        let account_dir = self.base_path.join("accounts").join(account_id);
        let _ = std::fs::create_dir_all(&account_dir);
        account_dir
    }

    fn isolated_snapshot_path(&self, account_id: &str, file_name: &str) -> PathBuf {
        self.account_dir(account_id).join(file_name)
    }

    fn temp_snapshot_path(path: &Path) -> PathBuf {
        path.with_extension("stronghold.argon2.tmp")
    }

    fn account_snapshot_path(&self, account_id: &str) -> PathBuf {
        self.isolated_snapshot_path(account_id, "snapshot.stronghold")
    }

    fn address_book_snapshot_path(&self, account_id: &str) -> PathBuf {
        self.isolated_snapshot_path(account_id, "address_book.snapshot.stronghold")
    }

    fn watched_vrpc_addresses_snapshot_path(&self, account_id: &str) -> PathBuf {
        self.isolated_snapshot_path(account_id, "watched_vrpc_addresses.snapshot.stronghold")
    }

    fn active_assets_snapshot_path(&self, account_id: &str) -> PathBuf {
        self.isolated_snapshot_path(account_id, "active_assets.snapshot.stronghold")
    }

    fn linked_identities_snapshot_path(&self, account_id: &str) -> PathBuf {
        self.isolated_snapshot_path(account_id, "linked_identities.snapshot.stronghold")
    }

    fn dlight_seed_snapshot_path(&self, account_id: &str) -> PathBuf {
        self.isolated_snapshot_path(account_id, "dlight_seed.snapshot.stronghold")
    }

    fn provisioning_jobs_snapshot_path(&self, account_id: &str) -> PathBuf {
        self.isolated_snapshot_path(account_id, "provisioning_jobs.snapshot.stronghold")
    }

    fn seed_temp_snapshot_path(&self, account_id: &str) -> PathBuf {
        Self::temp_snapshot_path(&self.account_snapshot_path(account_id))
    }

    fn address_book_temp_snapshot_path(&self, account_id: &str) -> PathBuf {
        Self::temp_snapshot_path(&self.address_book_snapshot_path(account_id))
    }

    fn linked_identities_temp_snapshot_path(&self, account_id: &str) -> PathBuf {
        Self::temp_snapshot_path(&self.linked_identities_snapshot_path(account_id))
    }

    fn dlight_seed_temp_snapshot_path(&self, account_id: &str) -> PathBuf {
        Self::temp_snapshot_path(&self.dlight_seed_snapshot_path(account_id))
    }

    #[cfg(debug_assertions)]
    fn read_snapshot_encrypt_work_factor(path: &std::path::Path) -> Option<u8> {
        // Snapshot format stores age header as plaintext after magic/version bytes.
        let bytes = std::fs::read(path).ok()?;
        let scan_len = bytes.len().min(256);
        let header = String::from_utf8_lossy(&bytes[..scan_len]);
        let marker = "-> scrypt ";
        let marker_offset = header.find(marker)?;
        let line = header[marker_offset + marker.len()..].lines().next()?;
        let (_, factor) = line.rsplit_once(' ')?;
        factor.trim().parse::<u8>().ok()
    }

    async fn load_optional_record_from_path(
        &self,
        account_id: &str,
        password_hash: &[u8],
        path: &Path,
        record_key: &[u8],
        snapshot_label: &str,
    ) -> Result<Option<Vec<u8>>, WalletError> {
        if !path.exists() {
            return Ok(None);
        }

        let account_id = account_id.as_bytes().to_vec();
        let password_hash = Zeroizing::new(password_hash.to_vec());
        let path = path.to_path_buf();
        let record_key = record_key.to_vec();
        let snapshot_label = snapshot_label.to_string();
        let worker_label = snapshot_label.clone();

        tokio::task::spawn_blocking(move || {
            let snapshot_path = SnapshotPath::from_path(&path);
            let keyprovider = Self::keyprovider_from_hash(&password_hash)?;
            let stronghold = Stronghold::default();
            let client = stronghold
                .load_client_from_snapshot(&account_id, &keyprovider, &snapshot_path)
                .map_err(|e| {
                    println!("[AUTH] Load {} snapshot failed: {}", worker_label, e);
                    WalletError::OperationFailed
                })?;

            client.store().get(&record_key).map_err(|e| {
                println!("[AUTH] Read {} record failed: {}", worker_label, e);
                WalletError::OperationFailed
            })
        })
        .await
        .map_err(|error| {
            println!(
                "[AUTH] {} snapshot worker failed: {}",
                snapshot_label, error
            );
            WalletError::OperationFailed
        })?
    }

    async fn load_json_snapshot_from_path<T: DeserializeOwned + Default>(
        &self,
        account_id: &str,
        password_hash: &[u8],
        path: &Path,
        record_key: &[u8],
        snapshot_label: &str,
    ) -> Result<T, WalletError> {
        let bytes = self
            .load_optional_record_from_path(
                account_id,
                password_hash,
                path,
                record_key,
                snapshot_label,
            )
            .await?;
        let Some(payload) = bytes else {
            return Ok(T::default());
        };

        serde_json::from_slice::<T>(&payload).map_err(|e| {
            println!("[AUTH] Parse {} snapshot failed: {}", snapshot_label, e);
            WalletError::OperationFailed
        })
    }

    fn store_json_snapshot_to_path<T: Serialize>(
        &self,
        account_id: &str,
        password_hash: &[u8],
        path: &Path,
        record_key: &[u8],
        snapshot: &T,
        snapshot_label: &str,
    ) -> Result<(), WalletError> {
        let payload = serde_json::to_vec(snapshot).map_err(|e| {
            println!("[AUTH] Serialize {} snapshot failed: {}", snapshot_label, e);
            WalletError::OperationFailed
        })?;
        self.commit_record_to_path(account_id, path, password_hash, record_key, &payload)
    }

    async fn load_watched_vrpc_addresses_snapshot(
        &self,
        account_id: &str,
        password_hash: &[u8],
    ) -> Result<WatchedVrpcAddressesSnapshot, WalletError> {
        let path = self.watched_vrpc_addresses_snapshot_path(account_id);
        self.load_json_snapshot_from_path(
            account_id,
            password_hash,
            &path,
            WATCHED_VRPC_ADDRESSES_RECORD_KEY,
            "watched VRPC addresses",
        )
        .await
    }

    async fn load_active_assets_snapshot(
        &self,
        account_id: &str,
        password_hash: &[u8],
    ) -> Result<ActiveAssetsSnapshot, WalletError> {
        let path = self.active_assets_snapshot_path(account_id);
        self.load_json_snapshot_from_path(
            account_id,
            password_hash,
            &path,
            ACTIVE_ASSETS_RECORD_KEY,
            "active assets",
        )
        .await
    }

    async fn load_linked_identities_snapshot(
        &self,
        account_id: &str,
        password_hash: &[u8],
    ) -> Result<LinkedIdentitiesSnapshot, WalletError> {
        let path = self.linked_identities_snapshot_path(account_id);
        self.load_json_snapshot_from_path(
            account_id,
            password_hash,
            &path,
            LINKED_IDENTITIES_RECORD_KEY,
            "linked identities",
        )
        .await
    }

    async fn load_dlight_seed_snapshot(
        &self,
        account_id: &str,
        password_hash: &[u8],
    ) -> Result<DlightSeedSnapshot, WalletError> {
        let path = self.dlight_seed_snapshot_path(account_id);
        self.load_json_snapshot_from_path(
            account_id,
            password_hash,
            &path,
            DLIGHT_SEED_RECORD_KEY,
            "dlight seed",
        )
        .await
    }

    async fn load_provisioning_jobs_snapshot(
        &self,
        account_id: &str,
        password_hash: &[u8],
    ) -> Result<ProvisioningJobsSnapshot, WalletError> {
        let path = self.provisioning_jobs_snapshot_path(account_id);
        self.load_json_snapshot_from_path(
            account_id,
            password_hash,
            &path,
            PROVISIONING_JOBS_RECORD_KEY,
            "provisioning jobs",
        )
        .await
    }

    fn sanitize_linked_identities(identities: &[LinkedIdentity]) -> Vec<LinkedIdentity> {
        use std::collections::HashSet;

        let mut seen = HashSet::<String>::new();
        let mut out = Vec::<LinkedIdentity>::new();
        let mut favorite_count = 0usize;

        for identity in identities {
            let normalized_identity_address = identity.identity_address.trim();
            if normalized_identity_address.is_empty() {
                continue;
            }

            let key = normalized_identity_address.to_ascii_lowercase();
            if !seen.insert(key) {
                continue;
            }

            let favorite = identity.favorite && favorite_count < MAX_FAVORITE_LINKED_IDENTITIES;
            if favorite {
                favorite_count += 1;
            }

            out.push(LinkedIdentity {
                identity_address: normalized_identity_address.to_string(),
                name: identity
                    .name
                    .as_ref()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                fully_qualified_name: identity
                    .fully_qualified_name
                    .as_ref()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                status: identity
                    .status
                    .as_ref()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                system_id: identity
                    .system_id
                    .as_ref()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                favorite,
            });

            if out.len() >= MAX_LINKED_IDENTITIES {
                break;
            }
        }

        out
    }

    pub fn salt_path(&self) -> &std::path::Path {
        &self.salt_path
    }

    pub fn current_key_derivation_version(&self) -> u8 {
        CURRENT_KEY_DERIVATION_VERSION
    }

    pub fn current_key_derivation_version_static() -> u8 {
        CURRENT_KEY_DERIVATION_VERSION
    }

    pub fn derive_current_password_hash(
        &self,
        password: &str,
        allow_create_salt: bool,
    ) -> Result<Zeroizing<Vec<u8>>, WalletError> {
        derive_current_argon2id(password, &self.salt_path, allow_create_salt).map(Zeroizing::new)
    }

    /// Argon2id deliberately consumes CPU and memory. Keep that fixed cost off Tokio's
    /// async workers while retaining the password only for the blocking job's lifetime.
    pub async fn derive_current_password_hash_async(
        &self,
        password: &str,
        allow_create_salt: bool,
    ) -> Result<Zeroizing<Vec<u8>>, WalletError> {
        let password = Zeroizing::new(password.to_string());
        let salt_path = self.salt_path.clone();
        tokio::task::spawn_blocking(move || {
            derive_current_argon2id(password.as_str(), &salt_path, allow_create_salt)
                .map(Zeroizing::new)
        })
        .await
        .map_err(|error| {
            println!("[AUTH] Password KDF worker failed: {}", error);
            WalletError::OperationFailed
        })?
    }

    pub fn derive_legacy_password_hash(password: &str) -> Zeroizing<Vec<u8>> {
        Zeroizing::new(derive_legacy_sha256(password))
    }

    fn keyprovider_from_hash(password_hash: &[u8]) -> Result<KeyProvider, WalletError> {
        KeyProvider::try_from(Zeroizing::new(password_hash.to_vec())).map_err(|e| {
            println!("[AUTH] KeyProvider failed: {}", e);
            WalletError::OperationFailed
        })
    }

    fn get_or_create_client(
        stronghold: &Stronghold,
        snapshot_path: &SnapshotPath,
        account_id: &str,
        keyprovider: &KeyProvider,
        snapshot_exists: bool,
    ) -> Result<iota_stronghold::Client, WalletError> {
        if snapshot_exists {
            stronghold
                .load_client_from_snapshot(account_id.as_bytes(), keyprovider, snapshot_path)
                .map_err(|e| {
                    println!("[AUTH] Load client from snapshot failed: {}", e);
                    WalletError::InvalidPassword
                })
        } else {
            stronghold
                .create_client(account_id.as_bytes())
                .map_err(|e| {
                    println!("[AUTH] Create client failed: {}", e);
                    WalletError::OperationFailed
                })
        }
    }

    fn commit_record_to_path(
        &self,
        account_id: &str,
        path: &std::path::Path,
        password_hash: &[u8],
        record_key: &[u8],
        payload: &[u8],
    ) -> Result<(), WalletError> {
        let snapshot_path = SnapshotPath::from_path(path);
        let keyprovider = Self::keyprovider_from_hash(password_hash)?;
        let stronghold = Stronghold::default();
        let client = Self::get_or_create_client(
            &stronghold,
            &snapshot_path,
            account_id,
            &keyprovider,
            path.exists(),
        )?;
        client
            .store()
            .insert(record_key.to_vec(), payload.to_vec(), None)
            .map_err(|e| {
                println!("[AUTH] Store insert failed: {}", e);
                WalletError::OperationFailed
            })?;
        stronghold
            .commit_with_keyprovider(&snapshot_path, &keyprovider)
            .map_err(|e| {
                println!("[AUTH] Commit failed: {}", e);
                WalletError::OperationFailed
            })?;
        Ok(())
    }

    async fn load_seed_by_hash_internal(
        &self,
        account_id: &str,
        password_hash: &[u8],
    ) -> Result<String, WalletError> {
        let path = self.account_snapshot_path(account_id);
        if !path.exists() {
            println!(
                "[AUTH] Stronghold snapshot missing for account: {}",
                account_id
            );
            return Err(WalletError::InvalidPassword);
        }

        let account_id = account_id.to_string();
        let password_hash = Zeroizing::new(password_hash.to_vec());
        tokio::task::spawn_blocking(move || {
            let keyprovider = Self::keyprovider_from_hash(password_hash.as_ref())?;
            let snapshot_path = SnapshotPath::from_path(&path);
            #[cfg(debug_assertions)]
            let snapshot_work_factor_before_unlock = Self::read_snapshot_encrypt_work_factor(&path);

            let stronghold = Stronghold::default();
            let client = stronghold
                .load_client_from_snapshot(account_id.as_bytes(), &keyprovider, &snapshot_path)
                .map_err(|e| {
                    println!("[AUTH] Load client from snapshot failed: {}", e);
                    WalletError::InvalidPassword
                })?;
            let data = client.store().get(SEED_RECORD_KEY).map_err(|e| {
                println!("[AUTH] Store get failed: {}", e);
                WalletError::OperationFailed
            })?;
            let bytes = data.ok_or_else(|| {
                println!("[AUTH] Seed record missing for account: {}", account_id);
                WalletError::InvalidPassword
            })?;
            let seed = String::from_utf8(bytes).map_err(|_| WalletError::OperationFailed)?;

            #[cfg(debug_assertions)]
            if let Some(existing_work_factor) = snapshot_work_factor_before_unlock {
                let target_work_factor =
                    iota_stronghold::engine::snapshot::get_encrypt_work_factor();
                if existing_work_factor > target_work_factor {
                    println!(
                        "[AUTH] Migrating snapshot work factor for account {} from {} to {}",
                        account_id, existing_work_factor, target_work_factor
                    );
                    if let Err(error) =
                        stronghold.commit_with_keyprovider(&snapshot_path, &keyprovider)
                    {
                        println!(
                            "[AUTH] Snapshot work-factor migration commit failed for account {}: {}",
                            account_id, error
                        );
                    }
                }
            }

            Ok(seed)
        })
        .await
        .map_err(|error| {
            println!("[AUTH] Seed snapshot worker failed: {}", error);
            WalletError::OperationFailed
        })?
    }

    fn promote_temp_snapshot(
        canonical_path: &std::path::Path,
        temp_path: &std::path::Path,
    ) -> Result<(), WalletError> {
        if !temp_path.exists() {
            return Ok(());
        }
        if let Some(parent) = canonical_path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| WalletError::OperationFailed)?;
        }
        std::fs::rename(temp_path, canonical_path).map_err(|_| WalletError::OperationFailed)
    }

    fn remove_file_if_exists(path: &std::path::Path) -> Result<(), WalletError> {
        if path.exists() {
            std::fs::remove_file(path).map_err(|_| WalletError::OperationFailed)?;
        }
        Ok(())
    }

    async fn load_legacy_linked_identities_snapshot(
        &self,
        account_id: &str,
        legacy_hash: &[u8],
    ) -> Option<LinkedIdentitiesSnapshot> {
        let mainnet = self
            .load_linked_identities(account_id, legacy_hash, WalletNetwork::Mainnet)
            .await
            .ok()?;
        let testnet = self
            .load_linked_identities(account_id, legacy_hash, WalletNetwork::Testnet)
            .await
            .ok()?;
        Some(LinkedIdentitiesSnapshot {
            schema_version: LINKED_IDENTITIES_SCHEMA_VERSION,
            mainnet,
            testnet,
        })
    }

    async fn read_legacy_migration_bundle(
        &self,
        account_id: &str,
        legacy_hash: &[u8],
    ) -> Result<LegacyMigrationBundle, WalletError> {
        Ok(LegacyMigrationBundle {
            seed: self
                .load_seed_by_hash_internal(account_id, legacy_hash)
                .await?,
            address_book: self.load_address_book(account_id, legacy_hash).await?,
            linked_identities: self
                .load_legacy_linked_identities_snapshot(account_id, legacy_hash)
                .await,
            dlight_seed: self
                .load_dlight_seed_snapshot(account_id, legacy_hash)
                .await?,
            watched_vrpc_addresses: WatchedVrpcAddressesSnapshot {
                schema_version: WATCHED_VRPC_ADDRESSES_SCHEMA_VERSION,
                mainnet: self
                    .load_watched_vrpc_addresses(account_id, legacy_hash, WalletNetwork::Mainnet)
                    .await
                    .unwrap_or_default(),
                testnet: self
                    .load_watched_vrpc_addresses(account_id, legacy_hash, WalletNetwork::Testnet)
                    .await
                    .unwrap_or_default(),
            },
            active_assets: ActiveAssetsSnapshot {
                schema_version: ACTIVE_ASSETS_SCHEMA_VERSION,
                mainnet: {
                    let (initialized, coin_ids, profile_version) = self
                        .load_active_assets(account_id, legacy_hash, WalletNetwork::Mainnet)
                        .await
                        .unwrap_or((false, vec![], 0));
                    ActiveAssetsNetworkSnapshot {
                        initialized,
                        profile_version,
                        coin_ids,
                    }
                },
                testnet: {
                    let (initialized, coin_ids, profile_version) = self
                        .load_active_assets(account_id, legacy_hash, WalletNetwork::Testnet)
                        .await
                        .unwrap_or((false, vec![], 0));
                    ActiveAssetsNetworkSnapshot {
                        initialized,
                        profile_version,
                        coin_ids,
                    }
                },
            },
            provisioning_jobs: ProvisioningJobsSnapshot {
                schema_version: PROVISIONING_JOBS_SCHEMA_VERSION,
                mainnet: self
                    .load_provisioning_jobs(account_id, legacy_hash, WalletNetwork::Mainnet)
                    .await
                    .unwrap_or_default(),
                testnet: self
                    .load_provisioning_jobs(account_id, legacy_hash, WalletNetwork::Testnet)
                    .await
                    .unwrap_or_default(),
            },
        })
    }

    fn write_migrated_secret_snapshots(
        &self,
        account_id: &str,
        current_hash: &[u8],
        bundle: &LegacyMigrationBundle,
    ) -> Result<(), WalletError> {
        self.commit_record_to_path(
            account_id,
            &self.seed_temp_snapshot_path(account_id),
            current_hash,
            SEED_RECORD_KEY,
            bundle.seed.as_bytes(),
        )?;

        if let Some(payload) = bundle.address_book.as_deref() {
            self.commit_record_to_path(
                account_id,
                &self.address_book_temp_snapshot_path(account_id),
                current_hash,
                ADDRESS_BOOK_RECORD_KEY,
                payload,
            )?;
        }

        if let Some(linked_identities) = bundle.linked_identities.as_ref() {
            self.store_json_snapshot_to_path(
                account_id,
                current_hash,
                &self.linked_identities_temp_snapshot_path(account_id),
                LINKED_IDENTITIES_RECORD_KEY,
                linked_identities,
                "linked identities",
            )?;
        }

        self.store_json_snapshot_to_path(
            account_id,
            current_hash,
            &self.dlight_seed_temp_snapshot_path(account_id),
            DLIGHT_SEED_RECORD_KEY,
            &bundle.dlight_seed,
            "dlight seed",
        )
    }

    fn migrate_legacy_account_state(
        &self,
        account_id: &str,
        account_state_store: &AccountStateStore,
        bundle: &LegacyMigrationBundle,
    ) -> Result<(), WalletError> {
        account_state_store.store_watched_vrpc_addresses(
            account_id,
            WalletNetwork::Mainnet,
            &bundle.watched_vrpc_addresses.mainnet,
        )?;
        account_state_store.store_watched_vrpc_addresses(
            account_id,
            WalletNetwork::Testnet,
            &bundle.watched_vrpc_addresses.testnet,
        )?;
        account_state_store.store_active_assets(
            account_id,
            WalletNetwork::Mainnet,
            bundle.active_assets.mainnet.initialized,
            bundle.active_assets.mainnet.profile_version,
            &bundle.active_assets.mainnet.coin_ids,
        )?;
        account_state_store.store_active_assets(
            account_id,
            WalletNetwork::Testnet,
            bundle.active_assets.testnet.initialized,
            bundle.active_assets.testnet.profile_version,
            &bundle.active_assets.testnet.coin_ids,
        )?;
        account_state_store.store_provisioning_jobs(
            account_id,
            WalletNetwork::Mainnet,
            &bundle.provisioning_jobs.mainnet,
        )?;
        account_state_store.store_provisioning_jobs(
            account_id,
            WalletNetwork::Testnet,
            &bundle.provisioning_jobs.testnet,
        )
    }

    fn promote_migrated_secret_snapshots(&self, account_id: &str) -> Result<(), WalletError> {
        Self::promote_temp_snapshot(
            &self.account_snapshot_path(account_id),
            &self.seed_temp_snapshot_path(account_id),
        )?;
        Self::promote_temp_snapshot(
            &self.address_book_snapshot_path(account_id),
            &self.address_book_temp_snapshot_path(account_id),
        )?;
        Self::promote_temp_snapshot(
            &self.linked_identities_snapshot_path(account_id),
            &self.linked_identities_temp_snapshot_path(account_id),
        )?;
        Self::promote_temp_snapshot(
            &self.dlight_seed_snapshot_path(account_id),
            &self.dlight_seed_temp_snapshot_path(account_id),
        )
    }

    async fn persist_current_account_kdf_version(
        &self,
        account: &AccountRecord,
        wallet_manager: &WalletManager,
    ) -> Result<(), WalletError> {
        let mut updated = account.clone();
        updated.key_derivation_version = CURRENT_KEY_DERIVATION_VERSION;
        wallet_manager
            .save_account_record_by_account_id(&account.id, &updated)
            .await
    }

    fn remove_legacy_account_state_snapshots(&self, account_id: &str) -> Result<(), WalletError> {
        Self::remove_file_if_exists(&self.watched_vrpc_addresses_snapshot_path(account_id))?;
        Self::remove_file_if_exists(&self.active_assets_snapshot_path(account_id))?;
        Self::remove_file_if_exists(&self.provisioning_jobs_snapshot_path(account_id))
    }

    pub async fn ensure_account_password_hash(
        &self,
        account: &AccountRecord,
        password: &str,
        wallet_manager: &WalletManager,
        account_state_store: &AccountStateStore,
    ) -> Result<Zeroizing<Vec<u8>>, WalletError> {
        if account.key_derivation_version >= CURRENT_KEY_DERIVATION_VERSION {
            return self
                .derive_current_password_hash_async(password, false)
                .await;
        }

        if account.key_derivation_version != LEGACY_KEY_DERIVATION_VERSION {
            return Err(WalletError::SecureStorageUnavailable);
        }

        let legacy_hash = Self::derive_legacy_password_hash(password);
        let bundle = self
            .read_legacy_migration_bundle(&account.id, legacy_hash.as_ref())
            .await?;
        let current_hash = self
            .derive_current_password_hash_async(password, true)
            .await?;

        self.write_migrated_secret_snapshots(&account.id, current_hash.as_ref(), &bundle)?;
        self.migrate_legacy_account_state(&account.id, account_state_store, &bundle)?;
        self.promote_migrated_secret_snapshots(&account.id)?;
        self.persist_current_account_kdf_version(account, wallet_manager)
            .await?;
        self.remove_legacy_account_state_snapshots(&account.id)?;

        Ok(current_hash)
    }

    /// Store seed in a Stronghold vault for this account (encrypted with password).
    pub async fn store_seed(
        &self,
        account_id: &str,
        seed: &str,
        password_hash: &[u8],
    ) -> Result<(), WalletError> {
        println!("[AUTH] Storing seed for account: {}", account_id);

        let path = self.account_snapshot_path(account_id);
        self.commit_record_to_path(
            account_id,
            &path,
            password_hash,
            SEED_RECORD_KEY,
            seed.as_bytes(),
        )?;
        println!("[AUTH] Seed stored successfully");
        Ok(())
    }

    pub async fn load_seed(
        &self,
        account_id: &str,
        password_hash: &[u8],
    ) -> Result<String, WalletError> {
        println!("[AUTH] Loading seed for account: {}", account_id);
        let seed = self
            .load_seed_by_hash_internal(account_id, password_hash)
            .await?;
        println!("[AUTH] Seed loaded successfully");
        Ok(seed)
    }

    /// Store address book snapshot for an account in an isolated Stronghold snapshot.
    pub async fn store_address_book(
        &self,
        account_id: &str,
        password_hash: &[u8],
        data: &[u8],
    ) -> Result<(), WalletError> {
        let path = self.address_book_snapshot_path(account_id);
        self.commit_record_to_path(
            account_id,
            &path,
            password_hash,
            ADDRESS_BOOK_RECORD_KEY,
            data,
        )
    }

    /// Load address book snapshot bytes for an account from isolated Stronghold snapshot.
    pub async fn load_address_book(
        &self,
        account_id: &str,
        password_hash: &[u8],
    ) -> Result<Option<Vec<u8>>, WalletError> {
        let path = self.address_book_snapshot_path(account_id);
        self.load_optional_record_from_path(
            account_id,
            password_hash,
            &path,
            ADDRESS_BOOK_RECORD_KEY,
            "address book",
        )
        .await
    }

    pub async fn load_watched_vrpc_addresses(
        &self,
        account_id: &str,
        password_hash: &[u8],
        network: WalletNetwork,
    ) -> Result<Vec<String>, WalletError> {
        let snapshot = self
            .load_watched_vrpc_addresses_snapshot(account_id, password_hash)
            .await?;
        Ok(network_value(snapshot.mainnet, snapshot.testnet, network))
    }

    pub async fn store_watched_vrpc_addresses(
        &self,
        account_id: &str,
        password_hash: &[u8],
        network: WalletNetwork,
        addresses: &[String],
    ) -> Result<(), WalletError> {
        let path = self.watched_vrpc_addresses_snapshot_path(account_id);
        let mut snapshot = self
            .load_watched_vrpc_addresses_snapshot(account_id, password_hash)
            .await?;
        *network_mut(&mut snapshot.mainnet, &mut snapshot.testnet, network) = addresses.to_vec();
        self.store_json_snapshot_to_path(
            account_id,
            password_hash,
            &path,
            WATCHED_VRPC_ADDRESSES_RECORD_KEY,
            &snapshot,
            "watched VRPC addresses",
        )
    }

    pub async fn load_active_assets(
        &self,
        account_id: &str,
        password_hash: &[u8],
        network: WalletNetwork,
    ) -> Result<(bool, Vec<String>, u8), WalletError> {
        let snapshot = self
            .load_active_assets_snapshot(account_id, password_hash)
            .await?;
        let network_snapshot = network_value(snapshot.mainnet, snapshot.testnet, network);
        Ok((
            network_snapshot.initialized,
            network_snapshot.coin_ids,
            network_snapshot.profile_version,
        ))
    }

    pub async fn store_active_assets(
        &self,
        account_id: &str,
        password_hash: &[u8],
        network: WalletNetwork,
        initialized: bool,
        profile_version: u8,
        coin_ids: &[String],
    ) -> Result<(), WalletError> {
        let path = self.active_assets_snapshot_path(account_id);
        let mut snapshot = self
            .load_active_assets_snapshot(account_id, password_hash)
            .await?;
        snapshot.schema_version = ACTIVE_ASSETS_SCHEMA_VERSION;
        let network_snapshot = network_mut(&mut snapshot.mainnet, &mut snapshot.testnet, network);
        network_snapshot.initialized = initialized;
        network_snapshot.profile_version = profile_version;
        network_snapshot.coin_ids = coin_ids.to_vec();
        self.store_json_snapshot_to_path(
            account_id,
            password_hash,
            &path,
            ACTIVE_ASSETS_RECORD_KEY,
            &snapshot,
            "active assets",
        )
    }

    pub async fn load_linked_identities(
        &self,
        account_id: &str,
        password_hash: &[u8],
        network: WalletNetwork,
    ) -> Result<Vec<LinkedIdentity>, WalletError> {
        let snapshot = self
            .load_linked_identities_snapshot(account_id, password_hash)
            .await?;
        let records = network_value(snapshot.mainnet, snapshot.testnet, network);
        Ok(Self::sanitize_linked_identities(&records))
    }

    pub async fn store_linked_identities(
        &self,
        account_id: &str,
        password_hash: &[u8],
        network: WalletNetwork,
        identities: &[LinkedIdentity],
    ) -> Result<(), WalletError> {
        let path = self.linked_identities_snapshot_path(account_id);
        let mut snapshot = self
            .load_linked_identities_snapshot(account_id, password_hash)
            .await?;
        snapshot.schema_version = LINKED_IDENTITIES_SCHEMA_VERSION;

        let sanitized = Self::sanitize_linked_identities(identities);
        *network_mut(&mut snapshot.mainnet, &mut snapshot.testnet, network) = sanitized;
        self.store_json_snapshot_to_path(
            account_id,
            password_hash,
            &path,
            LINKED_IDENTITIES_RECORD_KEY,
            &snapshot,
            "linked identities",
        )
    }

    pub async fn load_dlight_seed(
        &self,
        account_id: &str,
        password_hash: &[u8],
        network: WalletNetwork,
    ) -> Result<Option<String>, WalletError> {
        let snapshot = self
            .load_dlight_seed_snapshot(account_id, password_hash)
            .await?;
        let seed = network_value(snapshot.mainnet, snapshot.testnet, network);

        Ok(seed
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()))
    }

    pub async fn load_dlight_runtime_material(
        &self,
        account_id: &str,
        password_hash: &[u8],
        network: WalletNetwork,
    ) -> Result<
        Option<(
            zeroize::Zeroizing<String>,
            crate::core::channels::dlight_private::DlightSeedMetadata,
        )>,
        WalletError,
    > {
        let snapshot = self
            .load_dlight_seed_snapshot(account_id, password_hash)
            .await?;
        let (seed, metadata) = match network {
            WalletNetwork::Mainnet => (snapshot.mainnet, snapshot.mainnet_metadata),
            WalletNetwork::Testnet => (snapshot.testnet, snapshot.testnet_metadata),
        };
        seed.filter(|seed| !seed.trim().is_empty())
            .map(|seed| {
                let seed = zeroize::Zeroizing::new(seed);
                let key =
                    crate::core::channels::dlight_private::runtime_spending_key(&seed, network)?;
                Ok((key, metadata))
            })
            .transpose()
    }

    pub async fn store_dlight_seed(
        &self,
        account_id: &str,
        password_hash: &[u8],
        network: WalletNetwork,
        seed: Option<&str>,
    ) -> Result<(), WalletError> {
        self.store_dlight_seed_with_metadata(
            account_id,
            password_hash,
            network,
            seed,
            Default::default(),
        )
        .await
    }

    pub async fn store_dlight_seed_with_metadata(
        &self,
        account_id: &str,
        password_hash: &[u8],
        network: WalletNetwork,
        seed: Option<&str>,
        metadata: crate::core::channels::dlight_private::DlightSeedMetadata,
    ) -> Result<(), WalletError> {
        let path = self.dlight_seed_snapshot_path(account_id);
        let mut snapshot = self
            .load_dlight_seed_snapshot(account_id, password_hash)
            .await?;
        snapshot.schema_version = DLIGHT_SEED_SCHEMA_VERSION;

        let normalized_seed = seed
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        *network_mut(&mut snapshot.mainnet, &mut snapshot.testnet, network) = normalized_seed;
        *network_mut(
            &mut snapshot.mainnet_metadata,
            &mut snapshot.testnet_metadata,
            network,
        ) = metadata;
        self.store_json_snapshot_to_path(
            account_id,
            password_hash,
            &path,
            DLIGHT_SEED_RECORD_KEY,
            &snapshot,
            "dlight seed",
        )
    }

    pub async fn load_provisioning_jobs(
        &self,
        account_id: &str,
        password_hash: &[u8],
        network: WalletNetwork,
    ) -> Result<Vec<ProvisioningJobRecord>, WalletError> {
        let snapshot = self
            .load_provisioning_jobs_snapshot(account_id, password_hash)
            .await?;
        Ok(network_value(snapshot.mainnet, snapshot.testnet, network))
    }

    pub async fn store_provisioning_jobs(
        &self,
        account_id: &str,
        password_hash: &[u8],
        network: WalletNetwork,
        jobs: &[ProvisioningJobRecord],
    ) -> Result<(), WalletError> {
        let path = self.provisioning_jobs_snapshot_path(account_id);
        let mut snapshot = self
            .load_provisioning_jobs_snapshot(account_id, password_hash)
            .await?;
        snapshot.schema_version = PROVISIONING_JOBS_SCHEMA_VERSION;
        *network_mut(&mut snapshot.mainnet, &mut snapshot.testnet, network) = jobs.to_vec();
        self.store_json_snapshot_to_path(
            account_id,
            password_hash,
            &path,
            PROVISIONING_JOBS_RECORD_KEY,
            &snapshot,
            "provisioning jobs",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{StrongholdStore, ACTIVE_ASSETS_PROFILE_VERSION};
    use crate::core::auth::kdf::{CURRENT_KEY_DERIVATION_VERSION, LEGACY_KEY_DERIVATION_VERSION};
    use crate::core::wallet::{AccountStateStore, WalletManager};
    use crate::types::generic_request::ProvisioningJobRecord;
    use crate::types::identity::LinkedIdentity;
    use crate::types::wallet::{AccountRecord, WalletNetwork, WalletSecretKind};
    use crate::types::WalletError;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_store() -> StrongholdStore {
        let base_path = std::env::temp_dir().join(format!(
            "lite_wallet_stronghold_store_{}",
            uuid::Uuid::new_v4()
        ));
        let salt_path = base_path.join("argon2_salt.bin");
        StrongholdStore {
            base_path,
            salt_path,
        }
    }

    fn test_account(
        account_id: &str,
        key_derivation_version: u8,
        network: WalletNetwork,
    ) -> AccountRecord {
        AccountRecord {
            id: account_id.to_string(),
            account_hash: format!("hash-{account_id}"),
            key_derivation_version,
            created_at: 1_700_000_000,
            last_unlocked_at: None,
            network,
            emoji: "💰".to_string(),
            color: "blue".to_string(),
            secret_kind: WalletSecretKind::SeedText,
        }
    }

    fn write_account_metadata(
        wallet_manager: &WalletManager,
        wallet_name: &str,
        account: &AccountRecord,
    ) {
        let path = wallet_manager
            .get_metadata_path(wallet_name)
            .expect("metadata path");
        let bytes = serde_json::to_vec_pretty(account).expect("serialize account");
        std::fs::write(path, bytes).expect("write metadata");
    }

    #[tokio::test]
    async fn active_assets_round_trip_preserves_network_and_initialized_flag() {
        let _ = iota_stronghold::engine::snapshot::try_set_encrypt_work_factor(0);

        let store = temp_store();
        let account_id = "account_roundtrip";
        let password_hash = StrongholdStore::derive_legacy_password_hash("test-password");

        store
            .store_active_assets(
                account_id,
                password_hash.as_ref(),
                WalletNetwork::Mainnet,
                true,
                ACTIVE_ASSETS_PROFILE_VERSION,
                &["VRSC".to_string(), "vUSDC".to_string()],
            )
            .await
            .expect("store mainnet active assets");

        store
            .store_active_assets(
                account_id,
                password_hash.as_ref(),
                WalletNetwork::Testnet,
                false,
                1,
                &["VRSCTEST".to_string()],
            )
            .await
            .expect("store testnet active assets");

        let mainnet = store
            .load_active_assets(account_id, password_hash.as_ref(), WalletNetwork::Mainnet)
            .await
            .expect("load mainnet");
        assert_eq!(mainnet.0, true);
        assert_eq!(mainnet.1, vec!["VRSC".to_string(), "vUSDC".to_string()]);
        assert_eq!(mainnet.2, ACTIVE_ASSETS_PROFILE_VERSION);

        let testnet = store
            .load_active_assets(account_id, password_hash.as_ref(), WalletNetwork::Testnet)
            .await
            .expect("load testnet");
        assert_eq!(testnet.0, false);
        assert_eq!(testnet.1, vec!["VRSCTEST".to_string()]);
        assert_eq!(testnet.2, 1);

        let _ = std::fs::remove_dir_all(store.base_path);
    }

    #[tokio::test]
    async fn linked_identities_round_trip_is_network_scoped_and_deduped() {
        let _ = iota_stronghold::engine::snapshot::try_set_encrypt_work_factor(0);

        let store = temp_store();
        let account_id = "account_linked_ids";
        let password_hash = StrongholdStore::derive_legacy_password_hash("test-password");

        store
            .store_linked_identities(
                account_id,
                password_hash.as_ref(),
                WalletNetwork::Mainnet,
                &[
                    LinkedIdentity {
                        identity_address: "iMainnetAlpha".to_string(),
                        name: Some("alpha".to_string()),
                        fully_qualified_name: Some("alpha@".to_string()),
                        status: Some("active".to_string()),
                        system_id: Some("i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string()),
                        favorite: true,
                    },
                    LinkedIdentity {
                        identity_address: "iMAINNETALPHA".to_string(),
                        name: Some("duplicate".to_string()),
                        fully_qualified_name: None,
                        status: None,
                        system_id: None,
                        favorite: false,
                    },
                ],
            )
            .await
            .expect("store mainnet linked identities");

        store
            .store_linked_identities(
                account_id,
                password_hash.as_ref(),
                WalletNetwork::Testnet,
                &[LinkedIdentity {
                    identity_address: "iTestnetBeta".to_string(),
                    name: Some("beta".to_string()),
                    fully_qualified_name: Some("beta@".to_string()),
                    status: Some("active".to_string()),
                    system_id: Some("iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq".to_string()),
                    favorite: true,
                }],
            )
            .await
            .expect("store testnet linked identities");

        let mainnet = store
            .load_linked_identities(account_id, password_hash.as_ref(), WalletNetwork::Mainnet)
            .await
            .expect("load mainnet linked identities");
        assert_eq!(mainnet.len(), 1);
        assert_eq!(mainnet[0].identity_address, "iMainnetAlpha");
        assert_eq!(mainnet[0].name.as_deref(), Some("alpha"));
        assert!(mainnet[0].favorite);

        let testnet = store
            .load_linked_identities(account_id, password_hash.as_ref(), WalletNetwork::Testnet)
            .await
            .expect("load testnet linked identities");
        assert_eq!(testnet.len(), 1);
        assert_eq!(testnet[0].identity_address, "iTestnetBeta");
        assert_eq!(testnet[0].name.as_deref(), Some("beta"));
        assert!(testnet[0].favorite);

        let _ = std::fs::remove_dir_all(store.base_path);
    }

    #[tokio::test]
    async fn linked_identities_sanitize_caps_favorites_to_two() {
        let _ = iota_stronghold::engine::snapshot::try_set_encrypt_work_factor(0);

        let store = temp_store();
        let account_id = "account_linked_ids_favorites";
        let password_hash = StrongholdStore::derive_legacy_password_hash("test-password");

        store
            .store_linked_identities(
                account_id,
                password_hash.as_ref(),
                WalletNetwork::Mainnet,
                &[
                    LinkedIdentity {
                        identity_address: "iAlpha".to_string(),
                        name: None,
                        fully_qualified_name: None,
                        status: None,
                        system_id: None,
                        favorite: true,
                    },
                    LinkedIdentity {
                        identity_address: "iBeta".to_string(),
                        name: None,
                        fully_qualified_name: None,
                        status: None,
                        system_id: None,
                        favorite: true,
                    },
                    LinkedIdentity {
                        identity_address: "iGamma".to_string(),
                        name: None,
                        fully_qualified_name: None,
                        status: None,
                        system_id: None,
                        favorite: true,
                    },
                ],
            )
            .await
            .expect("store linked identities");

        let mainnet = store
            .load_linked_identities(account_id, password_hash.as_ref(), WalletNetwork::Mainnet)
            .await
            .expect("load linked identities");

        assert_eq!(mainnet.len(), 3);
        assert!(mainnet[0].favorite);
        assert!(mainnet[1].favorite);
        assert!(!mainnet[2].favorite);

        let _ = std::fs::remove_dir_all(store.base_path);
    }

    #[tokio::test]
    async fn provisioning_jobs_round_trip_is_network_scoped() {
        let _ = iota_stronghold::engine::snapshot::try_set_encrypt_work_factor(0);

        let store = temp_store();
        let account_id = "account_provisioning_jobs";
        let password_hash = StrongholdStore::derive_legacy_password_hash("test-password");

        store
            .store_provisioning_jobs(
                account_id,
                password_hash.as_ref(),
                WalletNetwork::Mainnet,
                &[ProvisioningJobRecord {
                    job_id: "job-mainnet".to_string(),
                    request_type: "generic".to_string(),
                    request_hex: "deadbeef".to_string(),
                    requested_identity_address: Some("iMainnetAlpha".to_string()),
                    requested_fqn: "alpha.verus".to_string(),
                    signing_id: "iSigner".to_string(),
                    has_response_uris: true,
                    info_uri: Some("https://example.com/info".to_string()),
                    status: "pending".to_string(),
                    created_at: 1_234_567,
                    error: None,
                }],
            )
            .await
            .expect("store mainnet provisioning jobs");

        store
            .store_provisioning_jobs(
                account_id,
                password_hash.as_ref(),
                WalletNetwork::Testnet,
                &[ProvisioningJobRecord {
                    job_id: "job-testnet".to_string(),
                    request_type: "generic".to_string(),
                    request_hex: "cafebabe".to_string(),
                    requested_identity_address: Some("iTestnetBeta".to_string()),
                    requested_fqn: "beta.vrsctest".to_string(),
                    signing_id: "iSignerTestnet".to_string(),
                    has_response_uris: false,
                    info_uri: None,
                    status: "ready".to_string(),
                    created_at: 7_654_321,
                    error: Some("none".to_string()),
                }],
            )
            .await
            .expect("store testnet provisioning jobs");

        let mainnet = store
            .load_provisioning_jobs(account_id, password_hash.as_ref(), WalletNetwork::Mainnet)
            .await
            .expect("load mainnet provisioning jobs");
        assert_eq!(mainnet.len(), 1);
        assert_eq!(mainnet[0].job_id, "job-mainnet");
        assert_eq!(mainnet[0].requested_fqn, "alpha.verus");

        let testnet = store
            .load_provisioning_jobs(account_id, password_hash.as_ref(), WalletNetwork::Testnet)
            .await
            .expect("load testnet provisioning jobs");
        assert_eq!(testnet.len(), 1);
        assert_eq!(testnet[0].job_id, "job-testnet");
        assert_eq!(testnet[0].status, "ready");

        let _ = std::fs::remove_dir_all(store.base_path);
    }

    #[tokio::test]
    async fn ensure_account_password_hash_migrates_legacy_records_and_state() {
        let _ = iota_stronghold::engine::snapshot::try_set_encrypt_work_factor(0);

        let store = temp_store();
        let wallet_data_dir = std::env::temp_dir().join(format!(
            "lite_wallet_stronghold_wallet_data_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let wallet_manager = WalletManager::new(wallet_data_dir.clone());
        let account_state_store = AccountStateStore::new(wallet_data_dir.clone());
        let account_id = "legacy-account";
        let account = test_account(
            account_id,
            LEGACY_KEY_DERIVATION_VERSION,
            WalletNetwork::Mainnet,
        );
        write_account_metadata(&wallet_manager, "legacy-wallet", &account);

        let legacy_hash = StrongholdStore::derive_legacy_password_hash("test-password");
        store
            .store_seed(account_id, "legacy seed phrase", legacy_hash.as_ref())
            .await
            .expect("store legacy seed");
        store
            .store_address_book(account_id, legacy_hash.as_ref(), br#"{"contacts":[]}"#)
            .await
            .expect("store address book");
        store
            .store_linked_identities(
                account_id,
                legacy_hash.as_ref(),
                WalletNetwork::Mainnet,
                &[LinkedIdentity {
                    identity_address: "iMainnetAlpha".to_string(),
                    name: Some("alpha".to_string()),
                    fully_qualified_name: Some("alpha@".to_string()),
                    status: Some("active".to_string()),
                    system_id: Some("i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string()),
                    favorite: true,
                }],
            )
            .await
            .expect("store linked identities");
        store
            .store_dlight_seed(
                account_id,
                legacy_hash.as_ref(),
                WalletNetwork::Mainnet,
                Some("dlight-mainnet"),
            )
            .await
            .expect("store dlight mainnet");
        store
            .store_watched_vrpc_addresses(
                account_id,
                legacy_hash.as_ref(),
                WalletNetwork::Mainnet,
                &["RMain".to_string()],
            )
            .await
            .expect("store watched");
        store
            .store_active_assets(
                account_id,
                legacy_hash.as_ref(),
                WalletNetwork::Mainnet,
                true,
                ACTIVE_ASSETS_PROFILE_VERSION,
                &["VRSC".to_string()],
            )
            .await
            .expect("store active assets");
        store
            .store_provisioning_jobs(
                account_id,
                legacy_hash.as_ref(),
                WalletNetwork::Mainnet,
                &[ProvisioningJobRecord {
                    job_id: "job-mainnet".to_string(),
                    request_type: "generic".to_string(),
                    request_hex: "deadbeef".to_string(),
                    requested_identity_address: Some("iMainnetAlpha".to_string()),
                    requested_fqn: "alpha.verus".to_string(),
                    signing_id: "iSigner".to_string(),
                    has_response_uris: true,
                    info_uri: Some("https://example.com/info".to_string()),
                    status: "pending".to_string(),
                    created_at: 1_234_567,
                    error: None,
                }],
            )
            .await
            .expect("store jobs");

        let current_hash = store
            .ensure_account_password_hash(
                &account,
                "test-password",
                &wallet_manager,
                &account_state_store,
            )
            .await
            .expect("migrate account");

        let migrated_account = wallet_manager
            .get_account_record_by_account_id(account_id)
            .await
            .expect("lookup account")
            .expect("account record");
        assert_eq!(
            migrated_account.key_derivation_version,
            CURRENT_KEY_DERIVATION_VERSION
        );
        assert!(store.salt_path().exists());

        let seed = store
            .load_seed(account_id, current_hash.as_ref())
            .await
            .expect("load migrated seed");
        assert_eq!(seed, "legacy seed phrase");
        let address_book = store
            .load_address_book(account_id, current_hash.as_ref())
            .await
            .expect("load migrated address book");
        assert_eq!(address_book, Some(br#"{"contacts":[]}"#.to_vec()));
        let linked = store
            .load_linked_identities(account_id, current_hash.as_ref(), WalletNetwork::Mainnet)
            .await
            .expect("load migrated linked identities");
        assert_eq!(linked.len(), 1);
        let dlight = store
            .load_dlight_seed(account_id, current_hash.as_ref(), WalletNetwork::Mainnet)
            .await
            .expect("load migrated dlight");
        assert_eq!(dlight.as_deref(), Some("dlight-mainnet"));

        let watched = account_state_store
            .load_watched_vrpc_addresses(account_id, WalletNetwork::Mainnet)
            .expect("load watched state");
        assert_eq!(watched, vec!["RMain".to_string()]);
        let active_assets = account_state_store
            .load_active_assets(account_id, WalletNetwork::Mainnet)
            .expect("load active assets state");
        assert_eq!(active_assets.0, true);
        assert_eq!(active_assets.1, vec!["VRSC".to_string()]);
        assert_eq!(active_assets.2, ACTIVE_ASSETS_PROFILE_VERSION);
        let jobs = account_state_store
            .load_provisioning_jobs(account_id, WalletNetwork::Mainnet)
            .expect("load jobs state");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].job_id, "job-mainnet");

        assert!(!store
            .watched_vrpc_addresses_snapshot_path(account_id)
            .exists());
        assert!(!store.active_assets_snapshot_path(account_id).exists());
        assert!(!store.provisioning_jobs_snapshot_path(account_id).exists());

        let _ = std::fs::remove_dir_all(store.base_path);
        let _ = std::fs::remove_dir_all(wallet_data_dir);
    }

    #[tokio::test]
    async fn ensure_account_password_hash_wrong_legacy_password_does_not_mutate_metadata_or_salt() {
        let _ = iota_stronghold::engine::snapshot::try_set_encrypt_work_factor(0);

        let store = temp_store();
        let wallet_data_dir = std::env::temp_dir().join(format!(
            "lite_wallet_stronghold_wrong_password_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let wallet_manager = WalletManager::new(wallet_data_dir.clone());
        let account_state_store = AccountStateStore::new(wallet_data_dir.clone());
        let account_id = "legacy-account";
        let account = test_account(
            account_id,
            LEGACY_KEY_DERIVATION_VERSION,
            WalletNetwork::Mainnet,
        );
        write_account_metadata(&wallet_manager, "legacy-wallet", &account);

        let legacy_hash = StrongholdStore::derive_legacy_password_hash("test-password");
        store
            .store_seed(account_id, "legacy seed phrase", legacy_hash.as_ref())
            .await
            .expect("store legacy seed");

        let result = store
            .ensure_account_password_hash(
                &account,
                "wrong-password",
                &wallet_manager,
                &account_state_store,
            )
            .await;
        assert!(matches!(result, Err(WalletError::InvalidPassword)));
        assert!(!store.salt_path().exists());

        let persisted_account = wallet_manager
            .get_account_record_by_account_id(account_id)
            .await
            .expect("lookup account")
            .expect("account record");
        assert_eq!(
            persisted_account.key_derivation_version,
            LEGACY_KEY_DERIVATION_VERSION
        );

        let _ = std::fs::remove_dir_all(store.base_path);
        let _ = std::fs::remove_dir_all(wallet_data_dir);
    }

    #[tokio::test]
    async fn ensure_account_password_hash_requires_existing_salt_for_current_accounts() {
        let store = temp_store();
        let wallet_data_dir = std::env::temp_dir().join(format!(
            "lite_wallet_stronghold_missing_salt_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let wallet_manager = WalletManager::new(wallet_data_dir.clone());
        let account_state_store = AccountStateStore::new(wallet_data_dir.clone());
        let account = test_account(
            "argon2-account",
            CURRENT_KEY_DERIVATION_VERSION,
            WalletNetwork::Mainnet,
        );

        let result = store
            .ensure_account_password_hash(
                &account,
                "test-password",
                &wallet_manager,
                &account_state_store,
            )
            .await;
        assert!(matches!(result, Err(WalletError::SecureStorageUnavailable)));

        let _ = std::fs::remove_dir_all(store.base_path);
        let _ = std::fs::remove_dir_all(wallet_data_dir);
    }
    #[tokio::test]
    async fn dlight_birthday_survives_storage_and_phrases_match_mobile() {
        use crate::core::channels::dlight_private::{self, DlightBirthday, DlightSeedMetadata};
        let _ = iota_stronghold::engine::snapshot::try_set_encrypt_work_factor(0);
        let store = temp_store();
        let password_hash = [7; 32];
        let phrase = bip39::Mnemonic::from_entropy(&[0; 32]).unwrap().to_string();
        store
            .store_dlight_seed(
                "fixture",
                &password_hash,
                WalletNetwork::Mainnet,
                Some(&phrase),
            )
            .await
            .unwrap();
        store
            .store_dlight_seed_with_metadata(
                "fixture",
                &password_hash,
                WalletNetwork::Testnet,
                Some(&phrase),
                DlightSeedMetadata {
                    birthday: Some(DlightBirthday {
                        height: 123,
                        block_hash_hex: "ab".repeat(32),
                        sapling_tree: "000000".into(),
                    }),
                },
            )
            .await
            .unwrap();
        let (key, metadata) = store
            .load_dlight_runtime_material("fixture", &password_hash, WalletNetwork::Testnet)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(metadata.birthday.unwrap().height, 123);
        assert_eq!(
            dlight_private::derive_scope_address(&key, WalletNetwork::Testnet).unwrap(),
            dlight_private::derive_scope_address(&phrase, WalletNetwork::Mainnet).unwrap()
        );
        assert_eq!(
            store
                .load_dlight_seed("fixture", &password_hash, WalletNetwork::Testnet)
                .await
                .unwrap()
                .unwrap(),
            phrase
        );
        let (_, mainnet_metadata) = store
            .load_dlight_runtime_material("fixture", &password_hash, WalletNetwork::Mainnet)
            .await
            .unwrap()
            .unwrap();
        assert!(mainnet_metadata.birthday.is_none());
        let legacy: super::DlightSeedSnapshot =
            serde_json::from_value(serde_json::json!({"schema_version":1, "testnet": phrase}))
                .unwrap();
        assert!(legacy.testnet_metadata.birthday.is_none());
        // Replacing a secret clears its obsolete checkpoint atomically.
        store
            .store_dlight_seed(
                "fixture",
                &password_hash,
                WalletNetwork::Testnet,
                Some(&phrase),
            )
            .await
            .unwrap();
        let (_, replaced_metadata) = store
            .load_dlight_runtime_material("fixture", &password_hash, WalletNetwork::Testnet)
            .await
            .unwrap()
            .unwrap();
        assert!(replaced_metadata.birthday.is_none());
        std::fs::remove_dir_all(store.base_path).unwrap();
    }
}
