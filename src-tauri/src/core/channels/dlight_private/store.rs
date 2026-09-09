use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::wallet::WalletNetwork;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredTransaction {
    pub txid: String,
    pub net_sats: i128,
    pub block_height: u64,
    pub block_time: u64,
    #[serde(default)]
    pub to_address: Option<String>,
}

pub fn resolve_paths(
    app_data_dir: &Path,
    network: WalletNetwork,
    account_hash: &str,
    coin_id: &str,
) -> PathBuf {
    let network_key = match network {
        WalletNetwork::Mainnet => "mainnet",
        WalletNetwork::Testnet => "testnet",
    };

    app_data_dir
        .join("dlight")
        .join(network_key)
        .join(account_hash)
        .join(coin_id)
}

pub fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}
