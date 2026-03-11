//
// Wallet management core business logic with Stronghold integration
// Security: Handles sensitive key operations - all functions require proper validation
// Last Updated: data_directory now set from app data dir (unified with Stronghold storage)

use bip39::{Language, Mnemonic};
use rand::rngs::OsRng;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{
    AccountRecord, CreateWalletRequest, WalletError, WalletListItem, WalletMetadata,
};

pub struct WalletManager {
    data_directory: PathBuf,
}

fn sort_wallet_list_items(wallets: &mut [(WalletListItem, Option<u64>)]) {
    wallets.sort_by(
        |(left_wallet, left_last_unlocked), (right_wallet, right_last_unlocked)| {
            right_last_unlocked
                .cmp(left_last_unlocked)
                .then_with(|| {
                    left_wallet
                        .wallet_name
                        .to_ascii_lowercase()
                        .cmp(&right_wallet.wallet_name.to_ascii_lowercase())
                })
                .then_with(|| left_wallet.account_id.cmp(&right_wallet.account_id))
        },
    );
}

impl WalletManager {
    /// Create wallet manager with an explicit data directory (e.g. app data dir).
    /// Call from setup with `app_handle.path().app_data_dir().join("wallet_data")` so metadata
    /// and Stronghold vaults share the same root.
    pub fn new(data_directory: PathBuf) -> Self {
        Self { data_directory }
    }

    pub fn data_directory(&self) -> PathBuf {
        self.data_directory.clone()
    }

    /// Generate a new BIP39 mnemonic phrase
    pub async fn generate_mnemonic(&self, word_count: usize) -> Result<String, WalletError> {
        let entropy_bits = match word_count {
            12 => 16,
            15 => 20,
            18 => 24,
            21 => 28,
            24 => 32,
            _ => return Err(WalletError::InvalidSeedPhrase),
        };

        println!("[WALLET] Generating {} word mnemonic", word_count);

        // Generate entropy
        let mut entropy = vec![0u8; entropy_bits];
        rand::RngCore::fill_bytes(&mut OsRng, &mut entropy);

        // Create mnemonic from entropy
        let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy).map_err(|e| {
            println!("[WALLET] Mnemonic generation failed: {}", e);
            WalletError::OperationFailed
        })?;

