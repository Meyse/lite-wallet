use std::env;

use crate::types::wallet::WalletNetwork;

pub const ENV_VRPC_MAINNET_URL: &str = "VRPC_MAINNET_URL";
pub const ENV_VRPC_TESTNET_URL: &str = "VRPC_TESTNET_URL";
pub const ENV_VRPC_PRIVATE_TESTNET_URL: &str = "VRPC_PRIVATE_TESTNET_URL";
pub const ENV_VRPC_VARRR_MAINNET_URL: &str = "VRPC_VARRR_MAINNET_URL";
pub const ENV_VRPC_VDEX_MAINNET_URL: &str = "VRPC_VDEX_MAINNET_URL";
pub const ENV_VRPC_CHIPS_MAINNET_URL: &str = "VRPC_CHIPS_MAINNET_URL";

pub const ENV_BTC_MAINNET_API_URL: &str = "BTC_MAINNET_API_URL";
pub const ENV_BTC_TESTNET_API_URL: &str = "BTC_TESTNET_API_URL";

pub const ENV_DLIGHT_MAINNET_ENDPOINTS: &str = "DLIGHT_MAINNET_ENDPOINTS";
pub const ENV_DLIGHT_TESTNET_ENDPOINTS: &str = "DLIGHT_TESTNET_ENDPOINTS";
pub const ENV_DLIGHT_PRIVATE_TESTNET_ENDPOINTS: &str = "DLIGHT_PRIVATE_TESTNET_ENDPOINTS";
pub const ENV_ELECTRUM_MAINNET_ENDPOINTS: &str = "ELECTRUM_MAINNET_ENDPOINTS";
pub const ENV_ELECTRUM_TESTNET_ENDPOINTS: &str = "ELECTRUM_TESTNET_ENDPOINTS";

const DEFAULT_VRPC_MAINNET_URL: &str = "https://vrpc-mainnet.invalid/";
const DEFAULT_VRPC_TESTNET_URL: &str = "https://vrpc-testnet.invalid/";
const DEFAULT_VRPC_PRIVATE_TESTNET_URL: &str = "https://vrpc-private-testnet.invalid/";
const DEFAULT_VRPC_VARRR_MAINNET_URL: &str = "https://vrpc-varrr-mainnet.invalid/";
const DEFAULT_VRPC_VDEX_MAINNET_URL: &str = "https://vrpc-vdex-mainnet.invalid/";
const DEFAULT_VRPC_CHIPS_MAINNET_URL: &str = "https://vrpc-chips-mainnet.invalid/";

const DEFAULT_BTC_MAINNET_API_URL: &str = "https://btc-mainnet.invalid/api";
const DEFAULT_BTC_TESTNET_API_URL: &str = "https://btc-testnet.invalid/api";

const DEFAULT_DLIGHT_MAINNET_ENDPOINTS: &[&str] = &["dlight-mainnet.invalid:8120"];
const DEFAULT_DLIGHT_TESTNET_ENDPOINTS: &[&str] = &["dlight-testnet.invalid:8125"];
const DEFAULT_ELECTRUM_MAINNET_ENDPOINTS: &[&str] = &["https://electrum-mainnet.invalid"];
const DEFAULT_ELECTRUM_TESTNET_ENDPOINTS: &[&str] = &["https://electrum-testnet.invalid"];

