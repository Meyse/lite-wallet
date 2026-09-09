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

use crate::core::auth::SessionManager;
use crate::core::channels::btc::BtcProviderPool;
use crate::core::channels::eth::EthProviderPool;
use crate::core::channels::vrpc::VrpcProviderPool;
use crate::core::channels::{route_get_balances, route_get_info, route_get_transactions};
use crate::core::coins::{Channel, CoinDefinition, CoinRegistry, Protocol};
use crate::core::rates::cache::{
    unix_timestamp_secs, CachedEcbReferenceRates, CachedMarketRates, PublicRateIdentity,
    PublicRateSource, PublicRatesCache, ECB_REFERENCE_REFRESH_SECS, MARKET_RATE_REFRESH_SECS,
};
use crate::core::rates::{build_rates_http_client, coinpaprika, ecb, pbaas};
use crate::core::updates::events::{
    BalancesUpdatedPayload, BootstrapUpdatedPayload, InfoUpdatedPayload, RatesUpdatedPayload,
    TransactionsUpdatedPayload, UpdateErrorPayload,
};
use crate::core::updates::params::{
    jitter_duration, BALANCE_REFRESH_SECS, CHAIN_INFO_REFRESH_SECS, DLIGHT_POST_SYNC_REFRESH_SECS,
    DLIGHT_SYNC_BALANCE_REFRESH_SECS, DLIGHT_SYNC_INFO_REFRESH_SECS,
    DLIGHT_SYNC_TRANSACTION_REFRESH_SECS, TRANSACTION_REFRESH_SECS,
};
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

/// Tauri event names (frontend listens via listen()).
pub const EVENT_BALANCES_UPDATED: &str = "wallet://balances-updated";
pub const EVENT_TRANSACTIONS_UPDATED: &str = "wallet://transactions-updated";
pub const EVENT_INFO_UPDATED: &str = "wallet://info-updated";
pub const EVENT_RATES_UPDATED: &str = "wallet://rates-updated";
pub const EVENT_BOOTSTRAP_UPDATED: &str = "wallet://bootstrap-updated";
pub const EVENT_TX_SEND_PROGRESS: &str = "wallet://tx-send-progress";
pub const EVENT_ERROR: &str = "wallet://error";
const BOOTSTRAP_BALANCE_CONCURRENCY: usize = 4;
const BOOTSTRAP_RATE_CONCURRENCY: usize = 4;
const BOOTSTRAP_RATE_RESOLVE_TIMEOUT_SECS: u64 = 10;
const BOOTSTRAP_PBAAS_PROVIDER_TIMEOUT_SECS: u64 = 8;
const BOOTSTRAP_BRIDGE_VETH_LOOKUP_TIMEOUT_SECS: u64 = 8;
const BOOTSTRAP_RATE_SLOW_LOG_MS: u128 = 2_000;
const RATE_FAILURE_RETRY_BASE_SECS: u64 = 30;
const RATE_FAILURE_RETRY_MAX_SECS: u64 = 5 * 60;
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

#[derive(Clone, Debug)]
struct RateAttemptState {
    next_attempt_at: Instant,
    consecutive_failures: u32,
}

impl RateAttemptState {
    fn due(now: Instant) -> Self {
        Self {
            next_attempt_at: now,
            consecutive_failures: 0,
        }
    }

    fn is_due(&self, now: Instant) -> bool {
        now >= self.next_attempt_at
    }

    fn record_cached_success(&mut self, now: Instant, cache_age_secs: u64) {
        self.consecutive_failures = 0;
        let remaining = MARKET_RATE_REFRESH_SECS.saturating_sub(cache_age_secs);
        self.next_attempt_at = now + Duration::from_secs(remaining);
    }

    fn record_failure(&mut self, now: Instant) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.next_attempt_at =
            now + Duration::from_secs(rate_failure_retry_secs(self.consecutive_failures));
    }

    fn record_unsupported(&mut self, now: Instant) {
        self.consecutive_failures = 0;
        self.next_attempt_at = now + Duration::from_secs(MARKET_RATE_REFRESH_SECS);
    }
}

fn rate_failure_retry_secs(consecutive_failures: u32) -> u64 {
    let exponent = consecutive_failures.saturating_sub(1).min(8);
    RATE_FAILURE_RETRY_BASE_SECS
        .saturating_mul(1_u64 << exponent)
        .min(RATE_FAILURE_RETRY_MAX_SECS)
}

fn session_event_is_allowed(
    cancelled: bool,
    active_session_id: Option<&str>,
    expected_session_id: &str,
) -> bool {
    !cancelled && active_session_id == Some(expected_session_id)
}

async fn update_session_is_current(
    cancel_token: &CancellationToken,
    session_manager: &Arc<Mutex<SessionManager>>,
    expected_session_id: &str,
) -> bool {
    if cancel_token.is_cancelled() {
        return false;
    }
    let session = session_manager.lock().await;
    session_event_is_allowed(
        cancel_token.is_cancelled(),
        session.active_session_id(),
        expected_session_id,
    ) && session.is_unlocked()
}

/// Update engine: polls VRPC, BTC, ETH and ERC20 channels when unlocked, emits Tauri events.
/// Hold in tauri::State; start() from start_update_engine, stop() from lock_wallet.
pub struct UpdateEngine {
    cancel_token: Mutex<Option<CancellationToken>>,
    task_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    channel_state: Mutex<HashMap<String, ChannelState>>,
    public_rates_cache: Arc<Mutex<PublicRatesCache>>,
}

impl UpdateEngine {
    pub fn new() -> Self {
        Self {
            cancel_token: Mutex::new(None),
            task_handle: Mutex::new(None),
            channel_state: Mutex::new(HashMap::new()),
            public_rates_cache: Arc::new(Mutex::new(PublicRatesCache::default())),
        }
    }

