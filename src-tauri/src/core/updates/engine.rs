//
// Module 7: Update engine — Tokio polling for balances and transactions, Tauri event emission.
// Lifecycle: start on unlock, stop on lock. No sensitive data in logs.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::core::auth::{
    clear_wallet_session_if_current, GuardSessionManager, ProvisioningSignatureStore,
    SessionManager,
};
use crate::core::channels::btc::BtcProviderPool;
use crate::core::channels::dlight_private;
use crate::core::channels::eth::EthProviderPool;
use crate::core::channels::vrpc::VrpcProviderPool;
use crate::core::channels::{
    route_get_balances, route_get_info, route_get_transactions, PreflightStore,
};
use crate::core::coins::{Channel, CoinDefinition, CoinRegistry, Protocol};
use crate::core::rates::{build_rates_http_client, coinpaprika, ecb, pbaas};
use crate::core::updates::events::{
    BalancesUpdatedPayload, BootstrapUpdatedPayload, InfoUpdatedPayload, RatesUpdatedPayload,
    TransactionsUpdatedPayload, UpdateErrorPayload,
};
use crate::core::updates::params::{
    jitter_duration, BALANCE_REFRESH_SECS, CHAIN_INFO_REFRESH_SECS, DLIGHT_POST_SYNC_REFRESH_SECS,
    DLIGHT_SYNC_BALANCE_REFRESH_SECS, DLIGHT_SYNC_INFO_REFRESH_SECS,
    DLIGHT_SYNC_TRANSACTION_REFRESH_SECS, RATES_REFRESH_SECS, TRANSACTION_REFRESH_SECS,
};
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

/// Tauri event names (frontend listens via listen()).
pub const EVENT_BALANCES_UPDATED: &str = "wallet://balances-updated";
pub const EVENT_TRANSACTIONS_UPDATED: &str = "wallet://transactions-updated";
pub const EVENT_INFO_UPDATED: &str = "wallet://info-updated";
pub const EVENT_RATES_UPDATED: &str = "wallet://rates-updated";
pub const EVENT_BOOTSTRAP_UPDATED: &str = "wallet://bootstrap-updated";
pub const EVENT_SESSION_EXPIRED: &str = "wallet://session-expired";
pub const EVENT_TX_SEND_PROGRESS: &str = "wallet://tx-send-progress";
pub const EVENT_ERROR: &str = "wallet://error";
const BOOTSTRAP_BALANCE_CONCURRENCY: usize = 4;
const BOOTSTRAP_RATE_RESOLVE_TIMEOUT_SECS: u64 = 10;
const BOOTSTRAP_PBAAS_PROVIDER_TIMEOUT_SECS: u64 = 8;
const BOOTSTRAP_BRIDGE_VETH_LOOKUP_TIMEOUT_SECS: u64 = 8;
const BOOTSTRAP_RATE_SLOW_LOG_MS: u128 = 2_000;
const VRSC_COIN_ID: &str = "VRSC";
const VRSCTEST_COIN_ID: &str = "VRSCTEST";
const ETH_COIN_ID: &str = "ETH";
const VETH_SYSTEM_ID: &str = "i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X";
const VRSC_SYSTEM_ID: &str = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";
const BRIDGE_VETH_CURRENCY_ID: &str = "i3f7tSctFkiPpiedY8QR5Tep9p4qDVebDx";
const BRIDGE_VETH_TICKER: &str = "Bridge.vETH";
const DAI_VETH_CURRENCY_ID: &str = "iGBs4DWztRNvNEJBt4mqHszLxfKTNHTkhM";
const VUSDC_VETH_CURRENCY_ID: &str = "i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd";
const DAI_USDC_PARITY_MIN: f64 = 0.95;
const DAI_USDC_PARITY_MAX: f64 = 1.05;

#[derive(Clone, Debug, Default)]
pub struct UpdateEngineStartConfig {
    pub poll_transactions: bool,
    pub priority_coin_ids: Vec<String>,
    pub priority_channel_ids: Vec<String>,
}

/// Per-channel expiry state for balance and transaction data.
#[derive(Default)]
struct ChannelState {
    last_balance_fetch: Option<Instant>,
    last_tx_fetch: Option<Instant>,
    last_info_fetch: Option<Instant>,
    last_info_syncing: Option<bool>,
}

/// Update engine: polls VRPC, BTC, ETH and ERC20 channels when unlocked, emits Tauri events.
/// Hold in tauri::State; start() from start_update_engine, stop() from lock_wallet.
pub struct UpdateEngine {
    cancel_token: Mutex<Option<CancellationToken>>,
    task_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    channel_state: Mutex<HashMap<String, ChannelState>>,
}

impl UpdateEngine {
    pub fn new() -> Self {
        Self {
            cancel_token: Mutex::new(None),
            task_handle: Mutex::new(None),
            channel_state: Mutex::new(HashMap::new()),
        }
    }

    /// Start polling. Call after successful unlock. Spawns a single task that always runs
    /// balance polling and can optionally run transaction polling.
    pub async fn start(
        &self,
        app_handle: AppHandle,
        session_id: String,
        session_manager: Arc<Mutex<SessionManager>>,
        guard_session_manager: Arc<Mutex<GuardSessionManager>>,
        preflight_store: PreflightStore,
        provisioning_signature_store: ProvisioningSignatureStore,
        coin_registry: Arc<CoinRegistry>,
        vrpc_provider_pool: Arc<VrpcProviderPool>,
        btc_provider_pool: Arc<BtcProviderPool>,
        eth_provider_pool: Arc<EthProviderPool>,
        start_config: UpdateEngineStartConfig,
    ) {
        self.stop().await;

        let token = CancellationToken::new();
        let child = token.child_token();

        let session_manager = Arc::clone(&session_manager);
        let task_handle = tokio::spawn(async move {
            run_update_loop(
                child,
                app_handle,
                session_id,
                session_manager,
                guard_session_manager,
                preflight_store,
                provisioning_signature_store,
                coin_registry,
                vrpc_provider_pool,
                btc_provider_pool,
                eth_provider_pool,
                start_config,
            )
            .await;
        });

        *self.cancel_token.lock().await = Some(token);
        *self.task_handle.lock().await = Some(task_handle);
        println!("[UPDATE] Engine started");
    }

    /// Stop polling and wait for the task to finish. Call before lock.
    pub async fn stop(&self) {
        let mut token_guard = self.cancel_token.lock().await;
        if let Some(token) = token_guard.take() {
            token.cancel();
            drop(token_guard);
            let mut handle_guard = self.task_handle.lock().await;
            if let Some(handle) = handle_guard.take() {
                // Abort to avoid waiting on long in-flight network requests.
                handle.abort();
                let _ = handle.await;
            }
            println!("[UPDATE] Engine stopped");
        }
    }
}

/// Build list of (coin_id, channel_id) for active channels.
fn active_channels(
    coin_registry: &CoinRegistry,
    is_testnet: bool,
    vrpc_address: &str,
    eth_enabled: bool,
    dlight_scope_address: Option<&str>,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for c in coin_registry.get_all() {
        if c.is_testnet != is_testnet {
            continue;
        }
        for ch in &c.compatible_channels {
            match ch {
                Channel::Vrpc => {
                    out.push((
                        c.id.clone(),
                        format!("vrpc.{}.{}", vrpc_address, c.system_id),
                    ));
                }
                Channel::DlightPrivate if dlight_scope_address.is_some() => {
                    let scope_address = dlight_scope_address.unwrap_or(vrpc_address);
                    out.push((
                        c.id.clone(),
                        format!("dlight_private.{}.{}", scope_address, c.system_id),
                    ));
                }
                Channel::Btc => {
                    out.push((c.id.clone(), format!("btc.{}", c.id)));
                }
                Channel::Eth if eth_enabled => {
                    out.push((c.id.clone(), format!("eth.{}", c.id)));
                }
                Channel::Erc20 if eth_enabled => {
                    out.push((c.id.clone(), format!("erc20.{}", c.id)));
                }
                _ => {}
            }
        }
    }
    out
}

fn fiat_rate_candidates(
    coin_registry: &CoinRegistry,
    is_testnet: bool,
) -> Vec<crate::core::coins::CoinDefinition> {
    if is_testnet {
        return Vec::new();
    }

    coin_registry
        .get_all()
        .into_iter()
        .filter(|coin| !coin.is_testnet)
        .collect()
}

