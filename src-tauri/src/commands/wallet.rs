//
// Tauri command handlers for wallet operations
// Security: Thin wrappers that validate inputs and delegate to core logic
// Last Updated: Module 10 — unlock/session and update-engine start are decoupled

use secp256k1::SecretKey;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, State};
use tokio::sync::Mutex;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::core::address_book::manager as address_book_manager;
use crate::core::auth::{
    capture_active_wallet_access_context, clear_wallet_session_if_current,
    ensure_active_wallet_session, load_primary_secret_material_for_context,
    spawn_session_expiry_monitor, stronghold_store::ACTIVE_ASSETS_PROFILE_VERSION, SessionManager,
};
use crate::core::channels::btc::BtcProviderPool;
use crate::core::channels::dlight_private;
use crate::core::channels::eth::config::metadata_for_network as eth_metadata_for_network;
use crate::core::channels::eth::EthProviderPool;
use crate::core::channels::vrpc::{ConfiguredVrpcSystem, VrpcProviderPool};
use crate::core::coins::Channel;
use crate::core::coins::{CoinDefinition, CoinRegistry};
use crate::core::crypto::wif_encoding::{decode_wif_unchecked_network, encode_btc_wif};
use crate::core::crypto::{
    derive_keys_from_material, derive_public_profile_from_material, Network,
};
use crate::core::identity_display::format_identity_display_name_or_fallback;
use crate::core::updates::UpdateEngineStartConfig;
use crate::core::wallet::{AccountStateStore, WalletManager};
use crate::core::{
    GuardSessionManager, PreflightStore, ProvisioningSignatureStore, StrongholdStore, UpdateEngine,
};
use crate::types::wallet::{DlightSeedSetupMode, ScopeKind, WalletNetwork};
use crate::types::{
    AccountRecord, ActiveAssetsState, ActiveWalletResponse, AddressEndpointKind, AddressResponse,
    AssetPreferencesState, CoinScope, CoinScopesResult, CreateWalletRequest, CreateWalletResult,
    DlightProverFileStatusResult, DlightProverStatusResult, DlightRecoverySecretKind,
    DlightRuntimeStatusResult, DlightSeedStatusResult, GenerateMnemonicRequest,
    ImportWalletTextRequest, LinkedIdentity, MnemonicResult, RecoverySecretKind,
    SetupDlightSeedRequest, SetupDlightSeedResult, WalletError, WalletListItem,
    WalletRecoverySecretsResult, WalletSecretKind,
};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct StartUpdateEngineRequest {
    pub include_transactions: Option<bool>,
    pub priority_coin_ids: Option<Vec<String>>,
    pub priority_channel_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
struct VrpcSystemDescriptor {
    system_id: String,
    system_ticker: String,
    system_display_name: String,
    is_root: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VrpcScopeAddress {
    address: String,
    address_label: String,
    is_primary_address: bool,
    is_read_only: bool,
}

#[derive(Debug, Clone)]
struct ActiveWalletState {
    account_id: String,
    network: WalletNetwork,
    addresses: (String, String, String),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDiscoveryHolding {
    asset_key: String,
    currency_id: String,
    system_id: String,
    system_ticker: String,
    system_display_name: String,
    balance: String,
    includes_read_only: bool,
    balance_status: String,
    coin: Option<CoinDefinition>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDiscoverySource {
    system_id: String,
    system_ticker: String,
    system_display_name: String,
    status: String,
    checked_scope_count: usize,
    unchecked_scope_count: usize,
    includes_read_only: bool,
    private_scope_checked: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDiscoveryResult {
    network: WalletNetwork,
    holdings: Vec<AssetDiscoveryHolding>,
    sources: Vec<AssetDiscoverySource>,
    scope_metadata_complete: bool,
}

struct NewAccountPersistenceRequest<'a> {
    wallet_name: &'a str,
    password: &'a str,
    network: WalletNetwork,
    emoji: &'a str,
    color: &'a str,
    secret_kind: WalletSecretKind,
    secret_material: &'a str,
    initial_dlight_secret: Option<&'a str>,
}

const VRSC_MAINNET_SYSTEM_ID: &str = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";
const VRSC_TESTNET_SYSTEM_ID: &str = "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq";
const VETH_SYSTEM_ID: &str = "i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X";
const MAINNET_DEFAULT_ACTIVE_COIN_IDS: &[&str] =
    &["VRSC", "ETH", "BTC", "i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd"];
const TESTNET_DEFAULT_ACTIVE_COIN_IDS: &[&str] = &["VRSCTEST", "GETH", "BTCTEST"];

async fn resolve_current_password_hash(
    account: &AccountRecord,
    password: &str,
    stronghold_store: &StrongholdStore,
    wallet_manager: &WalletManager,
    account_state_store: &AccountStateStore,
) -> Result<zeroize::Zeroizing<Vec<u8>>, WalletError> {
    stronghold_store
        .ensure_account_password_hash(account, password, wallet_manager, account_state_store)
        .await
}

async fn stronghold_store_for_session(
    session_manager: &Arc<Mutex<SessionManager>>,
) -> StrongholdStore {
    let session = session_manager.lock().await;
    session.stronghold_store().clone()
}

async fn capture_active_wallet_state(
    session_manager: &Arc<Mutex<SessionManager>>,
) -> Result<ActiveWalletState, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    let account_id = session
        .active_account_id()
        .cloned()
        .ok_or(WalletError::WalletLocked)?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    let addresses = session.get_addresses()?;

    Ok(ActiveWalletState {
        account_id,
        network,
        addresses,
    })
}

fn ensure_expected_session_id(
    active_session_id: &str,
    expected_session_id: &str,
) -> Result<(), WalletError> {
    if active_session_id == expected_session_id {
        Ok(())
    } else {
        Err(WalletError::WalletSessionChanged)
    }
}

fn validate_new_wallet_password(password: &str) -> Result<(), WalletError> {
    if password.trim().is_empty() {
        return Err(WalletError::InvalidPassword);
    }

    if password.len() < 7 {
        return Err(WalletError::PasswordTooShort);
    }

    Ok(())
}

fn current_unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn persist_account_metadata(
    wallet_manager: &WalletManager,
    wallet_name: &str,
    account: &AccountRecord,
) -> Result<(), WalletError> {
    let metadata_path = wallet_manager.get_metadata_path(wallet_name)?;
    let metadata_json = serde_json::to_string_pretty(account)?;
    std::fs::write(metadata_path, metadata_json).map_err(|_| WalletError::OperationFailed)
}

async fn persist_new_account(
    request: NewAccountPersistenceRequest<'_>,
    wallet_manager: &WalletManager,
    account_state_store: &AccountStateStore,
    session_manager: &Arc<Mutex<SessionManager>>,
) -> Result<CreateWalletResult, WalletError> {
    let account_id = Uuid::new_v4().to_string();
    let stronghold_store = stronghold_store_for_session(session_manager).await;
    let password_hash = stronghold_store
        .derive_current_password_hash_async(request.password, true)
        .await?;

    stronghold_store
        .store_seed(&account_id, request.secret_material, password_hash.as_ref())
        .await?;

    if let Some(dlight_seed) = request.initial_dlight_secret {
        stronghold_store
            .store_dlight_seed(
                &account_id,
                password_hash.as_ref(),
                request.network,
                Some(dlight_seed),
            )
            .await?;
    }

    account_state_store.store_active_assets(
        &account_id,
        request.network,
        false,
        ACTIVE_ASSETS_PROFILE_VERSION,
        &[],
    )?;

    let account = AccountRecord {
        id: account_id.clone(),
        account_hash: hash_account_id(&account_id),
        key_derivation_version: stronghold_store.current_key_derivation_version(),
        created_at: current_unix_timestamp_secs(),
        last_unlocked_at: None,
        network: request.network,
        emoji: request.emoji.to_string(),
        color: request.color.to_string(),
        secret_kind: request.secret_kind,
    };

    persist_account_metadata(wallet_manager, request.wallet_name, &account)?;

    Ok(CreateWalletResult {
        wallet_id: account_id,
        success: true,
    })
}

fn coin_supports_channel(coin: &CoinDefinition, channel: Channel) -> bool {
    coin.compatible_channels.iter().any(|item| *item == channel)
}

fn network_root_system_id(network: WalletNetwork) -> &'static str {
    if matches!(network, WalletNetwork::Testnet) {
        VRSC_TESTNET_SYSTEM_ID
    } else {
        VRSC_MAINNET_SYSTEM_ID
    }
}

fn coin_expands_vrpc_system_scope(coin: &CoinDefinition) -> bool {
    coin_supports_channel(coin, Channel::Vrpc)
        && coin.currency_id.eq_ignore_ascii_case(&coin.system_id)
}

fn normalize_watched_vrpc_address(address: &str, network: WalletNetwork) -> Option<String> {
    address_book_manager::normalize_destination_address(AddressEndpointKind::Vrpc, address, network)
        .ok()
}

fn dedupe_preserve_order(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::<String>::new();

    for value in values {
        let key = value.trim().to_ascii_lowercase();
        if key.is_empty() || !seen.insert(key) {
            continue;
        }
        out.push(value);
    }

    out
}

fn normalize_identity_scope_label(linked_identity: &LinkedIdentity) -> String {
    format_identity_display_name_or_fallback(
        linked_identity.fully_qualified_name.as_deref(),
        linked_identity.name.as_deref(),
        &linked_identity.identity_address,
    )
}

fn collect_vrpc_scope_addresses(
    primary_vrpc_address: &str,
    linked_identities: &[LinkedIdentity],
    watched_addresses: &[String],
    network: WalletNetwork,
) -> Vec<VrpcScopeAddress> {
    let mut scope_addresses = Vec::<VrpcScopeAddress>::new();
    let mut seen = HashSet::<String>::new();

    let normalized_primary = normalize_watched_vrpc_address(primary_vrpc_address, network)
        .unwrap_or_else(|| primary_vrpc_address.to_string());
    seen.insert(normalized_primary.to_ascii_lowercase());
    scope_addresses.push(VrpcScopeAddress {
        address: normalized_primary.clone(),
        address_label: normalized_primary.clone(),
        is_primary_address: true,
        is_read_only: false,
    });

    let mut linked_scope_candidates = linked_identities
        .iter()
        .filter_map(|linked_identity| {
            let normalized_address =
                normalize_watched_vrpc_address(&linked_identity.identity_address, network)?;
            Some((
                normalized_address,
                normalize_identity_scope_label(linked_identity),
            ))
        })
        .collect::<Vec<_>>();
    linked_scope_candidates.sort_by(|left, right| {
        left.1
            .to_ascii_lowercase()
            .cmp(&right.1.to_ascii_lowercase())
            .then(
                left.0
                    .to_ascii_lowercase()
                    .cmp(&right.0.to_ascii_lowercase()),
            )
    });

    for (address, label) in linked_scope_candidates {
        let key = address.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }

        scope_addresses.push(VrpcScopeAddress {
            address: address.clone(),
            address_label: label,
            is_primary_address: false,
            is_read_only: false,
        });
    }

    let mut watched = dedupe_preserve_order(
        watched_addresses
            .iter()
            .filter_map(|address| normalize_watched_vrpc_address(address, network))
            .filter(|address| !address.eq_ignore_ascii_case(&normalized_primary))
            .collect(),
    );
    watched.sort_by(|left, right| left.to_ascii_lowercase().cmp(&right.to_ascii_lowercase()));

    for address in watched {
        let key = address.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        scope_addresses.push(VrpcScopeAddress {
            address: address.clone(),
            address_label: address,
            is_primary_address: false,
            is_read_only: true,
        });
    }

    scope_addresses
}

fn canonical_coin_id_lookup(
    coin_registry: &CoinRegistry,
    network: WalletNetwork,
) -> HashMap<String, String> {
    let is_testnet = matches!(network, WalletNetwork::Testnet);
    let mut lookup = HashMap::<String, String>::new();

    for coin in coin_registry
        .get_all()
        .into_iter()
        .filter(|coin| coin.is_testnet == is_testnet)
    {
        let key = coin.id.trim().to_ascii_lowercase();
        if key.is_empty() {
            continue;
        }
        lookup.entry(key).or_insert(coin.id);
    }

    lookup
}

fn sanitize_active_coin_ids(
    coin_registry: &CoinRegistry,
    network: WalletNetwork,
    coin_ids: &[String],
) -> Vec<String> {
    let lookup = canonical_coin_id_lookup(coin_registry, network);
    let mut seen = HashSet::<String>::new();
    let mut sanitized = Vec::<String>::new();

    for coin_id in coin_ids {
        let normalized = coin_id.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            continue;
        }

        let Some(canonical_id) = lookup.get(&normalized) else {
            continue;
        };
        let canonical_key = canonical_id.to_ascii_lowercase();
        if !seen.insert(canonical_key) {
            continue;
        }

        sanitized.push(canonical_id.clone());
    }

    sanitized
}

