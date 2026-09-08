//
// Module 3: Coin Registry — static coin definitions and dynamic PBaaS additions.
// Endpoint defaults come from backend runtime config.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::core::coins::types::{Channel, CoinDefinition, Protocol};
use crate::core::runtime_config;
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

/// Verus mainnet system ID (i-address format).
const VRSC_SYSTEM_ID: &str = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";
const VETH_MAINNET_SYSTEM_ID: &str = "i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X";
const VUSDC_ON_VERUS_ID: &str = "i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd";
const ETH_ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

/// Registry of coins: static defaults plus dynamically added PBaaS currencies.
pub struct CoinRegistry {
    dynamic_coins_by_account: Mutex<HashMap<String, Vec<CoinDefinition>>>,
    active_account_id: Mutex<Option<String>>,
    dynamic_store_path: Option<PathBuf>,
}

impl CoinRegistry {
    pub fn new() -> Self {
        Self {
            dynamic_coins_by_account: Mutex::new(HashMap::new()),
            active_account_id: Mutex::new(None),
            dynamic_store_path: None,
        }
    }

    pub fn with_dynamic_store(path: PathBuf) -> Self {
        let dynamic_coins_by_account = Self::load_dynamic_coins(&path);
        Self {
            dynamic_coins_by_account: Mutex::new(dynamic_coins_by_account),
            active_account_id: Mutex::new(None),
            dynamic_store_path: Some(path),
        }
    }

    /// Returns all coins: static list first, then dynamic PBaaS entries.
    pub fn get_all(&self) -> Vec<CoinDefinition> {
        self.get_all_for_active_account()
    }

    pub fn get_all_for_active_account(&self) -> Vec<CoinDefinition> {
        let static_coins = Self::default_coins();
        let active_account_id = self
            .active_account_id
            .lock()
            .ok()
            .and_then(|guard| guard.clone());
        let Some(account_id) = active_account_id else {
            return static_coins;
        };

        let dynamic_by_account = self
            .dynamic_coins_by_account
            .lock()
            .expect("coin registry lock");
        let dynamic = dynamic_by_account
            .get(&account_id)
            .cloned()
            .unwrap_or_default();
        static_coins.into_iter().chain(dynamic).collect()
    }

