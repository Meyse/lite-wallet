use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tauri::State;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::commands::coins::pbaas_coin_definition_from_payload;
use crate::commands::identity::{
    build_identity_details_from_payload, map_identity_lookup_error, parse_getidentity_payload,
};
use crate::core::address_book::manager as address_book_manager;
use crate::core::auth::{
    capture_active_wallet_access_context, ensure_active_wallet_session, SessionManager,
};
use crate::core::channels::vrpc::{ConfiguredVrpcSystem, VrpcProviderPool};
use crate::core::coins::{Channel, CoinDefinition, CoinRegistry};
use crate::core::wallet::AccountStateStore;
use crate::types::wallet::WalletNetwork;
use crate::types::{
    AddressEndpointKind, ResolveWatchlistTargetRequest, WalletError, WatchlistEntry,
    WatchlistEntrySnapshot, WatchlistHolding, WatchlistRefreshResult, WatchlistResolvedTarget,
    WatchlistSource, WatchlistTargetKind,
};

const MAX_WATCHLIST_ENTRIES: usize = 100;
const VRSC_MAINNET_SYSTEM_ID: &str = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";
const VRSCTEST_SYSTEM_ID: &str = "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq";

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn normalize_non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn normalize_watchlist_entries(
    entries: Vec<WatchlistEntry>,
    legacy_addresses: Vec<String>,
    network: WalletNetwork,
) -> Vec<WatchlistEntry> {
    let mut seen = HashSet::<String>::new();
    let mut normalized = Vec::<WatchlistEntry>::new();

    for mut entry in entries {
        let Ok(address) = address_book_manager::normalize_destination_address(
            AddressEndpointKind::Vrpc,
            &entry.address,
            network,
        ) else {
            continue;
        };
        let key = address.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }

        entry.address = address.clone();
        entry.display_name = normalize_non_empty(&entry.display_name).unwrap_or(address);
        entry.system_id = entry.system_id.as_deref().and_then(normalize_non_empty);
        if entry.id.trim().is_empty() {
            entry.id = Uuid::new_v4().to_string();
        }
        normalized.push(entry);
        if normalized.len() == MAX_WATCHLIST_ENTRIES {
            return normalized;
        }
    }

    for address in legacy_addresses {
        let Ok(address) = address_book_manager::normalize_destination_address(
            AddressEndpointKind::Vrpc,
            &address,
            network,
        ) else {
            continue;
        };
        let key = address.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        normalized.push(WatchlistEntry {
            id: format!("legacy-{address}"),
            target_kind: if address.starts_with('i') {
                WatchlistTargetKind::Identity
            } else {
                WatchlistTargetKind::Address
            },
            display_name: address.clone(),
            address,
            system_id: None,
            created_at: 0,
            updated_at: 0,
        });
        if normalized.len() == MAX_WATCHLIST_ENTRIES {
            break;
        }
    }

    normalized
}

fn root_system(network: WalletNetwork) -> ConfiguredVrpcSystem {
    match network {
        WalletNetwork::Mainnet => ConfiguredVrpcSystem {
            system_id: VRSC_MAINNET_SYSTEM_ID.to_string(),
            system_ticker: "VRSC".to_string(),
            system_display_name: "Verus".to_string(),
        },
        WalletNetwork::Testnet => ConfiguredVrpcSystem {
            system_id: VRSCTEST_SYSTEM_ID.to_string(),
            system_ticker: "VRSCTEST".to_string(),
            system_display_name: "Verus Testnet".to_string(),
        },
    }
}

fn watchlist_systems(pool: &VrpcProviderPool, network: WalletNetwork) -> Vec<ConfiguredVrpcSystem> {
    let mut systems = pool.configured_systems(network);
    let root = root_system(network);
    if !systems
        .iter()
        .any(|system| system.system_id.eq_ignore_ascii_case(&root.system_id))
    {
        systems.insert(0, root);
    }
    systems
}

fn numeric_value(value: &Value) -> Option<f64> {
    let value = value
        .as_f64()
        .or_else(|| value.as_i64().map(|number| number as f64))
        .or_else(|| value.as_u64().map(|number| number as f64))
        .or_else(|| value.as_str().and_then(|number| number.trim().parse().ok()))?;
    value.is_finite().then_some(value)
}

fn balance_entries(raw: &Value, system_id: &str) -> HashMap<String, (String, f64)> {
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
        for (currency_id, raw_amount) in currency_balances {
            let Some(amount) = numeric_value(raw_amount).filter(|amount| *amount > 0.0) else {
                continue;
            };
            entries
                .entry(currency_id.to_ascii_lowercase())
                .and_modify(|entry| entry.1 += amount)
                .or_insert_with(|| (currency_id.clone(), amount));
        }
    }
    entries
}