fn sanitize_hidden_asset_keys(asset_keys: &[String]) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    asset_keys
        .iter()
        .filter_map(|asset_key| {
            let normalized = asset_key.trim().to_ascii_lowercase();
            if normalized.is_empty()
                || normalized.len() > 256
                || !normalized.contains(':')
                || !seen.insert(normalized.clone())
            {
                return None;
            }
            Some(normalized)
        })
        .take(512)
        .collect()
}

fn default_active_coin_ids(network: WalletNetwork) -> Vec<String> {
    match network {
        WalletNetwork::Mainnet => MAINNET_DEFAULT_ACTIVE_COIN_IDS
            .iter()
            .map(|coin_id| (*coin_id).to_string())
            .collect(),
        WalletNetwork::Testnet => TESTNET_DEFAULT_ACTIVE_COIN_IDS
            .iter()
            .map(|coin_id| (*coin_id).to_string())
            .collect(),
    }
}

fn canonical_network_label_for_system(system_id: &str) -> Option<(String, String)> {
    if system_id.eq_ignore_ascii_case(VETH_SYSTEM_ID) {
        return Some(("ETH".to_string(), "Ethereum".to_string()));
    }
    if system_id.eq_ignore_ascii_case(VRSC_MAINNET_SYSTEM_ID) {
        return Some(("VRSC".to_string(), "Verus".to_string()));
    }
    if system_id.eq_ignore_ascii_case(VRSC_TESTNET_SYSTEM_ID) {
        return Some(("VRSCTEST".to_string(), "Verus Testnet".to_string()));
    }

    None
}

fn collect_vrpc_system_descriptors(
    coin_registry: &CoinRegistry,
    network: WalletNetwork,
    root_coin: &CoinDefinition,
    configured_systems: &[ConfiguredVrpcSystem],
) -> Vec<VrpcSystemDescriptor> {
    let root_system_id = network_root_system_id(network);
    let is_testnet = matches!(network, WalletNetwork::Testnet);
    let mut allowed_systems = HashMap::<String, VrpcSystemDescriptor>::new();
    let mut insert_system = |system_id: &str, ticker: &str, display_name: &str| {
        let trimmed = system_id.trim();
        if trimmed.is_empty() {
            return;
        }
        allowed_systems
            .entry(trimmed.to_ascii_lowercase())
            .or_insert_with(|| VrpcSystemDescriptor {
                system_id: trimmed.to_string(),
                system_ticker: ticker.trim().to_string(),
                system_display_name: display_name.trim().to_string(),
                is_root: trimmed.eq_ignore_ascii_case(root_system_id),
            });
    };

    for system in configured_systems {
        insert_system(
            &system.system_id,
            &system.system_ticker,
            &system.system_display_name,
        );
    }
    if coin_expands_vrpc_system_scope(root_coin) {
        insert_system(&root_coin.system_id, "", "");
    }
    insert_system(root_system_id, "", "");

    let vrpc_network_coins = coin_registry
        .get_all()
        .into_iter()
        .filter(|coin| coin.is_testnet == is_testnet && coin_supports_channel(coin, Channel::Vrpc))
        .collect::<Vec<_>>();

    let mut systems = allowed_systems
        .into_values()
        .map(|mut descriptor| {
            let native_system_definition = vrpc_network_coins.iter().find(|coin| {
                coin.system_id.eq_ignore_ascii_case(&descriptor.system_id)
                    && coin.currency_id.eq_ignore_ascii_case(&descriptor.system_id)
            });

            let system_id = native_system_definition
                .map(|coin| coin.system_id.clone())
                .unwrap_or_else(|| descriptor.system_id.clone());

            let (system_ticker, system_display_name) = if let Some((ticker, display_name)) =
                canonical_network_label_for_system(&system_id)
            {
                (ticker, display_name)
            } else if !descriptor.system_ticker.is_empty()
                && !descriptor.system_display_name.is_empty()
            {
                (
                    descriptor.system_ticker.clone(),
                    descriptor.system_display_name.clone(),
                )
            } else if let Some(system_coin) = native_system_definition {
                let ticker = system_coin.display_ticker.trim();
                let display_name = system_coin.display_name.trim();

                (
                    if ticker.is_empty() {
                        system_id.clone()
                    } else {
                        ticker.to_string()
                    },
                    if display_name.is_empty() {
                        system_id.clone()
                    } else {
                        display_name.to_string()
                    },
                )
            } else {
                // Never fall back to arbitrary token labels for network names.
                (system_id.clone(), system_id.clone())
            };

            descriptor.system_id = system_id.clone();
            descriptor.system_ticker = system_ticker;
            descriptor.system_display_name = system_display_name;
            descriptor.is_root = system_id.eq_ignore_ascii_case(root_system_id);
            descriptor
        })
        .collect::<Vec<_>>();

    systems.sort_by(|left, right| {
        right
            .is_root
            .cmp(&left.is_root)
            .then(
                left.system_ticker
                    .to_ascii_lowercase()
                    .cmp(&right.system_ticker.to_ascii_lowercase()),
            )
            .then(
                left.system_id
                    .to_ascii_lowercase()
                    .cmp(&right.system_id.to_ascii_lowercase()),
            )
    });
    systems
}

#[derive(Debug, Clone)]
struct DiscoveredVrpcAmount {
    currency_id: String,
    amount: f64,
    includes_read_only: bool,
    partial: bool,
}

fn numeric_value(value: &Value) -> Option<f64> {
    let parsed = value
        .as_f64()
        .or_else(|| value.as_i64().map(|number| number as f64))
        .or_else(|| value.as_u64().map(|number| number as f64))
        .or_else(|| value.as_str().and_then(|number| number.trim().parse().ok()))?;
    parsed.is_finite().then_some(parsed)
}

fn vrpc_balance_entries(raw: &Value, system_id: &str) -> HashMap<String, (String, f64)> {
    let mut entries = HashMap::<String, (String, f64)>::new();
    if let Some(native_satoshis) = raw.get("balance").and_then(numeric_value) {
        let amount = native_satoshis / 100_000_000.0;
        if amount > 0.0 {
            entries.insert(
                system_id.to_ascii_lowercase(),
                (system_id.to_string(), amount),
            );
        }
    }

    if let Some(currency_balances) = raw.get("currencybalance").and_then(Value::as_object) {
        for (currency_id, value) in currency_balances {
            let Some(amount) = numeric_value(value) else {
                continue;
            };
            if amount <= 0.0 {
                continue;
            }
            let key = currency_id.to_ascii_lowercase();
            entries
                .entry(key)
                .and_modify(|entry| entry.1 += amount)
                .or_insert_with(|| (currency_id.clone(), amount));
        }
    }

    entries
}

