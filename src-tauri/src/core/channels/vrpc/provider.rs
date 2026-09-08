//
// Module 5: VRPC HTTP JSON-RPC client. Runtime-configured endpoints; TTL cache; no sensitive data in logs.

use std::collections::{HashMap, HashSet};
use std::error::Error as _;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reqwest::Client;
use serde_json::Value;
use tokio::sync::watch;

use crate::core::runtime_config;
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

const VRSC_MAINNET_SYSTEM_ID: &str = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";
const VRSCTEST_SYSTEM_ID: &str = "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq";
const VARRR_SYSTEM_ID: &str = "iExBJfZYK7KREDpuhj6PzZBzqMAKaFg7d2";
const VDEX_SYSTEM_ID: &str = "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N";
const CHIPS_SYSTEM_ID: &str = "iJ3WZocnjG9ufv7GKUA4LijQno5gTMb7tP";

/// TTL for cached responses (seconds).
const TTL_BALANCE: u64 = 5;
const TTL_DELTAS: u64 = 10;
const TTL_MEMPOOL: u64 = 10;
const TTL_UTXOS: u64 = 5;
const TTL_GETINFO: u64 = 1;
const TTL_GETBLOCK: u64 = 600;
const TTL_GETIDENTITY: u64 = 5;
const TTL_GETIDENTITYCONTENT: u64 = 5;
const TTL_GETIDENTITIES_WITH_ADDRESS: u64 = 5;
const TTL_CURRENCY: u64 = 60;
const TTL_LISTCURRENCIES: u64 = 600;
const TTL_CURRENCY_CONVERSION_PATHS: u64 = 15;
const READ_RETRY_ATTEMPTS: u8 = 3;
const READ_RETRY_BASE_DELAY_MS: u64 = 250;
const STALE_CACHE_MAX_AGE_SECS: u64 = 120;

fn params_getaddressbalance(addresses: &[String]) -> Result<Value, WalletError> {
    if addresses.is_empty() {
        return Err(WalletError::InvalidAddress);
    }
    Ok(serde_json::json!([{"addresses": addresses, "friendlynames": true}]))
}

fn params_getaddressdeltas(addresses: &[String], start: Option<u64>, end: Option<u64>) -> Value {
    let mut request = serde_json::Map::new();
    if !addresses.is_empty() {
        request.insert("addresses".to_string(), serde_json::json!(addresses));
    }
    request.insert("friendlynames".to_string(), Value::Bool(true));
    request.insert("verbosity".to_string(), Value::from(1));
    if let Some(start_block) = start {
        request.insert("start".to_string(), Value::from(start_block));
    }
    if let Some(end_block) = end {
        request.insert("end".to_string(), Value::from(end_block));
    }

    Value::Array(vec![Value::Object(request)])
}

fn params_getaddressmempool(addresses: &[String]) -> Value {
    if addresses.is_empty() {
        serde_json::json!([{}])
    } else {
        serde_json::json!([{"addresses": addresses, "friendlynames": true, "verbosity": 1}])
    }
}

fn params_getidentitieswithaddress(address: &str, unspent: bool) -> Result<Value, WalletError> {
    let normalized = address.trim();
    if normalized.is_empty() {
        return Err(WalletError::InvalidAddress);
    }

    Ok(serde_json::json!([{
        "address": normalized,
        "unspent": unspent
    }]))
}

#[derive(Clone)]
struct CachedEntry {
    value: Value,
    expires_at: Instant,
}

struct InFlightCall {
    result: watch::Sender<Option<Result<Value, WalletError>>>,
}

#[derive(Clone, Copy)]
enum RpcErrorMode {
    Default,
    Identity,
    Bridge,
}

/// HTTP JSON-RPC client for Verus daemon API. Endpoint comes from runtime config.
pub struct VrpcProvider {
    client: Client,
    base_url: String,
    cache: Mutex<HashMap<String, CachedEntry>>,
    in_flight: Mutex<HashMap<String, Arc<InFlightCall>>>,
}

struct InFlightLeader<'a> {
    provider: &'a VrpcProvider,
    key: String,
    call: Arc<InFlightCall>,
    completed: bool,
}

impl InFlightLeader<'_> {
    fn complete(mut self, result: Result<Value, WalletError>) {
        self.call.result.send_replace(Some(result));
        self.provider.remove_in_flight(&self.key, &self.call);
        self.completed = true;
    }
}

impl Drop for InFlightLeader<'_> {
    fn drop(&mut self) {
        if self.completed {
            return;
        }

        self.call
            .result
            .send_replace(Some(Err(WalletError::NetworkError)));
        self.provider.remove_in_flight(&self.key, &self.call);
    }
}

impl VrpcProvider {
    fn build_http_client() -> Client {
        Client::builder()
            // Force a deterministic transport stack in-app:
            // rustls only, direct connection, and HTTP/1.1.
            .use_rustls_tls()
            .no_proxy()
            .http1_only()
            .connect_timeout(Duration::from_secs(4))
            .timeout(Duration::from_secs(12))
            .build()
            .unwrap_or_else(|_| Client::new())
    }

