use super::runtime::{ensure_runtime, get_runtime_snapshot, RuntimeStatusKind};
use super::spend_sync::get_spend_cache_status;
use super::{DlightInfo, DlightRuntimeDiagnostics, DlightRuntimeRequest};
use crate::types::transaction::{BalanceResult, Transaction};
use crate::types::WalletError;

const SATOSHIS_PER_COIN: i128 = 100_000_000;

fn satoshis_to_decimal_string(value: i128) -> String {
    let is_negative = value.is_negative();
    let absolute = value.unsigned_abs();
    let whole = absolute / SATOSHIS_PER_COIN as u128;
    let frac = absolute % SATOSHIS_PER_COIN as u128;
    if is_negative {
        format!("-{whole}.{frac:08}")
    } else {
        format!("{whole}.{frac:08}")
    }
}

pub async fn get_balances(request: &DlightRuntimeRequest) -> Result<BalanceResult, WalletError> {
    ensure_runtime(request).await?;
    let snapshot = get_runtime_snapshot(&request.runtime_key).unwrap_or_default();

    Ok(BalanceResult {
        confirmed: satoshis_to_decimal_string(snapshot.confirmed_sats),
        pending: satoshis_to_decimal_string(snapshot.pending_sats),
        total: satoshis_to_decimal_string(snapshot.total_sats),
    })
}

pub async fn get_transactions(
    request: &DlightRuntimeRequest,
) -> Result<Vec<Transaction>, WalletError> {
    ensure_runtime(request).await?;
    let snapshot = get_runtime_snapshot(&request.runtime_key).unwrap_or_default();
    let tip_height = snapshot.chain_tip_height.unwrap_or(0);

    let mut transactions = snapshot
        .transactions
        .iter()
        .map(|item| {
            let confirmations = if !item.pending && tip_height >= item.block_height {
                tip_height
                    .saturating_sub(item.block_height)
                    .saturating_add(1)
                    .min(i64::MAX as u64) as i64
            } else {
                0
            };
            let is_outgoing = item.net_sats.is_negative();

            Transaction {
                txid: item.txid.clone(),
                amount: satoshis_to_decimal_string(item.net_sats),
                from_address: if is_outgoing {
                    request.scope_address.clone()
                } else {
                    "shielded".to_string()
                },
                to_address: if is_outgoing {
                    item.to_address
                        .clone()
                        .unwrap_or_else(|| "shielded".to_string())
                } else {
                    request.scope_address.clone()
                },
                confirmations,
                timestamp: Some(item.block_time),
                pending: item.pending,
            }
        })
        .collect::<Vec<_>>();

    transactions.sort_by(|left, right| {
        right
            .timestamp
            .cmp(&left.timestamp)
            .then(left.txid.cmp(&right.txid))
    });

    Ok(transactions)
}

pub async fn get_info(request: &DlightRuntimeRequest) -> Result<DlightInfo, WalletError> {
    ensure_runtime(request).await?;
    let snapshot = get_runtime_snapshot(&request.runtime_key).unwrap_or_default();
    let runtime_tip_hint = snapshot
        .chain_tip_height
        .or(snapshot.estimated_tip_height)
        .filter(|tip| *tip > 0);
    let spend_cache = get_spend_cache_status(request, runtime_tip_hint);

    let mut info = snapshot.info;
    if snapshot.status_kind == RuntimeStatusKind::Error {
        info.syncing = false;
        info.status_kind = Some("error".to_string());
        return Ok(info);
    }

    if snapshot.status_kind == RuntimeStatusKind::Synced {
        if let Some(spend_cache) = spend_cache {
            if spend_cache.ready {
                info.syncing = false;
                info.status_kind = Some("synced".to_string());
                info.percent = Some(100.0);
            } else {
                info.syncing = true;
                info.status_kind = Some("syncing".to_string());
                info.percent = match (info.percent, spend_cache.percent) {
                    (Some(runtime_percent), Some(spend_percent)) => {
                        Some(runtime_percent.min(spend_percent))
                    }
                    (None, Some(spend_percent)) => Some(spend_percent),
                    (Some(runtime_percent), None) => Some(runtime_percent),
                    (None, None) => None,
                };
            }
        } else {
            info.syncing = true;
            info.status_kind = Some("syncing".to_string());
            info.percent = match info.percent {
                Some(percent) => Some(percent.min(99.9)),
                None => Some(0.0),
            };
        }
    } else if info.status_kind.is_none() {
        info.status_kind = Some("syncing".to_string());
    }

    Ok(info)
}

pub async fn get_runtime_diagnostics(
    request: &DlightRuntimeRequest,
) -> Result<DlightRuntimeDiagnostics, WalletError> {
    ensure_runtime(request).await?;
    let snapshot = get_runtime_snapshot(&request.runtime_key).unwrap_or_default();
    let runtime_tip_hint = snapshot
        .chain_tip_height
        .or(snapshot.estimated_tip_height)
        .filter(|tip| *tip > 0);

    let status_kind = match snapshot.status_kind {
        RuntimeStatusKind::Initializing => "initializing",
        RuntimeStatusKind::Syncing => "syncing",
        RuntimeStatusKind::Synced => "synced",
        RuntimeStatusKind::Error => "error",
    };
    let spend_cache = get_spend_cache_status(request, runtime_tip_hint);

    Ok(DlightRuntimeDiagnostics {
        runtime_key: request.runtime_key.clone(),
        status_kind: status_kind.to_string(),
        percent: snapshot.info.percent,
        scanned_height: snapshot.scanned_height,
        tip_height: snapshot.chain_tip_height,
        estimated_tip_height: snapshot.estimated_tip_height,
        syncing: snapshot.info.syncing,
        last_updated: snapshot.last_updated,
        last_progress_at: snapshot.last_progress_at,
        last_tip_probe_at: snapshot.last_tip_probe_at,
        consecutive_failures: snapshot.consecutive_failures,
        scan_rate_blocks_per_sec: snapshot.scan_rate_blocks_per_sec,
        stalled: snapshot.stalled,
        last_error: snapshot.last_error,
        spend_cache_ready: spend_cache.as_ref().map(|value| value.ready),
        spend_cache_status_kind: spend_cache.as_ref().map(|value| value.status_kind.clone()),
        spend_cache_percent: spend_cache.as_ref().and_then(|value| value.percent),
        spend_cache_lag_blocks: spend_cache.as_ref().map(|value| value.lag_blocks),
        spend_cache_last_error: spend_cache
            .as_ref()
            .and_then(|value| value.last_error.clone()),
        spend_cache_scanned_height: spend_cache.as_ref().map(|value| value.scanned_height),
        spend_cache_tip_height: spend_cache.as_ref().map(|value| value.effective_tip_height),
        spend_cache_last_updated: spend_cache.as_ref().map(|value| value.last_updated),
        spend_cache_note_count: spend_cache.as_ref().map(|value| value.note_count),
    })
}