fn format_discovered_amount(amount: f64) -> String {
    let formatted = format!("{amount:.8}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn merge_discovered_vrpc_amounts(
    target: &mut HashMap<String, DiscoveredVrpcAmount>,
    entries: HashMap<String, (String, f64)>,
    includes_read_only: bool,
    partial: bool,
) {
    for (key, (currency_id, amount)) in entries {
        target
            .entry(key)
            .and_modify(|entry| {
                entry.amount += amount;
                entry.includes_read_only |= includes_read_only;
                entry.partial |= partial;
            })
            .or_insert(DiscoveredVrpcAmount {
                currency_id,
                amount,
                includes_read_only,
                partial,
            });
    }
}

fn registry_coin_for_currency(
    coin_registry: &CoinRegistry,
    network: WalletNetwork,
    currency_id: &str,
) -> Option<CoinDefinition> {
    let is_testnet = matches!(network, WalletNetwork::Testnet);
    coin_registry.get_all().into_iter().find(|coin| {
        coin.is_testnet == is_testnet
            && coin_supports_channel(coin, Channel::Vrpc)
            && coin.currency_id.eq_ignore_ascii_case(currency_id)
    })
}

fn channel_id_for_non_vrpc_coin(coin: &CoinDefinition) -> Option<String> {
    if coin_supports_channel(coin, Channel::Btc) {
        return Some(format!("btc.{}", coin.id));
    }
    if coin_supports_channel(coin, Channel::Eth) {
        return Some(format!("eth.{}", coin.id));
    }
    if coin_supports_channel(coin, Channel::Erc20) {
        return Some(format!("erc20.{}", coin.id));
    }
    None
}

fn canonical_non_vrpc_network_metadata(
    coin: &CoinDefinition,
    network: WalletNetwork,
) -> (String, String, String) {
    if coin_supports_channel(coin, Channel::Btc) {
        return match network {
            WalletNetwork::Mainnet => ("BTC".to_string(), "BTC".to_string(), "Bitcoin".to_string()),
            WalletNetwork::Testnet => (
                "BTCTEST".to_string(),
                "BTCTEST".to_string(),
                "Bitcoin Testnet".to_string(),
            ),
        };
    }
    if coin_supports_channel(coin, Channel::Eth) || coin_supports_channel(coin, Channel::Erc20) {
        return match network {
            WalletNetwork::Mainnet => {
                ("ETH".to_string(), "ETH".to_string(), "Ethereum".to_string())
            }
            WalletNetwork::Testnet => (
                "GETH".to_string(),
                "ETH".to_string(),
                eth_metadata_for_network(network)
                    .native_asset_name
                    .to_string(),
            ),
        };
    }

    (
        coin.system_id.clone(),
        coin.display_ticker.clone(),
        coin.display_name.clone(),
    )
}

fn address_for_non_vrpc_coin(
    coin: &CoinDefinition,
    addresses: &(String, String, String),
) -> String {
    if coin_supports_channel(coin, Channel::Btc) {
        return addresses.2.clone();
    }
    if coin_supports_channel(coin, Channel::Eth) || coin_supports_channel(coin, Channel::Erc20) {
        return addresses.1.clone();
    }
    addresses.0.clone()
}

/// Generate a new BIP39 mnemonic phrase
#[tauri::command(rename_all = "snake_case")]
pub async fn generate_mnemonic(
    request: GenerateMnemonicRequest,
    wallet_manager: State<'_, WalletManager>,
) -> Result<MnemonicResult, WalletError> {
    // Input validation
    if request.word_count != 24 {
        return Err(WalletError::InvalidSeedPhrase);
    }

    println!(
        "[WALLET] Generate mnemonic requested: {} words",
        request.word_count
    );

    // Delegate to core logic
    let seed_phrase = wallet_manager.generate_mnemonic(request.word_count).await?;

    println!("[WALLET] Mnemonic generation completed");

    Ok(MnemonicResult { seed_phrase })
}

/// Validate a BIP39 mnemonic phrase
#[tauri::command(rename_all = "snake_case")]
pub async fn validate_mnemonic(
    seed_phrase: String,
    wallet_manager: State<'_, WalletManager>,
) -> Result<bool, WalletError> {
    // Basic input validation
    if seed_phrase.trim().is_empty() {
        return Ok(false);
    }

    println!("[WALLET] Mnemonic validation requested");

    // Delegate to core logic
    let is_valid = wallet_manager.validate_mnemonic(&seed_phrase).await?;

    println!("[WALLET] Mnemonic validation completed: {}", is_valid);

    Ok(is_valid)
}

/// Get the BIP39 English word list used for mnemonic entry suggestions.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_mnemonic_wordlist(
    wallet_manager: State<'_, WalletManager>,
) -> Result<Vec<String>, WalletError> {
    wallet_manager.get_mnemonic_wordlist().await
}

fn normalize_hex_private_key_candidate(input: &str) -> Option<String> {
    let stripped = input
        .strip_prefix("0x")
        .or_else(|| input.strip_prefix("0X"))
        .unwrap_or(input);
    if stripped.len() != 64 || !stripped.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let decoded = hex::decode(stripped).ok()?;
    if decoded.len() != 32 {
        return None;
    }
    SecretKey::from_slice(&decoded).ok()?;
    Some(stripped.to_lowercase())
}

fn classify_import_text(import_text: &str) -> Result<(WalletSecretKind, String), WalletError> {
    let trimmed = import_text.trim();
    if trimmed.is_empty() {
        return Err(WalletError::InvalidImportText);
    }

    if decode_wif_unchecked_network(trimmed).is_ok() {
        return Ok((WalletSecretKind::Wif, trimmed.to_string()));
    }

    if let Some(private_key_hex) = normalize_hex_private_key_candidate(trimmed) {
        return Ok((WalletSecretKind::PrivateKeyHex, private_key_hex));
    }

    // Parity behavior with valu-mobile: any remaining non-empty input is treated as seed text.
    Ok((WalletSecretKind::SeedText, trimmed.to_string()))
}

async fn is_valid_24_word_mnemonic(
    wallet_manager: &WalletManager,
    seed: &str,
) -> Result<bool, WalletError> {
    let normalized = seed.trim();
    if normalized.is_empty() {
        return Ok(false);
    }
    if normalized.split_whitespace().count() != 24 {
        return Ok(false);
    }
    wallet_manager.validate_mnemonic(normalized).await
}

fn is_probable_dlight_spending_key(seed: &str) -> bool {
    let normalized = seed.trim();
    normalized.starts_with("secret-extended-key-")
}

fn crypto_network_for_wallet(network: WalletNetwork) -> Network {
    match network {
        WalletNetwork::Mainnet => Network::Mainnet,
        WalletNetwork::Testnet => Network::Testnet,
    }
}

fn recovery_secret_kind_from_wallet_secret_kind(
    secret_kind: WalletSecretKind,
) -> RecoverySecretKind {
    match secret_kind {
        WalletSecretKind::SeedText => RecoverySecretKind::SeedText,
        WalletSecretKind::Wif => RecoverySecretKind::Wif,
        WalletSecretKind::PrivateKeyHex => RecoverySecretKind::PrivateKeyHex,
    }
}

fn dlight_recovery_secret_kind_from_seed(seed: &str) -> DlightRecoverySecretKind {
    if is_probable_dlight_spending_key(seed) {
        DlightRecoverySecretKind::SpendingKey
    } else {
        DlightRecoverySecretKind::Mnemonic
    }
}

/// Create a new wallet with Stronghold encryption
#[tauri::command(rename_all = "snake_case")]
pub async fn create_wallet(
    request: CreateWalletRequest,
    password: String,
    wallet_manager: State<'_, WalletManager>,
    account_state_store: State<'_, AccountStateStore>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<CreateWalletResult, WalletError> {
    request.validate()?;
    validate_new_wallet_password(&password)?;

    println!("[WALLET] Create wallet requested: {}", request.wallet_name);

    if wallet_manager.wallet_exists(&request.wallet_name).await? {
        return Err(WalletError::WalletExists);
    }

    let result = persist_new_account(
        NewAccountPersistenceRequest {
            wallet_name: &request.wallet_name,
            password: &password,
            network: request.network,
            emoji: &request.emoji,
            color: &request.color,
            secret_kind: WalletSecretKind::SeedText,
            secret_material: &request.seed_phrase,
            initial_dlight_secret: request
                .setup_dlight_with_primary
                .then_some(request.seed_phrase.as_str()),
        },
        wallet_manager.inner(),
        account_state_store.inner(),
        session_manager.inner(),
    )
    .await?;

    println!("[WALLET] Wallet created successfully: {}", result.wallet_id);
    Ok(result)
}

/// Import wallet from pasted private key or seed text.
#[tauri::command(rename_all = "snake_case")]
pub async fn import_wallet_text(
    request: ImportWalletTextRequest,
    password: String,
    wallet_manager: State<'_, WalletManager>,
    account_state_store: State<'_, AccountStateStore>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<CreateWalletResult, WalletError> {
    request.validate()?;
    validate_new_wallet_password(&password)?;

    println!(
        "[WALLET] Import wallet text requested: {}",
        request.wallet_name
    );

    if wallet_manager.wallet_exists(&request.wallet_name).await? {
        return Err(WalletError::WalletExists);
    }

    let (secret_kind, secret_material) = classify_import_text(&request.import_text)?;

    let result = persist_new_account(
        NewAccountPersistenceRequest {
            wallet_name: &request.wallet_name,
            password: &password,
            network: request.network,
            emoji: &request.emoji,
            color: &request.color,
            secret_kind,
            secret_material: &secret_material,
            initial_dlight_secret: None,
        },
        wallet_manager.inner(),
        account_state_store.inner(),
        session_manager.inner(),
    )
    .await?;

    println!(
        "[WALLET] Wallet text import created account: {}",
        result.wallet_id
    );
    Ok(result)
}

/// Unlock wallet with password.
#[tauri::command(rename_all = "snake_case")]
pub async fn unlock_wallet(
    account_id: String,
    password: String,
    wallet_manager: State<'_, WalletManager>,
    account_state_store: State<'_, AccountStateStore>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    guard_session_manager: State<'_, Arc<Mutex<GuardSessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    provisioning_signature_store: State<'_, ProvisioningSignatureStore>,
    update_engine: State<'_, Arc<UpdateEngine>>,
    app_handle: AppHandle,
) -> Result<(), WalletError> {
    println!("[WALLET] Unlock wallet requested");
    let unlock_started_at = Instant::now();

    let account_lookup_started_at = Instant::now();
    let wallet = wallet_manager
        .get_account_record_by_account_id(&account_id)
        .await?
        .ok_or(WalletError::OperationFailed)?;
    println!(
        "[WALLET_PERF] unlock phase=account_record elapsed_ms={}",
        account_lookup_started_at.elapsed().as_millis()
    );

    let stronghold_store = stronghold_store_for_session(session_manager.inner()).await;

    let kdf_started_at = Instant::now();
    let password_hash = resolve_current_password_hash(
        &wallet,
        &password,
        &stronghold_store,
        wallet_manager.inner(),
        account_state_store.inner(),
    )
    .await;
    let password_hash = match password_hash {
        Ok(value) => value,
        Err(err) => {
            println!("[WALLET] Unlock failed during KDF resolution: {:?}", err);
            return Err(err);
        }
    };
    println!(
        "[WALLET_PERF] unlock phase=password_kdf elapsed_ms={}",
        kdf_started_at.elapsed().as_millis()
    );
    let seed_snapshot_started_at = Instant::now();
    let seed = match stronghold_store
        .load_seed(&account_id, password_hash.as_ref())
        .await
    {
        Ok(value) => Zeroizing::new(value),
        Err(err) => {
            println!("[WALLET] Unlock failed while loading seed: {:?}", err);
            return Err(err);
        }
    };
    println!(
        "[WALLET_PERF] unlock phase=seed_snapshot elapsed_ms={}",
        seed_snapshot_started_at.elapsed().as_millis()
    );

    let public_profile_started_at = Instant::now();
    let secret_kind = wallet.secret_kind;
    let crypto_network = crypto_network_for_wallet(wallet.network);
    let public_profile = tokio::task::spawn_blocking(move || {
        derive_public_profile_from_material(seed.as_str(), secret_kind, crypto_network)
            .map_err(|_| WalletError::OperationFailed)
    })
    .await
    .map_err(|error| {
        println!("[WALLET] Public-profile worker failed: {}", error);
        WalletError::OperationFailed
    })??;
    println!(
        "[WALLET_PERF] unlock phase=public_profile elapsed_ms={}",
        public_profile_started_at.elapsed().as_millis()
    );

    update_engine.stop().await;
    clear_wallet_session_if_current(
        None,
        session_manager.inner(),
        guard_session_manager.inner(),
        preflight_store.inner(),
        provisioning_signature_store.inner(),
        coin_registry.inner(),
    )
    .await;

    let mut session = session_manager.lock().await;
    let session_id = session.unlock_with_profile(
        account_id.clone(),
        wallet.network,
        wallet.secret_kind,
        public_profile,
        password_hash,
    );
    preflight_store.activate_wallet_session(&session_id);
    provisioning_signature_store.activate_wallet_session(&session_id);
    coin_registry.set_active_account(Some(account_id.clone()));
    let expiry_registration = session.expiry_monitor_registration()?;
    spawn_session_expiry_monitor(
        app_handle,
        expiry_registration,
        session_manager.inner().clone(),
        guard_session_manager.inner().clone(),
        preflight_store.inner().clone(),
        provisioning_signature_store.inner().clone(),
        coin_registry.inner().clone(),
    );
    drop(session);
    if let Err(error) = wallet_manager.mark_wallet_last_unlocked(&account_id).await {
        println!(
            "[WALLET] Failed to persist last-unlocked wallet metadata: {:?}",
            error
        );
    }
    println!(
        "[WALLET] Wallet unlocked successfully total_ms={}",
        unlock_started_at.elapsed().as_millis()
    );
    Ok(())
}