    pub fn set_active_account(&self, account_id: Option<String>) {
        let normalized = account_id.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });

        if let Ok(mut active) = self.active_account_id.lock() {
            *active = normalized;
        }
    }

    #[cfg(test)]
    pub(crate) fn active_account_id_for_tests(&self) -> Option<String> {
        self.active_account_id
            .lock()
            .expect("coin registry lock")
            .clone()
    }

    /// Find a coin by system ID and network.
    pub fn find_by_system_id(&self, system_id: &str, is_testnet: bool) -> Option<CoinDefinition> {
        self.get_all()
            .into_iter()
            .find(|c| c.system_id == system_id && c.is_testnet == is_testnet)
    }

    /// Find a coin by ID and network.
    pub fn find_by_id(&self, id: &str, is_testnet: bool) -> Option<CoinDefinition> {
        self.get_all()
            .into_iter()
            .find(|c| c.id.eq_ignore_ascii_case(id) && c.is_testnet == is_testnet)
    }

    fn load_dynamic_coins(path: &Path) -> HashMap<String, Vec<CoinDefinition>> {
        let Ok(raw) = fs::read_to_string(path) else {
            return HashMap::new();
        };
        serde_json::from_str::<HashMap<String, Vec<CoinDefinition>>>(&raw).unwrap_or_default()
    }

    fn persist_dynamic_coins(
        &self,
        coins_by_account: &HashMap<String, Vec<CoinDefinition>>,
    ) -> Result<(), WalletError> {
        let Some(path) = self.dynamic_store_path.as_ref() else {
            return Ok(());
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|_| WalletError::OperationFailed)?;
        }

        let payload = serde_json::to_string_pretty(coins_by_account)
            .map_err(|_| WalletError::OperationFailed)?;
        fs::write(path, payload).map_err(|_| WalletError::OperationFailed)?;
        Ok(())
    }

    fn validate_coin_definition(def: &CoinDefinition) -> Result<(), WalletError> {
        if def.id.trim().is_empty() || def.system_id.trim().is_empty() {
            return Err(WalletError::InvalidCoinDefinition);
        }
        if def.compatible_channels.is_empty() {
            return Err(WalletError::InvalidCoinDefinition);
        }
        Ok(())
    }

    fn apply_runtime_endpoint_defaults(mut def: CoinDefinition) -> CoinDefinition {
        let network = if def.is_testnet {
            WalletNetwork::Testnet
        } else {
            WalletNetwork::Mainnet
        };

        match def.proto {
            Protocol::Vrsc => {
                if def
                    .compatible_channels
                    .iter()
                    .any(|channel| matches!(channel, Channel::Vrpc))
                    && def.vrpc_endpoints.is_empty()
                {
                    def.vrpc_endpoints =
                        vec![runtime_config::default_vrpc_endpoint_for_network(network)];
                }

                if def
                    .compatible_channels
                    .iter()
                    .any(|channel| matches!(channel, Channel::DlightPrivate))
                    && def
                        .dlight_endpoints
                        .as_ref()
                        .map(|endpoints| endpoints.is_empty())
                        .unwrap_or(true)
                {
                    def.dlight_endpoints = Some(
                        runtime_config::default_dlight_endpoints_for_network(network),
                    );
                }
            }
            Protocol::Btc => {
                if def
                    .compatible_channels
                    .iter()
                    .any(|channel| matches!(channel, Channel::Btc))
                    && def
                        .electrum_endpoints
                        .as_ref()
                        .map(|endpoints| endpoints.is_empty())
                        .unwrap_or(true)
                {
                    def.electrum_endpoints = Some(
                        runtime_config::default_electrum_endpoints_for_network(network),
                    );
                }
            }
            Protocol::Eth | Protocol::Erc20 => {}
        }

        def
    }

    fn canonical_asset_key(def: &CoinDefinition) -> String {
        let network = if def.is_testnet { "testnet" } else { "mainnet" };
        match def.proto {
            Protocol::Vrsc => format!(
                "vrpc:{}:{}:{}",
                def.system_id.to_ascii_lowercase(),
                def.currency_id.to_ascii_lowercase(),
                network
            ),
            Protocol::Btc => format!("btc:{}:{}", def.id.to_ascii_lowercase(), network),
            Protocol::Eth => format!("eth:{}:{}", def.id.to_ascii_lowercase(), network),
            Protocol::Erc20 => {
                format!("erc20:{}:{}", def.currency_id.to_ascii_lowercase(), network)
            }
        }
    }

    fn has_duplicate(existing: &CoinDefinition, candidate: &CoinDefinition) -> bool {
        if existing.is_testnet != candidate.is_testnet {
            return false;
        }

        if existing.id.eq_ignore_ascii_case(&candidate.id) {
            return true;
        }

        if !existing.currency_id.is_empty()
            && !candidate.currency_id.is_empty()
            && existing
                .currency_id
                .eq_ignore_ascii_case(&candidate.currency_id)
            && existing.proto == candidate.proto
        {
            return true;
        }

        Self::canonical_asset_key(existing) == Self::canonical_asset_key(candidate)
    }

    /// Adds a coin definition to the dynamic registry for a specific account and persists it.
    /// Rejects duplicates by canonical asset key (network + protocol identity).
    pub fn add_coin_for_account(
        &self,
        account_id: &str,
        def: CoinDefinition,
    ) -> Result<CoinDefinition, WalletError> {
        let account_key = account_id.trim();
        if account_key.is_empty() {
            return Err(WalletError::WalletLocked);
        }

        let def = Self::apply_runtime_endpoint_defaults(def);
        Self::validate_coin_definition(&def)?;

        let static_coins = Self::default_coins();
        let mut dynamic_by_account = self
            .dynamic_coins_by_account
            .lock()
            .map_err(|_| WalletError::OperationFailed)?;

        let duplicate_static = static_coins.iter().any(|c| Self::has_duplicate(c, &def));
        if duplicate_static {
            return Err(WalletError::AssetAlreadyExists);
        }

        let previous_account_coins = dynamic_by_account.get(account_key).cloned();
        let mut next_account_coins = previous_account_coins.clone().unwrap_or_default();
        if let Some(existing_index) = next_account_coins
            .iter()
            .position(|coin| Self::has_duplicate(coin, &def))
        {
            if next_account_coins[existing_index] == def {
                return Err(WalletError::AssetAlreadyExists);
            }
            next_account_coins[existing_index] = def.clone();
        } else {
            next_account_coins.push(def.clone());
        }

        dynamic_by_account.insert(account_key.to_string(), next_account_coins);
        if let Err(err) = self.persist_dynamic_coins(&dynamic_by_account) {
            if let Some(previous) = previous_account_coins {
                dynamic_by_account.insert(account_key.to_string(), previous);
            } else {
                dynamic_by_account.remove(account_key);
            }
            return Err(err);
        }

        Ok(def)
    }

    /// Adds a coin definition to the currently active account registry.
    pub fn add_coin_for_active_account(
        &self,
        def: CoinDefinition,
    ) -> Result<CoinDefinition, WalletError> {
        let account_id = self
            .active_account_id
            .lock()
            .map_err(|_| WalletError::OperationFailed)?
            .clone()
            .ok_or(WalletError::WalletLocked)?;
        self.add_coin_for_account(&account_id, def)
    }

    /// Backward-compatible wrapper that writes to the currently active account.
    pub fn add_coin(&self, def: CoinDefinition) -> Result<CoinDefinition, WalletError> {
        self.add_coin_for_active_account(def)
    }

    /// Adds a PBaaS currency. Validates proto (Vrsc) and required fields.
    /// Returns error if id or currency_id already present (no duplicates).
    pub fn add_pbaas(&self, def: CoinDefinition) -> Result<(), WalletError> {
        if def.proto != Protocol::Vrsc {
            return Err(WalletError::InvalidCoinDefinition);
        }
        if def.id.is_empty() || def.currency_id.is_empty() || def.system_id.is_empty() {
            return Err(WalletError::InvalidCoinDefinition);
        }
        match self.add_coin_for_active_account(def) {
            Ok(_) => Ok(()),
            Err(WalletError::AssetAlreadyExists) => Err(WalletError::DuplicatePbaasCurrency),
            Err(err) => Err(err),
        }
    }

    /// Static coin definitions. Endpoint defaults come from runtime config.
    fn default_coins() -> Vec<CoinDefinition> {
        let vrpc_mainnet = runtime_config::vrpc_mainnet_url();
        let vrpc_testnet = runtime_config::vrpc_testnet_url();
        let dlight_mainnet = runtime_config::dlight_mainnet_endpoints();
        let dlight_testnet = runtime_config::dlight_testnet_endpoints();
        let electrum_mainnet = runtime_config::electrum_mainnet_endpoints();
        let electrum_testnet = runtime_config::electrum_testnet_endpoints();

        vec![
            // VRSC mainnet
            CoinDefinition {
                id: "VRSC".to_string(),
                currency_id: VRSC_SYSTEM_ID.to_string(),
                system_id: VRSC_SYSTEM_ID.to_string(),
                display_ticker: "VRSC".to_string(),
                display_name: "Verus".to_string(),
                coin_paprika_id: Some("vrsc-verus-coin".to_string()),
                proto: Protocol::Vrsc,
                compatible_channels: vec![Channel::Vrpc, Channel::DlightPrivate],
                decimals: 8,
                vrpc_endpoints: vec![vrpc_mainnet.clone()],
                dlight_endpoints: Some(dlight_mainnet.clone()),
                electrum_endpoints: None,
                seconds_per_block: 60,
                mapped_to: None,
                is_testnet: false,
            },
            // VRSCTEST
            CoinDefinition {
                id: "VRSCTEST".to_string(),
                currency_id: "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq".to_string(),
                system_id: "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq".to_string(),
                display_ticker: "VRSCTEST".to_string(),
                display_name: "Verus Testnet".to_string(),
                coin_paprika_id: None,
                proto: Protocol::Vrsc,
                compatible_channels: vec![Channel::Vrpc, Channel::DlightPrivate],
                decimals: 8,
                vrpc_endpoints: vec![vrpc_testnet.clone()],
                dlight_endpoints: Some(dlight_testnet.clone()),
                electrum_endpoints: None,
                seconds_per_block: 60,
                mapped_to: None,
                is_testnet: true,
            },
            // BTC mainnet
            CoinDefinition {
                id: "BTC".to_string(),
                currency_id: String::new(),
                system_id: "BTC".to_string(),
                display_ticker: "BTC".to_string(),
                display_name: "Bitcoin".to_string(),
                coin_paprika_id: Some("btc-bitcoin".to_string()),
                proto: Protocol::Btc,
                compatible_channels: vec![Channel::Btc],
                decimals: 8,
                vrpc_endpoints: vec![],
                dlight_endpoints: None,
                electrum_endpoints: Some(electrum_mainnet.clone()),
                seconds_per_block: 600,
                mapped_to: None,
                is_testnet: false,
            },
            // BTCTEST
            CoinDefinition {
                id: "BTCTEST".to_string(),
                currency_id: String::new(),
                system_id: "BTCTEST".to_string(),
                display_ticker: "BTCTEST".to_string(),
                display_name: "Bitcoin Testnet".to_string(),
                coin_paprika_id: None,
                proto: Protocol::Btc,
                compatible_channels: vec![Channel::Btc],
                decimals: 8,
                vrpc_endpoints: vec![],
                dlight_endpoints: None,
                electrum_endpoints: Some(electrum_testnet.clone()),
                seconds_per_block: 600,
                mapped_to: None,
                is_testnet: true,
            },
            // ETH mainnet
            CoinDefinition {
                id: "ETH".to_string(),
                currency_id: ETH_ZERO_ADDRESS.to_string(),
                system_id: "ETH".to_string(),
                display_ticker: "ETH".to_string(),
                display_name: "Ethereum".to_string(),
                coin_paprika_id: Some("eth-ethereum".to_string()),
                proto: Protocol::Eth,
                compatible_channels: vec![Channel::Eth],
                decimals: 18,
                vrpc_endpoints: vec![],
                dlight_endpoints: None,
                electrum_endpoints: None,
                seconds_per_block: 12,
                mapped_to: None,
                is_testnet: false,
            },
            // GETH testnet
            CoinDefinition {
                id: "GETH".to_string(),
                currency_id: ETH_ZERO_ADDRESS.to_string(),
                system_id: "GETH".to_string(),
                display_ticker: "GETH".to_string(),
                display_name: "Ethereum Testnet".to_string(),
                coin_paprika_id: None,
                proto: Protocol::Eth,
                compatible_channels: vec![Channel::Eth],
                decimals: 18,
                vrpc_endpoints: vec![],
                dlight_endpoints: None,
                electrum_endpoints: None,
                seconds_per_block: 12,
                mapped_to: None,
                is_testnet: true,
            },
            // ERC20 example: USDC on Ethereum
            CoinDefinition {
                id: "USDC".to_string(),
                currency_id: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
                system_id: "ETH".to_string(),
                display_ticker: "USDC".to_string(),
                display_name: "USD Coin".to_string(),
                coin_paprika_id: Some("usdc-usd-coin".to_string()),
                proto: Protocol::Erc20,
                compatible_channels: vec![Channel::Erc20],
                decimals: 6,
                vrpc_endpoints: vec![],
                dlight_endpoints: None,
                electrum_endpoints: None,
                seconds_per_block: 12,
                mapped_to: Some("ETH".to_string()),
                is_testnet: false,
            },
            // vUSDC.vETH on Verus
            CoinDefinition {
                id: VUSDC_ON_VERUS_ID.to_string(),
                currency_id: VUSDC_ON_VERUS_ID.to_string(),
                system_id: VETH_MAINNET_SYSTEM_ID.to_string(),
                display_ticker: "vUSDC.vETH".to_string(),
                display_name: "USDC on Verus".to_string(),
                coin_paprika_id: Some("usdc-usd-coin".to_string()),
                proto: Protocol::Vrsc,
                compatible_channels: vec![Channel::Vrpc],
                decimals: 8,
                vrpc_endpoints: vec![vrpc_mainnet.clone()],
                dlight_endpoints: None,
                electrum_endpoints: None,
                seconds_per_block: 60,
                mapped_to: Some("USDC".to_string()),
                is_testnet: false,
            },
        ]
    }
}