fn format_amount(amount: f64) -> String {
    let formatted = format!("{amount:.8}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn registered_coin(
    registry: &CoinRegistry,
    network: WalletNetwork,
    currency_id: &str,
) -> Option<CoinDefinition> {
    let is_testnet = matches!(network, WalletNetwork::Testnet);
    registry.get_all().into_iter().find(|coin| {
        coin.is_testnet == is_testnet
            && coin.compatible_channels.contains(&Channel::Vrpc)
            && coin.currency_id.eq_ignore_ascii_case(currency_id)
    })
}

fn availability_for_sources(sources: &[WatchlistSource]) -> String {
    let available = sources
        .iter()
        .filter(|source| source.status == "available")
        .count();
    if available == 0 {
        "unavailable".to_string()
    } else if available == sources.len() {
        "available".to_string()
    } else {
        "partial".to_string()
    }
}

async fn load_address_snapshot(
    entry: WatchlistEntry,
    network: WalletNetwork,
    provider_pool: Arc<VrpcProviderPool>,
    coin_registry: Arc<CoinRegistry>,
) -> WatchlistEntrySnapshot {
    let systems = watchlist_systems(provider_pool.as_ref(), network);
    let mut tasks = JoinSet::new();

    for system in systems {
        let address = entry.address.clone();
        let provider_pool = provider_pool.clone();
        let coin_registry = coin_registry.clone();
        tasks.spawn(async move {
            let provider = provider_pool.for_system(network, &system.system_id);
            let raw = match provider.getaddressbalance(&[address]).await {
                Ok(raw) => raw,
                Err(_) => {
                    return (
                        WatchlistSource {
                            system_id: system.system_id,
                            system_ticker: system.system_ticker,
                            system_display_name: system.system_display_name,
                            status: "unavailable".to_string(),
                        },
                        Vec::new(),
                    );
                }
            };

            let mut holdings = Vec::<WatchlistHolding>::new();
            for (_, (currency_id, amount)) in balance_entries(&raw, &system.system_id) {
                let coin = if let Some(known) =
                    registered_coin(coin_registry.as_ref(), network, &currency_id)
                {
                    Some(known)
                } else {
                    provider
                        .getcurrency(&currency_id)
                        .await
                        .ok()
                        .and_then(|payload| {
                            pbaas_coin_definition_from_payload(
                                &payload,
                                network,
                                provider_pool.endpoint_url_for_system(network, &system.system_id),
                            )
                        })
                };
                holdings.push(WatchlistHolding {
                    asset_key: format!(
                        "vrsc:{}:{}",
                        system.system_id.to_ascii_lowercase(),
                        currency_id.to_ascii_lowercase()
                    ),
                    currency_id,
                    system_id: system.system_id.clone(),
                    system_ticker: system.system_ticker.clone(),
                    system_display_name: system.system_display_name.clone(),
                    balance: format_amount(amount),
                    coin,
                });
            }

            (
                WatchlistSource {
                    system_id: system.system_id,
                    system_ticker: system.system_ticker,
                    system_display_name: system.system_display_name,
                    status: "available".to_string(),
                },
                holdings,
            )
        });
    }

    let mut sources = Vec::<WatchlistSource>::new();
    let mut holdings = Vec::<WatchlistHolding>::new();
    while let Some(result) = tasks.join_next().await {
        if let Ok((source, mut source_holdings)) = result {
            sources.push(source);
            holdings.append(&mut source_holdings);
        }
    }
    sources.sort_by(|left, right| left.system_ticker.cmp(&right.system_ticker));
    holdings.sort_by(|left, right| {
        let left_name = left
            .coin
            .as_ref()
            .map(|coin| coin.display_ticker.as_str())
            .unwrap_or(left.currency_id.as_str());
        let right_name = right
            .coin
            .as_ref()
            .map(|coin| coin.display_ticker.as_str())
            .unwrap_or(right.currency_id.as_str());
        left_name
            .to_ascii_lowercase()
            .cmp(&right_name.to_ascii_lowercase())
            .then(left.system_ticker.cmp(&right.system_ticker))
    });

    WatchlistEntrySnapshot {
        entry,
        availability: availability_for_sources(&sources),
        holdings,
        sources,
        refreshed_at: now_unix(),
    }
}

async fn resolve_target_metadata(
    query: &str,
    network: WalletNetwork,
    provider_pool: &VrpcProviderPool,
) -> Result<(WatchlistTargetKind, String, String, Option<String>), WalletError> {
    let normalized = address_book_manager::normalize_destination_address(
        AddressEndpointKind::Vrpc,
        query,
        network,
    )
    .map_err(|_| WalletError::WatchlistInvalidInput)?;

    if normalized.starts_with('R') {
        return Ok((
            WatchlistTargetKind::Address,
            normalized.clone(),
            normalized,
            None,
        ));
    }

    let mut saw_network_error = false;
    for provider in provider_pool.provider_candidates(network, None) {
        let raw = match provider.getidentity(&normalized).await {
            Ok(raw) => raw,
            Err(error) => {
                if matches!(error, WalletError::NetworkError) {
                    saw_network_error = true;
                }
                continue;
            }
        };
        let parsed = match parse_getidentity_payload(raw) {
            Ok(parsed) => parsed,
            Err(_) => continue,
        };
        let details = build_identity_details_from_payload(
            &parsed.identity,
            parsed.status,
            "",
            Some(&normalized),
            parsed.fully_qualified_name.as_deref(),
            parsed.friendly_name.as_deref(),
        )?;
        let display_name = details
            .fully_qualified_name
            .or(details
                .name
                .map(|name| format!("{}@", name.trim_end_matches('@'))))
            .unwrap_or_else(|| details.identity_address.clone());
        return Ok((
            WatchlistTargetKind::Identity,
            display_name,
            details.identity_address,
            details.system,
        ));
    }

    if saw_network_error {
        Err(WalletError::NetworkError)
    } else {
        Err(map_identity_lookup_error(WalletError::IdentityNotFound))
    }
}

fn load_entries(
    store: &AccountStateStore,
    account_id: &str,
    network: WalletNetwork,
) -> Result<Vec<WatchlistEntry>, WalletError> {
    Ok(normalize_watchlist_entries(
        store.load_watchlist_entries(account_id, network)?,
        store.load_watched_vrpc_addresses(account_id, network)?,
        network,
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_watchlist_entries(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
) -> Result<Vec<WatchlistEntry>, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let entries = load_entries(
        account_state_store.inner(),
        &context.account_id,
        context.wallet_network,
    )?;
    ensure_active_wallet_session(session_manager.inner(), &context.session_id).await?;
    Ok(entries)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn resolve_watchlist_target(
    request: ResolveWatchlistTargetRequest,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
) -> Result<WatchlistResolvedTarget, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let (target_kind, display_name, address, system_id) = resolve_target_metadata(
        &request.query,
        context.wallet_network,
        vrpc_provider_pool.inner().as_ref(),
    )
    .await?;
    let preview_entry = WatchlistEntry {
        id: String::new(),
        target_kind: target_kind.clone(),
        display_name: display_name.clone(),
        address: address.clone(),
        system_id: system_id.clone(),
        created_at: 0,
        updated_at: 0,
    };
    let snapshot = load_address_snapshot(
        preview_entry,
        context.wallet_network,
        vrpc_provider_pool.inner().clone(),
        coin_registry.inner().clone(),
    )
    .await;
    ensure_active_wallet_session(session_manager.inner(), &context.session_id).await?;

    Ok(WatchlistResolvedTarget {
        target_kind,
        display_name,
        address,
        system_id,
        visible_currency_count: (snapshot.availability != "unavailable")
            .then_some(snapshot.holdings.len()),
        availability: snapshot.availability,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn add_watchlist_entry(
    request: ResolveWatchlistTargetRequest,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
) -> Result<WatchlistEntrySnapshot, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let (target_kind, display_name, address, system_id) = resolve_target_metadata(
        &request.query,
        context.wallet_network,
        vrpc_provider_pool.inner().as_ref(),
    )
    .await?;
    let mut entries = load_entries(
        account_state_store.inner(),
        &context.account_id,
        context.wallet_network,
    )?;
    if entries
        .iter()
        .any(|entry| entry.address.eq_ignore_ascii_case(&address))
    {
        return Err(WalletError::WatchlistDuplicate);
    }
    if entries.len() >= MAX_WATCHLIST_ENTRIES {
        return Err(WalletError::WatchlistInvalidInput);
    }

    let timestamp = now_unix();
    let entry = WatchlistEntry {
        id: Uuid::new_v4().to_string(),
        target_kind,
        display_name,
        address,
        system_id,
        created_at: timestamp,
        updated_at: timestamp,
    };
    let snapshot = load_address_snapshot(
        entry.clone(),
        context.wallet_network,
        vrpc_provider_pool.inner().clone(),
        coin_registry.inner().clone(),
    )
    .await;

    let session = session_manager.lock().await;
    if !session.is_current_session(&context.session_id) {
        return Err(WalletError::WalletSessionChanged);
    }
    entries.insert(0, entry);
    account_state_store.store_watchlist_entries(
        &context.account_id,
        context.wallet_network,
        &entries,
    )?;
    drop(session);
    Ok(snapshot)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn remove_watchlist_entry(
    entry_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
) -> Result<bool, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let mut entries = load_entries(
        account_state_store.inner(),
        &context.account_id,
        context.wallet_network,
    )?;
    let before = entries.len();
    entries.retain(|entry| entry.id != entry_id);
    if entries.len() == before {
        return Err(WalletError::WatchlistEntryNotFound);
    }

    let session = session_manager.lock().await;
    if !session.is_current_session(&context.session_id) {
        return Err(WalletError::WalletSessionChanged);
    }
    account_state_store.store_watchlist_entries(
        &context.account_id,
        context.wallet_network,
        &entries,
    )?;
    drop(session);
    Ok(true)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn refresh_watchlist(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
) -> Result<WatchlistRefreshResult, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let entries = load_entries(
        account_state_store.inner(),
        &context.account_id,
        context.wallet_network,
    )?;
    let mut tasks = JoinSet::new();
    for (index, entry) in entries.iter().cloned().enumerate() {
        let provider_pool = vrpc_provider_pool.inner().clone();
        let coin_registry = coin_registry.inner().clone();
        let network = context.wallet_network;
        tasks.spawn(async move {
            (
                index,
                load_address_snapshot(entry, network, provider_pool, coin_registry).await,
            )
        });
    }

    let mut snapshots = vec![None; entries.len()];
    while let Some(result) = tasks.join_next().await {
        if let Ok((index, snapshot)) = result {
            snapshots[index] = Some(snapshot);
        }
    }
    let refreshed_at = now_unix();
    let snapshots = entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            snapshots[index]
                .take()
                .unwrap_or_else(|| WatchlistEntrySnapshot {
                    entry,
                    holdings: Vec::new(),
                    sources: Vec::new(),
                    availability: "unavailable".to_string(),
                    refreshed_at,
                })
        })
        .collect();
    ensure_active_wallet_session(session_manager.inner(), &context.session_id).await?;

    Ok(WatchlistRefreshResult {
        network: context.wallet_network,
        entries: snapshots,
        refreshed_at,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        availability_for_sources, balance_entries, format_amount, normalize_watchlist_entries,
    };
    use crate::types::wallet::WalletNetwork;
    use crate::types::{WatchlistEntry, WatchlistSource, WatchlistTargetKind};

    #[test]
    fn balance_parser_keeps_native_and_pbaas_values() {
        let entries = balance_entries(
            &serde_json::json!({
                "balance": 125_000_000,
                "currencybalance": { "iCurrency": "2.5", "ignored": 0 }
            }),
            "iSystem",
        );
        assert_eq!(format_amount(entries["isystem"].1), "1.25");
        assert_eq!(format_amount(entries["icurrency"].1), "2.5");
        assert!(!entries.contains_key("ignored"));
    }

    #[test]
    fn legacy_addresses_are_deduplicated_behind_structured_entries() {
        let address = format!("R{}", "1".repeat(33));
        let entry = WatchlistEntry {
            id: "saved".to_string(),
            target_kind: WatchlistTargetKind::Address,
            display_name: address.clone(),
            address: address.clone(),
            system_id: None,
            created_at: 1,
            updated_at: 1,
        };
        let entries = normalize_watchlist_entries(
            vec![entry],
            vec![address.clone(), format!("R{}", "2".repeat(33))],
            WalletNetwork::Mainnet,
        );
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, "saved");
        assert_eq!(entries[1].display_name, entries[1].address);
    }

    #[test]
    fn source_availability_distinguishes_partial_and_unavailable() {
        let source = |status: &str| WatchlistSource {
            system_id: status.to_string(),
            system_ticker: status.to_string(),
            system_display_name: status.to_string(),
            status: status.to_string(),
        };
        assert_eq!(
            availability_for_sources(&[source("available"), source("unavailable")]),
            "partial"
        );
        assert_eq!(
            availability_for_sources(&[source("unavailable")]),
            "unavailable"
        );
        assert_eq!(
            availability_for_sources(&[source("available")]),
            "available"
        );
    }
}