/// Start update engine polling after frontend event listeners are registered.
#[tauri::command(rename_all = "snake_case")]
pub async fn start_update_engine(
    request: Option<StartUpdateEngineRequest>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    btc_provider_pool: State<'_, Arc<BtcProviderPool>>,
    eth_provider_pool: State<'_, Arc<EthProviderPool>>,
    update_engine: State<'_, Arc<UpdateEngine>>,
    app_handle: AppHandle,
) -> Result<(), WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    let session_id = session
        .active_session_id()
        .ok_or(WalletError::WalletLocked)?
        .to_string();
    drop(session);

    let request = request.unwrap_or_default();
    let start_config = UpdateEngineStartConfig {
        poll_transactions: request.include_transactions.unwrap_or(false),
        priority_coin_ids: request.priority_coin_ids.unwrap_or_default(),
        priority_channel_ids: request.priority_channel_ids.unwrap_or_default(),
    };

    update_engine
        .start(
            app_handle,
            session_id,
            session_manager.inner().clone(),
            coin_registry.inner().clone(),
            vrpc_provider_pool.inner().clone(),
            btc_provider_pool.inner().clone(),
            eth_provider_pool.inner().clone(),
            start_config,
        )
        .await;

    Ok(())
}

/// Lock wallet and zeroize keys. Stops update engine, clears preflight store.
#[tauri::command(rename_all = "snake_case")]
pub async fn lock_wallet(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    guard_session_manager: State<'_, Arc<Mutex<GuardSessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    provisioning_signature_store: State<'_, ProvisioningSignatureStore>,
    update_engine: State<'_, Arc<UpdateEngine>>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
) -> Result<(), WalletError> {
    println!("[WALLET] Lock wallet requested");

    update_engine.stop().await;
    clear_wallet_session_if_current(
        None,
        session_manager.inner(),
        guard_session_manager.inner(),
        preflight_store.inner(),
        provisioning_signature_store.inner(),
        coin_registry.inner(),
    )
    .await;

    println!("[WALLET] Wallet locked successfully");
    Ok(())
}

/// Get derived addresses for active account
#[tauri::command(rename_all = "snake_case")]
pub async fn get_addresses(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<AddressResponse, WalletError> {
    println!("[WALLET] Get addresses requested");

    let session = session_manager.lock().await;
    let (vrsc_address, eth_address, btc_address) = session.get_addresses()?;

    println!("[WALLET] Addresses retrieved");
    Ok(AddressResponse {
        vrsc_address,
        eth_address,
        btc_address,
    })
}

/// Check if wallet is unlocked
#[tauri::command(rename_all = "snake_case")]
pub async fn is_unlocked(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<bool, WalletError> {
    let session = session_manager.lock().await;
    Ok(session.is_unlocked())
}

/// Returns the active wallet session timeout in minutes.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_session_timeout_minutes(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<u64, WalletError> {
    let session = session_manager.lock().await;
    Ok(session.timeout_minutes())
}

/// Sets the wallet session timeout in minutes, normalized to allowed values.
#[tauri::command(rename_all = "snake_case")]
pub async fn set_session_timeout_minutes(
    minutes: u64,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<u64, WalletError> {
    let mut session = session_manager.lock().await;
    let normalized = session.set_timeout_minutes(minutes);
    Ok(normalized)
}

/// Refreshes the wallet session inactivity timer for explicit user activity.
#[tauri::command(rename_all = "snake_case")]
pub async fn touch_session_activity(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<(), WalletError> {
    let mut session = session_manager.lock().await;
    session.touch_activity()
}

/// Get active wallet display info for dashboard (when unlocked)
#[tauri::command(rename_all = "snake_case")]
pub async fn get_active_wallet(
    wallet_manager: State<'_, WalletManager>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<Option<ActiveWalletResponse>, WalletError> {
    let session = session_manager.lock().await;
    let account_id = match session.active_account_id() {
        Some(id) => id.clone(),
        None => return Ok(None),
    };
    let session_id = session
        .active_session_id()
        .ok_or(WalletError::WalletLocked)?
        .to_string();
    drop(session);

    let wallet_name = wallet_manager.get_wallet_by_account_id(&account_id).await?;

    Ok(wallet_name.map(|w| ActiveWalletResponse {
        wallet_name: w.wallet_name,
        network: w.network,
        emoji: w.emoji,
        color: w.color,
        session_id,
    }))
}

/// Return watched read-only VRPC addresses for the active wallet/network.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_watched_vrpc_addresses(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
) -> Result<Vec<String>, WalletError> {
    let state = capture_active_wallet_state(session_manager.inner()).await?;

    let addresses =
        account_state_store.load_watched_vrpc_addresses(&state.account_id, state.network)?;
    Ok(dedupe_preserve_order(
        addresses
            .into_iter()
            .filter_map(|address| normalize_watched_vrpc_address(&address, state.network))
            .collect(),
    ))
}

/// Persist watched read-only VRPC addresses for the active wallet/network.
#[tauri::command(rename_all = "snake_case")]
pub async fn set_watched_vrpc_addresses(
    addresses: Vec<String>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
) -> Result<Vec<String>, WalletError> {
    const MAX_WATCHED_ADDRESSES: usize = 100;

    let state = capture_active_wallet_state(session_manager.inner()).await?;
    let primary_vrpc_address = &state.addresses.0;

    let mut sanitized = dedupe_preserve_order(
        addresses
            .iter()
            .filter_map(|address| normalize_watched_vrpc_address(address, state.network))
            .filter(|address| !address.eq_ignore_ascii_case(primary_vrpc_address))
            .collect(),
    );
    sanitized.truncate(MAX_WATCHED_ADDRESSES);

    account_state_store.store_watched_vrpc_addresses(
        &state.account_id,
        state.network,
        &sanitized,
    )?;

    Ok(sanitized)
}

/// Return active asset IDs for the active wallet/network.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_active_assets(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
) -> Result<ActiveAssetsState, WalletError> {
    let state = capture_active_wallet_state(session_manager.inner()).await?;

    let (initialized, coin_ids, profile_version) =
        account_state_store.load_active_assets(&state.account_id, state.network)?;
    let sanitized = sanitize_active_coin_ids(coin_registry.as_ref(), state.network, &coin_ids);
    if profile_version < ACTIVE_ASSETS_PROFILE_VERSION {
        let defaults = sanitize_active_coin_ids(
            coin_registry.as_ref(),
            state.network,
            &default_active_coin_ids(state.network),
        );
        account_state_store.store_active_assets(
            &state.account_id,
            state.network,
            true,
            ACTIVE_ASSETS_PROFILE_VERSION,
            &defaults,
        )?;

        return Ok(ActiveAssetsState {
            network: state.network,
            initialized: true,
            coin_ids: defaults,
        });
    }

    Ok(ActiveAssetsState {
        network: state.network,
        initialized,
        coin_ids: sanitized,
    })
}

/// Persist active asset IDs for the active wallet/network.
#[tauri::command(rename_all = "snake_case")]
pub async fn set_active_assets(
    coin_ids: Vec<String>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
) -> Result<ActiveAssetsState, WalletError> {
    let state = capture_active_wallet_state(session_manager.inner()).await?;

    let sanitized = sanitize_active_coin_ids(coin_registry.as_ref(), state.network, &coin_ids);
    account_state_store.store_active_assets(
        &state.account_id,
        state.network,
        true,
        ACTIVE_ASSETS_PROFILE_VERSION,
        &sanitized,
    )?;

    Ok(ActiveAssetsState {
        network: state.network,
        initialized: true,
        coin_ids: sanitized,
    })
}

/// Return portfolio visibility and explicitly hidden assets for the active wallet/network.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_asset_preferences(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
) -> Result<AssetPreferencesState, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    let session_id = session
        .active_session_id()
        .ok_or(WalletError::WalletLocked)?
        .to_string();
    let account_id = session
        .active_account_id()
        .cloned()
        .ok_or(WalletError::WalletLocked)?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    let (_initialized, coin_ids, profile_version, hidden_asset_keys) =
        account_state_store.load_asset_preferences(&account_id, network)?;
    let portfolio_coin_ids = if profile_version < ACTIVE_ASSETS_PROFILE_VERSION {
        sanitize_active_coin_ids(
            coin_registry.as_ref(),
            network,
            &default_active_coin_ids(network),
        )
    } else {
        sanitize_active_coin_ids(coin_registry.as_ref(), network, &coin_ids)
    };
    let hidden_asset_keys = sanitize_hidden_asset_keys(&hidden_asset_keys);

    if profile_version < ACTIVE_ASSETS_PROFILE_VERSION {
        account_state_store.store_asset_preferences(
            &account_id,
            network,
            ACTIVE_ASSETS_PROFILE_VERSION,
            &portfolio_coin_ids,
            &hidden_asset_keys,
        )?;
    }

    Ok(AssetPreferencesState {
        network,
        session_id,
        portfolio_coin_ids,
        hidden_asset_keys,
    })
}

