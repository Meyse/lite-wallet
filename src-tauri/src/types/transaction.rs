//
// Module 8: Preflight and send types — trust boundary for the send flow.
// The backend never signs UI-supplied tx hex; send accepts only preflight_id.

use serde::{Deserialize, Serialize};

/// Input to preflight_send (and future channel router). Amount as string to avoid float precision issues.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightParams {
    pub coin_id: String,
    pub channel_id: String,
    pub to_address: String,
    pub amount: String,
    pub memo: Option<String>,
}

/// Preflight result returned to UI. Contains only display fields; signing is keyed by preflight_id only.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightResult {
    pub preflight_id: String,
    pub fee: String,
    pub fee_currency: String,
    pub value: String,
    pub amount_submitted: String,
    pub to_address: String,
    pub from_address: String,
    pub fee_taken_from_amount: bool,
    pub fee_taken_message: Option<String>,
    pub warnings: Vec<PreflightWarning>,
    pub memo: Option<String>,
}

/// Single warning in a preflight result (e.g. insufficient_funds, slippage).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightWarning {
    pub warning_type: String,
    pub message: String,
}

/// Request to send: only preflight_id, no hex/inputs/callData.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendRequest {
    pub preflight_id: String,
}

/// Result of a successful send.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendResult {
    pub txid: String,
    pub fee: String,
    pub value: String,
    pub to_address: String,
    pub from_address: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_id: Option<String>,
}

/// Balance result shared across channels (VRPC, ETH, BTC). Decimal strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceResult {
    pub confirmed: String,
    pub pending: String,
    pub total: String,
}

/// Minimal transaction list item for history. Channels can map from chain-specific types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub txid: String,
    pub amount: String,
    pub from_address: String,
    pub to_address: String,
    pub confirmations: i64,
    pub timestamp: Option<u64>,
    pub pending: bool,
}

/// Request shape for paged transaction history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionHistoryPageRequest {
    pub channel_id: String,
    pub coin_id: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

/// One page of transaction history plus opaque cursor metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionHistoryPage {
    pub transactions: Vec<Transaction>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub warning: Option<String>,
}