    /// Start polling. Call after successful unlock. Spawns a single task that always runs
    /// balance polling and can optionally run transaction polling.
    pub async fn start(
        &self,
        app_handle: AppHandle,
        session_id: String,
        session_manager: Arc<Mutex<SessionManager>>,
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
        let public_rates_cache = Arc::clone(&self.public_rates_cache);
        let task_handle = tokio::spawn(async move {
            run_update_loop(
                child,
                app_handle,
                session_id,
                session_manager,
                coin_registry,
                vrpc_provider_pool,
                btc_provider_pool,
                eth_provider_pool,
                start_config,
                public_rates_cache,
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
    active_coin_ids: &HashSet<String>,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for c in coin_registry.get_all() {
        if c.is_testnet != is_testnet
            || !active_coin_ids.contains(&c.id.trim().to_ascii_lowercase())
        {
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

fn emit_rates(
    app_handle: &AppHandle,
    coin_id: &str,
    rates: &HashMap<String, f64>,
    usd_change_24h_pct: Option<f64>,
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
        true
    }
}

async fn emit_session_checked_rates(
    app_handle: &AppHandle,
    cancel_token: &CancellationToken,
    session_manager: &Arc<Mutex<SessionManager>>,
    session_id: &str,
    coin_id: &str,
    rates: &HashMap<String, f64>,
    usd_change_24h_pct: Option<f64>,
) -> bool {
    if !update_session_is_current(cancel_token, session_manager, session_id).await {
        return false;
    }
    emit_rates(app_handle, coin_id, rates, usd_change_24h_pct);
    true
}

fn rate_coin_ids_to_invalidate(
    rate_coins: &[CoinDefinition],
    previously_available: &HashMap<String, HashMap<String, f64>>,
    currently_available: &HashMap<String, HashMap<String, f64>>,
    invalidate_all_missing: bool,
) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    rate_coins
        .iter()
        .filter(|coin| {
            lookup_rates_case_insensitive(currently_available, &coin.id).is_none()
                && (invalidate_all_missing
                    || lookup_rates_case_insensitive(previously_available, &coin.id).is_some())
        })
        .filter_map(|coin| {
            let normalized = coin.id.trim().to_ascii_lowercase();
            seen.insert(normalized).then(|| coin.id.clone())
        })
        .collect()
}

async fn fetch_vrsc_anchor_from_bridge_veth(
    cancel_token: &CancellationToken,
    vrpc_provider_pool: &VrpcProviderPool,
    active_network: WalletNetwork,
    usd_reference_rates: &HashMap<String, f64>,
) -> Option<HashMap<String, f64>> {
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
        let payload = match tokio::select! {
            _ = cancel_token.cancelled() => return None,
            result = tokio::time::timeout(
                lookup_timeout,
                provider.getcurrency(BRIDGE_VETH_CURRENCY_ID),
            ) => result,
        } {
            Ok(Ok(payload)) => Some(payload),
            Ok(Err(_)) => {
                match tokio::select! {
                    _ = cancel_token.cancelled() => return None,
                    result = tokio::time::timeout(
                        lookup_timeout,
                        provider.getcurrency(BRIDGE_VETH_TICKER),
                    ) => result,
                } {
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

        return Some(rates);
    }

    None
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
    session_id: &str,
    prioritized_channels: &[(String, String)],
    session_manager: Arc<Mutex<SessionManager>>,
    coin_registry: Arc<CoinRegistry>,
    vrpc_provider_pool: Arc<VrpcProviderPool>,
    btc_provider_pool: Arc<BtcProviderPool>,
    eth_provider_pool: Arc<EthProviderPool>,
    channel_state: &mut HashMap<String, ChannelState>,
    first_balance_emitted: &mut bool,
    engine_started_at: Instant,
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

        let coin_id = coin_id.clone();
        let channel_id = channel_id.clone();
        let semaphore = Arc::clone(&semaphore);
        let session_manager = Arc::clone(&session_manager);
        let coin_registry = Arc::clone(&coin_registry);
        let vrpc_provider_pool = Arc::clone(&vrpc_provider_pool);
        let btc_provider_pool = Arc::clone(&btc_provider_pool);
        let eth_provider_pool = Arc::clone(&eth_provider_pool);
        let request_cancel_token = cancel_token.child_token();

        join_set.spawn(async move {
            let permit = semaphore.acquire_owned().await;
            let Ok(_permit) = permit else {
                return (coin_id, channel_id, Err(WalletError::OperationFailed));
            };
            let result = tokio::select! {
                _ = request_cancel_token.cancelled() => Err(WalletError::WalletLocked),
                result = route_get_balances(
                    &channel_id,
                    Some(coin_id.as_str()),
                    &session_manager,
                    coin_registry.as_ref(),
                    vrpc_provider_pool.as_ref(),
                    btc_provider_pool.as_ref(),
                    eth_provider_pool.as_ref(),
                ) => result,
            };
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

        if !update_session_is_current(cancel_token, &session_manager, session_id).await {
            join_set.abort_all();
            return;
        }

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
                } else if !*first_balance_emitted {
                    *first_balance_emitted = true;
                    println!(
                        "[WALLET_PERF] update_engine phase=first_balance_event_emitted elapsed_ms={}",
                        engine_started_at.elapsed().as_millis()
                    );
                }
                channel_state
                    .entry(format!("{}::{}", channel_id, coin_id))
                    .or_default()
                    .last_balance_fetch = Some(Instant::now());
            }
            Err(err) => {
                if should_emit_update_error(&channel_id, &err) {
                    let message = user_facing_error(&err);
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
                    .entry(format!("{}::{}", channel_id, coin_id))
                    .or_default()
                    .last_balance_fetch = Some(Instant::now());
            }
        }
    }
}

async fn publish_rate_result(
    app_handle: &AppHandle,
    cancel_token: &CancellationToken,
    session_manager: &Arc<Mutex<SessionManager>>,
    session_id: &str,
    active_network: WalletNetwork,
    coin: &CoinDefinition,
    rates: HashMap<String, f64>,
    usd_change_24h_pct: Option<f64>,
    source: PublicRateSource,
    fetched_at_unix_secs: u64,
    public_rates_cache: &Arc<Mutex<PublicRatesCache>>,
    latest_rates: &mut HashMap<String, HashMap<String, f64>>,
    first_rate_emitted: &mut bool,
    engine_started_at: Instant,
) -> bool {
    if !update_session_is_current(cancel_token, session_manager, session_id).await {
        return false;
    }
    let Some(usd_price) = rates
        .get(ecb::USD)
        .copied()
        .filter(|value| value.is_finite() && *value > 0.0)
    else {
        return false;
    };

    public_rates_cache
        .lock()
        .await
        .store_market_rate(CachedMarketRates {
            identity: PublicRateIdentity::for_coin(active_network, coin),
            rates: rates.clone(),
            usd_price,
            usd_change_24h_pct,
            source,
            fetched_at_unix_secs,
        });
    latest_rates.insert(coin.id.clone(), rates.clone());

    if emit_rates(app_handle, &coin.id, &rates, usd_change_24h_pct) {
        if !*first_rate_emitted {
            *first_rate_emitted = true;
            println!(
                "[WALLET_PERF] update_engine phase=first_non_empty_rate_event_emitted elapsed_ms={}",
                engine_started_at.elapsed().as_millis()
            );
        }
    }
    true
}

async fn publish_cached_rates(
    app_handle: &AppHandle,
    cancel_token: &CancellationToken,
    session_manager: &Arc<Mutex<SessionManager>>,
    session_id: &str,
    active_network: WalletNetwork,
    coins: &[CoinDefinition],
    public_rates_cache: &Arc<Mutex<PublicRatesCache>>,
    latest_rates: &mut HashMap<String, HashMap<String, f64>>,
    coin_rates_state: &mut HashMap<PublicRateIdentity, RateAttemptState>,
    first_rate_emitted: &mut bool,
    engine_started_at: Instant,
) -> bool {
    let now_unix_secs = unix_timestamp_secs();
    let cached_entries =
        displayable_cached_rate_entries(active_network, coins, now_unix_secs, public_rates_cache)
            .await;

    for (coin, entry) in cached_entries {
        if !update_session_is_current(cancel_token, session_manager, session_id).await {
            return false;
        }
        latest_rates.insert(coin.id.clone(), entry.rates.clone());
        coin_rates_state
            .entry(entry.identity.clone())
            .or_insert_with(|| RateAttemptState::due(Instant::now()))
            .record_cached_success(Instant::now(), entry.age_secs(now_unix_secs));
        if emit_rates(app_handle, &coin.id, &entry.rates, entry.usd_change_24h_pct)
            && !*first_rate_emitted
        {
            *first_rate_emitted = true;
            println!(
                "[WALLET_PERF] update_engine phase=first_non_empty_rate_event_emitted source=cache elapsed_ms={}",
                engine_started_at.elapsed().as_millis()
            );
        }
        println!(
            "[UPDATE] Reused public rate cache: coin={} source={} age_secs={}",
            coin.id,
            entry.source.label(),
            entry.age_secs(now_unix_secs)
        );
    }

    for coin_id in rate_coin_ids_to_invalidate(coins, latest_rates, latest_rates, true) {
        if !emit_session_checked_rates(
            app_handle,
            cancel_token,
            session_manager,
            session_id,
            &coin_id,
            &HashMap::new(),
            None,
        )
        .await
        {
            return false;
        }
        println!(
            "[UPDATE] Invalidated unavailable public rate on engine start: coin={}",
            coin_id
        );
    }
    true
}

async fn displayable_cached_rate_entries(
    active_network: WalletNetwork,
    coins: &[CoinDefinition],
    now_unix_secs: u64,
    public_rates_cache: &Arc<Mutex<PublicRatesCache>>,
) -> Vec<(CoinDefinition, CachedMarketRates)> {
    {
        let mut cache = public_rates_cache.lock().await;
        let usd_reference_rates = cache
            .usable_ecb_reference_rates(now_unix_secs)
            .map(|entry| entry.rates)
            .unwrap_or_else(|| HashMap::from([(ecb::USD.to_string(), 1.0)]));
        let mut entries = coins
            .iter()
            .filter_map(|coin| {
                cache
                    .market_rate(active_network, coin)
                    .filter(|entry| entry.is_displayable(now_unix_secs))
                    .cloned()
                    .map(|entry| (coin.clone(), entry))
            })
            .collect::<Vec<_>>();
        for (_, entry) in &mut entries {
            entry.rates = ecb::build_coin_fiat_rates(entry.usd_price, &usd_reference_rates);
            cache.store_market_rate(entry.clone());
        }
        entries
    }
}

async fn alias_cache_metadata(
    public_rates_cache: &Arc<Mutex<PublicRatesCache>>,
    active_network: WalletNetwork,
    rate_coins: &[CoinDefinition],
    counterpart_coin_id: &str,
) -> (u64, Option<f64>) {
    let counterpart = rate_coins
        .iter()
        .find(|coin| coin.id.trim().eq_ignore_ascii_case(counterpart_coin_id));
    let Some(counterpart) = counterpart else {
        return (unix_timestamp_secs(), None);
    };
    let cache = public_rates_cache.lock().await;
    cache
        .market_rate(active_network, counterpart)
        .map(|entry| (entry.fetched_at_unix_secs, entry.usd_change_24h_pct))
        .unwrap_or_else(|| (unix_timestamp_secs(), None))
}

async fn run_rate_refresh_cycle(
    app_handle: &AppHandle,
    cancel_token: &CancellationToken,
    session_manager: &Arc<Mutex<SessionManager>>,
    session_id: &str,
    active_network: WalletNetwork,
    due_coins: &[CoinDefinition],
    rate_coins: &[CoinDefinition],
    rates_http_client: &reqwest::Client,
    vrpc_provider_pool: &Arc<VrpcProviderPool>,
    usd_reference_rates: &HashMap<String, f64>,
    public_rates_cache: &Arc<Mutex<PublicRatesCache>>,
    latest_rates: &mut HashMap<String, HashMap<String, f64>>,
    coin_rates_state: &mut HashMap<PublicRateIdentity, RateAttemptState>,
    first_rate_emitted: &mut bool,
    engine_started_at: Instant,
) -> bool {
    if due_coins.is_empty() {
        return true;
    }

    let cycle_started_at = Instant::now();
    let semaphore = Arc::new(Semaphore::new(BOOTSTRAP_RATE_CONCURRENCY));
    let mut direct_tasks = JoinSet::new();
    let mut attempted = HashSet::<PublicRateIdentity>::new();
    let mut succeeded = HashSet::<PublicRateIdentity>::new();
    let mut direct_errors = HashMap::<PublicRateIdentity, String>::new();

    for coin in due_coins
        .iter()
        .filter(|coin| should_attempt_coinpaprika(coin))
    {
        let coin = coin.clone();
        let identity = PublicRateIdentity::for_coin(active_network, &coin);
        attempted.insert(identity);
        let semaphore = Arc::clone(&semaphore);
        let client = rates_http_client.clone();
        let request_cancel_token = cancel_token.child_token();
        direct_tasks.spawn(async move {
            let Ok(_permit) = semaphore.acquire_owned().await else {
                return (coin, Err("rate worker unavailable".to_string()));
            };
            let result = tokio::select! {
                _ = request_cancel_token.cancelled() => Err("rate request cancelled".to_string()),
                result = tokio::time::timeout(
                    Duration::from_secs(BOOTSTRAP_RATE_RESOLVE_TIMEOUT_SECS),
                    coinpaprika::fetch_usd_metrics(&client, &coin),
                ) => match result {
                    Ok(result) => result,
                    Err(_) => Err(format!(
                        "rate request timed out after {}s",
                        BOOTSTRAP_RATE_RESOLVE_TIMEOUT_SECS
                    )),
                },
            };
            (coin, result)
        });
    }

    while let Some(task_result) = direct_tasks.join_next().await {
        let (coin, result) = match task_result {
            Ok(result) => result,
            Err(error) => {
                println!("[UPDATE] Direct rate task join error: {}", error);
                continue;
            }
        };
        if !update_session_is_current(cancel_token, session_manager, session_id).await {
            direct_tasks.abort_all();
            return false;
        }

        let identity = PublicRateIdentity::for_coin(active_network, &coin);
        match result {
            Ok(metrics) => {
                let rates = ecb::build_coin_fiat_rates(metrics.usd_price, usd_reference_rates);
                if publish_rate_result(
                    app_handle,
                    cancel_token,
                    session_manager,
                    session_id,
                    active_network,
                    &coin,
                    rates,
                    metrics.usd_change_24h_pct,
                    PublicRateSource::CoinPaprika,
                    unix_timestamp_secs(),
                    public_rates_cache,
                    latest_rates,
                    first_rate_emitted,
                    engine_started_at,
                )
                .await
                {
                    succeeded.insert(identity);
                } else {
                    return false;
                }
            }
            Err(error) => {
                direct_errors.insert(identity, error);
            }
        }
    }

    let vrsc_coin = rate_coins
        .iter()
        .find(|coin| coin.id.trim().eq_ignore_ascii_case(VRSC_COIN_ID));
    if let Some(vrsc_coin) = vrsc_coin {
        let vrsc_identity = PublicRateIdentity::for_coin(active_network, vrsc_coin);
        let direct_failed = direct_errors.contains_key(&vrsc_identity);
        let anchor_missing = lookup_rates_case_insensitive(latest_rates, VRSC_COIN_ID).is_none();
        if direct_failed && anchor_missing {
            println!("[UPDATE] Attempting Bridge.vETH seed after direct VRSC rate miss");
            if let Some(rates) = fetch_vrsc_anchor_from_bridge_veth(
                cancel_token,
                vrpc_provider_pool.as_ref(),
                active_network,
                usd_reference_rates,
            )
            .await
            {
                if publish_rate_result(
                    app_handle,
                    cancel_token,
                    session_manager,
                    session_id,
                    active_network,
                    vrsc_coin,
                    rates,
                    None,
                    PublicRateSource::BridgeVeth,
                    unix_timestamp_secs(),
                    public_rates_cache,
                    latest_rates,
                    first_rate_emitted,
                    engine_started_at,
                )
                .await
                {
                    println!("[UPDATE] Seeded VRSC anchor rate from Bridge.vETH DAI path");
                    succeeded.insert(vrsc_identity);
                } else {
                    return false;
                }
            }
        }
    }

    let latest_snapshot = Arc::new(latest_rates.clone());
    let mut pbaas_tasks = JoinSet::new();
    for coin in due_coins.iter().filter(|coin| {
        let identity = PublicRateIdentity::for_coin(active_network, coin);
        !succeeded.contains(&identity) && pbaas::is_pbaas_derivation_candidate(coin)
    }) {
        let coin = coin.clone();
        attempted.insert(PublicRateIdentity::for_coin(active_network, &coin));
        let semaphore = Arc::clone(&semaphore);
        let provider_pool = Arc::clone(vrpc_provider_pool);
        let latest_snapshot = Arc::clone(&latest_snapshot);
        let request_cancel_token = cancel_token.child_token();
        pbaas_tasks.spawn(async move {
            let Ok(_permit) = semaphore.acquire_owned().await else {
                return (coin, None);
            };
            let result = tokio::select! {
                _ = request_cancel_token.cancelled() => None,
                result = tokio::time::timeout(
                    Duration::from_secs(BOOTSTRAP_RATE_RESOLVE_TIMEOUT_SECS),
                    derive_pbaas_rates_with_provider_candidates(
                        provider_pool.as_ref(),
                        active_network,
                        &coin,
                        latest_snapshot.as_ref(),
                    ),
                ) => result.ok().flatten(),
            };
            (coin, result)
        });
    }

    while let Some(task_result) = pbaas_tasks.join_next().await {
        let (coin, rates) = match task_result {
            Ok(result) => result,
            Err(error) => {
                println!("[UPDATE] PBaaS rate task join error: {}", error);
                continue;
            }
        };
        if !update_session_is_current(cancel_token, session_manager, session_id).await {
            pbaas_tasks.abort_all();
            return false;
        }
        let identity = PublicRateIdentity::for_coin(active_network, &coin);
        if let Some(rates) = rates {
            if publish_rate_result(
                app_handle,
                cancel_token,
                session_manager,
                session_id,
                active_network,
                &coin,
                rates,
                None,
                PublicRateSource::Pbaas,
                unix_timestamp_secs(),
                public_rates_cache,
                latest_rates,
                first_rate_emitted,
                engine_started_at,
            )
            .await
            {
                succeeded.insert(identity);
            } else {
                return false;
            }
        }
    }

    let alias_backfills = pending_strict_alias_backfill(due_coins, latest_rates);
    for (coin_id, rates) in alias_backfills {
        let Some(coin) = due_coins
            .iter()
            .find(|coin| coin.id.trim().eq_ignore_ascii_case(&coin_id))
        else {
            continue;
        };
        let identity = PublicRateIdentity::for_coin(active_network, coin);
        if succeeded.contains(&identity) {
            continue;
        }
        let Some(counterpart_coin_id) = strict_alias_counterpart_coin_id(coin) else {
            continue;
        };
        let (fetched_at_unix_secs, usd_change_24h_pct) = alias_cache_metadata(
            public_rates_cache,
            active_network,
            rate_coins,
            counterpart_coin_id,
        )
        .await;
        if publish_rate_result(
            app_handle,
            cancel_token,
            session_manager,
            session_id,
            active_network,
            coin,
            rates,
            usd_change_24h_pct,
            PublicRateSource::StrictAlias {
                counterpart_coin_id: counterpart_coin_id.to_string(),
            },
            fetched_at_unix_secs,
            public_rates_cache,
            latest_rates,
            first_rate_emitted,
            engine_started_at,
        )
        .await
        {
            succeeded.insert(identity);
        } else {
            return false;
        }
    }

    let now = Instant::now();
    let now_unix_secs = unix_timestamp_secs();
    for coin in due_coins {
        let identity = PublicRateIdentity::for_coin(active_network, coin);
        let state = coin_rates_state
            .entry(identity.clone())
            .or_insert_with(|| RateAttemptState::due(now));
        if succeeded.contains(&identity) {
            let cache_age_secs = {
                let cache = public_rates_cache.lock().await;
                cache
                    .market_rate(active_network, coin)
                    .map(|entry| entry.age_secs(now_unix_secs))
                    .unwrap_or_default()
            };
            state.record_cached_success(now, cache_age_secs);
        } else if attempted.contains(&identity) {
            state.record_failure(now);
            if let Some(error) = direct_errors.get(&identity) {
                println!(
                    "[UPDATE] Fiat rate unavailable for {}: {}; retry_secs={}",
                    coin.id,
                    error,
                    rate_failure_retry_secs(state.consecutive_failures)
                );
            } else {
                println!(
                    "[UPDATE] Fiat rate unavailable for {}; retry_secs={}",
                    coin.id,
                    rate_failure_retry_secs(state.consecutive_failures)
                );
            }
        } else {
            state.record_unsupported(now);
        }
    }

    println!(
        "[UPDATE] Rate cycle complete: candidates={} elapsed_ms={}",
        due_coins.len(),
        cycle_started_at.elapsed().as_millis()
    );
    true
}

async fn apply_ecb_refresh(
    app_handle: &AppHandle,
    cancel_token: &CancellationToken,
    session_manager: &Arc<Mutex<SessionManager>>,
    session_id: &str,
    active_network: WalletNetwork,
    rate_coins: &[CoinDefinition],
    snapshot: CachedEcbReferenceRates,
    public_rates_cache: &Arc<Mutex<PublicRatesCache>>,
    latest_rates: &mut HashMap<String, HashMap<String, f64>>,
    first_rate_emitted: &mut bool,
    engine_started_at: Instant,
) -> bool {
    if !update_session_is_current(cancel_token, session_manager, session_id).await {
        return false;
    }

    let now_unix_secs = unix_timestamp_secs();
    let entries = {
        let mut cache = public_rates_cache.lock().await;
        cache.store_ecb_reference_rates(snapshot.clone());
        let entries = cache.displayable_market_rates(active_network, rate_coins, now_unix_secs);
        for entry in &entries {
            let mut refreshed = entry.clone();
            refreshed.rates = ecb::build_coin_fiat_rates(refreshed.usd_price, &snapshot.rates);
            cache.store_market_rate(refreshed);
        }
        entries
    };

    for entry in entries {
        if !update_session_is_current(cancel_token, session_manager, session_id).await {
            return false;
        }
        let Some(coin) = rate_coins
            .iter()
            .find(|coin| PublicRateIdentity::for_coin(active_network, coin) == entry.identity)
        else {
            continue;
        };
        let rates = ecb::build_coin_fiat_rates(entry.usd_price, &snapshot.rates);
        latest_rates.insert(coin.id.clone(), rates.clone());
        if emit_rates(app_handle, &coin.id, &rates, entry.usd_change_24h_pct)
            && !*first_rate_emitted
        {
            *first_rate_emitted = true;
            println!(
                "[WALLET_PERF] update_engine phase=first_non_empty_rate_event_emitted source=ecb_refresh elapsed_ms={}",
                engine_started_at.elapsed().as_millis()
            );
        }
    }
    println!(
        "[UPDATE] ECB reference cache refreshed: published_on={}",
        snapshot.published_on.as_deref().unwrap_or("unknown")
    );
    true
}

async fn run_rate_schedule(
    cancel_token: CancellationToken,
    app_handle: AppHandle,
    session_id: String,
    session_manager: Arc<Mutex<SessionManager>>,
    coin_registry: Arc<CoinRegistry>,
    vrpc_provider_pool: Arc<VrpcProviderPool>,
    priority_coin_ids: HashSet<String>,
    public_rates_cache: Arc<Mutex<PublicRatesCache>>,
    engine_started_at: Instant,
) {
    let access_context = {
        let session = session_manager.lock().await;
        if !session_event_is_allowed(
            cancel_token.is_cancelled(),
            session.active_session_id(),
            &session_id,
        ) || !session.is_unlocked()
        {
            return;
        }
        match session.active_wallet_access_context() {
            Ok(context) => context,
            Err(_) => return,
        }
    };
    let active_network = access_context.wallet_network;
    let is_testnet = matches!(active_network, WalletNetwork::Testnet);
    let all_rate_coins = fiat_rate_candidates(coin_registry.as_ref(), is_testnet);
    let rate_coins = prioritized_rate_coins(&all_rate_coins, &[], &priority_coin_ids);
    if rate_coins.is_empty() {
        return;
    }

    let rates_http_client = build_rates_http_client();
    let mut latest_rates = HashMap::<String, HashMap<String, f64>>::new();
    let mut coin_rates_state = rate_coins
        .iter()
        .map(|coin| {
            (
                PublicRateIdentity::for_coin(active_network, coin),
                RateAttemptState::due(Instant::now()),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut first_rate_emitted = false;
    let mut ecb_failures = 0_u32;
    let mut next_ecb_attempt_at = Instant::now();

    if !publish_cached_rates(
        &app_handle,
        &cancel_token,
        &session_manager,
        &session_id,
        active_network,
        &rate_coins,
        &public_rates_cache,
        &mut latest_rates,
        &mut coin_rates_state,
        &mut first_rate_emitted,
        engine_started_at,
    )
    .await
    {
        return;
    }

    loop {
        if !update_session_is_current(&cancel_token, &session_manager, &session_id).await {
            return;
        }

        let now = Instant::now();
        let now_unix_secs = unix_timestamp_secs();
        let cached_entries = displayable_cached_rate_entries(
            active_network,
            &rate_coins,
            now_unix_secs,
            &public_rates_cache,
        )
        .await;
        let next_latest_rates = cached_entries
            .iter()
            .map(|(coin, entry)| (coin.id.clone(), entry.rates.clone()))
            .collect::<HashMap<_, _>>();

        for (coin, entry) in &cached_entries {
            let previously_published = lookup_rates_case_insensitive(&latest_rates, &coin.id);
            if previously_published == Some(&entry.rates) {
                continue;
            }
            if !emit_session_checked_rates(
                &app_handle,
                &cancel_token,
                &session_manager,
                &session_id,
                &coin.id,
                &entry.rates,
                entry.usd_change_24h_pct,
            )
            .await
            {
                return;
            }
        }
        for coin_id in
            rate_coin_ids_to_invalidate(&rate_coins, &latest_rates, &next_latest_rates, false)
        {
            if !emit_session_checked_rates(
                &app_handle,
                &cancel_token,
                &session_manager,
                &session_id,
                &coin_id,
                &HashMap::new(),
                None,
            )
            .await
            {
                return;
            }
            println!("[UPDATE] Invalidated expired public rate: coin={}", coin_id);
        }
        latest_rates = next_latest_rates;
        let due_coins = rate_coins
            .iter()
            .filter(|coin| {
                coin_rates_state
                    .get(&PublicRateIdentity::for_coin(active_network, coin))
                    .map_or(true, |state| state.is_due(now))
            })
            .cloned()
            .collect::<Vec<_>>();
        let (usd_reference_rates, cache_ecb_refresh_due) = {
            let cache = public_rates_cache.lock().await;
            let reference_rates = cache
                .usable_ecb_reference_rates(now_unix_secs)
                .map(|entry| entry.rates)
                .unwrap_or_else(|| HashMap::from([(ecb::USD.to_string(), 1.0)]));
            (reference_rates, cache.ecb_needs_refresh(now_unix_secs))
        };
        let ecb_refresh_due = cache_ecb_refresh_due && now >= next_ecb_attempt_at;

        if !due_coins.is_empty() || ecb_refresh_due {
            let rate_cycle = run_rate_refresh_cycle(
                &app_handle,
                &cancel_token,
                &session_manager,
                &session_id,
                active_network,
                &due_coins,
                &rate_coins,
                &rates_http_client,
                &vrpc_provider_pool,
                &usd_reference_rates,
                &public_rates_cache,
                &mut latest_rates,
                &mut coin_rates_state,
                &mut first_rate_emitted,
                engine_started_at,
            );
            let ecb_refresh = async {
                if !ecb_refresh_due {
                    return None;
                }
                tokio::select! {
                    _ = cancel_token.cancelled() => None,
                    result = ecb::fetch_usd_reference_snapshot(&rates_http_client) => Some(result),
                }
            };
            let (cycle_current, ecb_result) = tokio::join!(rate_cycle, ecb_refresh);
            if !cycle_current {
                return;
            }
            if let Some(result) = ecb_result {
                match result {
                    Ok(snapshot) => {
                        let fetched_at_unix_secs = unix_timestamp_secs();
                        let cached_snapshot = CachedEcbReferenceRates {
                            rates: snapshot.rates,
                            published_on: snapshot.published_on,
                            fetched_at_unix_secs,
                        };
                        if !cached_snapshot.is_usable(fetched_at_unix_secs) {
                            ecb_failures = ecb_failures.saturating_add(1);
                            let retry_secs = rate_failure_retry_secs(ecb_failures);
                            next_ecb_attempt_at = Instant::now() + Duration::from_secs(retry_secs);
                            println!(
                                "[UPDATE] ECB reference refresh rejected: published_on={} publication_age_days={:?}; retry_secs={}",
                                cached_snapshot.published_on.as_deref().unwrap_or("missing"),
                                cached_snapshot.publication_age_days(fetched_at_unix_secs),
                                retry_secs
                            );
                        } else {
                            if !apply_ecb_refresh(
                                &app_handle,
                                &cancel_token,
                                &session_manager,
                                &session_id,
                                active_network,
                                &rate_coins,
                                cached_snapshot,
                                &public_rates_cache,
                                &mut latest_rates,
                                &mut first_rate_emitted,
                                engine_started_at,
                            )
                            .await
                            {
                                return;
                            }
                            ecb_failures = 0;
                            next_ecb_attempt_at =
                                Instant::now() + Duration::from_secs(ECB_REFERENCE_REFRESH_SECS);
                        }
                    }
                    Err(error) => {
                        ecb_failures = ecb_failures.saturating_add(1);
                        let retry_secs = rate_failure_retry_secs(ecb_failures);
                        next_ecb_attempt_at = Instant::now() + Duration::from_secs(retry_secs);
                        println!(
                            "[UPDATE] ECB reference refresh unavailable: {}; retry_secs={}",
                            error, retry_secs
                        );
                    }
                }
            }
        }

        let sleep_secs = coin_rates_state
            .values()
            .map(|state| {
                state
                    .next_attempt_at
                    .saturating_duration_since(Instant::now())
                    .as_secs()
                    .max(1)
            })
            .min()
            .unwrap_or(30)
            .min(30);
        tokio::select! {
            _ = cancel_token.cancelled() => return,
            _ = tokio::time::sleep(jitter_duration(sleep_secs)) => {}
        }
    }
}

async fn run_update_loop(
    cancel_token: CancellationToken,
    app_handle: AppHandle,
    session_id: String,
    session_manager: Arc<Mutex<SessionManager>>,
    coin_registry: Arc<CoinRegistry>,
    vrpc_provider_pool: Arc<VrpcProviderPool>,
    btc_provider_pool: Arc<BtcProviderPool>,
    eth_provider_pool: Arc<EthProviderPool>,
    start_config: UpdateEngineStartConfig,
    public_rates_cache: Arc<Mutex<PublicRatesCache>>,
) {
    let engine_started_at = Instant::now();
    let priority_coin_ids = normalize_priority_entries(&start_config.priority_coin_ids);
    let balance_schedule = run_balance_schedule(
        cancel_token.child_token(),
        app_handle.clone(),
        session_id.clone(),
        Arc::clone(&session_manager),
        Arc::clone(&coin_registry),
        Arc::clone(&vrpc_provider_pool),
        btc_provider_pool,
        eth_provider_pool,
        start_config,
        engine_started_at,
    );
    let rate_schedule = run_rate_schedule(
        cancel_token.child_token(),
        app_handle,
        session_id,
        session_manager,
        coin_registry,
        vrpc_provider_pool,
        priority_coin_ids,
        public_rates_cache,
        engine_started_at,
    );

    run_independent_update_schedules(balance_schedule, rate_schedule).await;
}

async fn run_independent_update_schedules<BalanceSchedule, RateSchedule>(
    balance_schedule: BalanceSchedule,
    rate_schedule: RateSchedule,
) where
    BalanceSchedule: std::future::Future<Output = ()>,
    RateSchedule: std::future::Future<Output = ()>,
{
    // Both schedules are polled independently: price timeouts never hold the balance cadence.
    tokio::join!(balance_schedule, rate_schedule);
}

async fn run_balance_schedule(
    cancel_token: CancellationToken,
    app_handle: AppHandle,
    session_id: String,
    session_manager: Arc<Mutex<SessionManager>>,
    coin_registry: Arc<CoinRegistry>,
    vrpc_provider_pool: Arc<VrpcProviderPool>,
    btc_provider_pool: Arc<BtcProviderPool>,
    eth_provider_pool: Arc<EthProviderPool>,
    start_config: UpdateEngineStartConfig,
    engine_started_at: Instant,
) {
    let mut channel_state: HashMap<String, ChannelState> = HashMap::new();
    let poll_transactions = start_config.poll_transactions;
    let priority_coin_ids = normalize_priority_entries(&start_config.priority_coin_ids);
    let priority_channel_ids = normalize_priority_entries(&start_config.priority_channel_ids);
    let dlight_fast_updates = dlight_fast_sync_updates_enabled();
    let mut bootstrap_completed = false;
    let mut first_balance_emitted = false;
    emit_bootstrap_updated(&app_handle, true);
    println!(
        "[UPDATE] dlight fast sync updates enabled={}",
        dlight_fast_updates
    );

    let access_context = {
        let session = session_manager.lock().await;
        if !session_event_is_allowed(
            cancel_token.is_cancelled(),
            session.active_session_id(),
            &session_id,
        ) || !session.is_unlocked()
        {
            return;
        }
        match session.active_wallet_access_context() {
            Ok(context) => context,
            Err(_) => return,
        }
    };
    let session_vrpc_address = access_context.vrsc_address.clone();
    let active_network = access_context.wallet_network;
    let is_testnet = matches!(active_network, WalletNetwork::Testnet);
    let mut channels = dedupe_channel_pairs(&active_channels(
        &coin_registry,
        is_testnet,
        &session_vrpc_address,
        eth_provider_pool.is_enabled(),
        None,
        &priority_coin_ids,
    ));

    // Shielded metadata opens a separate Stronghold snapshot. It must not delay
    // transparent balance polling or the independent public-rate schedule.
    let mut dlight_tasks = JoinSet::new();
    dlight_tasks.spawn(async move {
        let account_id = access_context.account_id.clone();
        let result = access_context.load_dlight_public_metadata_cached().await;
        (account_id, result)
    });
    let mut dlight_resolved = false;

    loop {
        if !update_session_is_current(&cancel_token, &session_manager, &session_id).await {
            break;
        }

        if !dlight_resolved {
            if let Some(task_result) = dlight_tasks.try_join_next() {
                dlight_resolved = true;
                match task_result {
                    Ok((_, Ok(metadata))) => {
                        channels = dedupe_channel_pairs(&active_channels(
                            &coin_registry,
                            is_testnet,
                            &session_vrpc_address,
                            eth_provider_pool.is_enabled(),
                            metadata.shielded_address.as_deref(),
                            &priority_coin_ids,
                        ));
                    }
                    Ok((account_id, Err(error))) => {
                        println!(
                            "[UPDATE] Failed to resolve dlight status for {}: {:?}",
                            account_id, error
                        );
                    }
                    Err(error) => {
                        println!("[UPDATE] dlight metadata task failed: {}", error);
                    }
                }
            }
        }

        if channels.is_empty() {
            if !dlight_resolved {
                tokio::select! {
                    _ = cancel_token.cancelled() => break,
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {}
                }
                continue;
            }
            if !bootstrap_completed {
                emit_bootstrap_updated(&app_handle, false);
                bootstrap_completed = true;
            }
            tokio::select! {
                _ = cancel_token.cancelled() => break,
                _ = tokio::time::sleep(jitter_duration(30)) => {}
            }
            continue;
        }

        if !bootstrap_completed {
            let bootstrap_started_at = Instant::now();
            let (prioritized_channels, remainder_channels) =
                partition_bootstrap_channels(&channels, &priority_coin_ids, &priority_channel_ids);
            println!(
                "[UPDATE] Balance bootstrap start: prioritized_channels={} remaining_channels={}",
                prioritized_channels.len(),
                remainder_channels.len()
            );

            run_bootstrap_balance_fetches(
                &app_handle,
                &cancel_token,
                &session_id,
                &prioritized_channels,
                Arc::clone(&session_manager),
                Arc::clone(&coin_registry),
                Arc::clone(&vrpc_provider_pool),
                Arc::clone(&btc_provider_pool),
                Arc::clone(&eth_provider_pool),
                &mut channel_state,
                &mut first_balance_emitted,
                engine_started_at,
            )
            .await;
            if !update_session_is_current(&cancel_token, &session_manager, &session_id).await {
                return;
            }

            emit_bootstrap_updated(&app_handle, false);
            bootstrap_completed = true;
            println!(
                "[UPDATE] Balance bootstrap complete: prioritized_channels={} elapsed_ms={}",
                prioritized_channels.len(),
                bootstrap_started_at.elapsed().as_millis()
            );
        }

        let now = Instant::now();

        let mut due_balance_channels = Vec::new();
        for (coin_id, channel_id) in &channels {
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
                due_balance_channels.push((coin_id.clone(), channel_id.clone()));
            }
        }
        run_bootstrap_balance_fetches(
            &app_handle,
            &cancel_token,
            &session_id,
            &due_balance_channels,
            Arc::clone(&session_manager),
            Arc::clone(&coin_registry),
            Arc::clone(&vrpc_provider_pool),
            Arc::clone(&btc_provider_pool),
            Arc::clone(&eth_provider_pool),
            &mut channel_state,
            &mut first_balance_emitted,
            engine_started_at,
        )
        .await;

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
                let result = route_get_info(
                    channel_id,
                    Some(coin_id.as_str()),
                    &session_manager,
                    coin_registry.as_ref(),
                    vrpc_provider_pool.as_ref(),
                )
                .await;
                if !update_session_is_current(&cancel_token, &session_manager, &session_id).await {
                    return;
                }
                match result {
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
                    let result = route_get_transactions(
                        channel_id,
                        Some(coin_id.as_str()),
                        &session_manager,
                        coin_registry.as_ref(),
                        vrpc_provider_pool.as_ref(),
                        btc_provider_pool.as_ref(),
                        eth_provider_pool.as_ref(),
                    )
                    .await;
                    if !update_session_is_current(&cancel_token, &session_manager, &session_id)
                        .await
                    {
                        return;
                    }
                    match result {
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

        let sleep_secs = if !dlight_resolved
            || should_use_fast_loop_sleep(dlight_fast_updates, &channels, &channel_state)
        {
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
        displayable_cached_rate_entries, fiat_rate_candidates, is_coinpaprika_primary_candidate,
        partition_bootstrap_channels, pending_strict_alias_backfill, prioritized_rate_coins,
        rate_coin_ids_to_invalidate, rate_failure_retry_secs, run_independent_update_schedules,
        session_event_is_allowed, should_attempt_coinpaprika, strict_alias_fallback_rates,
        RateAttemptState, BRIDGE_VETH_CURRENCY_ID, DAI_VETH_CURRENCY_ID,
        RATE_FAILURE_RETRY_MAX_SECS, VETH_SYSTEM_ID, VRSC_SYSTEM_ID, VUSDC_VETH_CURRENCY_ID,
    };
    use crate::core::coins::{Channel, CoinDefinition, CoinRegistry, Protocol};
    use crate::core::rates::cache::{
        CachedMarketRates, PublicRateIdentity, PublicRateSource, PublicRatesCache,
        MARKET_RATE_DISPLAY_MAX_AGE_SECS,
    };
    use crate::types::wallet::WalletNetwork;
    use serde_json::json;
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    use std::time::Instant;
    use tokio::sync::Mutex;

    #[test]
    fn stale_session_and_cancelled_work_cannot_publish() {
        assert!(session_event_is_allowed(false, Some("current"), "current"));
        assert!(!session_event_is_allowed(
            false,
            Some("replacement"),
            "current"
        ));
        assert!(!session_event_is_allowed(false, None, "current"));
        assert!(!session_event_is_allowed(true, Some("current"), "current"));
    }

    #[test]
    fn transient_rate_retry_uses_bounded_exponential_backoff() {
        assert_eq!(rate_failure_retry_secs(1), 30);
        assert_eq!(rate_failure_retry_secs(2), 60);
        assert_eq!(rate_failure_retry_secs(3), 120);
        assert_eq!(rate_failure_retry_secs(20), RATE_FAILURE_RETRY_MAX_SECS);
    }

    #[tokio::test]
    async fn expired_rate_is_invalidated_when_failed_refresh_does_not_replace_it() {
        let coin = sample_coin("VRSC", VRSC_SYSTEM_ID, Protocol::Vrsc);
        let fetched_at_unix_secs = 10_000;
        let public_rates_cache = Arc::new(Mutex::new(PublicRatesCache::default()));
        public_rates_cache
            .lock()
            .await
            .store_market_rate(CachedMarketRates {
                identity: PublicRateIdentity::for_coin(WalletNetwork::Mainnet, &coin),
                rates: HashMap::from([("USD".to_string(), 2.0)]),
                usd_price: 2.0,
                usd_change_24h_pct: Some(1.0),
                source: PublicRateSource::CoinPaprika,
                fetched_at_unix_secs,
            });
        let previously_available =
            HashMap::from([(coin.id.clone(), HashMap::from([("USD".to_string(), 2.0)]))]);
        let expired_at = fetched_at_unix_secs + MARKET_RATE_DISPLAY_MAX_AGE_SECS + 1;

        let displayable = displayable_cached_rate_entries(
            WalletNetwork::Mainnet,
            std::slice::from_ref(&coin),
            expired_at,
            &public_rates_cache,
        )
        .await;
        let currently_available = displayable
            .into_iter()
            .map(|(coin, entry)| (coin.id, entry.rates))
            .collect::<HashMap<_, _>>();
        let mut failed_attempt = RateAttemptState::due(Instant::now());
        failed_attempt.record_failure(Instant::now());

        assert!(currently_available.is_empty());
        assert_eq!(failed_attempt.consecutive_failures, 1);
        assert_eq!(
            rate_coin_ids_to_invalidate(
                std::slice::from_ref(&coin),
                &previously_available,
                &currently_available,
                false,
            ),
            vec![coin.id]
        );
    }

    #[test]
    fn engine_restart_invalidates_a_missing_active_rate_without_local_history() {
        let coin = sample_coin("VRSC", VRSC_SYSTEM_ID, Protocol::Vrsc);
        let no_rates = HashMap::new();

        assert_eq!(
            rate_coin_ids_to_invalidate(std::slice::from_ref(&coin), &no_rates, &no_rates, true),
            vec![coin.id.clone()]
        );
        assert!(rate_coin_ids_to_invalidate(
            std::slice::from_ref(&coin),
            &no_rates,
            &no_rates,
            false,
        )
        .is_empty());
    }

    #[tokio::test]
    async fn balance_schedule_progresses_while_rate_schedule_is_pending() {
        let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();
        let (release_rates_tx, release_rates_rx) = tokio::sync::oneshot::channel::<()>();
        let balance_schedule = async move {
            progress_tx.send(1_u8).expect("first balance refresh");
            tokio::task::yield_now().await;
            progress_tx.send(2_u8).expect("second balance refresh");
        };
        let rate_schedule = async move {
            let _ = release_rates_rx.await;
        };

        let schedules = tokio::spawn(run_independent_update_schedules(
            balance_schedule,
            rate_schedule,
        ));
        assert_eq!(progress_rx.recv().await, Some(1));
        assert_eq!(progress_rx.recv().await, Some(2));
        assert!(
            !schedules.is_finished(),
            "rate schedule should still be pending"
        );

        release_rates_tx.send(()).expect("release rate schedule");
        schedules.await.expect("update schedules complete");
    }

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
        let active = HashSet::from(["eth".to_string(), "usdc".to_string()]);
        let channels = active_channels(&registry, false, "RtestAddress", true, None, &active);

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
        let active = HashSet::from(["eth".to_string(), "usdc".to_string()]);
        let channels = active_channels(&registry, false, "RtestAddress", false, None, &active);

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
        let active = HashSet::from(["geth".to_string(), "eth".to_string(), "usdc".to_string()]);
        let channels = active_channels(&registry, true, "RtestAddress", true, None, &active);

        assert!(channels
            .iter()
            .any(|(coin_id, channel_id)| coin_id == "GETH" && channel_id == "eth.GETH"));
        assert!(!channels
            .iter()
            .any(|(coin_id, _)| coin_id == "ETH" || coin_id == "USDC"));
    }

    #[test]
    fn active_channels_excludes_inactive_assets_on_shared_vrpc_channel() {
        let registry = CoinRegistry::new();
        let active = HashSet::from(["vrsc".to_string()]);
        let channels = active_channels(&registry, false, "RtestAddress", false, None, &active);

        assert!(channels.iter().any(|(coin_id, _)| coin_id == "VRSC"));
        assert!(channels.iter().all(|(coin_id, _)| coin_id == "VRSC"));
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