/// Atomically persist portfolio visibility and explicit hiding for the active wallet/network.
#[tauri::command(rename_all = "snake_case")]
pub async fn set_asset_preferences(
    expected_session_id: String,
    portfolio_coin_ids: Vec<String>,
    hidden_asset_keys: Vec<String>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
) -> Result<AssetPreferencesState, WalletError> {
    // Keep the session guard through the synchronous preference write so a
    // replacement unlock cannot cross the validation/mutation boundary.
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    let session_id = session
        .active_session_id()
        .ok_or(WalletError::WalletLocked)?
        .to_string();
    ensure_expected_session_id(&session_id, &expected_session_id)?;
    let account_id = session
        .active_account_id()
        .cloned()
        .ok_or(WalletError::WalletLocked)?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    let portfolio_coin_ids =
        sanitize_active_coin_ids(coin_registry.as_ref(), network, &portfolio_coin_ids);
    let hidden_asset_keys = sanitize_hidden_asset_keys(&hidden_asset_keys);

    account_state_store.store_asset_preferences(
        &account_id,
        network,
        ACTIVE_ASSETS_PROFILE_VERSION,
        &portfolio_coin_ids,
        &hidden_asset_keys,
    )?;

    Ok(AssetPreferencesState {
        network,
        session_id,
        portfolio_coin_ids,
        hidden_asset_keys,
    })
}

/// Discover positive PBaaS holdings through the wallet's configured VRPC providers.
/// This is an on-demand read. It checks transparent wallet, linked identity, and watched
/// address scopes without enabling background polling or returning any addresses.
#[tauri::command(rename_all = "snake_case")]
pub async fn discover_vrpc_assets(
    system_ids: Option<Vec<String>>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<AssetDiscoveryResult, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let account_id = context.account_id.clone();
    let network = context.wallet_network;
    coin_registry.set_active_account(Some(account_id.clone()));

    let watched = account_state_store.load_watched_vrpc_addresses(&account_id, network)?;
    let linked_identities_result = context.load_linked_identities_cached().await;
    let scope_metadata_complete = linked_identities_result.is_ok();
    let linked_identities = linked_identities_result.unwrap_or_default();
    let scope_addresses =
        collect_vrpc_scope_addresses(&context.vrsc_address, &linked_identities, &watched, network);
    let owned_addresses = scope_addresses
        .iter()
        .filter(|scope| !scope.is_read_only)
        .map(|scope| scope.address.clone())
        .collect::<Vec<_>>();
    let read_only_addresses = scope_addresses
        .iter()
        .filter(|scope| scope.is_read_only)
        .map(|scope| scope.address.clone())
        .collect::<Vec<_>>();

    let root_coin_id = if matches!(network, WalletNetwork::Testnet) {
        "VRSCTEST"
    } else {
        "VRSC"
    };
    let root_coin = coin_registry
        .find_by_id(root_coin_id, matches!(network, WalletNetwork::Testnet))
        .ok_or(WalletError::UnsupportedChannel)?;
    let configured_systems = vrpc_provider_pool.configured_systems(network);
    let mut systems = collect_vrpc_system_descriptors(
        coin_registry.as_ref(),
        network,
        &root_coin,
        &configured_systems,
    );
    if let Some(system_ids) = system_ids.filter(|system_ids| !system_ids.is_empty()) {
        let requested = system_ids
            .into_iter()
            .map(|system_id| system_id.trim().to_ascii_lowercase())
            .filter(|system_id| !system_id.is_empty())
            .collect::<HashSet<_>>();
        systems.retain(|system| requested.contains(&system.system_id.to_ascii_lowercase()));
    }

    let mut holdings = Vec::<AssetDiscoveryHolding>::new();
    let mut sources = Vec::<AssetDiscoverySource>::new();

    for system in systems {
        let provider = vrpc_provider_pool.for_system(network, &system.system_id);
        let owned_result = provider.getaddressbalance(&owned_addresses).await;
        let read_only_result = if read_only_addresses.is_empty() {
            None
        } else {
            Some(provider.getaddressbalance(&read_only_addresses).await)
        };
        let owned_ok = owned_result.is_ok();
        let read_only_ok = read_only_result.as_ref().map(Result::is_ok).unwrap_or(true);
        let any_ok = owned_ok
            || read_only_result
                .as_ref()
                .map(Result::is_ok)
                .unwrap_or(false);
        let all_ok = owned_ok && read_only_ok && scope_metadata_complete;
        let source_status = if all_ok {
            "available"
        } else if any_ok {
            "partial"
        } else {
            "unavailable"
        };
        let checked_scope_count = if owned_ok { owned_addresses.len() } else { 0 }
            + if read_only_result
                .as_ref()
                .map(Result::is_ok)
                .unwrap_or(false)
            {
                read_only_addresses.len()
            } else {
                0
            };
        let unchecked_scope_count = scope_addresses.len().saturating_sub(checked_scope_count);

        sources.push(AssetDiscoverySource {
            system_id: system.system_id.clone(),
            system_ticker: system.system_ticker.clone(),
            system_display_name: system.system_display_name.clone(),
            status: source_status.to_string(),
            checked_scope_count,
            unchecked_scope_count,
            includes_read_only: !read_only_addresses.is_empty(),
            private_scope_checked: false,
        });

        let partial = !all_ok;
        let mut discovered = HashMap::<String, DiscoveredVrpcAmount>::new();
        if let Ok(raw) = owned_result {
            merge_discovered_vrpc_amounts(
                &mut discovered,
                vrpc_balance_entries(&raw, &system.system_id),
                false,
                partial,
            );
        }
        if let Some(Ok(raw)) = read_only_result {
            merge_discovered_vrpc_amounts(
                &mut discovered,
                vrpc_balance_entries(&raw, &system.system_id),
                true,
                partial,
            );
        }

        for discovered_amount in discovered.into_values() {
            let coin = if let Some(known) = registry_coin_for_currency(
                coin_registry.as_ref(),
                network,
                &discovered_amount.currency_id,
            ) {
                Some(known)
            } else {
                match provider.getcurrency(&discovered_amount.currency_id).await {
                    Ok(payload) => crate::commands::coins::pbaas_coin_definition_from_payload(
                        &payload,
                        network,
                        vrpc_provider_pool.endpoint_url_for_system(network, &system.system_id),
                    ),
                    Err(_) => None,
                }
            };
            holdings.push(AssetDiscoveryHolding {
                asset_key: format!(
                    "vrsc:{}",
                    discovered_amount.currency_id.to_ascii_lowercase()
                ),
                currency_id: discovered_amount.currency_id,
                system_id: system.system_id.clone(),
                system_ticker: system.system_ticker.clone(),
                system_display_name: system.system_display_name.clone(),
                balance: format_discovered_amount(discovered_amount.amount),
                includes_read_only: discovered_amount.includes_read_only,
                balance_status: if discovered_amount.partial {
                    "partial".to_string()
                } else {
                    "available".to_string()
                },
                coin,
            });
        }
    }

    holdings.sort_by(|left, right| {
        left.asset_key
            .cmp(&right.asset_key)
            .then(left.system_ticker.cmp(&right.system_ticker))
    });

    ensure_active_wallet_session(session_manager.inner(), &context.session_id).await?;

    Ok(AssetDiscoveryResult {
        network,
        holdings,
        sources,
        scope_metadata_complete,
    })
}

/// Returns whether a dlight seed is configured for the active account/network.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_dlight_seed_status(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<DlightSeedStatusResult, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let metadata = context.load_dlight_public_metadata_cached().await?;

    Ok(DlightSeedStatusResult {
        configured: metadata.configured,
        shielded_address: metadata.shielded_address,
    })
}

/// Sets up dlight seed storage for the active account/network.
#[tauri::command(rename_all = "snake_case")]
pub async fn setup_dlight_seed(
    request: SetupDlightSeedRequest,
    wallet_manager: State<'_, WalletManager>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
) -> Result<SetupDlightSeedResult, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let reuse_primary_seed = if matches!(request.mode, DlightSeedSetupMode::ReusePrimary) {
        Some(load_primary_secret_material_for_context(&context).await?)
    } else {
        None
    };

    let mut generated_seed_phrase: Option<String> = None;
    let seed_to_store = match request.mode {
        DlightSeedSetupMode::ReusePrimary => {
            let seed = reuse_primary_seed.ok_or(WalletError::InvalidSeedPhrase)?;
            if !is_valid_24_word_mnemonic(wallet_manager.inner(), seed.as_str()).await? {
                return Err(WalletError::InvalidSeedPhrase);
            }
            seed.to_string()
        }
        DlightSeedSetupMode::CreateNew => {
            let seed = wallet_manager.generate_mnemonic(24).await?;
            generated_seed_phrase = Some(seed.clone());
            seed
        }
        DlightSeedSetupMode::ImportText => {
            let imported = request.import_text.unwrap_or_default().trim().to_string();
            if imported.is_empty() {
                return Err(WalletError::InvalidImportText);
            }
            if is_probable_dlight_spending_key(&imported) {
                imported
            } else if is_valid_24_word_mnemonic(wallet_manager.inner(), &imported).await? {
                imported
            } else {
                return Err(WalletError::InvalidImportText);
            }
        }
    };
    let seed_to_store = Zeroizing::new(seed_to_store);
    let fresh = matches!(request.mode, DlightSeedSetupMode::CreateNew);
    let canonical = dlight_private::runtime_spending_key(&seed_to_store, context.wallet_network)?;
    dlight_private::derive_scope_address(&canonical, context.wallet_network)?;
    let birthday = if fresh {
        let testnet = matches!(context.wallet_network, WalletNetwork::Testnet);
        let coin_id = if testnet { "VRSCTEST" } else { "VRSC" };
        if let Some(endpoint) = coin_registry
            .find_by_id(coin_id, testnet)
            .and_then(|coin| coin.dlight_endpoints)
            .and_then(|endpoints| endpoints.into_iter().next())
        {
            tokio::time::timeout(
                std::time::Duration::from_secs(15),
                dlight_private::creation_birthday(&endpoint, context.wallet_network),
            )
            .await
            .ok()
            .and_then(Result::ok)
        } else {
            None
        }
    } else {
        None
    };
    ensure_active_wallet_session(session_manager.inner(), &context.session_id).await?;
    context
        .stronghold_store
        .store_dlight_seed_with_metadata(
            &context.account_id,
            context.password_hash(),
            context.wallet_network,
            Some(seed_to_store.as_str()),
            dlight_private::DlightSeedMetadata { birthday },
        )
        .await?;

    Ok(SetupDlightSeedResult {
        configured: true,
        generated_seed_phrase,
        requires_relogin: true,
    })
}