    pub fn new_with_base_url(base_url: &str) -> Self {
        Self {
            client: Self::build_http_client(),
            base_url: base_url.to_string(),
            cache: Mutex::new(HashMap::new()),
            in_flight: Mutex::new(HashMap::new()),
        }
    }

    pub fn new_mainnet() -> Self {
        Self::new_with_base_url(&runtime_config::vrpc_mainnet_url())
    }

    pub fn new_testnet() -> Self {
        Self::new_with_base_url(&runtime_config::vrpc_testnet_url())
    }

    /// Default: mainnet.
    pub fn new() -> Self {
        Self::new_mainnet()
    }

    fn cache_key(method: &str, params: &Value) -> String {
        format!(
            "{}:{}",
            method,
            serde_json::to_string(params).unwrap_or_default()
        )
    }

    fn get_cached(&self, key: &str) -> Option<Value> {
        let cache = self.cache.lock().ok()?;
        let entry = cache.get(key)?;
        if entry.expires_at > Instant::now() {
            return Some(entry.value.clone());
        }
        None
    }

    fn get_stale_cached(&self, key: &str) -> Option<Value> {
        let cache = self.cache.lock().ok()?;
        let entry = cache.get(key)?;
        let now = Instant::now();

        if entry.expires_at > now {
            return Some(entry.value.clone());
        }

        let age = now.saturating_duration_since(entry.expires_at);
        if age <= Duration::from_secs(STALE_CACHE_MAX_AGE_SECS) {
            return Some(entry.value.clone());
        }
        None
    }

    fn set_cached(&self, key: String, value: Value, ttl_secs: u64) {
        if let Ok(mut cache) = self.cache.lock() {
            let now = Instant::now();
            let stale_limit = Duration::from_secs(STALE_CACHE_MAX_AGE_SECS);
            cache.retain(|_, entry| {
                if entry.expires_at > now {
                    return true;
                }
                now.saturating_duration_since(entry.expires_at) <= stale_limit
            });
            cache.insert(
                key,
                CachedEntry {
                    value,
                    expires_at: now + Duration::from_secs(ttl_secs),
                },
            );
        }
    }

    fn remove_in_flight(&self, key: &str, expected: &Arc<InFlightCall>) {
        if let Ok(mut in_flight) = self.in_flight.lock() {
            if in_flight
                .get(key)
                .is_some_and(|current| Arc::ptr_eq(current, expected))
            {
                in_flight.remove(key);
            }
        }
    }

    fn rpc_body(method: &str, params: Value) -> Value {
        serde_json::json!({
            "jsonrpc": "1.0",
            "id": "verus-express",
            "method": method,
            "params": params
        })
    }