fn is_bridge_veth(coin: &CoinDefinition) -> bool {
    if coin.proto != Protocol::Vrsc {
        return false;
    }

    coin.id.trim().eq_ignore_ascii_case(BRIDGE_VETH_CURRENCY_ID)
        || coin
            .display_ticker
            .trim()
            .eq_ignore_ascii_case(BRIDGE_VETH_TICKER)
        || coin
            .display_name
            .trim()
            .eq_ignore_ascii_case(BRIDGE_VETH_TICKER)
}

fn is_coinpaprika_primary_candidate(coin: &CoinDefinition) -> bool {
    match coin.proto {
        Protocol::Eth | Protocol::Erc20 | Protocol::Btc => true,
        Protocol::Vrsc => {
            if coin.id.trim().eq_ignore_ascii_case(VRSC_COIN_ID) {
                return true;
            }

            if is_bridge_veth(coin) {
                return false;
            }

            coin.id.trim().eq_ignore_ascii_case(VETH_SYSTEM_ID)
                || coin.system_id.trim().eq_ignore_ascii_case(VETH_SYSTEM_ID)
        }
    }
}

fn should_attempt_coinpaprika(coin: &CoinDefinition) -> bool {
    is_coinpaprika_primary_candidate(coin) && coinpaprika::has_known_coinpaprika_id(coin)
}

fn lookup_rates_case_insensitive<'a>(
    latest_rates: &'a HashMap<String, HashMap<String, f64>>,
    coin_id: &str,
) -> Option<&'a HashMap<String, f64>> {
    latest_rates
        .iter()
        .find(|(id, _)| id.trim().eq_ignore_ascii_case(coin_id.trim()))
        .map(|(_, rates)| rates)
}

fn strict_alias_counterpart_coin_id(coin: &CoinDefinition) -> Option<&'static str> {
    if coin.id.trim().eq_ignore_ascii_case(ETH_COIN_ID) {
        return Some(VETH_SYSTEM_ID);
    }
    if coin.id.trim().eq_ignore_ascii_case(VETH_SYSTEM_ID) {
        return Some(ETH_COIN_ID);
    }
    None
}

fn strict_alias_fallback_rates(
    coin: &CoinDefinition,
    latest_rates: &HashMap<String, HashMap<String, f64>>,
) -> Option<HashMap<String, f64>> {
    let counterpart = strict_alias_counterpart_coin_id(coin)?;
    lookup_rates_case_insensitive(latest_rates, counterpart).cloned()
}

fn pending_strict_alias_backfill(
    coins: &[CoinDefinition],
    latest_rates: &HashMap<String, HashMap<String, f64>>,
) -> Vec<(String, HashMap<String, f64>)> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::<(String, HashMap<String, f64>)>::new();

    for coin in coins {
        let key = coin.id.trim().to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        if lookup_rates_case_insensitive(latest_rates, &coin.id).is_some() {
            continue;
        }
        if let Some(rates) = strict_alias_fallback_rates(coin, latest_rates) {
            out.push((coin.id.clone(), rates));
        }
    }

    out
}