impl Default for CoinRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn set_active_account(registry: &CoinRegistry, account_id: &str) {
        registry.set_active_account(Some(account_id.to_string()));
    }

    fn sample_dynamic_coin() -> CoinDefinition {
        CoinDefinition {
            id: "iSampleCurrency".to_string(),
            currency_id: "iSampleCurrency".to_string(),
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            display_ticker: "SAMPLE".to_string(),
            display_name: "Sample currency".to_string(),
            coin_paprika_id: None,
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec![runtime_config::vrpc_mainnet_url()],
            dlight_endpoints: None,
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        }
    }

    #[test]
    fn vrsc_and_vrsctest_system_ids_match_reference() {
        let registry = CoinRegistry::new();
        let all = registry.get_all();

        let vrsc = all.iter().find(|c| c.id == "VRSC").expect("VRSC coin");
        let vrsctest = all
            .iter()
            .find(|c| c.id == "VRSCTEST")
            .expect("VRSCTEST coin");

        assert_eq!(
            vrsc.system_id, "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV",
            "VRSC mainnet system id must match mobile reference"
        );
        assert_eq!(
            vrsctest.system_id, "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq",
            "VRSCTEST system id must stay stable"
        );
    }

    #[test]
    fn add_coin_rejects_duplicate_static_asset() {
        let registry = CoinRegistry::new();
        set_active_account(&registry, "account_alpha");
        let duplicate = CoinDefinition {
            id: "USDC_DUP".to_string(),
            currency_id: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
            system_id: "ETH".to_string(),
            display_ticker: "USDC".to_string(),
            display_name: "USD Coin duplicate".to_string(),
            coin_paprika_id: None,
            proto: Protocol::Erc20,
            compatible_channels: vec![Channel::Erc20],
            decimals: 6,
            vrpc_endpoints: vec![],
            dlight_endpoints: None,
            electrum_endpoints: None,
            seconds_per_block: 12,
            mapped_to: Some("ETH".to_string()),
            is_testnet: false,
        };

        let result = registry.add_coin(duplicate);
        assert!(matches!(result, Err(WalletError::AssetAlreadyExists)));
    }

    #[test]
    fn add_coin_updates_existing_dynamic_duplicate_with_new_metadata() {
        let registry = CoinRegistry::new();
        set_active_account(&registry, "account_alpha");
        let stale = CoinDefinition {
            id: "i3d4vSCbXYEC3u6TzwohMvdghHkhBrXWpE".to_string(),
            currency_id: "i3d4vSCbXYEC3u6TzwohMvdghHkhBrXWpE".to_string(),
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            display_ticker: "VRSC".to_string(),
            display_name: "Verus".to_string(),
            coin_paprika_id: Some("vrsc-verus-coin".to_string()),
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec![runtime_config::vrpc_mainnet_url()],
            dlight_endpoints: None,
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        };
        registry.add_coin(stale).expect("add stale dynamic coin");

        let corrected = CoinDefinition {
            id: "i3d4vSCbXYEC3u6TzwohMvdghHkhBrXWpE".to_string(),
            currency_id: "i3d4vSCbXYEC3u6TzwohMvdghHkhBrXWpE".to_string(),
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            display_ticker: "Floralis".to_string(),
            display_name: "Floralis".to_string(),
            coin_paprika_id: None,
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec![runtime_config::vrpc_mainnet_url()],
            dlight_endpoints: None,
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        };
        let updated = registry
            .add_coin(corrected.clone())
            .expect("update stale duplicate coin");

        assert_eq!(updated.display_ticker, "Floralis");
        assert_eq!(updated.display_name, "Floralis");
        assert_eq!(updated.coin_paprika_id, None);

        let found = registry
            .find_by_id("i3d4vSCbXYEC3u6TzwohMvdghHkhBrXWpE", false)
            .expect("updated dynamic coin");
        assert_eq!(found.display_ticker, "Floralis");
        assert_eq!(found.display_name, "Floralis");
        assert_eq!(found, corrected);
    }

    #[test]
    fn add_coin_rejects_identical_dynamic_duplicate() {
        let registry = CoinRegistry::new();
        set_active_account(&registry, "account_alpha");
        let sample = sample_dynamic_coin();
        registry
            .add_coin(sample.clone())
            .expect("add first dynamic coin");

        let result = registry.add_coin(sample);
        assert!(matches!(result, Err(WalletError::AssetAlreadyExists)));
    }

    #[test]
    fn add_coin_persists_dynamic_entries_when_store_is_configured() {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("lite_wallet_coin_registry_{}.json", unique_suffix));

        let registry = CoinRegistry::with_dynamic_store(path.clone());
        set_active_account(&registry, "account_alpha");
        let added = registry
            .add_coin(sample_dynamic_coin())
            .expect("add dynamic coin");
        assert_eq!(added.id, "iSampleCurrency");

        let reloaded = CoinRegistry::with_dynamic_store(path.clone());
        set_active_account(&reloaded, "account_alpha");
        let found = reloaded
            .find_by_id("iSampleCurrency", false)
            .expect("reloaded coin");
        assert_eq!(found.currency_id, "iSampleCurrency");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn add_coin_isolated_between_accounts() {
        let registry = CoinRegistry::new();
        set_active_account(&registry, "account_alpha");
        registry
            .add_coin(sample_dynamic_coin())
            .expect("add dynamic coin in alpha");

        set_active_account(&registry, "account_beta");
        assert!(registry.find_by_id("iSampleCurrency", false).is_none());

        set_active_account(&registry, "account_alpha");
        assert!(registry.find_by_id("iSampleCurrency", false).is_some());
    }

    #[test]
    fn add_coin_persists_dynamic_entries_per_account() {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lite_wallet_coin_registry_by_account_{}.json",
            unique_suffix
        ));

        let registry = CoinRegistry::with_dynamic_store(path.clone());
        set_active_account(&registry, "account_alpha");
        registry
            .add_coin(sample_dynamic_coin())
            .expect("add coin for alpha");

        set_active_account(&registry, "account_beta");
        registry
            .add_coin(CoinDefinition {
                id: "iOtherCurrency".to_string(),
                currency_id: "iOtherCurrency".to_string(),
                system_id: VRSC_SYSTEM_ID.to_string(),
                display_ticker: "OTHER".to_string(),
                display_name: "Other currency".to_string(),
                coin_paprika_id: None,
                proto: Protocol::Vrsc,
                compatible_channels: vec![Channel::Vrpc],
                decimals: 8,
                vrpc_endpoints: vec![runtime_config::vrpc_mainnet_url()],
                dlight_endpoints: None,
                electrum_endpoints: None,
                seconds_per_block: 60,
                mapped_to: None,
                is_testnet: false,
            })
            .expect("add coin for beta");

        let reloaded = CoinRegistry::with_dynamic_store(path.clone());
        set_active_account(&reloaded, "account_alpha");
        assert!(reloaded.find_by_id("iSampleCurrency", false).is_some());
        assert!(reloaded.find_by_id("iOtherCurrency", false).is_none());

        set_active_account(&reloaded, "account_beta");
        assert!(reloaded.find_by_id("iSampleCurrency", false).is_none());
        assert!(reloaded.find_by_id("iOtherCurrency", false).is_some());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn legacy_global_dynamic_store_format_is_ignored() {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lite_wallet_coin_registry_legacy_format_{}.json",
            unique_suffix
        ));

        let payload = serde_json::to_string_pretty(&vec![sample_dynamic_coin()])
            .expect("serialize legacy payload");
        std::fs::write(&path, payload).expect("write legacy payload");

        let reloaded = CoinRegistry::with_dynamic_store(path.clone());
        set_active_account(&reloaded, "account_alpha");
        assert!(reloaded.find_by_id("iSampleCurrency", false).is_none());

        let _ = std::fs::remove_file(path);
    }
}
