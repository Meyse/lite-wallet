//
// Wallet type definitions
// Security: Contains only non-sensitive metadata, never private keys
// Last Updated: Added WalletListItem and ActiveWalletResponse for list/unlock and dashboard

use serde::{Deserialize, Serialize};
use zeroize::ZeroizeOnDrop;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum WalletNetwork {
    Mainnet,
    Testnet,
}

impl Default for WalletNetwork {
    fn default() -> Self {
        WalletNetwork::Mainnet
    }
}

fn default_wallet_network() -> WalletNetwork {
    WalletNetwork::Mainnet
}

fn default_wallet_emoji() -> String {
    "💰".to_string()
}

fn default_wallet_color() -> String {
    "blue".to_string()
}

fn default_wallet_secret_kind() -> WalletSecretKind {
    WalletSecretKind::SeedText
}

fn default_setup_dlight_with_primary() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WalletSecretKind {
    SeedText,
    Wif,
    PrivateKeyHex,
}

impl Default for WalletSecretKind {
    fn default() -> Self {
        WalletSecretKind::SeedText
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WalletMetadata {
    pub id: String,
    pub name: String,
    pub created_at: u64,
    pub coin_types: Vec<String>,
    pub version: u8,
}

#[derive(Serialize, Deserialize)]
pub struct KeyPair {
    pub public_key: String,
    pub address: String,
    // Note: Private key remains backend-only and is never serialized here.
}

#[derive(Serialize, Deserialize)]
pub struct CreateWalletRequest {
    pub wallet_name: String,
    pub seed_phrase: String,
    #[serde(default = "default_wallet_network")]
    pub network: WalletNetwork,
    #[serde(default = "default_wallet_emoji")]
    pub emoji: String,
    #[serde(default = "default_wallet_color")]
    pub color: String,
    #[serde(default = "default_setup_dlight_with_primary")]
    pub setup_dlight_with_primary: bool,
    // Note: No password field - will be handled separately for security
}

#[derive(Serialize, Deserialize)]
pub struct ImportWalletTextRequest {
    pub wallet_name: String,
    pub import_text: String,
    #[serde(default = "default_wallet_network")]
    pub network: WalletNetwork,
    #[serde(default = "default_wallet_emoji")]
    pub emoji: String,
    #[serde(default = "default_wallet_color")]
    pub color: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateWalletResult {
    pub wallet_id: String,
    pub success: bool,
}

#[derive(Serialize, Deserialize)]
pub struct GenerateMnemonicRequest {
    pub word_count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct MnemonicResult {
    pub seed_phrase: String,
}

/// Derived cryptographic keys for a wallet account
/// Security: Implements ZeroizeOnDrop to ensure keys are cleared from memory
#[derive(Clone, ZeroizeOnDrop)]
pub struct DerivedKeys {
    pub wif: String,
    pub address: String,
    pub pub_hex: String,
    pub eth_private_key: String,
    pub eth_address: String,
    pub btc_address: String,
}

/// Account metadata record stored in filesystem
#[derive(Serialize, Deserialize, Clone)]
pub struct AccountRecord {
    pub id: String,
    pub account_hash: String,
    pub key_derivation_version: u8,
    pub created_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_unlocked_at: Option<u64>,
    #[serde(default = "default_wallet_network")]
    pub network: WalletNetwork,
    #[serde(default = "default_wallet_emoji")]
    pub emoji: String,
    #[serde(default = "default_wallet_color")]
    pub color: String,
    #[serde(default = "default_wallet_secret_kind")]
    pub secret_kind: WalletSecretKind,
}

/// Response containing derived addresses
#[derive(Serialize, Deserialize)]
pub struct AddressResponse {
    pub vrsc_address: String,
    pub eth_address: String,
    pub btc_address: String,
}

/// List item for wallet selection (unlock screen)
#[derive(Serialize, Deserialize, Clone)]
pub struct WalletListItem {
    pub account_id: String,
    pub wallet_name: String,
    pub network: WalletNetwork,
    pub emoji: String,
    pub color: String,
}

/// Active wallet display info for dashboard
#[derive(Serialize, Deserialize)]
pub struct ActiveWalletResponse {
    pub wallet_name: String,
    pub network: WalletNetwork,
    pub emoji: String,
    pub color: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ScopeKind {
    Transparent,
    Shielded,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoinScope {
    pub channel_id: String,
    pub coin_id: String,
    pub address: String,
    pub address_label: String,
    pub system_id: String,
    pub system_ticker: String,
    pub system_display_name: String,
    pub is_primary_address: bool,
    pub is_read_only: bool,
    pub scope_kind: ScopeKind,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoinScopesResult {
    pub coin_id: String,
    pub scopes: Vec<CoinScope>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ActiveAssetsState {
    pub network: WalletNetwork,
    pub initialized: bool,
    pub coin_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DlightSeedStatusResult {
    pub configured: bool,
    pub shielded_address: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum RecoverySecretKind {
    SeedText,
    Wif,
    PrivateKeyHex,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum DlightRecoverySecretKind {
    Mnemonic,
    SpendingKey,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WalletRecoverySecretsResult {
    pub primary_secret_kind: RecoverySecretKind,
    pub primary_secret: String,
    pub verus_wif: String,
    pub btc_wif: String,
    pub eth_private_key: String,
    pub verus_address: String,
    pub btc_address: String,
    pub eth_address: String,
    pub dlight_secret: Option<String>,
    pub dlight_secret_kind: Option<DlightRecoverySecretKind>,
    pub dlight_shielded_address: Option<String>,
    pub dlight_derived_spending_key: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DlightRuntimeStatusResult {
    pub channel_id: String,
    pub runtime_key: String,
    pub status_kind: String,
    pub percent: Option<f64>,
    pub scanned_height: u64,
    pub tip_height: Option<u64>,
    pub estimated_tip_height: Option<u64>,
    pub syncing: bool,
    pub last_updated: u64,
    pub last_progress_at: Option<u64>,
    pub last_tip_probe_at: Option<u64>,
    pub consecutive_failures: u32,
    pub scan_rate_blocks_per_sec: Option<f64>,
    pub stalled: bool,
    pub last_error: Option<String>,
    pub spend_cache_ready: Option<bool>,
    pub spend_cache_status_kind: Option<String>,
    pub spend_cache_percent: Option<f64>,
    pub spend_cache_lag_blocks: Option<u64>,
    pub spend_cache_last_error: Option<String>,
    pub spend_cache_scanned_height: Option<u64>,
    pub spend_cache_tip_height: Option<u64>,
    pub spend_cache_last_updated: Option<u64>,
    pub spend_cache_note_count: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DlightProverFileStatusResult {
    pub path: String,
    pub exists: bool,
    pub size_bytes: Option<u64>,
    pub min_size_bytes: u64,
    pub checksum_algorithm: String,
    pub expected_checksum: String,
    pub actual_checksum: Option<String>,
    pub checksum_matches: bool,
    pub placeholder_detected: bool,
    pub errors: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DlightProverStatusResult {
    pub ready: bool,
    pub params_dir: Option<String>,
    pub spend: DlightProverFileStatusResult,
    pub output: DlightProverFileStatusResult,
    pub errors: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum DlightSeedSetupMode {
    ReusePrimary,
    CreateNew,
    ImportText,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SetupDlightSeedRequest {
    pub mode: DlightSeedSetupMode,
    pub import_text: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SetupDlightSeedResult {
    pub configured: bool,
    pub generated_seed_phrase: Option<String>,
    pub requires_relogin: bool,
}

impl WalletMetadata {
    pub fn new(name: String) -> Self {
        Self {
            id: name.clone(),
            name,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            coin_types: vec![
                "verus".to_string(),
                "ethereum".to_string(),
                "bitcoin".to_string(),
            ],
            version: 1,
        }
    }
}

impl CreateWalletRequest {
    pub fn validate(&self) -> Result<(), crate::types::errors::WalletError> {
        validate_wallet_name(&self.wallet_name)?;

        // Validate seed phrase format (basic check)
        let words: Vec<&str> = self.seed_phrase.split_whitespace().collect();
        if words.len() != 24 {
            return Err(crate::types::errors::WalletError::InvalidSeedPhrase);
        }

        Ok(())
    }
}

impl ImportWalletTextRequest {
    pub fn validate(&self) -> Result<(), crate::types::errors::WalletError> {
        validate_wallet_name(&self.wallet_name)?;
        if self.import_text.trim().is_empty() {
            return Err(crate::types::errors::WalletError::InvalidImportText);
        }
        Ok(())
    }
}

fn validate_wallet_name(wallet_name: &str) -> Result<(), crate::types::errors::WalletError> {
    let name = wallet_name.trim();
    if name.is_empty() || name.len() > 50 {
        return Err(crate::types::errors::WalletError::InvalidWalletName);
    }

    // Check for filesystem-unsafe characters
    if name
        .chars()
        .any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
    {
        return Err(crate::types::errors::WalletError::InvalidWalletName);
    }

    Ok(())
}