/// Returns wallet recovery secrets after an explicit password re-check.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_wallet_recovery_secrets(
    password: String,
    wallet_manager: State<'_, WalletManager>,
    account_state_store: State<'_, AccountStateStore>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<WalletRecoverySecretsResult, WalletError> {
    if password.trim().is_empty() {
        return Err(WalletError::InvalidPassword);
    }

    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let account_id = session
        .active_account_id()
        .cloned()
        .ok_or(WalletError::WalletLocked)?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    let stronghold_store = session.stronghold_store().clone();
    drop(session);

    let account = wallet_manager
        .get_account_record_by_account_id(&account_id)
        .await?
        .ok_or(WalletError::OperationFailed)?;
    let password_hash = resolve_current_password_hash(
        &account,
        &password,
        &stronghold_store,
        wallet_manager.inner(),
        account_state_store.inner(),
    )
    .await?;
    let primary_secret = stronghold_store
        .load_seed(&account_id, password_hash.as_ref())
        .await?;
    let primary_secret_kind = recovery_secret_kind_from_wallet_secret_kind(account.secret_kind);
    let keys = derive_keys_from_material(
        &primary_secret,
        account.secret_kind,
        crypto_network_for_wallet(network),
    )
    .map_err(|_| WalletError::OperationFailed)?;

    let private_bytes =
        decode_wif_unchecked_network(&keys.wif).map_err(|_| WalletError::OperationFailed)?;
    let btc_wif = encode_btc_wif(&private_bytes, crypto_network_for_wallet(network))?;
    let dlight_secret = stronghold_store
        .load_dlight_seed(&account_id, password_hash.as_slice(), network)
        .await?;
    let dlight_runtime = stronghold_store
        .load_dlight_runtime_material(&account_id, password_hash.as_slice(), network)
        .await?;
    let (dlight_secret_kind, dlight_shielded_address, dlight_derived_spending_key) =
        if let Some(secret) = dlight_secret.as_deref() {
            let secret_kind = dlight_recovery_secret_kind_from_seed(secret);
            let shielded_address = dlight_runtime
                .as_ref()
                .and_then(|(key, _)| dlight_private::derive_scope_address(key, network).ok());
            let derived_spending_key = if matches!(secret_kind, DlightRecoverySecretKind::Mnemonic)
            {
                dlight_runtime.as_ref().map(|(key, _)| key.to_string())
            } else {
                None
            };

            (Some(secret_kind), shielded_address, derived_spending_key)
        } else {
            (None, None, None)
        };

    Ok(WalletRecoverySecretsResult {
        primary_secret_kind,
        primary_secret,
        verus_wif: keys.wif.clone(),
        btc_wif,
        eth_private_key: keys.eth_private_key.clone(),
        verus_address: keys.address.clone(),
        btc_address: keys.btc_address.clone(),
        eth_address: keys.eth_address.clone(),
        dlight_secret,
        dlight_secret_kind,
        dlight_shielded_address,
        dlight_derived_spending_key,
    })
}

/// Returns runtime diagnostics for a dlight channel. Intended for support/debug visibility.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_dlight_runtime_status(
    channel_id: String,
    coin_id: Option<String>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
) -> Result<DlightRuntimeStatusResult, WalletError> {
    let diagnostics = crate::core::channels::route_get_dlight_runtime_status(
        &channel_id,
        coin_id.as_deref(),
        session_manager.inner(),
        coin_registry.as_ref(),
    )
    .await?;

    Ok(DlightRuntimeStatusResult {
        channel_id,
        runtime_key: diagnostics.runtime_key,
        status_kind: diagnostics.status_kind,
        percent: diagnostics.percent,
        scanned_height: diagnostics.scanned_height,
        tip_height: diagnostics.tip_height,
        estimated_tip_height: diagnostics.estimated_tip_height,
        syncing: diagnostics.syncing,
        last_updated: diagnostics.last_updated,
        last_progress_at: diagnostics.last_progress_at,
        last_tip_probe_at: diagnostics.last_tip_probe_at,
        consecutive_failures: diagnostics.consecutive_failures,
        scan_rate_blocks_per_sec: diagnostics.scan_rate_blocks_per_sec,
        stalled: diagnostics.stalled,
        last_error: diagnostics.last_error,
        spend_cache_ready: diagnostics.spend_cache_ready,
        spend_cache_status_kind: diagnostics.spend_cache_status_kind,
        spend_cache_percent: diagnostics.spend_cache_percent,
        spend_cache_lag_blocks: diagnostics.spend_cache_lag_blocks,
        spend_cache_last_error: diagnostics.spend_cache_last_error,
        spend_cache_scanned_height: diagnostics.spend_cache_scanned_height,
        spend_cache_tip_height: diagnostics.spend_cache_tip_height,
        spend_cache_last_updated: diagnostics.spend_cache_last_updated,
        spend_cache_note_count: diagnostics.spend_cache_note_count,
    })
}