fn value_to_f64(value: &Value) -> Option<f64> {
    if let Some(v) = value.as_f64() {
        return Some(v);
    }
    if let Some(v) = value.as_i64() {
        return Some(v as f64);
    }
    if let Some(v) = value.as_u64() {
        return Some(v as f64);
    }
    value
        .as_str()
        .and_then(|s| s.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn extract_last_conversion_price_from_map(
    currencies: &serde_json::Map<String, Value>,
    currency_id: &str,
) -> Option<f64> {
    currencies
        .iter()
        .find(|(key, _)| key.trim().eq_ignore_ascii_case(currency_id.trim()))
        .and_then(|(_, value)| value.get("lastconversionprice").and_then(value_to_f64))
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn extract_currency_state_map(currency_result: &Value) -> Option<&serde_json::Map<String, Value>> {
    currency_result
        .get("bestcurrencystate")
        .and_then(|value| value.get("currencies"))
        .and_then(|value| value.as_object())
        .or_else(|| {
            currency_result
                .get("lastconfirmedcurrencystate")
                .and_then(|value| value.get("currencies"))
                .and_then(|value| value.as_object())
        })
}

fn derive_vrsc_usd_anchor_from_bridge_currency_result(currency_result: &Value) -> Option<f64> {
    let currencies = extract_currency_state_map(currency_result)?;
    let vrsc_price = extract_last_conversion_price_from_map(currencies, VRSC_SYSTEM_ID)?;
    let dai_price = extract_last_conversion_price_from_map(currencies, DAI_VETH_CURRENCY_ID)?;
    let usdc_price = extract_last_conversion_price_from_map(currencies, VUSDC_VETH_CURRENCY_ID)?;

    let dai_per_usdc = dai_price / usdc_price;
    if !(dai_per_usdc.is_finite()
        && dai_per_usdc >= DAI_USDC_PARITY_MIN
        && dai_per_usdc <= DAI_USDC_PARITY_MAX)
    {
        return None;
    }

    let vrsc_usd = dai_price / vrsc_price;
    if vrsc_usd.is_finite() && vrsc_usd > 0.0 {
        Some(vrsc_usd)
    } else {
        None
    }
}

fn emit_and_store_rates(
    app_handle: &AppHandle,
    coin_id: &str,
    rates: HashMap<String, f64>,
    usd_change_24h_pct: Option<f64>,
    latest_rates: &mut HashMap<String, HashMap<String, f64>>,
) -> bool {
    let payload = RatesUpdatedPayload {
        coin_id: coin_id.to_string(),
        rates: rates.clone(),
        usd_change_24h_pct,
    };
    if let Err(err) = app_handle.emit(EVENT_RATES_UPDATED, &payload) {
        println!("[UPDATE] Emit rates-updated failed: {:?}", err);
        false
    } else {
        latest_rates.insert(coin_id.to_string(), rates);
        true
    }
}

async fn maybe_seed_vrsc_anchor_from_bridge_veth(
    app_handle: &AppHandle,
    vrpc_provider_pool: &VrpcProviderPool,
    active_network: WalletNetwork,
    usd_reference_rates: &HashMap<String, f64>,
    latest_rates: &mut HashMap<String, HashMap<String, f64>>,
) {
    if lookup_rates_case_insensitive(latest_rates, VRSC_COIN_ID).is_some() {
        return;
    }

    let mut providers = Vec::new();
    providers.push(vrpc_provider_pool.for_network(active_network));
    providers.extend(vrpc_provider_pool.provider_candidates(active_network, Some(VRSC_SYSTEM_ID)));

    let mut seen = HashSet::<usize>::new();
    for (provider_index, provider) in providers.into_iter().enumerate() {
        let provider_ptr = provider as *const _ as usize;
        if !seen.insert(provider_ptr) {
            continue;
        }

        let lookup_started_at = Instant::now();
        let lookup_timeout = Duration::from_secs(BOOTSTRAP_BRIDGE_VETH_LOOKUP_TIMEOUT_SECS);
        let payload = match tokio::time::timeout(
            lookup_timeout,
            provider.getcurrency(BRIDGE_VETH_CURRENCY_ID),
        )
        .await
        {
            Ok(Ok(payload)) => Some(payload),
            Ok(Err(_)) => {
                match tokio::time::timeout(lookup_timeout, provider.getcurrency(BRIDGE_VETH_TICKER))
                    .await
                {
                    Ok(Ok(payload)) => Some(payload),
                    Ok(Err(_)) => None,
                    Err(_) => {
                        println!(
                            "[UPDATE] Bridge.vETH ticker lookup timed out: provider_index={} timeout_secs={}",
                            provider_index, BOOTSTRAP_BRIDGE_VETH_LOOKUP_TIMEOUT_SECS
                        );
                        None
                    }
                }
            }
            Err(_) => {
                println!(
                    "[UPDATE] Bridge.vETH currency lookup timed out: provider_index={} timeout_secs={}",
                    provider_index, BOOTSTRAP_BRIDGE_VETH_LOOKUP_TIMEOUT_SECS
                );
                None
            }
        };
        let lookup_elapsed_ms = lookup_started_at.elapsed().as_millis();
        if lookup_elapsed_ms > BOOTSTRAP_RATE_SLOW_LOG_MS {
            println!(
                "[UPDATE] Bridge.vETH seed lookup slow: provider_index={} elapsed_ms={}",
                provider_index, lookup_elapsed_ms
            );
        }
        let Some(payload) = payload else {
            continue;
        };
        let result = payload.get("result").unwrap_or(&payload);

        let Some(vrsc_usd) = derive_vrsc_usd_anchor_from_bridge_currency_result(result) else {
            continue;
        };
        let rates = ecb::build_coin_fiat_rates(vrsc_usd, usd_reference_rates);
        if rates.is_empty() {
            continue;
        }

        if emit_and_store_rates(app_handle, VRSC_COIN_ID, rates, None, latest_rates) {
            println!("[UPDATE] Seeded VRSC anchor rate from Bridge.vETH DAI path");
            return;
        }
    }
}

async fn resolve_rates_for_coin(
    rates_http_client: &reqwest::Client,
    usd_reference_rates: &HashMap<String, f64>,
    vrpc_provider_pool: &VrpcProviderPool,
    active_network: WalletNetwork,
    coin: &CoinDefinition,
    latest_rates: &HashMap<String, HashMap<String, f64>>,
) -> (Option<HashMap<String, f64>>, Option<f64>, Option<String>) {
    let mut direct_error: Option<String> = None;

    if should_attempt_coinpaprika(coin) {
        match coinpaprika::fetch_usd_metrics(rates_http_client, coin).await {
            Ok(metrics) => {
                let rates = ecb::build_coin_fiat_rates(metrics.usd_price, usd_reference_rates);
                if !rates.is_empty() {
                    return (Some(rates), metrics.usd_change_24h_pct, None);
                }
            }
            Err(err) => direct_error = Some(err),
        }
    } else if is_coinpaprika_primary_candidate(coin) && !coinpaprika::has_known_coinpaprika_id(coin)
    {
        direct_error = Some(format!("coinpaprika known id missing for {}", coin.id));
    }

    if pbaas::is_pbaas_derivation_candidate(coin) {
        if let Some(rates) = derive_pbaas_rates_with_provider_candidates(
            vrpc_provider_pool,
            active_network,
            coin,
            latest_rates,
        )
        .await
        {
            return (Some(rates), None, direct_error);
        }
    }

    if let Some(rates) = strict_alias_fallback_rates(coin, latest_rates) {
        return (Some(rates), None, direct_error);
    }

    (None, None, direct_error)
}

fn normalize_priority_entries(entries: &[String]) -> HashSet<String> {
    entries
        .iter()
        .map(|entry| entry.trim().to_ascii_lowercase())
        .filter(|entry| !entry.is_empty())
        .collect()
}

fn dedupe_channel_pairs(channels: &[(String, String)]) -> Vec<(String, String)> {
    let mut seen = HashSet::<String>::new();
    let mut deduped = Vec::<(String, String)>::new();

    for (coin_id, channel_id) in channels {
        let dedupe_key = format!(
            "{}::{}",
            channel_id.trim().to_ascii_lowercase(),
            coin_id.trim().to_ascii_lowercase()
        );
        if seen.insert(dedupe_key) {
            deduped.push((coin_id.clone(), channel_id.clone()));
        }
    }

    deduped
}

fn partition_bootstrap_channels(
    channels: &[(String, String)],
    priority_coin_ids: &HashSet<String>,
    priority_channel_ids: &HashSet<String>,
) -> (Vec<(String, String)>, Vec<(String, String)>) {
    let mut prioritized = Vec::<(String, String)>::new();
    let mut remainder = Vec::<(String, String)>::new();

    for (coin_id, channel_id) in channels {
        let coin_key = coin_id.trim().to_ascii_lowercase();
        let channel_key = channel_id.trim().to_ascii_lowercase();
        if priority_coin_ids.contains(&coin_key) || priority_channel_ids.contains(&channel_key) {
            prioritized.push((coin_id.clone(), channel_id.clone()));
        } else {
            remainder.push((coin_id.clone(), channel_id.clone()));
        }
    }

    (prioritized, remainder)
}

fn prioritized_rate_coins(
    rate_coins: &[crate::core::coins::CoinDefinition],
    prioritized_channels: &[(String, String)],
    priority_coin_ids: &HashSet<String>,
) -> Vec<crate::core::coins::CoinDefinition> {
    let mut prioritized_coin_keys = priority_coin_ids.clone();
    for (coin_id, _) in prioritized_channels {
        prioritized_coin_keys.insert(coin_id.trim().to_ascii_lowercase());
    }

    // Ensure PBaaS anchor assets (VRSC/VRSCTEST) are included in bootstrap priority
    // whenever a PBaaS-derived coin is prioritized.
    let mut anchor_coin_keys = HashSet::<String>::new();
    for coin in rate_coins {
        let coin_key = coin.id.trim().to_ascii_lowercase();
        if !prioritized_coin_keys.contains(&coin_key) || !pbaas::is_pbaas_derivation_candidate(coin)
        {
            continue;
        }

        let anchor_coin_id = anchor_coin_id_for_pbaas_candidate(coin);
        anchor_coin_keys.insert(anchor_coin_id.to_ascii_lowercase());
    }
    prioritized_coin_keys.extend(anchor_coin_keys);

    let mut seen = HashSet::<String>::new();
    let mut prioritized = Vec::<crate::core::coins::CoinDefinition>::new();
    for coin in rate_coins {
        let coin_key = coin.id.trim().to_ascii_lowercase();
        if prioritized_coin_keys.contains(&coin_key) && seen.insert(coin_key) {
            prioritized.push(coin.clone());
        }
    }

    prioritized
}

fn anchor_coin_id_for_pbaas_candidate(coin: &crate::core::coins::CoinDefinition) -> &'static str {
    if coin.is_testnet {
        VRSCTEST_COIN_ID
    } else {
        VRSC_COIN_ID
    }
}

async fn derive_pbaas_rates_with_provider_candidates(
    vrpc_provider_pool: &VrpcProviderPool,
    active_network: WalletNetwork,
    coin: &crate::core::coins::CoinDefinition,
    latest_rates: &HashMap<String, HashMap<String, f64>>,
) -> Option<HashMap<String, f64>> {
    // Prefer root-network provider first,
    // then try system-specific endpoints as fallback.
    let mut providers = Vec::new();
    providers.push(vrpc_provider_pool.for_network(active_network));
    providers.extend(vrpc_provider_pool.provider_candidates(active_network, Some(&coin.system_id)));

    let mut seen = HashSet::<usize>::new();
    for (provider_index, provider) in providers.into_iter().enumerate() {
        let provider_ptr = provider as *const _ as usize;
        if !seen.insert(provider_ptr) {
            continue;
        }
        let derivation_started_at = Instant::now();
        let derivation_result = tokio::time::timeout(
            Duration::from_secs(BOOTSTRAP_PBAAS_PROVIDER_TIMEOUT_SECS),
            pbaas::derive_pbaas_rates(provider, coin, latest_rates),
        )
        .await;
        let derivation_elapsed_ms = derivation_started_at.elapsed().as_millis();
        if derivation_elapsed_ms > BOOTSTRAP_RATE_SLOW_LOG_MS {
            println!(
                "[UPDATE] PBaaS rate derivation slow: coin={} provider_index={} elapsed_ms={}",
                coin.id, provider_index, derivation_elapsed_ms
            );
        }

        match derivation_result {
            Ok(Some(rates)) => return Some(rates),
            Ok(None) => {}
            Err(_) => {
                println!(
                    "[UPDATE] PBaaS rate derivation timed out: coin={} provider_index={} timeout_secs={}",
                    coin.id, provider_index, BOOTSTRAP_PBAAS_PROVIDER_TIMEOUT_SECS
                );
            }
        }
    }

    None
}

fn emit_bootstrap_updated(app_handle: &AppHandle, in_progress: bool) {
    let payload = BootstrapUpdatedPayload { in_progress };
    if let Err(err) = app_handle.emit(EVENT_BOOTSTRAP_UPDATED, &payload) {
        println!("[UPDATE] Emit bootstrap-updated failed: {:?}", err);
    }
}

fn emit_session_expired(app_handle: &AppHandle) {
    if let Err(err) = app_handle.emit(EVENT_SESSION_EXPIRED, ()) {
        println!("[UPDATE] Emit session-expired failed: {:?}", err);
    }
}

fn supports_info_polling(channel_id: &str) -> bool {
    channel_id.starts_with("vrpc.") || channel_id.starts_with("dlight_private.")
}

fn is_dlight_channel(channel_id: &str) -> bool {
    channel_id.starts_with("dlight_private.")
}

fn parse_env_bool(value: &str) -> Option<bool> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn dlight_fast_sync_updates_enabled() -> bool {
    std::env::var("DLIGHT_FAST_SYNC_UPDATES")
        .ok()
        .as_deref()
        .and_then(parse_env_bool)
        .unwrap_or(cfg!(debug_assertions))
}

fn balance_refresh_secs(channel_id: &str, state: &ChannelState) -> u64 {
    if !is_dlight_channel(channel_id) {
        return BALANCE_REFRESH_SECS;
    }

    if state.last_info_syncing.unwrap_or(true) {
        DLIGHT_SYNC_BALANCE_REFRESH_SECS
    } else {
        DLIGHT_POST_SYNC_REFRESH_SECS
    }
}

fn info_refresh_secs(channel_id: &str, state: &ChannelState) -> u64 {
    if !is_dlight_channel(channel_id) {
        return CHAIN_INFO_REFRESH_SECS;
    }

    if state.last_info_syncing.unwrap_or(true) {
        DLIGHT_SYNC_INFO_REFRESH_SECS
    } else {
        DLIGHT_POST_SYNC_REFRESH_SECS
    }
}

fn transaction_refresh_secs(channel_id: &str, state: &ChannelState) -> u64 {
    if !is_dlight_channel(channel_id) {
        return TRANSACTION_REFRESH_SECS;
    }

    if state.last_info_syncing.unwrap_or(true) {
        DLIGHT_SYNC_TRANSACTION_REFRESH_SECS
    } else {
        DLIGHT_POST_SYNC_REFRESH_SECS
    }
}

fn should_emit_update_error(channel_id: &str, error: &WalletError) -> bool {
    if !is_dlight_channel(channel_id) {
        return true;
    }

    !matches!(
        error,
        WalletError::NetworkError | WalletError::DlightSynchronizerNotReady
    )
}

fn should_use_fast_loop_sleep(
    dlight_fast_updates: bool,
    channels: &[(String, String)],
    channel_state: &HashMap<String, ChannelState>,
) -> bool {
    if !dlight_fast_updates {
        return false;
    }

    for (coin_id, channel_id) in channels {
        if !is_dlight_channel(channel_id) {
            continue;
        }

        let state_key = format!("{}::{}", channel_id, coin_id);
        let is_syncing = channel_state
            .get(&state_key)
            .and_then(|state| state.last_info_syncing)
            .unwrap_or(true);
        if is_syncing {
            return true;
        }
    }

    false
}

async fn run_bootstrap_balance_fetches(
    app_handle: &AppHandle,
    cancel_token: &CancellationToken,
    prioritized_channels: &[(String, String)],
    session_manager: Arc<Mutex<SessionManager>>,
    coin_registry: Arc<CoinRegistry>,
    vrpc_provider_pool: Arc<VrpcProviderPool>,
    btc_provider_pool: Arc<BtcProviderPool>,
    eth_provider_pool: Arc<EthProviderPool>,
    channel_state: &mut HashMap<String, ChannelState>,
) {
    if prioritized_channels.is_empty() {
        return;
    }

    let semaphore = Arc::new(Semaphore::new(BOOTSTRAP_BALANCE_CONCURRENCY));
    let mut join_set = JoinSet::new();

    for (coin_id, channel_id) in prioritized_channels {
        if cancel_token.is_cancelled() {
            return;
        }

        let permit = match semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => return,
        };

        let coin_id = coin_id.clone();
        let channel_id = channel_id.clone();
        let session_manager = Arc::clone(&session_manager);
        let coin_registry = Arc::clone(&coin_registry);
        let vrpc_provider_pool = Arc::clone(&vrpc_provider_pool);
        let btc_provider_pool = Arc::clone(&btc_provider_pool);
        let eth_provider_pool = Arc::clone(&eth_provider_pool);

        join_set.spawn(async move {
            let _permit = permit;
            let result = route_get_balances(
                &channel_id,
                Some(coin_id.as_str()),
                &session_manager,
                coin_registry.as_ref(),
                vrpc_provider_pool.as_ref(),
                btc_provider_pool.as_ref(),
                eth_provider_pool.as_ref(),
            )
            .await;
            (coin_id, channel_id, result)
        });
    }

    while let Some(task_result) = join_set.join_next().await {
        let (coin_id, channel_id, result) = match task_result {
            Ok(value) => value,
            Err(err) => {
                println!("[UPDATE] Bootstrap balance task join error: {}", err);
                continue;
            }
        };

        match result {
            Ok(balance) => {
                let payload = BalancesUpdatedPayload {
                    coin_id: coin_id.clone(),
                    channel: channel_id.clone(),
                    confirmed: balance.confirmed,
                    pending: balance.pending,
                    total: balance.total,
                };
                if let Err(err) = app_handle.emit(EVENT_BALANCES_UPDATED, &payload) {
                    println!("[UPDATE] Emit balances-updated failed: {:?}", err);
                }
                channel_state
                    .entry(format!("{}::{}", channel_id, coin_id))
                    .or_default()
                    .last_balance_fetch = Some(Instant::now());
            }
            Err(err) => {
                let message = user_facing_error(&err);
                let _ = app_handle.emit(
                    EVENT_ERROR,
                    &UpdateErrorPayload {
                        data_type: "balance".to_string(),
                        coin_id,
                        channel: channel_id,
                        message,
                    },
                );
            }
        }
    }
}

async fn run_bootstrap_rate_fetches(
    app_handle: &AppHandle,
    cancel_token: &CancellationToken,
    prioritized_coins: &[crate::core::coins::CoinDefinition],
    active_network: WalletNetwork,
    rates_http_client: reqwest::Client,
    vrpc_provider_pool: Arc<VrpcProviderPool>,
    latest_rates: &mut HashMap<String, HashMap<String, f64>>,
    coin_rates_state: &mut HashMap<String, Instant>,
) {
    if prioritized_coins.is_empty() {
        return;
    }

    // Try anchor assets early so we only attempt Bridge.vETH fallback after an actual
    // direct-price failure for the anchor.
    let mut ordered_coins = prioritized_coins.to_vec();
    ordered_coins.sort_by_key(|coin| {
        if coin.id.trim().eq_ignore_ascii_case(VRSC_COIN_ID)
            || coin.id.trim().eq_ignore_ascii_case(VRSCTEST_COIN_ID)
        {
            0_u8
        } else {
            1_u8
        }
    });

    let usd_reference_rates = match ecb::fetch_usd_reference_rates(&rates_http_client).await {
        Ok(rates) => rates,
        Err(err) => {
            println!("[UPDATE] ECB rates unavailable during bootstrap: {}", err);
            HashMap::from([(ecb::USD.to_string(), 1.0)])
        }
    };
    let mut bridge_seed_attempted = false;
    let mut vrsc_direct_rate_failed = false;

    for coin in &ordered_coins {
        if cancel_token.is_cancelled() {
            return;
        }

        let coin_rate_started_at = Instant::now();
        let resolved_result = tokio::time::timeout(
            Duration::from_secs(BOOTSTRAP_RATE_RESOLVE_TIMEOUT_SECS),
            resolve_rates_for_coin(
                &rates_http_client,
                &usd_reference_rates,
                vrpc_provider_pool.as_ref(),
                active_network,
                coin,
                latest_rates,
            ),
        )
        .await;
        let coin_rate_elapsed_ms = coin_rate_started_at.elapsed().as_millis();
        if coin_rate_elapsed_ms > BOOTSTRAP_RATE_SLOW_LOG_MS {
            println!(
                "[UPDATE] Bootstrap rate fetch slow: coin={} elapsed_ms={}",
                coin.id, coin_rate_elapsed_ms
            );
        }
        let (mut resolved_rates, mut usd_change_24h_pct, mut rate_error) = match resolved_result {
            Ok(result) => result,
            Err(_) => {
                println!(
                    "[UPDATE] Bootstrap rate fetch timed out for {} after {}s",
                    coin.id, BOOTSTRAP_RATE_RESOLVE_TIMEOUT_SECS
                );
                coin_rates_state.insert(coin.id.clone(), Instant::now());
                continue;
            }
        };

        if coin.id.trim().eq_ignore_ascii_case(VRSC_COIN_ID) && rate_error.is_some() {
            vrsc_direct_rate_failed = true;
        }

        let needs_bridge_seed_for_vrsc = coin.id.trim().eq_ignore_ascii_case(VRSC_COIN_ID)
            && rate_error.is_some()
            && lookup_rates_case_insensitive(latest_rates, VRSC_COIN_ID).is_none();
        let needs_bridge_seed_for_pbaas = pbaas::is_pbaas_derivation_candidate(coin)
            && vrsc_direct_rate_failed
            && lookup_rates_case_insensitive(
                latest_rates,
                anchor_coin_id_for_pbaas_candidate(coin),
            )
            .is_none();

        if resolved_rates.is_none()
            && (needs_bridge_seed_for_vrsc || needs_bridge_seed_for_pbaas)
            && !bridge_seed_attempted
        {
            println!(
                "[UPDATE] Attempting on-demand Bridge.vETH seed after direct rate miss: coin={}",
                coin.id
            );
            maybe_seed_vrsc_anchor_from_bridge_veth(
                app_handle,
                vrpc_provider_pool.as_ref(),
                active_network,
                &usd_reference_rates,
                latest_rates,
            )
            .await;
            bridge_seed_attempted = true;

            if needs_bridge_seed_for_vrsc
                && lookup_rates_case_insensitive(latest_rates, VRSC_COIN_ID).is_some()
            {
                // Bridge seed already emitted VRSC rates and stored them.
                coin_rates_state.insert(coin.id.clone(), Instant::now());
                continue;
            }

            if needs_bridge_seed_for_pbaas {
                let retry_started_at = Instant::now();
                let retry_result = tokio::time::timeout(
                    Duration::from_secs(BOOTSTRAP_RATE_RESOLVE_TIMEOUT_SECS),
                    resolve_rates_for_coin(
                        &rates_http_client,
                        &usd_reference_rates,
                        vrpc_provider_pool.as_ref(),
                        active_network,
                        coin,
                        latest_rates,
                    ),
                )
                .await;
                let retry_elapsed_ms = retry_started_at.elapsed().as_millis();
                if retry_elapsed_ms > BOOTSTRAP_RATE_SLOW_LOG_MS {
                    println!(
                        "[UPDATE] Bootstrap rate retry slow: coin={} elapsed_ms={}",
                        coin.id, retry_elapsed_ms
                    );
                }
                match retry_result {
                    Ok(result) => {
                        (resolved_rates, usd_change_24h_pct, rate_error) = result;
                    }
                    Err(_) => {
                        println!(
                            "[UPDATE] Bootstrap rate retry timed out for {} after {}s",
                            coin.id, BOOTSTRAP_RATE_RESOLVE_TIMEOUT_SECS
                        );
                        coin_rates_state.insert(coin.id.clone(), Instant::now());
                        continue;
                    }
                }
            }
        }

        if let Some(rates) = resolved_rates {
            if emit_and_store_rates(
                app_handle,
                &coin.id,
                rates,
                usd_change_24h_pct,
                latest_rates,
            ) {
                coin_rates_state.insert(coin.id.clone(), Instant::now());
            }
        } else if let Some(rate_error) = rate_error {
            println!(
                "[UPDATE] Fiat rate unavailable during bootstrap for {}: {}",
                coin.id, rate_error
            );
        } else {
            println!(
                "[UPDATE] Fiat rate unavailable during bootstrap for {}",
                coin.id
            );
        }

        // Record attempted-at timestamp so the regular loop does not immediately refetch.
        coin_rates_state.insert(coin.id.clone(), Instant::now());
    }

    let alias_backfills = pending_strict_alias_backfill(&ordered_coins, latest_rates);
    for (coin_id, rates) in alias_backfills {
        if cancel_token.is_cancelled() {
            return;
        }
        if emit_and_store_rates(app_handle, &coin_id, rates, None, latest_rates) {
            coin_rates_state.insert(coin_id, Instant::now());
        }
    }
}

async fn run_update_loop(
    cancel_token: CancellationToken,
    app_handle: AppHandle,
    session_id: String,
    session_manager: Arc<Mutex<SessionManager>>,
    guard_session_manager: Arc<Mutex<GuardSessionManager>>,
    preflight_store: PreflightStore,
    provisioning_signature_store: ProvisioningSignatureStore,
    coin_registry: Arc<CoinRegistry>,
    vrpc_provider_pool: Arc<VrpcProviderPool>,
    btc_provider_pool: Arc<BtcProviderPool>,
    eth_provider_pool: Arc<EthProviderPool>,
    start_config: UpdateEngineStartConfig,
) {
    let mut channel_state: HashMap<String, ChannelState> = HashMap::new();
    let mut coin_rates_state: HashMap<String, Instant> = HashMap::new();
    let mut latest_rates: HashMap<String, HashMap<String, f64>> = HashMap::new();
    let rates_http_client = build_rates_http_client();
    let poll_transactions = start_config.poll_transactions;
    let priority_coin_ids = normalize_priority_entries(&start_config.priority_coin_ids);
    let priority_channel_ids = normalize_priority_entries(&start_config.priority_channel_ids);
    let dlight_fast_updates = dlight_fast_sync_updates_enabled();
    let mut bootstrap_completed = false;
    emit_bootstrap_updated(&app_handle, true);
    println!(
        "[UPDATE] dlight fast sync updates enabled={}",
        dlight_fast_updates
    );

    loop {
        if cancel_token.is_cancelled() {
            break;
        }

        let session = session_manager.lock().await;
        if session.active_session_id() != Some(session_id.as_str()) {
            break;
        }
        if session.is_expired() && session.active_account_id().is_some() {
            println!("[UPDATE] Session expired; clearing session state");
            drop(session);
            if clear_wallet_session_if_current(
                Some(&session_id),
                &session_manager,
                &guard_session_manager,
                &preflight_store,
                &provisioning_signature_store,
                &coin_registry,
            )
            .await
            {
                emit_session_expired(&app_handle);
            }
            break;
        }

        if !session.is_unlocked() {
            drop(session);
            tokio::select! {
                _ = cancel_token.cancelled() => break,
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {}
            }
            continue;
        }
        let (session_vrpc_address, _, _) = match session.get_addresses() {
            Ok(v) => v,
            Err(_) => {
                drop(session);
                tokio::select! {
                    _ = cancel_token.cancelled() => break,
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {}
                }
                continue;
            }
        };
        let account_id = match session.active_account_id() {
            Some(value) => value.clone(),
            None => {
                drop(session);
                tokio::select! {
                    _ = cancel_token.cancelled() => break,
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {}
                }
                continue;
            }
        };
        let active_network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
        let is_testnet = matches!(active_network, WalletNetwork::Testnet);
        let password_hash = session.stronghold_password_hash_for_storage().ok();
        let stronghold_store = session.stronghold_store().clone();
        drop(session);

        let dlight_scope_address = if let Some(password_hash) = password_hash {
            match stronghold_store
                .load_dlight_seed(&account_id, password_hash.as_ref(), active_network)
                .await
            {
                Ok(seed) => seed.as_deref().and_then(|seed_value| {
                    dlight_private::derive_scope_address(seed_value, active_network)
                        .map_err(|error| {
                            println!(
                                "[UPDATE] Failed to derive dlight scope address for {}: {:?}",
                                account_id, error
                            );
                            error
                        })
                        .ok()
                }),
                Err(error) => {
                    println!(
                        "[UPDATE] Failed to resolve dlight seed status for {}: {:?}",
                        account_id, error
                    );
                    None
                }
            }
        } else {
            None
        };

        let raw_channels = active_channels(
            &coin_registry,
            is_testnet,
            &session_vrpc_address,
            eth_provider_pool.is_enabled(),
            dlight_scope_address.as_deref(),
        );
        let channels = dedupe_channel_pairs(&raw_channels);
        if channels.is_empty() {
            if !bootstrap_completed {
                emit_bootstrap_updated(&app_handle, false);
                bootstrap_completed = true;
            }
            tokio::time::sleep(jitter_duration(30)).await;
            continue;
        }

        if !bootstrap_completed {
            let bootstrap_started_at = Instant::now();
            let (prioritized_channels, remainder_channels) =
                partition_bootstrap_channels(&channels, &priority_coin_ids, &priority_channel_ids);
            println!(
                "[UPDATE] Bootstrap start: prioritized_channels={} remaining_channels={}",
                prioritized_channels.len(),
                remainder_channels.len()
            );

            let rate_coins = fiat_rate_candidates(coin_registry.as_ref(), is_testnet);
            let prioritized_coins =
                prioritized_rate_coins(&rate_coins, &prioritized_channels, &priority_coin_ids);

            let balance_bootstrap = async {
                let started_at = Instant::now();
                run_bootstrap_balance_fetches(
                    &app_handle,
                    &cancel_token,
                    &prioritized_channels,
                    Arc::clone(&session_manager),
                    Arc::clone(&coin_registry),
                    Arc::clone(&vrpc_provider_pool),
                    Arc::clone(&btc_provider_pool),
                    Arc::clone(&eth_provider_pool),
                    &mut channel_state,
                )
                .await;
                started_at.elapsed()
            };

            let rate_bootstrap = async {
                let started_at = Instant::now();
                run_bootstrap_rate_fetches(
                    &app_handle,
                    &cancel_token,
                    &prioritized_coins,
                    active_network,
                    rates_http_client.clone(),
                    Arc::clone(&vrpc_provider_pool),
                    &mut latest_rates,
                    &mut coin_rates_state,
                )
                .await;
                started_at.elapsed()
            };

            let (balance_bootstrap_elapsed, rate_bootstrap_elapsed) =
                tokio::join!(balance_bootstrap, rate_bootstrap);
            let bootstrap_elapsed = bootstrap_started_at.elapsed();

            println!(
                "[UPDATE] Bootstrap complete: prioritized_channels={} prioritized_rates={} balance_ms={} rate_ms={} total_ms={}",
                prioritized_channels.len(),
                prioritized_coins.len(),
                balance_bootstrap_elapsed.as_millis(),
                rate_bootstrap_elapsed.as_millis(),
                bootstrap_elapsed.as_millis()
            );

            emit_bootstrap_updated(&app_handle, false);
            bootstrap_completed = true;
        }

        let now = Instant::now();

        for (coin_id, channel_id) in &channels {
            if cancel_token.is_cancelled() {
                return;
            }
            let channel_state_key = format!("{}::{}", channel_id, coin_id);

            let needs_balance = {
                let state = channel_state.entry(channel_state_key.clone()).or_default();
                let refresh_secs = if dlight_fast_updates {
                    balance_refresh_secs(channel_id, state)
                } else {
                    BALANCE_REFRESH_SECS
                };
                state
                    .last_balance_fetch
                    .map_or(true, |t| now.duration_since(t).as_secs() >= refresh_secs)
            };

            if needs_balance {
                match route_get_balances(
                    channel_id,
                    Some(coin_id.as_str()),
                    &session_manager,
                    coin_registry.as_ref(),
                    vrpc_provider_pool.as_ref(),
                    btc_provider_pool.as_ref(),
                    eth_provider_pool.as_ref(),
                )
                .await
                {
                    Ok(bal) => {
                        let payload = BalancesUpdatedPayload {
                            coin_id: coin_id.clone(),
                            channel: channel_id.clone(),
                            confirmed: bal.confirmed,
                            pending: bal.pending,
                            total: bal.total,
                        };
                        if let Err(e) = app_handle.emit(EVENT_BALANCES_UPDATED, &payload) {
                            println!("[UPDATE] Emit balances-updated failed: {:?}", e);
                        }
                        channel_state
                            .entry(channel_state_key.clone())
                            .or_default()
                            .last_balance_fetch = Some(Instant::now());
                    }
                    Err(e) => {
                        if should_emit_update_error(channel_id, &e) {
                            let message = user_facing_error(&e);
                            let _ = app_handle.emit(
                                EVENT_ERROR,
                                &UpdateErrorPayload {
                                    data_type: "balance".to_string(),
                                    coin_id: coin_id.clone(),
                                    channel: channel_id.clone(),
                                    message,
                                },
                            );
                        }
                        channel_state
                            .entry(channel_state_key.clone())
                            .or_default()
                            .last_balance_fetch = Some(Instant::now());
                    }
                }
                tokio::time::sleep(jitter_duration(2)).await;
            }
        }

        for (coin_id, channel_id) in &channels {
            if cancel_token.is_cancelled() {
                return;
            }
            if !supports_info_polling(channel_id) {
                continue;
            }

            let channel_state_key = format!("{}::{}", channel_id, coin_id);
            let needs_info = {
                let state = channel_state.entry(channel_state_key.clone()).or_default();
                let refresh_secs = if dlight_fast_updates {
                    info_refresh_secs(channel_id, state)
                } else {
                    CHAIN_INFO_REFRESH_SECS
                };
                state
                    .last_info_fetch
                    .map_or(true, |t| now.duration_since(t).as_secs() >= refresh_secs)
            };

            if needs_info {
                match route_get_info(
                    channel_id,
                    Some(coin_id.as_str()),
                    &session_manager,
                    coin_registry.as_ref(),
                    vrpc_provider_pool.as_ref(),
                )
                .await
                {
                    Ok(info) => {
                        let payload = InfoUpdatedPayload {
                            coin_id: coin_id.clone(),
                            channel: channel_id.clone(),
                            percent: info.percent,
                            blocks: info.blocks,
                            longest_chain: info.longest_chain,
                            syncing: info.syncing,
                            status_kind: info.status_kind.clone(),
                            last_updated: info.last_updated,
                            last_progress_at: info.last_progress_at,
                            stalled: info.stalled,
                            scan_rate_blocks_per_sec: info.scan_rate_blocks_per_sec,
                        };
                        if let Err(e) = app_handle.emit(EVENT_INFO_UPDATED, &payload) {
                            println!("[UPDATE] Emit info-updated failed: {:?}", e);
                        }
                        let state = channel_state.entry(channel_state_key.clone()).or_default();
                        state.last_info_fetch = Some(Instant::now());
                        state.last_info_syncing = Some(info.syncing);
                    }
                    Err(e) => {
                        if should_emit_update_error(channel_id, &e) {
                            let message = user_facing_error(&e);
                            let _ = app_handle.emit(
                                EVENT_ERROR,
                                &UpdateErrorPayload {
                                    data_type: "info".to_string(),
                                    coin_id: coin_id.clone(),
                                    channel: channel_id.clone(),
                                    message,
                                },
                            );
                        }
                        channel_state
                            .entry(channel_state_key.clone())
                            .or_default()
                            .last_info_fetch = Some(Instant::now());
                    }
                }
                tokio::time::sleep(jitter_duration(1)).await;
            }
        }

        if poll_transactions {
            for (coin_id, channel_id) in &channels {
                if cancel_token.is_cancelled() {
                    return;
                }
                let channel_state_key = format!("{}::{}", channel_id, coin_id);

                let state = channel_state.entry(channel_state_key.clone()).or_default();
                let refresh_secs = if dlight_fast_updates {
                    transaction_refresh_secs(channel_id, state)
                } else {
                    TRANSACTION_REFRESH_SECS
                };
                let needs_tx = state
                    .last_tx_fetch
                    .map_or(true, |t| now.duration_since(t).as_secs() >= refresh_secs);

                if needs_tx {
                    match route_get_transactions(
                        channel_id,
                        Some(coin_id.as_str()),
                        &session_manager,
                        coin_registry.as_ref(),
                        vrpc_provider_pool.as_ref(),
                        btc_provider_pool.as_ref(),
                        eth_provider_pool.as_ref(),
                    )
                    .await
                    {
                        Ok(txs) => {
                            let payload = TransactionsUpdatedPayload {
                                coin_id: coin_id.clone(),
                                channel: channel_id.clone(),
                                transactions: txs.transactions,
                            };
                            if let Err(e) = app_handle.emit(EVENT_TRANSACTIONS_UPDATED, &payload) {
                                println!("[UPDATE] Emit transactions-updated failed: {:?}", e);
                            }
                            if let Some(warning) = txs.warning {
                                let _ = app_handle.emit(
                                    EVENT_ERROR,
                                    &UpdateErrorPayload {
                                        data_type: "transactions_warning".to_string(),
                                        coin_id: coin_id.clone(),
                                        channel: channel_id.clone(),
                                        message: warning,
                                    },
                                );
                            }
                            state.last_tx_fetch = Some(Instant::now());
                        }
                        Err(e) => {
                            if should_emit_update_error(channel_id, &e) {
                                let message = user_facing_error(&e);
                                let _ = app_handle.emit(
                                    EVENT_ERROR,
                                    &UpdateErrorPayload {
                                        data_type: "transactions".to_string(),
                                        coin_id: coin_id.clone(),
                                        channel: channel_id.clone(),
                                        message,
                                    },
                                );
                            }
                            state.last_tx_fetch = Some(Instant::now());
                        }
                    }
                    tokio::time::sleep(jitter_duration(2)).await;
                }
            }
        }

        let rate_coins = fiat_rate_candidates(coin_registry.as_ref(), is_testnet);

        let needs_any_rates = rate_coins.iter().any(|coin| {
            coin_rates_state.get(&coin.id).map_or(true, |t| {
                now.duration_since(*t).as_secs() >= RATES_REFRESH_SECS
            })
        });

        if needs_any_rates {
            let usd_reference_rates = match ecb::fetch_usd_reference_rates(&rates_http_client).await
            {
                Ok(rates) => rates,
                Err(err) => {
                    println!("[UPDATE] ECB rates unavailable: {}", err);
                    HashMap::from([(ecb::USD.to_string(), 1.0)])
                }
            };

            maybe_seed_vrsc_anchor_from_bridge_veth(
                &app_handle,
                vrpc_provider_pool.as_ref(),
                active_network,
                &usd_reference_rates,
                &mut latest_rates,
            )
            .await;

            for coin in &rate_coins {
                if cancel_token.is_cancelled() {
                    return;
                }

                let needs_rates = coin_rates_state.get(&coin.id).map_or(true, |t| {
                    now.duration_since(*t).as_secs() >= RATES_REFRESH_SECS
                });
                if !needs_rates {
                    continue;
                }

                let (resolved_rates, usd_change_24h_pct, rate_error) = resolve_rates_for_coin(
                    &rates_http_client,
                    &usd_reference_rates,
                    vrpc_provider_pool.as_ref(),
                    active_network,
                    coin,
                    &latest_rates,
                )
                .await;

                if let Some(rates) = resolved_rates {
                    let _ = emit_and_store_rates(
                        &app_handle,
                        &coin.id,
                        rates,
                        usd_change_24h_pct,
                        &mut latest_rates,
                    );
                } else if let Some(rate_error) = rate_error {
                    println!(
                        "[UPDATE] Fiat rate unavailable for {}: {}",
                        coin.id, rate_error
                    );
                } else {
                    println!("[UPDATE] Fiat rate unavailable for {}", coin.id);
                }

                // Throttle retries after both success and failure.
                coin_rates_state.insert(coin.id.clone(), Instant::now());
                tokio::time::sleep(jitter_duration(1)).await;
            }

            let alias_backfills = pending_strict_alias_backfill(&rate_coins, &latest_rates);
            for (coin_id, rates) in alias_backfills {
                if cancel_token.is_cancelled() {
                    return;
                }
                if emit_and_store_rates(&app_handle, &coin_id, rates, None, &mut latest_rates) {
                    coin_rates_state.insert(coin_id, Instant::now());
                }
            }
        }

        let sleep_secs =
            if should_use_fast_loop_sleep(dlight_fast_updates, &channels, &channel_state) {
                1
            } else {
                60u64.min(BALANCE_REFRESH_SECS / 2)
            };
        tokio::select! {
            _ = cancel_token.cancelled() => break,
            _ = tokio::time::sleep(jitter_duration(sleep_secs)) => {}
        }
    }
}

fn user_facing_error(e: &WalletError) -> String {
    match e {
        WalletError::WalletLocked => "Wallet is locked".to_string(),
        WalletError::UnsupportedChannel => "Unsupported channel".to_string(),
        WalletError::InvalidPreflight => "Invalid preflight".to_string(),
        WalletError::NetworkError => "Network error".to_string(),
        WalletError::DlightSynchronizerNotReady => "dlight synchronizer not ready".to_string(),
        WalletError::EthNotConfigured => "Ethereum channels are not configured".to_string(),
        WalletError::OperationFailed => "Temporarily unavailable".to_string(),
        _ => "Temporarily unavailable".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        active_channels, dedupe_channel_pairs, derive_vrsc_usd_anchor_from_bridge_currency_result,
        fiat_rate_candidates, is_coinpaprika_primary_candidate, partition_bootstrap_channels,
        pending_strict_alias_backfill, prioritized_rate_coins, should_attempt_coinpaprika,
        strict_alias_fallback_rates, BRIDGE_VETH_CURRENCY_ID, DAI_VETH_CURRENCY_ID, VETH_SYSTEM_ID,
        VRSC_SYSTEM_ID, VUSDC_VETH_CURRENCY_ID,
    };
    use crate::core::coins::{Channel, CoinDefinition, CoinRegistry, Protocol};
    use serde_json::json;
    use std::collections::HashSet;

    fn sample_coin(id: &str, system_id: &str, proto: Protocol) -> CoinDefinition {
        CoinDefinition {
            id: id.to_string(),
            currency_id: id.to_string(),
            system_id: system_id.to_string(),
            display_ticker: id.to_string(),
            display_name: id.to_string(),
            coin_paprika_id: None,
            proto,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec![],
            dlight_endpoints: None,
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        }
    }

    #[test]
    fn active_channels_includes_eth_and_erc20_when_eth_enabled() {
        let registry = CoinRegistry::new();
        let channels = active_channels(&registry, false, "RtestAddress", true, None);

        assert!(channels
            .iter()
            .any(|(coin_id, channel_id)| coin_id == "ETH" && channel_id == "eth.ETH"));
        assert!(channels
            .iter()
            .any(|(coin_id, channel_id)| coin_id == "USDC" && channel_id == "erc20.USDC"));
    }

    #[test]
    fn active_channels_omits_eth_and_erc20_when_eth_disabled() {
        let registry = CoinRegistry::new();
        let channels = active_channels(&registry, false, "RtestAddress", false, None);

        assert!(!channels
            .iter()
            .any(|(_, channel_id)| channel_id.starts_with("eth.")));
        assert!(!channels
            .iter()
            .any(|(_, channel_id)| channel_id.starts_with("erc20.")));
    }

    #[test]
    fn active_channels_respects_testnet_network() {
        let registry = CoinRegistry::new();
        let channels = active_channels(&registry, true, "RtestAddress", true, None);

        assert!(channels
            .iter()
            .any(|(coin_id, channel_id)| coin_id == "GETH" && channel_id == "eth.GETH"));
        assert!(!channels
            .iter()
            .any(|(coin_id, _)| coin_id == "ETH" || coin_id == "USDC"));
    }

    #[test]
    fn fiat_rate_candidates_skip_testnet() {
        let registry = CoinRegistry::new();
        let testnet_candidates = fiat_rate_candidates(&registry, true);
        assert!(
            testnet_candidates.is_empty(),
            "testnet should not fetch fiat rates"
        );

        let mainnet_candidates = fiat_rate_candidates(&registry, false);
        assert!(
            !mainnet_candidates.is_empty(),
            "mainnet rates should remain enabled"
        );
        assert!(mainnet_candidates.iter().all(|coin| !coin.is_testnet));
    }

    #[test]
    fn dedupe_channel_pairs_preserves_first_seen_order() {
        let channels = vec![
            ("VRSC".to_string(), "vrpc.Raddr.iSystem".to_string()),
            ("VRSC".to_string(), "vrpc.Raddr.iSystem".to_string()),
            ("BTC".to_string(), "btc.BTC".to_string()),
            ("btc".to_string(), "BTC.btc".to_string()),
        ];

        let deduped = dedupe_channel_pairs(&channels);
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].0, "VRSC");
        assert_eq!(deduped[1].0, "BTC");
    }

    #[test]
    fn partition_bootstrap_channels_keeps_priority_order() {
        let channels = vec![
            ("VRSC".to_string(), "vrpc.Raddr.iSystem".to_string()),
            ("BTC".to_string(), "btc.BTC".to_string()),
            ("ETH".to_string(), "eth.ETH".to_string()),
        ];
        let priority_coin_ids = HashSet::from(["btc".to_string()]);
        let priority_channel_ids = HashSet::from(["eth.eth".to_string()]);

        let (prioritized, remainder) =
            partition_bootstrap_channels(&channels, &priority_coin_ids, &priority_channel_ids);

        assert_eq!(prioritized.len(), 2);
        assert_eq!(prioritized[0].0, "BTC");
        assert_eq!(prioritized[1].0, "ETH");
        assert_eq!(remainder.len(), 1);
        assert_eq!(remainder[0].0, "VRSC");
    }

    #[test]
    fn partition_bootstrap_channels_without_priorities_falls_back_to_remainder() {
        let channels = vec![
            ("VRSC".to_string(), "vrpc.Raddr.iSystem".to_string()),
            ("BTC".to_string(), "btc.BTC".to_string()),
        ];
        let priority_coin_ids = HashSet::new();
        let priority_channel_ids = HashSet::new();

        let (prioritized, remainder) =
            partition_bootstrap_channels(&channels, &priority_coin_ids, &priority_channel_ids);

        assert!(prioritized.is_empty());
        assert_eq!(remainder, channels);
    }

    #[test]
    fn prioritized_rate_coins_adds_vrsc_anchor_for_prioritized_pbaas() {
        let vrsc = CoinDefinition {
            id: "VRSC".to_string(),
            currency_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            display_ticker: "VRSC".to_string(),
            display_name: "Verus".to_string(),
            coin_paprika_id: Some("vrsc-verus-coin".to_string()),
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec![],
            dlight_endpoints: None,
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        };
        let pure = CoinDefinition {
            id: "iHax5qYQGbcMGqJKKrPorpzUBX2oFFXGnY".to_string(),
            currency_id: "iHax5qYQGbcMGqJKKrPorpzUBX2oFFXGnY".to_string(),
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            display_ticker: "Pure".to_string(),
            display_name: "Pure".to_string(),
            coin_paprika_id: None,
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec![],
            dlight_endpoints: None,
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        };
        let rate_coins = vec![vrsc.clone(), pure.clone()];
        let prioritized_channels = vec![(pure.id.clone(), "vrpc.Raddr.iSystem".to_string())];

        let prioritized =
            prioritized_rate_coins(&rate_coins, &prioritized_channels, &HashSet::new());
        let prioritized_ids = prioritized
            .into_iter()
            .map(|coin| coin.id)
            .collect::<HashSet<_>>();

        assert!(prioritized_ids.contains(&pure.id));
        assert!(prioritized_ids.contains(&vrsc.id));
    }

    #[test]
    fn prioritized_rate_coins_adds_vrsc_anchor_for_prioritized_root_pbaas_system() {
        let vrsc = CoinDefinition {
            id: "VRSC".to_string(),
            currency_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            display_ticker: "VRSC".to_string(),
            display_name: "Verus".to_string(),
            coin_paprika_id: Some("vrsc-verus-coin".to_string()),
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec![],
            dlight_endpoints: None,
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        };
        let vdex = CoinDefinition {
            id: "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N".to_string(),
            currency_id: "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N".to_string(),
            system_id: "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N".to_string(),
            display_ticker: "vDEX".to_string(),
            display_name: "vDEX".to_string(),
            coin_paprika_id: None,
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec![],
            dlight_endpoints: None,
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        };
        let rate_coins = vec![vrsc.clone(), vdex.clone()];
        let prioritized_channels = vec![(vdex.id.clone(), "vrpc.Raddr.iSystem".to_string())];

        let prioritized =
            prioritized_rate_coins(&rate_coins, &prioritized_channels, &HashSet::new());
        let prioritized_ids = prioritized
            .into_iter()
            .map(|coin| coin.id)
            .collect::<HashSet<_>>();

        assert!(prioritized_ids.contains(&vdex.id));
        assert!(prioritized_ids.contains(&vrsc.id));
    }

    #[test]
    fn coinpaprika_primary_candidates_follow_asset_classes() {
        let mut eth = sample_coin("ETH", "ETH", Protocol::Eth);
        eth.coin_paprika_id = Some("eth-ethereum".to_string());
        assert!(is_coinpaprika_primary_candidate(&eth));
        assert!(should_attempt_coinpaprika(&eth));

        let mut btc = sample_coin("BTC", "BTC", Protocol::Btc);
        btc.coin_paprika_id = Some("btc-bitcoin".to_string());
        assert!(is_coinpaprika_primary_candidate(&btc));
        assert!(should_attempt_coinpaprika(&btc));

        let mut vrsc = sample_coin("VRSC", VRSC_SYSTEM_ID, Protocol::Vrsc);
        vrsc.coin_paprika_id = Some("vrsc-verus-coin".to_string());
        assert!(is_coinpaprika_primary_candidate(&vrsc));
        assert!(should_attempt_coinpaprika(&vrsc));

        let veth = sample_coin(VETH_SYSTEM_ID, VRSC_SYSTEM_ID, Protocol::Vrsc);
        assert!(is_coinpaprika_primary_candidate(&veth));

        let mut veth_family = sample_coin("iUnknownBridgeAsset", VETH_SYSTEM_ID, Protocol::Vrsc);
        veth_family.display_ticker = "Unknown.vETH".to_string();
        assert!(is_coinpaprika_primary_candidate(&veth_family));
        assert!(
            !should_attempt_coinpaprika(&veth_family),
            "known coinpaprika id is required for primary assets"
        );

        let mut bridge = sample_coin(BRIDGE_VETH_CURRENCY_ID, VRSC_SYSTEM_ID, Protocol::Vrsc);
        bridge.display_ticker = "Bridge.vETH".to_string();
        bridge.display_name = "Bridge.vETH".to_string();
        assert!(!is_coinpaprika_primary_candidate(&bridge));
        assert!(!should_attempt_coinpaprika(&bridge));

        let pure = sample_coin(
            "iHax5qYQGbcMGqJKKrPorpzUBX2oFFXGnY",
            VRSC_SYSTEM_ID,
            Protocol::Vrsc,
        );
        assert!(!is_coinpaprika_primary_candidate(&pure));
        assert!(!should_attempt_coinpaprika(&pure));
    }

    #[test]
    fn strict_alias_fallback_copies_between_eth_and_veth() {
        let mut latest_rates =
            std::collections::HashMap::<String, std::collections::HashMap<String, f64>>::new();
        latest_rates.insert(
            VETH_SYSTEM_ID.to_string(),
            std::collections::HashMap::from([
                ("USD".to_string(), 2500.0),
                ("EUR".to_string(), 2300.0),
            ]),
        );

        let eth = sample_coin("ETH", "ETH", Protocol::Eth);
        let fallback = strict_alias_fallback_rates(&eth, &latest_rates);
        assert_eq!(
            fallback.and_then(|rates| rates.get("USD").copied()),
            Some(2500.0)
        );

        latest_rates.insert(
            "ETH".to_string(),
            std::collections::HashMap::from([("USD".to_string(), 2550.0)]),
        );
        let veth = sample_coin(VETH_SYSTEM_ID, VRSC_SYSTEM_ID, Protocol::Vrsc);
        let fallback_veth = strict_alias_fallback_rates(&veth, &latest_rates);
        assert_eq!(
            fallback_veth.and_then(|rates| rates.get("USD").copied()),
            Some(2550.0)
        );
    }

    #[test]
    fn pending_alias_backfill_resolves_missing_eth_after_veth() {
        let coins = vec![
            sample_coin("ETH", "ETH", Protocol::Eth),
            sample_coin(VETH_SYSTEM_ID, VRSC_SYSTEM_ID, Protocol::Vrsc),
        ];

        let latest_rates = std::collections::HashMap::from([(
            VETH_SYSTEM_ID.to_string(),
            std::collections::HashMap::from([("USD".to_string(), 2400.0)]),
        )]);

        let backfills = pending_strict_alias_backfill(&coins, &latest_rates);
        assert_eq!(backfills.len(), 1);
        assert!(backfills[0].0.eq_ignore_ascii_case("ETH"));
        assert_eq!(backfills[0].1.get("USD").copied(), Some(2400.0));
    }

    #[test]
    fn derive_vrsc_anchor_from_bridge_currency_result_uses_dai_guard() {
        let payload = json!({
            "bestcurrencystate": {
                "currencies": {
                    VRSC_SYSTEM_ID: { "lastconversionprice": 0.0005 },
                    DAI_VETH_CURRENCY_ID: { "lastconversionprice": 1.0 },
                    VUSDC_VETH_CURRENCY_ID: { "lastconversionprice": 1.0 }
                }
            }
        });

        let vrsc_usd = derive_vrsc_usd_anchor_from_bridge_currency_result(&payload);
        assert_eq!(vrsc_usd, Some(2000.0));
    }

    #[test]
    fn derive_vrsc_anchor_from_bridge_currency_result_rejects_dai_depeg() {
        let payload = json!({
            "bestcurrencystate": {
                "currencies": {
                    VRSC_SYSTEM_ID: { "lastconversionprice": 0.0005 },
                    DAI_VETH_CURRENCY_ID: { "lastconversionprice": 1.0 },
                    VUSDC_VETH_CURRENCY_ID: { "lastconversionprice": 2.0 }
                }
            }
        });

        let vrsc_usd = derive_vrsc_usd_anchor_from_bridge_currency_result(&payload);
        assert_eq!(vrsc_usd, None);
    }
}