        println!("[WALLET] Mnemonic generation successful");
        Ok(mnemonic.to_string())
    }

    /// Validate a BIP39 mnemonic phrase
    pub async fn validate_mnemonic(&self, seed_phrase: &str) -> Result<bool, WalletError> {
        let is_valid = Mnemonic::parse_in(Language::English, seed_phrase).is_ok();
        Ok(is_valid)
    }

    /// Return the BIP39 English word list used for mnemonic validation.
    pub async fn get_mnemonic_wordlist(&self) -> Result<Vec<String>, WalletError> {
        Ok(Language::English
            .word_list()
            .iter()
            .map(|word| (*word).to_string())
            .collect())
    }

    /// Create a new wallet (simplified implementation for now)
    /// TODO: Implement full Stronghold integration using frontend API
    pub async fn create_wallet(
        &self,
        request: &CreateWalletRequest,
        _password: &str,
    ) -> Result<String, WalletError> {
        // Validate seed phrase
        let _mnemonic =
            Mnemonic::parse_in(Language::English, &request.seed_phrase).map_err(|e| {
                println!("[WALLET] Invalid seed phrase format: {}", e);
                WalletError::InvalidSeedPhrase
            })?;

        // Check if wallet already exists
        if self.wallet_exists(&request.wallet_name).await? {
            return Err(WalletError::WalletExists);
        }

        println!("[WALLET] Creating wallet: {}", request.wallet_name);

        // Create wallet metadata file (simplified for now)
        let metadata = WalletMetadata::new(request.wallet_name.clone());
        let metadata_path = self.get_metadata_path(&request.wallet_name)?;

        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(metadata_path, metadata_json).map_err(|e| {
            println!("[WALLET] Failed to write metadata: {}", e);
            WalletError::OperationFailed
        })?;

        println!("[WALLET] Wallet created successfully (simplified)");
        println!("[WALLET] TODO: Integrate Stronghold for secure seed storage");

        Ok(request.wallet_name.clone())
    }

    /// List available wallets with account_id for unlock flow.
    /// Most recently unlocked wallets appear first.
    pub async fn list_wallets(&self) -> Result<Vec<WalletListItem>, WalletError> {
        let mut wallets_with_last_unlock: Vec<(WalletListItem, Option<u64>)> = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.data_directory) {
            for entry in entries.flatten() {
                let Some(name) = entry.file_name().to_str().map(|value| value.to_string()) else {
                    continue;
                };
                if !name.ends_with("_metadata.json") {
                    continue;
                }
                let wallet_name = name.trim_end_matches("_metadata.json").to_string();
                let path = self.data_directory.join(name);
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let Ok(account) = serde_json::from_str::<AccountRecord>(&content) else {
                    continue;
                };
                wallets_with_last_unlock.push((
                    WalletListItem {
                        account_id: account.id,
                        wallet_name,
                        network: account.network,
                        emoji: account.emoji,
                        color: account.color,
                    },
                    account.last_unlocked_at,
                ));
            }
        }

        sort_wallet_list_items(&mut wallets_with_last_unlock);

        Ok(wallets_with_last_unlock
            .into_iter()
            .map(|(wallet, _last_unlocked_at)| wallet)
            .collect())
    }

    /// Persist last-unlocked timestamp for the selected account.
    pub async fn mark_wallet_last_unlocked(&self, account_id: &str) -> Result<(), WalletError> {
        let unlocked_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                println!(
                    "[WALLET] Failed to compute current timestamp for last-unlocked update: {}",
                    error
                );
                WalletError::OperationFailed
            })?
            .as_secs();

        if let Ok(entries) = std::fs::read_dir(&self.data_directory) {
            for entry in entries.flatten() {
                let Some(name) = entry.file_name().to_str().map(|value| value.to_string()) else {
                    continue;
                };
                if !name.ends_with("_metadata.json") {
                    continue;
                }

                let path = self.data_directory.join(name);
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let Ok(mut account) = serde_json::from_str::<AccountRecord>(&content) else {
                    continue;
                };
                if account.id != account_id {
                    continue;
                }

                account.last_unlocked_at = Some(unlocked_at);
                let metadata_json = serde_json::to_string_pretty(&account)?;
                std::fs::write(&path, metadata_json).map_err(|error| {
                    println!(
                        "[WALLET] Failed to write last-unlocked timestamp for account {}: {}",
                        account_id, error
                    );
                    WalletError::OperationFailed
                })?;
                return Ok(());
            }
        }

        println!(
            "[WALLET] Failed to find metadata record when persisting last-unlocked for account {}",
            account_id
        );
        Err(WalletError::OperationFailed)
    }

    /// Resolve wallet display name for an account_id (for active wallet display)
    pub async fn get_wallet_name_by_account_id(
        &self,
        account_id: &str,
    ) -> Result<Option<String>, WalletError> {
        let item = self.get_wallet_by_account_id(account_id).await?;
        Ok(item.map(|w| w.wallet_name))
    }

    /// Resolve wallet list item for account_id (name + network).
    pub async fn get_wallet_by_account_id(
        &self,
        account_id: &str,
    ) -> Result<Option<WalletListItem>, WalletError> {
        let list = self.list_wallets().await?;
        Ok(list.into_iter().find(|w| w.account_id == account_id))
    }

    /// Resolve full account metadata record for an account_id.
    pub async fn get_account_record_by_account_id(
        &self,
        account_id: &str,
    ) -> Result<Option<AccountRecord>, WalletError> {
        if let Ok(entries) = std::fs::read_dir(&self.data_directory) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with("_metadata.json") {
                            let path = self.data_directory.join(name);
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Ok(account) = serde_json::from_str::<AccountRecord>(&content)
                                {
                                    if account.id == account_id {
                                        return Ok(Some(account));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    pub async fn save_account_record_by_account_id(
        &self,
        account_id: &str,
        updated: &AccountRecord,
    ) -> Result<(), WalletError> {
        if let Ok(entries) = std::fs::read_dir(&self.data_directory) {
            for entry in entries.flatten() {
                let Some(name) = entry.file_name().to_str().map(|value| value.to_string()) else {
                    continue;
                };
                if !name.ends_with("_metadata.json") {
                    continue;
                }

                let path = self.data_directory.join(name);
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let Ok(account) = serde_json::from_str::<AccountRecord>(&content) else {
                    continue;
                };
                if account.id != account_id {
                    continue;
                }

                let bytes = serde_json::to_vec_pretty(updated)?;
                std::fs::write(path, bytes).map_err(|error| {
                    println!(
                        "[WALLET] Failed to write metadata for account {}: {}",
                        account_id, error
                    );
                    WalletError::OperationFailed
                })?;
                return Ok(());
            }
        }

        Err(WalletError::OperationFailed)
    }

    pub fn has_account_with_key_derivation_version(
        &self,
        version: u8,
    ) -> Result<bool, WalletError> {
        if let Ok(entries) = std::fs::read_dir(&self.data_directory) {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let Some(name) = file_name.to_str() else {
                    continue;
                };
                if !name.ends_with("_metadata.json") {
                    continue;
                }
                let path = self.data_directory.join(name);
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let Ok(account) = serde_json::from_str::<AccountRecord>(&content) else {
                    continue;
                };
                if account.key_derivation_version == version {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Returns true if a wallet with the given name already exists (used to prevent duplicate names).
    pub async fn wallet_exists(&self, wallet_name: &str) -> Result<bool, WalletError> {
        let metadata_path = self.get_metadata_path(wallet_name)?;
        Ok(metadata_path.exists())
    }

    pub fn get_metadata_path(&self, wallet_name: &str) -> Result<PathBuf, WalletError> {
        std::fs::create_dir_all(&self.data_directory).map_err(|e| {
            println!("[WALLET] Failed to create data directory: {}", e);
            WalletError::OperationFailed
        })?;
        Ok(self
            .data_directory
            .join(format!("{}_metadata.json", wallet_name)))
    }
}

#[cfg(test)]
mod tests {
    use super::sort_wallet_list_items;
    use crate::types::{wallet::WalletNetwork, WalletListItem};

    fn wallet(account_id: &str, wallet_name: &str) -> WalletListItem {
        WalletListItem {
            account_id: account_id.to_string(),
            wallet_name: wallet_name.to_string(),
            network: WalletNetwork::Mainnet,
            emoji: "💰".to_string(),
            color: "blue".to_string(),
        }
    }

    #[test]
    fn sort_wallets_prioritizes_latest_unlock_timestamp() {
        let mut wallets = vec![
            (wallet("wallet_a", "Alpha"), None),
            (wallet("wallet_b", "Bravo"), Some(1700000000)),
            (wallet("wallet_c", "Charlie"), Some(1800000000)),
        ];

        sort_wallet_list_items(&mut wallets);

        let sorted_ids: Vec<String> = wallets
            .into_iter()
            .map(|(item, _last_unlocked_at)| item.account_id)
            .collect();
        assert_eq!(sorted_ids, vec!["wallet_c", "wallet_b", "wallet_a"]);
    }

    #[test]
    fn sort_wallets_uses_name_and_id_when_timestamp_ties() {
        let mut wallets = vec![
            (wallet("wallet_c", "Charlie"), Some(1700000000)),
            (wallet("wallet_a", "Alpha"), Some(1700000000)),
            (wallet("wallet_b", "alpha"), Some(1700000000)),
        ];

        sort_wallet_list_items(&mut wallets);

        let sorted_ids: Vec<String> = wallets
            .into_iter()
            .map(|(item, _last_unlocked_at)| item.account_id)
            .collect();
        assert_eq!(sorted_ids, vec!["wallet_a", "wallet_b", "wallet_c"]);
    }
}