    fn classify_network_error(err: &reqwest::Error) -> &'static str {
        let msg = err.to_string().to_ascii_lowercase();
        if err.is_timeout() {
            return "timeout";
        }
        if err.is_connect() {
            if msg.contains("dns")
                || msg.contains("lookup")
                || msg.contains("name or service not known")
            {
                return "dns";
            }
            return "connect";
        }
        if err.is_request() {
            return "request";
        }
        if err.is_decode() {
            return "decode";
        }
        if err.is_body() {
            return "body";
        }
        "network"
    }

    fn rpc_error_code(error_obj: &Value) -> Option<i64> {
        error_obj.get("code").and_then(|value| value.as_i64())
    }

    fn rpc_error_message(error_obj: &Value) -> String {
        error_obj
            .get("message")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
    }

    fn is_insufficient_funds_rpc(method: &str, error_obj: &Value) -> bool {
        if Self::rpc_error_code(error_obj) == Some(-6) {
            return true;
        }

        let message = Self::rpc_error_message(error_obj);
        if message.contains("insufficient funds") || message.contains("insufficient balance") {
            return true;
        }

        method == "fundrawtransaction" && message.contains("utxos provided")
    }

    fn is_invalid_address_rpc(method: &str, error_obj: &Value) -> bool {
        if Self::rpc_error_code(error_obj) == Some(-5) {
            return true;
        }

        let message = Self::rpc_error_message(error_obj);
        message.contains("invalid destination address")
            || message.contains("invalid transparent address")
            || (method == "sendcurrency" && message.contains("invalid destination"))
    }

    fn format_network_error_details(err: &reqwest::Error) -> String {
        let mut out = err.to_string();
        let mut source = err.source();
        while let Some(cause) = source {
            out.push_str(" | caused by: ");
            out.push_str(&cause.to_string());
            source = cause.source();
        }
        out
    }

    fn log_http_failure(method: &str, status: reqwest::StatusCode, body: &str) {
        if body.trim().is_empty() {
            println!("[VRPC] {} HTTP {}", method, status);
        } else {
            println!("[VRPC] {} HTTP {} body: {}", method, status, body);
        }
    }

    fn missing_result_error(mode: RpcErrorMode) -> WalletError {
        match mode {
            RpcErrorMode::Identity => WalletError::IdentityBuildFailed,
            RpcErrorMode::Default | RpcErrorMode::Bridge => WalletError::OperationFailed,
        }
    }

    fn map_rpc_error(method: &str, error_obj: &Value, mode: RpcErrorMode) -> WalletError {
        match mode {
            RpcErrorMode::Identity => {
                if Self::rpc_error_code(error_obj) == Some(-32601) {
                    WalletError::IdentityRpcUnsupported
                } else {
                    WalletError::IdentityBuildFailed
                }
            }
            RpcErrorMode::Bridge => {
                if Self::rpc_error_code(error_obj) == Some(-32601) {
                    WalletError::BridgeNotImplemented
                } else {
                    WalletError::OperationFailed
                }
            }
            RpcErrorMode::Default => {
                if Self::rpc_error_code(error_obj) == Some(-32601) {
                    WalletError::UnsupportedChannel
                } else if Self::is_insufficient_funds_rpc(method, error_obj) {
                    WalletError::InsufficientFunds
                } else if Self::is_invalid_address_rpc(method, error_obj) {
                    WalletError::InvalidAddress
                } else {
                    WalletError::OperationFailed
                }
            }
        }
    }

    fn extract_result_from_json(
        method: &str,
        json: Value,
        mode: RpcErrorMode,
    ) -> Result<Value, WalletError> {
        if let Some(error_obj) = json.get("error").filter(|error| !error.is_null()) {
            println!("[VRPC] {} RPC error response: {}", method, error_obj);
            return Err(Self::map_rpc_error(method, error_obj, mode));
        }

        match json.get("result").filter(|value| !value.is_null()) {
            Some(result) => Ok(result.clone()),
            None => {
                println!(
                    "[VRPC] {} RPC response missing non-null result payload: {}",
                    method, json
                );
                Err(Self::missing_result_error(mode))
            }
        }
    }

    async fn request_json_once(&self, method: &str, body: &Value) -> Result<Value, WalletError> {
        let res = self
            .client
            .post(&self.base_url)
            .json(body)
            .send()
            .await
            .map_err(|err| {
                println!("[VRPC] {} network failure: {}", method, err);
                WalletError::NetworkError
            })?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            Self::log_http_failure(method, status, &body);
            return Err(WalletError::OperationFailed);
        }

        res.json().await.map_err(|err| {
            println!("[VRPC] {} response parse failure: {}", method, err);
            WalletError::NetworkError
        })
    }

    async fn call_uncached_with_mode(
        &self,
        method: &str,
        params: Value,
        mode: RpcErrorMode,
    ) -> Result<Value, WalletError> {
        let body = Self::rpc_body(method, params);
        let json = self.request_json_once(method, &body).await?;
        Self::extract_result_from_json(method, json, mode)
    }

    async fn fetch_result_with_retry(
        &self,
        method: &str,
        body: &Value,
        attempts: u8,
        mode: RpcErrorMode,
    ) -> Result<Value, WalletError> {
        for attempt in 0..attempts {
            let is_last_attempt = attempt + 1 >= attempts;
            let delay = Duration::from_millis(
                READ_RETRY_BASE_DELAY_MS.saturating_mul((attempt as u64) + 1),
            );

            let res = match self.client.post(&self.base_url).json(body).send().await {
                Ok(v) => v,
                Err(err) => {
                    let category = Self::classify_network_error(&err);
                    if !is_last_attempt {
                        println!(
                            "[VRPC] {} network failure ({}) on attempt {}/{}: {}",
                            method,
                            category,
                            attempt + 1,
                            attempts,
                            err
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    let details = Self::format_network_error_details(&err);
                    println!(
                        "[VRPC] {} network failure ({}) on final attempt {}/{}: {}",
                        method,
                        category,
                        attempt + 1,
                        attempts,
                        details
                    );
                    return Err(WalletError::NetworkError);
                }
            };

            if !res.status().is_success() {
                if !is_last_attempt && res.status().is_server_error() {
                    println!(
                        "[VRPC] {} HTTP {} on attempt {}/{}",
                        method,
                        res.status(),
                        attempt + 1,
                        attempts
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                }
                return Err(WalletError::OperationFailed);
            }

            let json: Value = match res.json().await {
                Ok(v) => v,
                Err(err) => {
                    if !is_last_attempt {
                        println!(
                            "[VRPC] {} response parse failure on attempt {}/{}: {}",
                            method,
                            attempt + 1,
                            attempts,
                            err
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(WalletError::NetworkError);
                }
            };

            return Self::extract_result_from_json(method, json, mode);
        }

        Err(WalletError::NetworkError)
    }

    async fn call_without_cache_with_error_mapping(
        &self,
        method: &str,
        params: Value,
    ) -> Result<Value, WalletError> {
        self.call_uncached_with_mode(method, params, RpcErrorMode::Identity)
            .await
    }

    async fn call_without_cache(&self, method: &str, params: Value) -> Result<Value, WalletError> {
        self.call_uncached_with_mode(method, params, RpcErrorMode::Default)
            .await
    }

    async fn call_without_cache_with_bridge_mapping(
        &self,
        method: &str,
        params: Value,
    ) -> Result<Value, WalletError> {
        self.call_uncached_with_mode(method, params, RpcErrorMode::Bridge)
            .await
    }

    async fn call(&self, method: &str, params: Value, ttl_secs: u64) -> Result<Value, WalletError> {
        let key = Self::cache_key(method, &params);
        if let Some(cached) = self.get_cached(&key) {
            return Ok(cached);
        }

        let body = Self::rpc_body(method, params);

        if ttl_secs == 0 {
            return self
                .fetch_result_with_retry(method, &body, 1, RpcErrorMode::Default)
                .await;
        }

        let stale = self.get_stale_cached(&key);
        let (call, is_leader) = {
            let mut in_flight = self
                .in_flight
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(existing) = in_flight.get(&key) {
                (existing.clone(), false)
            } else {
                let (result, _) = watch::channel(None);
                let call = Arc::new(InFlightCall { result });
                in_flight.insert(key.clone(), call.clone());
                (call, true)
            }
        };

        if !is_leader {
            let mut result = call.result.subscribe();
            if result.borrow().is_none() && result.changed().await.is_err() {
                return Err(WalletError::NetworkError);
            }

            return result
                .borrow()
                .clone()
                .unwrap_or(Err(WalletError::NetworkError));
        }

        let leader = InFlightLeader {
            provider: self,
            key: key.clone(),
            call,
            completed: false,
        };

        let result = self
            .fetch_result_with_retry(method, &body, READ_RETRY_ATTEMPTS, RpcErrorMode::Default)
            .await;

        let delivered_result = match result {
            Ok(value) => {
                // Publish to the cache before waking joined callers.
                self.set_cached(key.clone(), value.clone(), ttl_secs);
                Ok(value)
            }
            Err(err) => {
                if let Some(stale_cached) = stale {
                    println!(
                        "[VRPC] {} using stale cached response after retries exhausted",
                        method
                    );
                    Ok(stale_cached)
                } else {
                    Err(err)
                }
            }
        };
        leader.complete(delivered_result.clone());
        delivered_result
    }

    /// getaddressbalance: params [{"addresses": ["R..."], "friendlynames": true}]
    pub async fn getaddressbalance(&self, addresses: &[String]) -> Result<Value, WalletError> {
        let params = params_getaddressbalance(addresses)?;
        self.call("getaddressbalance", params, TTL_BALANCE).await
    }

    /// getaddressdeltas: params [{"addresses": ["R..."], "friendlynames": true, "verbosity": 1}]
    pub async fn getaddressdeltas(&self, addresses: &[String]) -> Result<Value, WalletError> {
        let params = params_getaddressdeltas(addresses, None, None);
        self.call("getaddressdeltas", params, TTL_DELTAS).await
    }

    /// getaddressdeltas with block window selectors.
    pub async fn getaddressdeltas_window(
        &self,
        addresses: &[String],
        start: Option<u64>,
        end: Option<u64>,
    ) -> Result<Value, WalletError> {
        let params = params_getaddressdeltas(addresses, start, end);
        self.call("getaddressdeltas", params, TTL_DELTAS).await
    }

    /// getaddressmempool: unconfirmed tx for address(es)
    pub async fn getaddressmempool(&self, addresses: &[String]) -> Result<Value, WalletError> {
        let params = params_getaddressmempool(addresses);
        self.call("getaddressmempool", params, TTL_MEMPOOL).await
    }

    /// getaddressutxos: spendable UTXOs for funding
    pub async fn getaddressutxos(&self, addresses: &[String]) -> Result<Value, WalletError> {
        let params = if addresses.is_empty() {
            return Err(WalletError::InvalidAddress);
        } else if addresses.len() == 1 {
            serde_json::json!([{"addresses": [addresses[0]]}])
        } else {
            serde_json::json!([{"addresses": addresses}])
        };
        self.call("getaddressutxos", params, TTL_UTXOS).await
    }

    /// createrawtransaction: inputs (array), outputs (object address -> amount)
    pub async fn createrawtransaction(
        &self,
        inputs: &[Value],
        outputs: &Value,
    ) -> Result<Value, WalletError> {
        let params = serde_json::json!([inputs, outputs]);
        self.call("createrawtransaction", params, 0).await
    }

    /// fundrawtransaction with optional UTXOs/change/explicit fee.
    /// Parity path used by valu-mobile against public VRPC endpoints.
    pub async fn fundrawtransaction_with_options(
        &self,
        hex_tx: &str,
        utxos: Option<&[Value]>,
        change_address: Option<&str>,
        explicit_fee: Option<f64>,
    ) -> Result<Value, WalletError> {
        let mut params = vec![Value::String(hex_tx.to_string())];
        if let Some(utxo_list) = utxos {
            params.push(Value::Array(utxo_list.to_vec()));
            if let Some(change) = change_address {
                params.push(Value::String(change.to_string()));
                if let Some(fee) = explicit_fee {
                    params.push(Value::from(fee));
                }
            }
        }
        self.call("fundrawtransaction", Value::Array(params), 0)
            .await
    }

    /// sendrawtransaction: signed hex
    pub async fn sendrawtransaction(&self, signed_hex: &str) -> Result<Value, WalletError> {
        let params = serde_json::json!([signed_hex]);
        self.call("sendrawtransaction", params, 0).await
    }

    /// getinfo: chain sync status
    pub async fn getinfo(&self) -> Result<Value, WalletError> {
        self.call("getinfo", serde_json::json!([]), TTL_GETINFO)
            .await
    }

    /// getblock: load a block by height or hash.
    pub async fn getblock(&self, hash_or_height: &str) -> Result<Value, WalletError> {
        if hash_or_height.trim().is_empty() {
            return Err(WalletError::OperationFailed);
        }

        let params = serde_json::json!([hash_or_height]);
        self.call("getblock", params, TTL_GETBLOCK).await
    }

    /// getcurrency: resolve by i-address or fully-qualified currency name.
    pub async fn getcurrency(&self, currency: &str) -> Result<Value, WalletError> {
        if currency.trim().is_empty() {
            return Err(WalletError::InvalidAddress);
        }
        let params = serde_json::json!([currency]);
        self.call("getcurrency", params, TTL_CURRENCY).await
    }

    /// listcurrencies: returns known currencies from the endpoint.
    pub async fn listcurrencies(&self) -> Result<Value, WalletError> {
        self.call("listcurrencies", serde_json::json!([]), TTL_LISTCURRENCIES)
            .await
    }

    /// listcurrencies with a specific system type filter (`local`, `pbaas`, `imported`).
    pub async fn listcurrencies_with_systemtype(
        &self,
        systemtype: &str,
    ) -> Result<Value, WalletError> {
        let normalized = systemtype.trim();
        if normalized.is_empty() {
            return self.listcurrencies().await;
        }

        let params = serde_json::json!([{ "systemtype": normalized }]);
        self.call("listcurrencies", params, TTL_LISTCURRENCIES)
            .await
    }

    /// listcurrencies with a specific launch state filter (for example `prelaunch`).
    pub async fn listcurrencies_with_launchstate(
        &self,
        launchstate: &str,
    ) -> Result<Value, WalletError> {
        let normalized = launchstate.trim();
        if normalized.is_empty() {
            return self.listcurrencies().await;
        }

        let params = serde_json::json!([{ "launchstate": normalized }]);
        self.call("listcurrencies", params, TTL_LISTCURRENCIES)
            .await
    }

    /// getcurrencyconversionpaths: discover available conversion routes between currencies.
    pub async fn getcurrencyconversionpaths(
        &self,
        source_definition: &Value,
        destination_definition: Option<&Value>,
    ) -> Result<Value, WalletError> {
        let params = if let Some(destination_definition) = destination_definition {
            serde_json::json!([source_definition, destination_definition])
        } else {
            serde_json::json!([source_definition])
        };
        let key = Self::cache_key("getcurrencyconversionpaths", &params);
        if let Some(cached) = self.get_cached(&key) {
            return Ok(cached);
        }
        let result = self
            .call_without_cache_with_bridge_mapping("getcurrencyconversionpaths", params)
            .await?;
        self.set_cached(key, result.clone(), TTL_CURRENCY_CONVERSION_PATHS);
        Ok(result)
    }

    /// estimateconversion: estimate conversion output for a source/target pair and optional via.
    pub async fn estimateconversion(
        &self,
        currency: &str,
        convert_to: &str,
        amount: f64,
        via: Option<&str>,
        preconvert: Option<bool>,
    ) -> Result<Value, WalletError> {
        if currency.trim().is_empty()
            || convert_to.trim().is_empty()
            || !amount.is_finite()
            || amount <= 0.0
        {
            return Err(WalletError::OperationFailed);
        }

        let mut request = serde_json::Map::new();
        request.insert(
            "currency".to_string(),
            Value::String(currency.trim().to_string()),
        );
        request.insert(
            "convertto".to_string(),
            Value::String(convert_to.trim().to_string()),
        );
        request.insert("amount".to_string(), serde_json::json!(amount));

        if let Some(via_value) = via.map(str::trim).filter(|value| !value.is_empty()) {
            request.insert("via".to_string(), Value::String(via_value.to_string()));
        }
        if let Some(preconvert_value) = preconvert {
            request.insert("preconvert".to_string(), Value::Bool(preconvert_value));
        }

        self.call_without_cache(
            "estimateconversion",
            Value::Array(vec![Value::Object(request)]),
        )
        .await
    }

    /// sendcurrency: build and optionally return unsigned tx template.
    /// Params follow verusd RPC signature.
    pub async fn sendcurrency(
        &self,
        source: &str,
        outputs: &[Value],
        min_conf: u32,
        fee: f64,
        return_tx: bool,
    ) -> Result<Value, WalletError> {
        if source.trim().is_empty() {
            return Err(WalletError::InvalidAddress);
        }
        if outputs.is_empty() {
            return Err(WalletError::OperationFailed);
        }
        let params = serde_json::json!([source, outputs, min_conf, fee, return_tx]);
        self.call_without_cache("sendcurrency", params).await
    }

    /// getidentity: resolve by i-address or name.
    pub async fn getidentity(&self, identity: &str) -> Result<Value, WalletError> {
        if identity.trim().is_empty() {
            return Err(WalletError::InvalidAddress);
        }
        let params = serde_json::json!([identity]);
        self.call("getidentity", params, TTL_GETIDENTITY).await
    }

    /// getidentitycontent: resolve identity content and history by i-address or name.
    pub async fn getidentitycontent(&self, identity: &str) -> Result<Value, WalletError> {
        if identity.trim().is_empty() {
            return Err(WalletError::InvalidAddress);
        }
        let params = serde_json::json!([identity]);
        self.call("getidentitycontent", params, TTL_GETIDENTITYCONTENT)
            .await
    }

    /// getidentitieswithaddress: discover identities associated with an R-address.
    pub async fn getidentitieswithaddress(
        &self,
        address: &str,
        unspent: bool,
    ) -> Result<Value, WalletError> {
        let params = params_getidentitieswithaddress(address, unspent)?;
        self.call(
            "getidentitieswithaddress",
            params,
            TTL_GETIDENTITIES_WITH_ADDRESS,
        )
        .await
    }

    /// getrawtransaction: return raw tx hex (verbosity = 0) or tx object.
    pub async fn getrawtransaction(&self, txid: &str, verbosity: u8) -> Result<Value, WalletError> {
        if txid.trim().is_empty() {
            return Err(WalletError::OperationFailed);
        }
        let params = serde_json::json!([txid, verbosity]);
        self.call("getrawtransaction", params, 0).await
    }

    /// updateidentity: build identity update tx template.
    /// Maps RPC -32601 to IdentityRpcUnsupported.
    pub async fn updateidentity(
        &self,
        identity_json: &Value,
        return_tx_hex: bool,
    ) -> Result<Value, WalletError> {
        let params = serde_json::json!([identity_json, return_tx_hex]);
        self.call_without_cache_with_error_mapping("updateidentity", params)
            .await
    }
}

impl Default for VrpcProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared mainnet/testnet VRPC providers; select by active wallet network.
pub struct VrpcProviderPool {
    mainnet: VrpcProvider,
    testnet: VrpcProvider,
    mainnet_by_system: HashMap<String, VrpcProvider>,
    testnet_by_system: HashMap<String, VrpcProvider>,
}

impl VrpcProviderPool {
    fn normalize_system_id(system_id: &str) -> String {
        system_id.trim().to_ascii_lowercase()
    }

    fn providers_by_system(&self, network: WalletNetwork) -> &HashMap<String, VrpcProvider> {
        match network {
            WalletNetwork::Mainnet => &self.mainnet_by_system,
            WalletNetwork::Testnet => &self.testnet_by_system,
        }
    }

    pub fn new() -> Self {
        let vrpc_mainnet = runtime_config::vrpc_mainnet_url();
        let vrpc_testnet = runtime_config::vrpc_testnet_url();
        let vrpc_varrr_mainnet = runtime_config::vrpc_varrr_mainnet_url();
        let vrpc_vdex_mainnet = runtime_config::vrpc_vdex_mainnet_url();
        let vrpc_chips_mainnet = runtime_config::vrpc_chips_mainnet_url();

        let mut mainnet_by_system = HashMap::<String, VrpcProvider>::new();
        mainnet_by_system.insert(
            Self::normalize_system_id(VRSC_MAINNET_SYSTEM_ID),
            VrpcProvider::new_with_base_url(&vrpc_mainnet),
        );
        mainnet_by_system.insert(
            Self::normalize_system_id(VARRR_SYSTEM_ID),
            VrpcProvider::new_with_base_url(&vrpc_varrr_mainnet),
        );
        mainnet_by_system.insert(
            Self::normalize_system_id(VDEX_SYSTEM_ID),
            VrpcProvider::new_with_base_url(&vrpc_vdex_mainnet),
        );
        mainnet_by_system.insert(
            Self::normalize_system_id(CHIPS_SYSTEM_ID),
            VrpcProvider::new_with_base_url(&vrpc_chips_mainnet),
        );

        let mut testnet_by_system = HashMap::<String, VrpcProvider>::new();
        testnet_by_system.insert(
            Self::normalize_system_id(VRSCTEST_SYSTEM_ID),
            VrpcProvider::new_with_base_url(&vrpc_testnet),
        );

        Self {
            mainnet: VrpcProvider::new_with_base_url(&vrpc_mainnet),
            testnet: VrpcProvider::new_with_base_url(&vrpc_testnet),
            mainnet_by_system,
            testnet_by_system,
        }
    }

    pub fn for_network(&self, network: WalletNetwork) -> &VrpcProvider {
        match network {
            WalletNetwork::Mainnet => &self.mainnet,
            WalletNetwork::Testnet => &self.testnet,
        }
    }

    pub fn for_system(&self, network: WalletNetwork, system_id: &str) -> &VrpcProvider {
        let normalized = Self::normalize_system_id(system_id);
        match network {
            WalletNetwork::Mainnet => self
                .mainnet_by_system
                .get(&normalized)
                .unwrap_or(&self.mainnet),
            WalletNetwork::Testnet => self
                .testnet_by_system
                .get(&normalized)
                .unwrap_or(&self.testnet),
        }
    }

    pub fn has_system_provider(&self, network: WalletNetwork, system_id: &str) -> bool {
        let normalized = Self::normalize_system_id(system_id);
        match network {
            WalletNetwork::Mainnet => self.mainnet_by_system.contains_key(&normalized),
            WalletNetwork::Testnet => self.testnet_by_system.contains_key(&normalized),
        }
    }

    /// Returns provider candidates for reads that may target a specific system.
    /// Order: preferred hint (if any), network default, then remaining system providers.
    pub fn provider_candidates(
        &self,
        network: WalletNetwork,
        preferred_system_hint: Option<&str>,
    ) -> Vec<&VrpcProvider> {
        let mut candidates = Vec::<&VrpcProvider>::new();
        if let Some(system_hint) = preferred_system_hint
            .map(str::trim)
            .filter(|system_hint| !system_hint.is_empty())
        {
            candidates.push(self.for_system(network, system_hint));
        }
        candidates.push(self.for_network(network));
        candidates.extend(self.providers_by_system(network).values());

        let mut seen_base_urls = HashSet::<String>::new();
        candidates
            .into_iter()
            .filter(|provider| seen_base_urls.insert(provider.base_url.to_ascii_lowercase()))
            .collect()
    }

    pub fn endpoint_url_for_system(&self, network: WalletNetwork, system_id: &str) -> String {
        self.for_system(network, system_id).base_url.clone()
    }
}

impl Default for VrpcProviderPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::wallet::WalletNetwork;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn success_rpc_server() -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let request_count = Arc::new(AtomicUsize::new(0));
        let server_request_count = request_count.clone();
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let request_count = server_request_count.clone();
                tokio::spawn(async move {
                    let mut request = vec![0_u8; 4096];
                    let _ = socket.read(&mut request).await;
                    request_count.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(75)).await;
                    let body = r#"{"result":{"ok":true},"error":null}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                });
            }
        });

        (format!("http://{}", address), request_count, task)
    }

    #[tokio::test]
    async fn concurrent_cached_calls_share_one_request() {
        let (base_url, request_count, server) = success_rpc_server().await;
        let provider = Arc::new(VrpcProvider::new_with_base_url(&base_url));

        let first_provider = provider.clone();
        let first = tokio::spawn(async move {
            first_provider
                .call("display-read", serde_json::json!([]), 5)
                .await
        });
        tokio::task::yield_now().await;
        let second_provider = provider.clone();
        let second = tokio::spawn(async move {
            second_provider
                .call("display-read", serde_json::json!([]), 5)
                .await
        });

        assert_eq!(
            first.await.expect("first task").expect("first result"),
            serde_json::json!({"ok": true})
        );
        assert_eq!(
            second.await.expect("second task").expect("second result"),
            serde_json::json!({"ok": true})
        );
        assert_eq!(request_count.load(Ordering::SeqCst), 1);
        server.abort();
    }

    #[tokio::test]
    async fn cancelled_leader_releases_key_and_wakes_waiters() {
        let provider = VrpcProvider::new_with_base_url("http://127.0.0.1:1");
        let key = "cancelled".to_string();
        let (result, _) = watch::channel(None);
        let call = Arc::new(InFlightCall { result });
        provider
            .in_flight
            .lock()
            .expect("in-flight lock")
            .insert(key.clone(), call.clone());
        let waiter = call.result.subscribe();

        let leader = InFlightLeader {
            provider: &provider,
            key: key.clone(),
            call,
            completed: false,
        };
        drop(leader);

        assert!(matches!(
            waiter.borrow().clone(),
            Some(Err(WalletError::NetworkError))
        ));
        assert!(!provider
            .in_flight
            .lock()
            .expect("in-flight lock")
            .contains_key(&key));
    }

    #[tokio::test]
    async fn leader_failure_is_delivered_to_joined_callers() {
        let provider = VrpcProvider::new_with_base_url("http://127.0.0.1:1");
        let key = "failure".to_string();
        let (result, _) = watch::channel(None);
        let call = Arc::new(InFlightCall { result });
        provider
            .in_flight
            .lock()
            .expect("in-flight lock")
            .insert(key.clone(), call.clone());
        let waiter = call.result.subscribe();

        InFlightLeader {
            provider: &provider,
            key: key.clone(),
            call,
            completed: false,
        }
        .complete(Err(WalletError::OperationFailed));

        assert!(matches!(
            waiter.borrow().clone(),
            Some(Err(WalletError::OperationFailed))
        ));
        assert!(!provider
            .in_flight
            .lock()
            .expect("in-flight lock")
            .contains_key(&key));
    }

    #[test]
    fn getaddressbalance_params_are_object_form() {
        let addresses = vec!["RExampleAddress".to_string()];
        let params = params_getaddressbalance(&addresses).expect("params");
        assert_eq!(
            params,
            serde_json::json!([{"addresses": ["RExampleAddress"], "friendlynames": true}])
        );
    }

    #[test]
    fn getaddressdeltas_and_mempool_include_verbosity_and_friendlynames() {
        let addresses = vec!["RExampleAddress".to_string()];
        let deltas = params_getaddressdeltas(&addresses, None, None);
        let mempool = params_getaddressmempool(&addresses);
        let expected = serde_json::json!([{"addresses": ["RExampleAddress"], "friendlynames": true, "verbosity": 1}]);
        assert_eq!(deltas, expected);
        assert_eq!(mempool, expected);
    }

    #[test]
    fn getaddressdeltas_window_params_include_start_and_end() {
        let addresses = vec!["RExampleAddress".to_string()];
        let deltas = params_getaddressdeltas(&addresses, Some(10), Some(42));
        let expected = serde_json::json!([{
            "addresses": ["RExampleAddress"],
            "friendlynames": true,
            "verbosity": 1,
            "start": 10,
            "end": 42
        }]);
        assert_eq!(deltas, expected);
    }

    #[test]
    fn getidentitieswithaddress_params_are_object_form_with_unspent_flag() {
        let params = params_getidentitieswithaddress("RAutMoGh771ECTDbTq2qwwZo7MF5Tov3ka", false)
            .expect("params");
        assert_eq!(
            params,
            serde_json::json!([{
                "address": "RAutMoGh771ECTDbTq2qwwZo7MF5Tov3ka",
                "unspent": false
            }])
        );
    }

    #[test]
    fn has_mainnet_system_providers_for_known_chains() {
        let pool = VrpcProviderPool::new();
        assert!(
            pool.has_system_provider(WalletNetwork::Mainnet, "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV")
        );
        assert!(
            pool.has_system_provider(WalletNetwork::Mainnet, "iExBJfZYK7KREDpuhj6PzZBzqMAKaFg7d2")
        );
        assert!(
            pool.has_system_provider(WalletNetwork::Mainnet, "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N")
        );
        assert!(
            pool.has_system_provider(WalletNetwork::Mainnet, "iJ3WZocnjG9ufv7GKUA4LijQno5gTMb7tP")
        );
    }

    #[test]
    fn unknown_system_falls_back_to_network_provider() {
        let pool = VrpcProviderPool::new();
        assert!(
            !pool.has_system_provider(WalletNetwork::Mainnet, "iUnknownSystemAddress1234567890")
        );
        let fallback = pool.for_system(WalletNetwork::Mainnet, "iUnknownSystemAddress1234567890");
        let default = pool.for_network(WalletNetwork::Mainnet);
        assert_eq!(fallback.base_url, default.base_url);
    }

    #[test]
    fn provider_candidates_prioritize_preferred_hint_and_deduplicate_urls() {
        let pool = VrpcProviderPool::new();
        let candidates = pool.provider_candidates(WalletNetwork::Mainnet, Some(VDEX_SYSTEM_ID));

        assert!(!candidates.is_empty());
        assert_eq!(
            candidates[0].base_url,
            runtime_config::vrpc_vdex_mainnet_url()
        );

        let urls = candidates
            .iter()
            .map(|provider| provider.base_url.clone())
            .collect::<Vec<_>>();
        let unique = urls.iter().cloned().collect::<HashSet<_>>();

        assert_eq!(urls.len(), unique.len());
        assert!(urls
            .iter()
            .any(|url| url == &runtime_config::vrpc_mainnet_url()));
        assert!(urls
            .iter()
            .any(|url| url == &runtime_config::vrpc_varrr_mainnet_url()));
        assert!(urls
            .iter()
            .any(|url| url == &runtime_config::vrpc_vdex_mainnet_url()));
        assert!(urls
            .iter()
            .any(|url| url == &runtime_config::vrpc_chips_mainnet_url()));
    }

    #[test]
    fn endpoint_url_for_system_uses_system_provider_when_available() {
        let pool = VrpcProviderPool::new();
        assert_eq!(
            pool.endpoint_url_for_system(WalletNetwork::Mainnet, VDEX_SYSTEM_ID),
            runtime_config::vrpc_vdex_mainnet_url()
        );
        assert_eq!(
            pool.endpoint_url_for_system(WalletNetwork::Mainnet, "iUnknownSystemAddress1234567890"),
            runtime_config::vrpc_mainnet_url()
        );
    }

    #[test]
    fn rpc_result_parser_preserves_default_error_mapping() {
        let result = VrpcProvider::extract_result_from_json(
            "sendcurrency",
            serde_json::json!({
                "error": {
                    "code": -5,
                    "message": "invalid destination address"
                }
            }),
            RpcErrorMode::Default,
        );

        assert!(matches!(result, Err(WalletError::InvalidAddress)));
    }

    #[test]
    fn rpc_result_parser_maps_unsupported_default_method() {
        let result = VrpcProvider::extract_result_from_json(
            "unknownmethod",
            serde_json::json!({
                "error": {
                    "code": -32601,
                    "message": "method not found"
                }
            }),
            RpcErrorMode::Default,
        );

        assert!(matches!(result, Err(WalletError::UnsupportedChannel)));
    }

    #[test]
    fn rpc_result_parser_preserves_identity_error_mapping() {
        let result = VrpcProvider::extract_result_from_json(
            "updateidentity",
            serde_json::json!({
                "error": {
                    "code": -32601,
                    "message": "method not found"
                }
            }),
            RpcErrorMode::Identity,
        );

        assert!(matches!(result, Err(WalletError::IdentityRpcUnsupported)));
    }

    #[test]
    fn rpc_result_parser_preserves_bridge_error_mapping() {
        let result = VrpcProvider::extract_result_from_json(
            "getcurrencyconversionpaths",
            serde_json::json!({
                "error": {
                    "code": -32601,
                    "message": "method not found"
                }
            }),
            RpcErrorMode::Bridge,
        );

        assert!(matches!(result, Err(WalletError::BridgeNotImplemented)));
    }

    #[test]
    fn rpc_result_parser_uses_identity_missing_result_error() {
        let result = VrpcProvider::extract_result_from_json(
            "updateidentity",
            serde_json::json!({
                "result": null
            }),
            RpcErrorMode::Identity,
        );

        assert!(matches!(result, Err(WalletError::IdentityBuildFailed)));
    }
}
