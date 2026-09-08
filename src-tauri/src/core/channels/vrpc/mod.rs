//
// Module 5: VRPC channel — balance, transactions, preflight, send for Verus (VRSC/VRSCTEST). Runtime-configured endpoints.

mod balance;
mod common;
pub mod identity;
mod preflight;
mod provider;
mod send;
mod transactions;
mod transfer;

use crate::types::WalletError;

pub use balance::get_balances;
pub use preflight::preflight;
pub use provider::{VrpcProvider, VrpcProviderPool};
pub use send::send;
pub use transactions::{get_transactions, get_transactions_page, VrpcHistoryCursor};
pub use transfer::preflight_transfer;

#[derive(Debug, Clone)]
pub struct VrpcCoinContext {
    pub currency_id: String,
    pub system_id: String,
    pub decimals: u8,
    pub seconds_per_block: u64,
}

#[derive(Debug, Clone)]
pub struct VrpcTransactionsResult {
    pub transactions: Vec<crate::types::transaction::Transaction>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedVrpcChannel {
    pub address: String,
    pub system_id: String,
    pub used_legacy_fallback: bool,
}

impl ResolvedVrpcChannel {
    pub fn canonical_channel_id(&self) -> String {
        canonical_vrpc_channel_id(&self.address, &self.system_id)
    }
}

/// Basic heuristic for legacy-vs-canonical VRPC channel middle segment.
/// Canonical format is `vrpc.<address>.<systemId>`.
pub fn is_probable_vrpc_address(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    value.starts_with('R') || value.starts_with('i') || value.contains('@')
}

/// Parse VRPC channel id with optional legacy fallback.
/// Legacy format accepted for one compatibility release: `vrpc.<coinId>.<systemId>`.
pub fn parse_vrpc_channel_id(
    channel_id: &str,
    legacy_fallback_address: Option<&str>,
) -> Result<ResolvedVrpcChannel, WalletError> {
    let rest = channel_id
        .strip_prefix("vrpc.")
        .ok_or(WalletError::UnsupportedChannel)?;
    let (middle, system_id) = rest
        .rsplit_once('.')
        .ok_or(WalletError::UnsupportedChannel)?;
    if middle.is_empty() || system_id.is_empty() {
        return Err(WalletError::UnsupportedChannel);
    }

    if is_probable_vrpc_address(middle) {
        return Ok(ResolvedVrpcChannel {
            address: middle.to_string(),
            system_id: system_id.to_string(),
            used_legacy_fallback: false,
        });
    }

    let fallback = legacy_fallback_address.ok_or(WalletError::InvalidAddress)?;
    Ok(ResolvedVrpcChannel {
        address: fallback.to_string(),
        system_id: system_id.to_string(),
        used_legacy_fallback: true,
    })
}

pub fn canonical_vrpc_channel_id(address: &str, system_id: &str) -> String {
    format!("vrpc.{}.{}", address, system_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_vrpc_channel() {
        let parsed = parse_vrpc_channel_id("vrpc.Rabc123.iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq", None)
            .expect("parse");
        assert_eq!(parsed.address, "Rabc123");
        assert_eq!(parsed.system_id, "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq");
        assert!(!parsed.used_legacy_fallback);
    }

    #[test]
    fn legacy_channel_falls_back_to_session_address() {
        let parsed = parse_vrpc_channel_id(
            "vrpc.VRSCTEST.iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq",
            Some("RsessionAddress"),
        )
        .expect("legacy parse");
        assert_eq!(parsed.address, "RsessionAddress");
        assert!(parsed.used_legacy_fallback);
    }
}