fn read_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_list_env(key: &str) -> Option<Vec<String>> {
    let raw = read_env(key)?;
    let values = raw
        .split([',', '\n', ';'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

fn read_env_or_default(key: &str, fallback: &str) -> String {
    read_env(key).unwrap_or_else(|| fallback.to_string())
}

fn read_list_env_or_default(key: &str, fallback: &[&str]) -> Vec<String> {
    read_list_env(key)
        .unwrap_or_else(|| fallback.iter().map(|value| (*value).to_string()).collect())
}

pub fn vrpc_mainnet_url() -> String {
    read_env_or_default(ENV_VRPC_MAINNET_URL, DEFAULT_VRPC_MAINNET_URL)
}

pub fn vrpc_testnet_url() -> String {
    read_env_or_default(ENV_VRPC_TESTNET_URL, DEFAULT_VRPC_TESTNET_URL)
}

pub fn vrpc_private_testnet_url() -> String {
    read_env_or_default(
        ENV_VRPC_PRIVATE_TESTNET_URL,
        DEFAULT_VRPC_PRIVATE_TESTNET_URL,
    )
}

pub fn vrpc_varrr_mainnet_url() -> String {
    read_env_or_default(ENV_VRPC_VARRR_MAINNET_URL, DEFAULT_VRPC_VARRR_MAINNET_URL)
}

pub fn vrpc_vdex_mainnet_url() -> String {
    read_env_or_default(ENV_VRPC_VDEX_MAINNET_URL, DEFAULT_VRPC_VDEX_MAINNET_URL)
}

pub fn vrpc_chips_mainnet_url() -> String {
    read_env_or_default(ENV_VRPC_CHIPS_MAINNET_URL, DEFAULT_VRPC_CHIPS_MAINNET_URL)
}

pub fn btc_mainnet_api_url() -> String {
    read_env_or_default(ENV_BTC_MAINNET_API_URL, DEFAULT_BTC_MAINNET_API_URL)
}

pub fn btc_testnet_api_url() -> String {
    read_env_or_default(ENV_BTC_TESTNET_API_URL, DEFAULT_BTC_TESTNET_API_URL)
}

pub fn dlight_mainnet_endpoints() -> Vec<String> {
    read_list_env_or_default(
        ENV_DLIGHT_MAINNET_ENDPOINTS,
        DEFAULT_DLIGHT_MAINNET_ENDPOINTS,
    )
}

pub fn dlight_testnet_endpoints() -> Vec<String> {
    read_list_env_or_default(
        ENV_DLIGHT_TESTNET_ENDPOINTS,
        DEFAULT_DLIGHT_TESTNET_ENDPOINTS,
    )
}

/// The private testnet has no dlight server, so it has no default. Without this variable,
/// shielded features stay unavailable on private testnet.
pub fn dlight_private_testnet_endpoints() -> Vec<String> {
    read_list_env(ENV_DLIGHT_PRIVATE_TESTNET_ENDPOINTS).unwrap_or_default()
}

pub fn electrum_mainnet_endpoints() -> Vec<String> {
    read_list_env_or_default(
        ENV_ELECTRUM_MAINNET_ENDPOINTS,
        DEFAULT_ELECTRUM_MAINNET_ENDPOINTS,
    )
}

pub fn electrum_testnet_endpoints() -> Vec<String> {
    read_list_env_or_default(
        ENV_ELECTRUM_TESTNET_ENDPOINTS,
        DEFAULT_ELECTRUM_TESTNET_ENDPOINTS,
    )
}

pub fn default_vrpc_endpoint_for_network(network: WalletNetwork) -> String {
    match network {
        WalletNetwork::Mainnet => vrpc_mainnet_url(),
        WalletNetwork::Testnet => vrpc_testnet_url(),
        WalletNetwork::PrivateTestnet => vrpc_private_testnet_url(),
    }
}

pub fn default_dlight_endpoints_for_network(network: WalletNetwork) -> Vec<String> {
    match network {
        WalletNetwork::Mainnet => dlight_mainnet_endpoints(),
        WalletNetwork::Testnet => dlight_testnet_endpoints(),
        WalletNetwork::PrivateTestnet => dlight_private_testnet_endpoints(),
    }
}

pub fn default_electrum_endpoints_for_network(network: WalletNetwork) -> Vec<String> {
    match network {
        WalletNetwork::Mainnet => electrum_mainnet_endpoints(),
        WalletNetwork::Testnet | WalletNetwork::PrivateTestnet => electrum_testnet_endpoints(),
    }
}
