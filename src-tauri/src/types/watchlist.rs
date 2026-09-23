use serde::{Deserialize, Serialize};

use crate::core::coins::CoinDefinition;
use crate::types::wallet::WalletNetwork;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WatchlistTargetKind {
    Identity,
    Address,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistEntry {
    pub id: String,
    pub target_kind: WatchlistTargetKind,
    pub display_name: String,
    pub address: String,
    pub system_id: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveWatchlistTargetRequest {
    pub query: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistResolvedTarget {
    pub target_kind: WatchlistTargetKind,
    pub display_name: String,
    pub address: String,
    pub system_id: Option<String>,
    pub visible_currency_count: Option<usize>,
    pub availability: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistHolding {
    pub asset_key: String,
    pub currency_id: String,
    pub system_id: String,
    pub system_ticker: String,
    pub system_display_name: String,
    pub balance: String,
    pub coin: Option<CoinDefinition>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistSource {
    pub system_id: String,
    pub system_ticker: String,
    pub system_display_name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistEntrySnapshot {
    pub entry: WatchlistEntry,
    pub holdings: Vec<WatchlistHolding>,
    pub sources: Vec<WatchlistSource>,
    pub availability: String,
    pub refreshed_at: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistRefreshResult {
    pub network: WalletNetwork,
    pub entries: Vec<WatchlistEntrySnapshot>,
    pub refreshed_at: u64,
}