fn map_prover_file_status(
    diagnostics: crate::core::channels::dlight_private::DlightProverFileDiagnostics,
) -> DlightProverFileStatusResult {
    DlightProverFileStatusResult {
        path: diagnostics.path,
        exists: diagnostics.exists,
        size_bytes: diagnostics.size_bytes,
        min_size_bytes: diagnostics.min_size_bytes,
        checksum_algorithm: diagnostics.checksum_algorithm,
        expected_checksum: diagnostics.expected_checksum,
        actual_checksum: diagnostics.actual_checksum,
        checksum_matches: diagnostics.checksum_matches,
        placeholder_detected: diagnostics.placeholder_detected,
        errors: diagnostics.errors,
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_dlight_prover_status() -> Result<DlightProverStatusResult, WalletError> {
    let diagnostics = crate::core::channels::dlight_private::get_prover_status();
    Ok(DlightProverStatusResult {
        ready: diagnostics.ready,
        params_dir: diagnostics.params_dir,
        spend: map_prover_file_status(diagnostics.spend),
        output: map_prover_file_status(diagnostics.output),
        errors: diagnostics.errors,
    })
}

/// Get available subwallet scopes for a coin.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_coin_scopes(
    coin_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<CoinScopesResult, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let account_id = context.account_id.clone();
    let network = context.wallet_network;
    let addresses = (
        context.vrsc_address.clone(),
        context.eth_address.clone(),
        context.btc_address.clone(),
    );
    let primary_vrpc_address = context.vrsc_address.clone();

    let is_testnet = matches!(network, WalletNetwork::Testnet);
    let coin = coin_registry
        .find_by_id(&coin_id, is_testnet)
        .ok_or(WalletError::UnsupportedChannel)?;

    if coin_supports_channel(&coin, Channel::Vrpc) {
        let watched = account_state_store.load_watched_vrpc_addresses(&account_id, network)?;
        let linked_identities = context
            .load_linked_identities_cached()
            .await
            .unwrap_or_else(|error| {
                println!(
                    "[WALLET] Failed to load linked identities for coin scopes; using address-only fallback: {:?}",
                    error
                );
                vec![]
            });
        let scope_addresses = collect_vrpc_scope_addresses(
            &primary_vrpc_address,
            &linked_identities,
            &watched,
            network,
        );

        let configured_systems = vrpc_provider_pool.configured_systems(network);
        let systems = collect_vrpc_system_descriptors(
            coin_registry.as_ref(),
            network,
            &coin,
            &configured_systems,
        );

        let mut scopes = Vec::<CoinScope>::new();
        for address_scope in scope_addresses {
            for system in &systems {
                scopes.push(CoinScope {
                    channel_id: crate::core::channels::vrpc::canonical_vrpc_channel_id(
                        &address_scope.address,
                        &system.system_id,
                    ),
                    coin_id: coin.id.clone(),
                    address: address_scope.address.clone(),
                    address_label: address_scope.address_label.clone(),
                    system_id: system.system_id.clone(),
                    system_ticker: system.system_ticker.clone(),
                    system_display_name: system.system_display_name.clone(),
                    is_primary_address: address_scope.is_primary_address,
                    is_read_only: address_scope.is_read_only,
                    scope_kind: ScopeKind::Transparent,
                });
            }
        }

        if coin_supports_channel(&coin, Channel::DlightPrivate) {
            let dlight = context.load_dlight_public_metadata_cached().await?;
            if let Some(shielded_address) = dlight.shielded_address {
                if let Some(system) = systems.iter().find(|system| system.is_root) {
                    scopes.push(CoinScope {
                        channel_id: dlight_private::canonical_dlight_channel_id(
                            &shielded_address,
                            &system.system_id,
                        ),
                        coin_id: coin.id.clone(),
                        address: shielded_address.clone(),
                        address_label: "Shielded wallet".to_string(),
                        system_id: system.system_id.clone(),
                        system_ticker: system.system_ticker.clone(),
                        system_display_name: system.system_display_name.clone(),
                        is_primary_address: true,
                        is_read_only: false,
                        scope_kind: ScopeKind::Shielded,
                    });
                }
            }
        }

        return Ok(CoinScopesResult {
            coin_id: coin.id,
            scopes,
        });
    }

    let channel_id = channel_id_for_non_vrpc_coin(&coin).ok_or(WalletError::UnsupportedChannel)?;
    let address = address_for_non_vrpc_coin(&coin, &addresses);
    let (system_id, system_ticker, system_display_name) =
        canonical_non_vrpc_network_metadata(&coin, network);
    Ok(CoinScopesResult {
        coin_id: coin.id.clone(),
        scopes: vec![CoinScope {
            channel_id,
            coin_id: coin.id.clone(),
            address: address.clone(),
            address_label: address,
            system_id,
            system_ticker,
            system_display_name,
            is_primary_address: true,
            is_read_only: false,
            scope_kind: ScopeKind::Transparent,
        }],
    })
}

/// Hash account ID for metadata record
fn hash_account_id(account_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(account_id.as_bytes());
    hex::encode(hasher.finalize())
}

/// List available wallets (account_id + wallet_name for unlock flow)
#[tauri::command(rename_all = "snake_case")]
pub async fn list_wallets(
    wallet_manager: State<'_, WalletManager>,
) -> Result<Vec<WalletListItem>, WalletError> {
    println!("[WALLET] List wallets requested");

    let wallets = wallet_manager.list_wallets().await?;

    println!("[WALLET] Found {} wallets", wallets.len());

    Ok(wallets)
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_non_vrpc_network_metadata, channel_id_for_non_vrpc_coin,
        collect_vrpc_scope_addresses, collect_vrpc_system_descriptors, dedupe_preserve_order,
        dlight_recovery_secret_kind_from_seed, ensure_expected_session_id,
        format_discovered_amount, merge_discovered_vrpc_amounts, persist_new_account,
        recovery_secret_kind_from_wallet_secret_kind, sanitize_active_coin_ids,
        vrpc_balance_entries, DiscoveredVrpcAmount, NewAccountPersistenceRequest,
    };
    use crate::core::auth::kdf::CURRENT_KEY_DERIVATION_VERSION;
    use crate::core::auth::{stronghold_store::ACTIVE_ASSETS_PROFILE_VERSION, SessionManager};
    use crate::core::channels::vrpc::ConfiguredVrpcSystem;
    use crate::core::coins::{Channel, CoinDefinition, CoinRegistry, Protocol};
    use crate::core::crypto::{derive_keys_v1, Network};
    use crate::core::runtime_config;
    use crate::core::wallet::{AccountStateStore, WalletManager};
    use crate::core::StrongholdStore;
    use crate::types::wallet::{DlightRecoverySecretKind, RecoverySecretKind, WalletNetwork};
    use crate::types::LinkedIdentity;
    use crate::types::WalletSecretKind;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    const VRSC_SYSTEM_ID: &str = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";
    const VETH_SYSTEM_ID: &str = "i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X";
    const VDEX_SYSTEM_ID: &str = "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N";
    const CHIPS_SYSTEM_ID: &str = "iJ3WZocnjG9ufv7GKUA4LijQno5gTMb7tP";

    fn configured_system(
        system_id: &str,
        ticker: &str,
        display_name: &str,
    ) -> ConfiguredVrpcSystem {
        ConfiguredVrpcSystem {
            system_id: system_id.to_string(),
            system_ticker: ticker.to_string(),
            system_display_name: display_name.to_string(),
        }
    }

    #[test]
    fn discovery_balance_parser_preserves_tiny_native_and_pbaas_amounts() {
        let entries = vrpc_balance_entries(
            &serde_json::json!({
                "balance": 1,
                "currencybalance": {
                    "iCurrency": "0.00000001"
                }
            }),
            VRSC_SYSTEM_ID,
        );

        assert_eq!(
            format_discovered_amount(entries[&VRSC_SYSTEM_ID.to_ascii_lowercase()].1),
            "0.00000001"
        );
        assert_eq!(
            format_discovered_amount(entries["icurrency"].1),
            "0.00000001"
        );
    }

    #[test]
    fn discovery_merge_keeps_scope_and_partial_provenance() {
        let mut discovered = std::collections::HashMap::<String, DiscoveredVrpcAmount>::new();
        merge_discovered_vrpc_amounts(
            &mut discovered,
            std::collections::HashMap::from([(
                "icurrency".to_string(),
                ("iCurrency".to_string(), 1.25),
            )]),
            false,
            false,
        );
        merge_discovered_vrpc_amounts(
            &mut discovered,
            std::collections::HashMap::from([(
                "icurrency".to_string(),
                ("iCurrency".to_string(), 0.75),
            )]),
            true,
            true,
        );

        let amount = &discovered["icurrency"];
        assert_eq!(format_discovered_amount(amount.amount), "2");
        assert!(amount.includes_read_only);
        assert!(amount.partial);
    }

    #[test]
    fn asset_preferences_reject_a_replacement_unlock_session() {
        assert!(ensure_expected_session_id("session-a", "session-a").is_ok());
        assert!(matches!(
            ensure_expected_session_id("session-b", "session-a"),
            Err(crate::types::WalletError::WalletSessionChanged)
        ));
    }

    #[test]
    fn non_vrpc_scope_labels_name_execution_networks_not_tokens() {
        let registry = CoinRegistry::new();
        let all = registry.get_all();
        let usdc = all.iter().find(|coin| coin.id == "USDC").expect("USDC");
        let btc = all.iter().find(|coin| coin.id == "BTC").expect("BTC");

        assert_eq!(
            canonical_non_vrpc_network_metadata(usdc, WalletNetwork::Mainnet),
            ("ETH".to_string(), "ETH".to_string(), "Ethereum".to_string())
        );
        assert_eq!(
            canonical_non_vrpc_network_metadata(usdc, WalletNetwork::Testnet),
            (
                "GETH".to_string(),
                "ETH".to_string(),
                "Sepolia ETH".to_string()
            )
        );
        assert_eq!(
            canonical_non_vrpc_network_metadata(btc, WalletNetwork::Mainnet),
            ("BTC".to_string(), "BTC".to_string(), "Bitcoin".to_string())
        );
    }

    fn set_active_account(registry: &CoinRegistry) {
        registry.set_active_account(Some("wallet_tests_account".to_string()));
    }

    fn sample_vrpc_coin(
        id: &str,
        currency_id: &str,
        system_id: &str,
        ticker: &str,
        name: &str,
    ) -> CoinDefinition {
        CoinDefinition {
            id: id.to_string(),
            currency_id: currency_id.to_string(),
            system_id: system_id.to_string(),
            display_ticker: ticker.to_string(),
            display_name: name.to_string(),
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

    fn temp_wallet_test_dirs() -> (std::path::PathBuf, std::path::PathBuf) {
        let base_dir = std::env::temp_dir().join(format!(
            "lite_wallet_wallet_command_tests_{}",
            Uuid::new_v4()
        ));
        (base_dir.join("stronghold"), base_dir.join("wallet_data"))
    }

    #[test]
    fn dedupe_preserve_order_is_case_insensitive() {
        let deduped = dedupe_preserve_order(vec![
            "RAlpha".to_string(),
            "rbeta".to_string(),
            "rBeta".to_string(),
            "RGamma".to_string(),
            "  ".to_string(),
        ]);

        assert_eq!(
            deduped,
            vec![
                "RAlpha".to_string(),
                "rbeta".to_string(),
                "RGamma".to_string()
            ]
        );
    }

    #[test]
    fn recovery_secret_kind_mapping_matches_wallet_secret_kind() {
        assert!(matches!(
            recovery_secret_kind_from_wallet_secret_kind(WalletSecretKind::SeedText),
            RecoverySecretKind::SeedText
        ));
        assert!(matches!(
            recovery_secret_kind_from_wallet_secret_kind(WalletSecretKind::Wif),
            RecoverySecretKind::Wif
        ));
        assert!(matches!(
            recovery_secret_kind_from_wallet_secret_kind(WalletSecretKind::PrivateKeyHex),
            RecoverySecretKind::PrivateKeyHex
        ));
    }

    #[test]
    fn dlight_recovery_secret_kind_detects_spending_key_prefix() {
        assert!(matches!(
            dlight_recovery_secret_kind_from_seed("secret-extended-key-main1abcd"),
            DlightRecoverySecretKind::SpendingKey
        ));
        assert!(matches!(
            dlight_recovery_secret_kind_from_seed(
                "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
            ),
            DlightRecoverySecretKind::Mnemonic
        ));
    }

    #[test]
    fn vrpc_scope_addresses_include_primary_linked_and_watched_with_expected_priority() {
        let scope_addresses = collect_vrpc_scope_addresses(
            "RAutMoGh771ECTDbTq2qwwZo7MF5Tov3ka",
            &[
                LinkedIdentity {
                    identity_address: "i8A7LnkQfA97VQv64G4H4vbRb4vP7h4G8b".to_string(),
                    name: Some("scam".to_string()),
                    fully_qualified_name: Some("scam@".to_string()),
                    status: Some("active".to_string()),
                    system_id: Some(VRSC_SYSTEM_ID.to_string()),
                    favorite: false,
                },
                LinkedIdentity {
                    identity_address: "i8a7lnkqfa97vqv64g4h4vbrb4vp7h4g8b".to_string(),
                    name: Some("duplicate".to_string()),
                    fully_qualified_name: Some("duplicate@".to_string()),
                    status: Some("active".to_string()),
                    system_id: Some(VRSC_SYSTEM_ID.to_string()),
                    favorite: false,
                },
            ],
            &[
                "RAutMoGh771ECTDbTq2qwwZo7MF5Tov3ka".to_string(),
                "RWatchedAddr1111111111111111111111".to_string(),
            ],
            WalletNetwork::Mainnet,
        );

        assert_eq!(scope_addresses.len(), 3);
        assert_eq!(
            scope_addresses[0].address,
            "RAutMoGh771ECTDbTq2qwwZo7MF5Tov3ka"
        );
        assert_eq!(scope_addresses[0].is_primary_address, true);
        assert_eq!(scope_addresses[0].is_read_only, false);

        assert_eq!(
            scope_addresses[1].address,
            "i8A7LnkQfA97VQv64G4H4vbRb4vP7h4G8b"
        );
        assert_eq!(scope_addresses[1].address_label, "scam@");
        assert_eq!(scope_addresses[1].is_primary_address, false);
        assert_eq!(scope_addresses[1].is_read_only, false);

        assert_eq!(
            scope_addresses[2].address,
            "RWatchedAddr1111111111111111111111"
        );
        assert_eq!(scope_addresses[2].is_primary_address, false);
        assert_eq!(scope_addresses[2].is_read_only, true);
    }

    #[test]
    fn vrpc_system_descriptors_put_root_first() {
        let registry = CoinRegistry::new();
        let root_coin = registry.find_by_id("VRSC", false).expect("VRSC root coin");
        let systems =
            collect_vrpc_system_descriptors(&registry, WalletNetwork::Mainnet, &root_coin, &[]);
        assert!(!systems.is_empty());
        assert_eq!(systems[0].system_id, VRSC_SYSTEM_ID.to_string());
        assert!(systems[0].is_root);
    }

    #[test]
    fn vrpc_system_descriptors_follow_configured_providers_not_visibility() {
        let registry = CoinRegistry::new();
        set_active_account(&registry);
        registry
            .add_coin(sample_vrpc_coin(
                "vUSDC",
                "i6nreNEZpMML7Qw8PWcXh4BB6nffF7tA8Y",
                VETH_SYSTEM_ID,
                "vUSDC.vETH",
                "USDC on Verus",
            ))
            .expect("add vUSDC");
        registry
            .add_coin(sample_vrpc_coin(
                "CHIPS",
                CHIPS_SYSTEM_ID,
                CHIPS_SYSTEM_ID,
                "CHIPS",
                "CHIPS",
            ))
            .expect("add CHIPS");

        let root_coin = registry.find_by_id("VRSC", false).expect("VRSC root coin");
        let root_only =
            collect_vrpc_system_descriptors(&registry, WalletNetwork::Mainnet, &root_coin, &[]);
        assert_eq!(root_only.len(), 1);
        assert_eq!(root_only[0].system_id, VRSC_SYSTEM_ID);

        let with_configured_veth = collect_vrpc_system_descriptors(
            &registry,
            WalletNetwork::Mainnet,
            &root_coin,
            &[configured_system(VETH_SYSTEM_ID, "ETH", "Ethereum")],
        );
        assert_eq!(with_configured_veth.len(), 2);
        assert_eq!(with_configured_veth[0].system_id, VRSC_SYSTEM_ID);

        let with_chips = collect_vrpc_system_descriptors(
            &registry,
            WalletNetwork::Mainnet,
            &root_coin,
            &[configured_system(CHIPS_SYSTEM_ID, "CHIPS", "CHIPS")],
        );
        assert_eq!(with_chips.len(), 2);
        assert_eq!(with_chips[0].system_id, VRSC_SYSTEM_ID);
        assert!(with_chips
            .iter()
            .any(|descriptor| descriptor.system_id == CHIPS_SYSTEM_ID));
    }

    #[test]
    fn vrpc_system_descriptors_use_canonical_network_labels() {
        let registry = CoinRegistry::new();
        set_active_account(&registry);
        registry
            .add_coin(sample_vrpc_coin(
                "VETHCHAIN",
                VETH_SYSTEM_ID,
                VETH_SYSTEM_ID,
                "vETH",
                "Ethereum on Verus",
            ))
            .expect("add VETHCHAIN");

        let root_coin = registry.find_by_id("VRSC", false).expect("VRSC root coin");
        let descriptors = collect_vrpc_system_descriptors(
            &registry,
            WalletNetwork::Mainnet,
            &root_coin,
            &[configured_system(VETH_SYSTEM_ID, "ETH", "Ethereum")],
        );

        let veth = descriptors
            .iter()
            .find(|descriptor| descriptor.system_id == VETH_SYSTEM_ID)
            .expect("vETH descriptor");
        assert_eq!(veth.system_ticker, "ETH");
        assert_eq!(veth.system_display_name, "Ethereum");
    }

    #[test]
    fn fresh_registry_keeps_configured_system_identity_and_name() {
        let registry = CoinRegistry::new();
        let root_coin = registry.find_by_id("VRSC", false).expect("VRSC root coin");
        let descriptors = collect_vrpc_system_descriptors(
            &registry,
            WalletNetwork::Mainnet,
            &root_coin,
            &[configured_system(VDEX_SYSTEM_ID, "vDEX", "vDEX")],
        );

        let vdex = descriptors
            .iter()
            .find(|descriptor| descriptor.system_id == VDEX_SYSTEM_ID)
            .expect("configured vDEX descriptor");
        assert_eq!(vdex.system_ticker, "vDEX");
        assert_eq!(vdex.system_display_name, "vDEX");
    }

    #[test]
    fn vrpc_system_descriptors_do_not_use_token_ticker_as_network_label() {
        let registry = CoinRegistry::new();
        set_active_account(&registry);
        registry
            .add_coin(sample_vrpc_coin(
                "vDAI",
                "i7exampleTokenCurrencyForDai",
                VETH_SYSTEM_ID,
                "DAI",
                "Dai Stablecoin",
            ))
            .expect("add vDAI");

        let root_coin = registry.find_by_id("VRSC", false).expect("VRSC root coin");
        let descriptors =
            collect_vrpc_system_descriptors(&registry, WalletNetwork::Mainnet, &root_coin, &[]);

        assert_eq!(descriptors.len(), 1);
        assert_eq!(descriptors[0].system_id, VRSC_SYSTEM_ID);
    }

    #[test]
    fn vrpc_system_descriptors_always_include_root_system() {
        let registry = CoinRegistry::new();
        set_active_account(&registry);
        registry
            .add_coin(sample_vrpc_coin(
                "CHIPS",
                CHIPS_SYSTEM_ID,
                CHIPS_SYSTEM_ID,
                "CHIPS",
                "CHIPS",
            ))
            .expect("add CHIPS");

        let root_coin = registry.find_by_id("VRSC", false).expect("VRSC root coin");
        let descriptors = collect_vrpc_system_descriptors(
            &registry,
            WalletNetwork::Mainnet,
            &root_coin,
            &[configured_system(CHIPS_SYSTEM_ID, "CHIPS", "CHIPS")],
        );
        assert!(descriptors
            .iter()
            .any(|descriptor| descriptor.system_id == VRSC_SYSTEM_ID));
        assert_eq!(descriptors[0].system_id, VRSC_SYSTEM_ID);
    }

    #[test]
    fn sanitize_active_coin_ids_dedupes_and_drops_unknown() {
        let registry = CoinRegistry::new();
        let sanitized = sanitize_active_coin_ids(
            &registry,
            WalletNetwork::Mainnet,
            &[
                "vrsc".to_string(),
                "VRSC".to_string(),
                "UNKNOWN".to_string(),
                " ".to_string(),
            ],
        );
        assert_eq!(sanitized, vec!["VRSC".to_string()]);
    }

    #[test]
    fn existing_geth_activation_remains_one_testnet_asset() {
        let registry = CoinRegistry::new();
        let sanitized = sanitize_active_coin_ids(
            &registry,
            WalletNetwork::Testnet,
            &["geth".to_string(), "GETH".to_string()],
        );

        assert_eq!(sanitized, vec!["GETH".to_string()]);
    }

    #[test]
    fn channel_id_for_non_vrpc_coin_respects_channel_priority() {
        let btc_coin = CoinDefinition {
            id: "BTC".to_string(),
            currency_id: "BTC".to_string(),
            system_id: "BTC".to_string(),
            display_ticker: "BTC".to_string(),
            display_name: "Bitcoin".to_string(),
            coin_paprika_id: None,
            proto: Protocol::Btc,
            compatible_channels: vec![Channel::Btc, Channel::Eth],
            decimals: 8,
            vrpc_endpoints: vec![],
            dlight_endpoints: None,
            electrum_endpoints: None,
            seconds_per_block: 600,
            mapped_to: None,
            is_testnet: false,
        };

        let channel_id = channel_id_for_non_vrpc_coin(&btc_coin).expect("channel");
        assert_eq!(channel_id, "btc.BTC");
    }

    #[tokio::test]
    async fn persist_new_account_keeps_create_and_import_storage_contracts() {
        let _ = iota_stronghold::engine::snapshot::try_set_encrypt_work_factor(0);

        let (stronghold_dir, wallet_data_dir) = temp_wallet_test_dirs();
        let wallet_manager = WalletManager::new(wallet_data_dir.clone());
        let account_state_store = AccountStateStore::new(wallet_data_dir.clone());
        let session_manager = Arc::new(Mutex::new(SessionManager::new(
            StrongholdStore::new_for_tests(stronghold_dir.clone()),
        )));
        let create_seed = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let import_wif = derive_keys_v1("wallet import helper regression", Network::Testnet)
            .expect("derive test WIF")
            .wif
            .clone();

        let created = persist_new_account(
            NewAccountPersistenceRequest {
                wallet_name: "Created wallet",
                password: "password-one",
                network: WalletNetwork::Mainnet,
                emoji: "\u{1F4B0}",
                color: "blue",
                secret_kind: WalletSecretKind::SeedText,
                secret_material: create_seed,
                initial_dlight_secret: Some(create_seed),
            },
            &wallet_manager,
            &account_state_store,
            &session_manager,
        )
        .await
        .expect("create wallet");
        let imported = persist_new_account(
            NewAccountPersistenceRequest {
                wallet_name: "Imported wallet",
                password: "password-two",
                network: WalletNetwork::Testnet,
                emoji: "\u{1F680}",
                color: "green",
                secret_kind: WalletSecretKind::Wif,
                secret_material: &import_wif,
                initial_dlight_secret: None,
            },
            &wallet_manager,
            &account_state_store,
            &session_manager,
        )
        .await
        .expect("import wallet");

        let stronghold_store = super::stronghold_store_for_session(&session_manager).await;
        let create_hash = stronghold_store
            .derive_current_password_hash("password-one", false)
            .expect("create hash");
        let import_hash = stronghold_store
            .derive_current_password_hash("password-two", false)
            .expect("import hash");

        let created_account = wallet_manager
            .get_account_record_by_account_id(&created.wallet_id)
            .await
            .expect("load created account")
            .expect("created account");
        assert_eq!(
            created_account.key_derivation_version,
            CURRENT_KEY_DERIVATION_VERSION
        );
        assert_eq!(created_account.secret_kind, WalletSecretKind::SeedText);
        assert_eq!(
            account_state_store
                .load_active_assets(&created.wallet_id, WalletNetwork::Mainnet)
                .expect("created active assets"),
            (false, vec![], ACTIVE_ASSETS_PROFILE_VERSION)
        );
        assert_eq!(
            stronghold_store
                .load_dlight_seed(
                    &created.wallet_id,
                    create_hash.as_ref(),
                    WalletNetwork::Mainnet
                )
                .await
                .expect("created dlight")
                .as_deref(),
            Some(create_seed)
        );

        let imported_account = wallet_manager
            .get_account_record_by_account_id(&imported.wallet_id)
            .await
            .expect("load imported account")
            .expect("imported account");
        assert_eq!(
            imported_account.key_derivation_version,
            CURRENT_KEY_DERIVATION_VERSION
        );
        assert_eq!(imported_account.secret_kind, WalletSecretKind::Wif);
        assert_eq!(
            account_state_store
                .load_active_assets(&imported.wallet_id, WalletNetwork::Testnet)
                .expect("imported active assets"),
            (false, vec![], ACTIVE_ASSETS_PROFILE_VERSION)
        );
        assert_eq!(
            stronghold_store
                .load_dlight_seed(
                    &imported.wallet_id,
                    import_hash.as_ref(),
                    WalletNetwork::Testnet
                )
                .await
                .expect("imported dlight"),
            None
        );

        let _ = std::fs::remove_dir_all(stronghold_dir);
        let _ = std::fs::remove_dir_all(wallet_data_dir);
    }
}
