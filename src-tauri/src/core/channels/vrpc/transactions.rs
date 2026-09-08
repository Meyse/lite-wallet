//
// Module 5: VRPC transaction history via getaddressdeltas and getaddressmempool.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::core::channels::vrpc::common::{value_as_f64, HistoryEntry};
use crate::core::channels::vrpc::provider::VrpcProvider;
use crate::core::channels::vrpc::{VrpcCoinContext, VrpcTransactionsResult};
use crate::types::transaction::Transaction;
use crate::types::WalletError;

const RESERVE_TRANSFER_DESTINATION_ADDRESS: &str = "RTqQe58LSj2yr5CrwYFwcsAQ1edQwmrkUU";
const VRPC_WINDOW_BLOCK_COUNT: u64 = 2000;
const MAX_VRPC_WINDOW_REQUESTS: usize = 24;
const SATOSHIS_PER_COIN: f64 = 100_000_000.0;
const MEMPOOL_UNAVAILABLE_WARNING: &str = "Mempool temporarily unavailable";
const CHAIN_INFO_UNAVAILABLE_WARNING: &str = "Chain info unavailable";
const PAGED_HISTORY_UNAVAILABLE_WARNING: &str =
    "Paged history unavailable for this endpoint. Falling back to first page.";
const WINDOWED_PAGING_UNSUPPORTED_WARNING: &str = "VRPC endpoint does not support windowed paging.";

#[derive(Debug, Clone, Copy)]
pub struct VrpcHistoryCursor {
    pub end_block: u64,
    pub include_pending: bool,
}

#[derive(Debug, Clone)]
pub struct VrpcTransactionsPage {
    pub transactions: Vec<Transaction>,
    pub next_cursor: Option<VrpcHistoryCursor>,
    pub has_more: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Clone)]
struct AggregatedTransaction {
    transaction: Transaction,
    height: Option<u64>,
}

#[derive(Default)]
struct TxAggregate {
    delta: f64,
    pending: bool,
    height: Option<u64>,
    blocktime: Option<u64>,
    address: Option<String>,
    sent_output_addresses: Vec<String>,
}

/// Fetch transaction history for addresses with coin context.
pub async fn get_transactions(
    provider: &VrpcProvider,
    addresses: &[String],
    coin: &VrpcCoinContext,
) -> Result<VrpcTransactionsResult, WalletError> {
    if addresses.is_empty() {
        return Ok(VrpcTransactionsResult {
            transactions: vec![],
            warning: None,
        });
    }

    let mut warning_messages: Vec<String> = Vec::new();
    let longest_chain = fetch_longest_chain(provider, &mut warning_messages).await;
    let mut ordered_entries = Vec::<HistoryEntry>::new();
    append_mempool_entries(
        provider,
        addresses,
        &mut ordered_entries,
        &mut warning_messages,
    )
    .await;
    let deltas = provider.getaddressdeltas(addresses).await?;
    append_delta_entries(&mut ordered_entries, &deltas)?;

    let txs = aggregate_transactions(ordered_entries, addresses, coin, longest_chain);
    let warning = build_warning(&warning_messages);

    Ok(VrpcTransactionsResult {
        transactions: txs,
        warning,
    })
}

pub async fn get_transactions_page(
    provider: &VrpcProvider,
    addresses: &[String],
    coin: &VrpcCoinContext,
    cursor: Option<VrpcHistoryCursor>,
    limit: usize,
) -> Result<VrpcTransactionsPage, WalletError> {
    let safe_limit = limit.clamp(1, 100);
    if addresses.is_empty() {
        return Ok(VrpcTransactionsPage {
            transactions: vec![],
            next_cursor: None,
            has_more: false,
            warning: None,
        });
    }

    let mut warning_messages = Vec::<String>::new();
    let longest_chain = fetch_longest_chain(provider, &mut warning_messages).await;

    let include_pending = cursor.map(|value| value.include_pending).unwrap_or(true);
    let mut current_end_block = cursor
        .map(|value| value.end_block)
        .or(longest_chain)
        .unwrap_or(0);
    if current_end_block == 0 {
        if cursor.is_some() {
            return Ok(VrpcTransactionsPage {
                transactions: vec![],
                next_cursor: None,
                has_more: false,
                warning: build_warning(&warning_messages),
            });
        }

        warning_messages.push(PAGED_HISTORY_UNAVAILABLE_WARNING.to_string());
        return fallback_to_first_page(provider, addresses, coin, safe_limit, warning_messages)
            .await;
    }

    let mut ordered_entries = Vec::<HistoryEntry>::new();
    if include_pending {
        append_mempool_entries(
            provider,
            addresses,
            &mut ordered_entries,
            &mut warning_messages,
        )
        .await;
    }

    let mut reached_oldest_window = false;
    for _ in 0..MAX_VRPC_WINDOW_REQUESTS {
        if current_end_block == 0 {
            reached_oldest_window = true;
            break;
        }

        let start_block = current_end_block
            .saturating_sub(VRPC_WINDOW_BLOCK_COUNT.saturating_sub(1))
            .max(1);
        let deltas = match provider
            .getaddressdeltas_window(addresses, Some(start_block), Some(current_end_block))
            .await
        {
            Ok(value) => value,
            Err(error) => {
                if cursor.is_none() {
                    warning_messages.push(WINDOWED_PAGING_UNSUPPORTED_WARNING.to_string());
                    return fallback_to_first_page(
                        provider,
                        addresses,
                        coin,
                        safe_limit,
                        warning_messages,
                    )
                    .await;
                }
                return Err(error);
            }
        };

        push_history_entries(&mut ordered_entries, deltas.as_array(), false);

        let aggregated =
            aggregate_transactions_with_meta(&ordered_entries, addresses, coin, longest_chain);

        if aggregated.len() > safe_limit {
            let page = paginate_aggregated_transactions(aggregated, safe_limit, current_end_block);
            return Ok(VrpcTransactionsPage {
                transactions: page.transactions,
                next_cursor: page.next_cursor,
                has_more: page.has_more,
                warning: build_warning(&warning_messages),
            });
        }

        if start_block <= 1 {
            reached_oldest_window = true;
            current_end_block = 0;
            break;
        }

        current_end_block = start_block.saturating_sub(1);
    }

    let aggregated =
        aggregate_transactions_with_meta(&ordered_entries, addresses, coin, longest_chain);
    let transactions = aggregated
        .into_iter()
        .map(|item| item.transaction)
        .collect::<Vec<_>>();
    let has_more = !reached_oldest_window && !transactions.is_empty();
    let next_cursor = if has_more {
        Some(VrpcHistoryCursor {
            end_block: current_end_block,
            include_pending: false,
        })
    } else {
        None
    };

    Ok(VrpcTransactionsPage {
        transactions,
        next_cursor,
        has_more,
        warning: build_warning(&warning_messages),
    })
}

async fn fetch_longest_chain(
    provider: &VrpcProvider,
    warning_messages: &mut Vec<String>,
) -> Option<u64> {
    match provider.getinfo().await {
        Ok(info) => info
            .get("longestchain")
            .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64))),
        Err(_) => {
            warning_messages.push(CHAIN_INFO_UNAVAILABLE_WARNING.to_string());
            None
        }
    }
}

fn push_history_entries(
    ordered_entries: &mut Vec<HistoryEntry>,
    entries: Option<&Vec<Value>>,
    pending: bool,
) {
    if let Some(entries) = entries {
        ordered_entries.extend(
            entries
                .iter()
                .cloned()
                .map(|raw| HistoryEntry { raw, pending }),
        );
    }
}

async fn append_mempool_entries(
    provider: &VrpcProvider,
    addresses: &[String],
    ordered_entries: &mut Vec<HistoryEntry>,
    warning_messages: &mut Vec<String>,
) {
    match provider.getaddressmempool(addresses).await {
        Ok(value) => push_history_entries(ordered_entries, value.as_array(), true),
        Err(_) => warning_messages.push(MEMPOOL_UNAVAILABLE_WARNING.to_string()),
    }
}

fn append_delta_entries(
    ordered_entries: &mut Vec<HistoryEntry>,
    deltas: &Value,
) -> Result<(), WalletError> {
    let entries = deltas.as_array().ok_or(WalletError::OperationFailed)?;
    push_history_entries(ordered_entries, Some(entries), false);
    Ok(())
}

async fn fallback_to_first_page(
    provider: &VrpcProvider,
    addresses: &[String],
    coin: &VrpcCoinContext,
    safe_limit: usize,
    mut warning_messages: Vec<String>,
) -> Result<VrpcTransactionsPage, WalletError> {
    let fallback = get_transactions(provider, addresses, coin).await?;
    let mut transactions = fallback.transactions;
    if transactions.len() > safe_limit {
        transactions.truncate(safe_limit);
    }
    if let Some(warning) = fallback.warning {
        warning_messages.push(warning);
    }

    Ok(VrpcTransactionsPage {
        transactions,
        next_cursor: None,
        has_more: false,
        warning: build_warning(&warning_messages),
    })
}

fn build_warning(messages: &[String]) -> Option<String> {
    if messages.is_empty() {
        None
    } else {
        Some(messages.join("; "))
    }
}

struct PaginatedAggregateResult {
    transactions: Vec<Transaction>,
    has_more: bool,
    next_cursor: Option<VrpcHistoryCursor>,
}

fn paginate_aggregated_transactions(
    items: Vec<AggregatedTransaction>,
    limit: usize,
    current_end_block: u64,
) -> PaginatedAggregateResult {
    if items.len() <= limit {
        return PaginatedAggregateResult {
            transactions: items.into_iter().map(|item| item.transaction).collect(),
            has_more: false,
            next_cursor: None,
        };
    }

    let mut slice_end = limit;
    let boundary = &items[limit - 1];
    if boundary.transaction.pending {
        while slice_end < items.len() && items[slice_end].transaction.pending {
            slice_end += 1;
        }
    } else if let Some(boundary_height) = boundary.height {
        while slice_end < items.len() && items[slice_end].height == Some(boundary_height) {
            slice_end += 1;
        }
    }

    let has_more = slice_end < items.len();
    let next_cursor = if has_more {
        let next_end_block = items[slice_end - 1]
            .height
            .map(|height| height.saturating_sub(1))
            .unwrap_or(current_end_block);
        Some(VrpcHistoryCursor {
            end_block: next_end_block,
            include_pending: false,
        })
    } else {
        None
    };

    PaginatedAggregateResult {
        transactions: items
            .into_iter()
            .take(slice_end)
            .map(|item| item.transaction)
            .collect(),
        has_more,
        next_cursor,
    }
}

fn aggregate_transactions(
    ordered_entries: Vec<HistoryEntry>,
    owner_addresses: &[String],
    coin: &VrpcCoinContext,
    longest_chain: Option<u64>,
) -> Vec<Transaction> {
    aggregate_transactions_with_meta(&ordered_entries, owner_addresses, coin, longest_chain)
        .into_iter()
        .map(|item| item.transaction)
        .collect()
}

fn aggregate_transactions_with_meta(
    ordered_entries: &[HistoryEntry],
    owner_addresses: &[String],
    coin: &VrpcCoinContext,
    longest_chain: Option<u64>,
) -> Vec<AggregatedTransaction> {
    let owner_addresses_lower: Vec<String> = owner_addresses
        .iter()
        .map(|addr| addr.to_ascii_lowercase())
        .collect();
    let mut by_txid: HashMap<String, TxAggregate> = HashMap::new();

    for entry in ordered_entries {
        let Some(obj) = entry.raw.as_object() else {
            continue;
        };

        let txid = obj
            .get("txid")
            .or(obj.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if txid.is_empty() {
            continue;
        }

        let delta = extract_delta(obj, coin);
        if delta.abs() < f64::EPSILON {
            continue;
        }

        let raw_height = obj
            .get("height")
            .and_then(|v| v.as_i64().or_else(|| v.as_u64().map(|u| u as i64)));
        let blocktime = obj
            .get("blocktime")
            .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)));
        let address = obj
            .get("address")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let sent_output_addresses = extract_sent_output_addresses(obj, coin);

        let agg = by_txid.entry(txid).or_default();
        agg.delta += delta;
        agg.pending = agg.pending || entry.pending || raw_height.unwrap_or(0) <= 0;
        if let Some(height) = raw_height.and_then(|h| u64::try_from(h).ok()) {
            if height > 0 {
                agg.height = Some(agg.height.map_or(height, |prev| prev.max(height)));
            }
        }
        if agg.blocktime.is_none() {
            agg.blocktime = blocktime;
        }
        if agg.address.is_none() {
            agg.address = address;
        }
        for candidate in sent_output_addresses {
            push_unique_address(&mut agg.sent_output_addresses, candidate);
        }
    }

    let mut txs: Vec<AggregatedTransaction> = by_txid
        .into_iter()
        .filter_map(|(txid, agg)| {
            if agg.delta.abs() < f64::EPSILON {
                return None;
            }

            let confirmations = if agg.pending {
                0
            } else if let (Some(height), Some(longest)) = (agg.height, longest_chain) {
                if longest >= height {
                    (longest - height + 1) as i64
                } else {
                    1
                }
            } else if agg.height.is_some() {
                1
            } else {
                0
            };

            let timestamp = match (agg.blocktime, agg.height, longest_chain) {
                (Some(ts), _, _) => Some(ts),
                (None, Some(height), Some(longest)) if longest >= height => {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .ok()
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let confirms = longest - height;
                    let estimate_secs = confirms.saturating_mul(coin.seconds_per_block);
                    Some(now.saturating_sub(estimate_secs))
                }
                _ => None,
            };

            let amount = if coin.currency_id == coin.system_id {
                format!("{:.8}", agg.delta / 100_000_000.0)
            } else {
                format!("{:.*}", coin.decimals as usize, agg.delta)
            };

            let scope_address = agg.address.unwrap_or_default();
            let counterparty = select_sent_counterparty(
                &agg.sent_output_addresses,
                &owner_addresses_lower,
                &scope_address,
            )
            .unwrap_or_default();
            let (from_address, to_address) = if agg.delta < 0.0 {
                (scope_address, counterparty)
            } else {
                (counterparty, scope_address)
            };

            Some(AggregatedTransaction {
                height: agg.height,
                transaction: Transaction {
                    txid,
                    amount,
                    from_address,
                    to_address,
                    confirmations,
                    timestamp,
                    pending: agg.pending,
                },
            })
        })
        .collect();

    txs.sort_by(|a, b| {
        b.transaction.pending.cmp(&a.transaction.pending).then(
            b.transaction
                .timestamp
                .unwrap_or(0)
                .cmp(&a.transaction.timestamp.unwrap_or(0)),
        )
    });

    txs
}

fn extract_delta(obj: &serde_json::Map<String, Value>, coin: &VrpcCoinContext) -> f64 {
    if coin.currency_id == coin.system_id {
        let satoshis = obj.get("satoshis").and_then(value_as_f64).unwrap_or(0.0);
        if satoshis.abs() >= f64::EPSILON {
            return satoshis;
        }

        let native_currency_value =
            extract_currency_value_delta(obj, &coin.currency_id).unwrap_or(0.0);
        if native_currency_value.abs() >= f64::EPSILON {
            return native_currency_value * SATOSHIS_PER_COIN;
        }

        if obj
            .get("spending")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let scope_address = obj.get("address").and_then(Value::as_str).unwrap_or("");
            let sent_total = extract_sent_output_total(obj, &coin.currency_id, scope_address);
            if sent_total.abs() >= f64::EPSILON {
                return -sent_total * SATOSHIS_PER_COIN;
            }
        }

        return 0.0;
    }

    let currency_value = extract_currency_value_delta(obj, &coin.currency_id).unwrap_or(0.0);
    if currency_value.abs() >= f64::EPSILON {
        return currency_value;
    }

    if obj
        .get("spending")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let scope_address = obj.get("address").and_then(Value::as_str).unwrap_or("");
        let sent_total = extract_sent_output_total(obj, &coin.currency_id, scope_address);
        if sent_total.abs() >= f64::EPSILON {
            return -sent_total;
        }
    }

    0.0
}

fn extract_currency_value_delta(
    obj: &serde_json::Map<String, Value>,
    currency_id: &str,
) -> Option<f64> {
    obj.get("currencyvalues")
        .and_then(|value| value.get(currency_id))
        .and_then(value_as_f64)
}

fn extract_sent_output_total(
    obj: &serde_json::Map<String, Value>,
    currency_id: &str,
    scope_address: &str,
) -> f64 {
    let Some(outputs) = obj
        .get("sent")
        .and_then(|value| value.get("outputs"))
        .and_then(Value::as_array)
    else {
        return 0.0;
    };

    let mut total = 0.0;
    for output in outputs {
        let Some(output_object) = output.as_object() else {
            continue;
        };

        if !scope_address.trim().is_empty()
            && output_addresses_contains(output_object.get("addresses"), scope_address)
        {
            continue;
        }

        let amount = output_object
            .get("amounts")
            .and_then(|amounts| amounts.get(currency_id))
            .and_then(value_as_f64)
            .unwrap_or(0.0);
        total += amount;
    }

    total
}

fn output_addresses_contains(addresses: Option<&Value>, candidate: &str) -> bool {
    let normalized = candidate.trim();
    if normalized.is_empty() {
        return false;
    }

    match addresses {
        Some(Value::String(address)) => address.eq_ignore_ascii_case(normalized),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .any(|address| address.eq_ignore_ascii_case(normalized)),
        _ => false,
    }
}

fn extract_sent_output_addresses(
    obj: &serde_json::Map<String, Value>,
    coin: &VrpcCoinContext,
) -> Vec<String> {
    let mut addresses = Vec::new();

    let Some(outputs) = obj
        .get("sent")
        .and_then(|value| value.get("outputs"))
        .and_then(|value| value.as_array())
    else {
        return addresses;
    };

    for output in outputs {
        let include_output = match output.get("amounts") {
            Some(Value::Object(amounts)) => amounts
                .get(&coin.currency_id)
                .and_then(value_as_f64)
                .map(|amount| amount > 0.0)
                .unwrap_or(false),
            Some(_) => false,
            None => true,
        };

        if !include_output {
            continue;
        }

        let Some(raw_addresses) = output.get("addresses") else {
            continue;
        };

        match raw_addresses {
            Value::String(address) => {
                push_unique_address(&mut addresses, address.to_string());
            }
            Value::Array(items) => {
                for item in items {
                    if let Some(address) = item.as_str() {
                        push_unique_address(&mut addresses, address.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    addresses
}

fn push_unique_address(addresses: &mut Vec<String>, candidate: String) {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return;
    }

    if addresses
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(trimmed))
    {
        return;
    }

    addresses.push(trimmed.to_string());
}

fn select_sent_counterparty(
    sent_output_addresses: &[String],
    owner_addresses_lower: &[String],
    scope_address: &str,
) -> Option<String> {
    for candidate in sent_output_addresses {
        let candidate_lower = candidate.to_ascii_lowercase();
        if candidate.eq_ignore_ascii_case(RESERVE_TRANSFER_DESTINATION_ADDRESS) {
            continue;
        }
        if candidate.eq_ignore_ascii_case(scope_address) {
            continue;
        }
        if owner_addresses_lower
            .iter()
            .any(|owner| owner == &candidate_lower)
        {
            continue;
        }
        return Some(candidate.clone());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history_entry(raw: Value, pending: bool) -> HistoryEntry {
        HistoryEntry { raw, pending }
    }

    #[test]
    fn aggregates_same_txid_deltas_and_drops_zero_net() {
        let coin = VrpcCoinContext {
            currency_id: "iNative".to_string(),
            system_id: "iNative".to_string(),
            decimals: 8,
            seconds_per_block: 60,
        };
        let owner_addresses = vec!["Rwallet".to_string()];

        let entries = vec![
            history_entry(
                serde_json::json!({
                    "txid": "tx-1",
                    "satoshis": 100000000,
                    "height": 10
                }),
                false,
            ),
            history_entry(
                serde_json::json!({
                    "txid": "tx-1",
                    "satoshis": -50000000,
                    "height": 10
                }),
                false,
            ),
            history_entry(
                serde_json::json!({
                    "txid": "tx-zero",
                    "satoshis": 1000,
                    "height": 11
                }),
                false,
            ),
            history_entry(
                serde_json::json!({
                    "txid": "tx-zero",
                    "satoshis": -1000,
                    "height": 11
                }),
                false,
            ),
        ];

        let txs = aggregate_transactions(entries, &owner_addresses, &coin, Some(20));
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].txid, "tx-1");
        assert_eq!(txs[0].amount, "0.50000000");
        assert_eq!(txs[0].confirmations, 11);
    }

    #[test]
    fn mempool_entries_are_pending() {
        let coin = VrpcCoinContext {
            currency_id: "iNative".to_string(),
            system_id: "iNative".to_string(),
            decimals: 8,
            seconds_per_block: 60,
        };
        let owner_addresses = vec!["Rwallet".to_string()];
        let entries = vec![history_entry(
            serde_json::json!({
                "txid": "tx-mempool",
                "satoshis": 100000,
                "height": -1
            }),
            true,
        )];

        let txs = aggregate_transactions(entries, &owner_addresses, &coin, Some(100));
        assert_eq!(txs.len(), 1);
        assert!(txs[0].pending);
        assert_eq!(txs[0].confirmations, 0);
    }

    #[test]
    fn outgoing_uses_sent_output_counterparty() {
        let coin = VrpcCoinContext {
            currency_id: "iNative".to_string(),
            system_id: "iNative".to_string(),
            decimals: 8,
            seconds_per_block: 60,
        };
        let owner_addresses = vec!["Rwallet".to_string()];
        let entries = vec![history_entry(
            serde_json::json!({
                "txid": "tx-sent",
                "satoshis": -100000000,
                "height": 12,
                "address": "Rwallet",
                "sent": {
                    "outputs": [
                        {
                            "addresses": "Rwallet",
                            "amounts": { "iNative": 0.5 }
                        },
                        {
                            "addresses": "Rrecipient",
                            "amounts": { "iNative": 0.5 }
                        }
                    ]
                }
            }),
            false,
        )];

        let txs = aggregate_transactions(entries, &owner_addresses, &coin, Some(20));
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].from_address, "Rwallet");
        assert_eq!(txs[0].to_address, "Rrecipient");
    }

    #[test]
    fn outgoing_filters_reserve_transfer_destination() {
        let coin = VrpcCoinContext {
            currency_id: "iNative".to_string(),
            system_id: "iNative".to_string(),
            decimals: 8,
            seconds_per_block: 60,
        };
        let owner_addresses = vec!["Rwallet".to_string()];
        let entries = vec![history_entry(
            serde_json::json!({
                "txid": "tx-reserve",
                "satoshis": -50000000,
                "height": 12,
                "address": "Rwallet",
                "sent": {
                    "outputs": [
                        {
                            "addresses": "Rwallet",
                            "amounts": { "iNative": 0.25 }
                        },
                        {
                            "addresses": "RTqQe58LSj2yr5CrwYFwcsAQ1edQwmrkUU",
                            "amounts": { "iNative": 0.25 }
                        }
                    ]
                }
            }),
            false,
        )];

        let txs = aggregate_transactions(entries, &owner_addresses, &coin, Some(20));
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].from_address, "Rwallet");
        assert!(txs[0].to_address.is_empty());
    }

    #[test]
    fn outgoing_ignores_counterparty_with_zero_amount_for_selected_coin() {
        let coin = VrpcCoinContext {
            currency_id: "iNative".to_string(),
            system_id: "iNative".to_string(),
            decimals: 8,
            seconds_per_block: 60,
        };
        let owner_addresses = vec!["Rwallet".to_string()];
        let entries = vec![history_entry(
            serde_json::json!({
                "txid": "tx-zero-counterparty",
                "satoshis": -100000000,
                "height": 12,
                "address": "Rwallet",
                "sent": {
                    "outputs": [
                        {
                            "addresses": "Rwallet",
                            "amounts": { "iNative": 0.9998 }
                        },
                        {
                            "addresses": "Rexternal",
                            "amounts": {
                                "iNative": 0.0,
                                "iOther": 1.0
                            }
                        }
                    ]
                }
            }),
            false,
        )];

        let txs = aggregate_transactions(entries, &owner_addresses, &coin, Some(20));
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].from_address, "Rwallet");
        assert!(txs[0].to_address.is_empty());
    }

    #[test]
    fn pagination_keeps_same_block_entries_together() {
        let txs = vec![
            AggregatedTransaction {
                transaction: Transaction {
                    txid: "a".to_string(),
                    amount: "1".to_string(),
                    from_address: String::new(),
                    to_address: String::new(),
                    confirmations: 1,
                    timestamp: Some(100),
                    pending: false,
                },
                height: Some(20),
            },
            AggregatedTransaction {
                transaction: Transaction {
                    txid: "b".to_string(),
                    amount: "1".to_string(),
                    from_address: String::new(),
                    to_address: String::new(),
                    confirmations: 1,
                    timestamp: Some(99),
                    pending: false,
                },
                height: Some(20),
            },
            AggregatedTransaction {
                transaction: Transaction {
                    txid: "c".to_string(),
                    amount: "1".to_string(),
                    from_address: String::new(),
                    to_address: String::new(),
                    confirmations: 1,
                    timestamp: Some(98),
                    pending: false,
                },
                height: Some(19),
            },
        ];

        let page = paginate_aggregated_transactions(txs, 1, 22);
        assert_eq!(page.transactions.len(), 2);
        assert!(page.has_more);
        assert_eq!(page.next_cursor.expect("cursor").end_block, 19);
    }

    #[test]
    fn extract_delta_uses_currencyvalues_when_scope_system_differs() {
        let coin = VrpcCoinContext {
            currency_id: "iVdex".to_string(),
            system_id: "iVrsc".to_string(),
            decimals: 8,
            seconds_per_block: 60,
        };
        let obj = serde_json::json!({
            "satoshis": -39278147,
            "currencyvalues": {
                "iVrsc": -0.39278147,
                "iVdex": -0.1003001
            }
        });
        let map = obj.as_object().expect("delta object");

        let delta = extract_delta(map, &coin);
        assert!((delta - (-0.1003001f64)).abs() < 1e-12);
    }

    #[test]
    fn extract_delta_uses_currencyvalues_for_native_when_satoshis_missing() {
        let coin = VrpcCoinContext {
            currency_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            decimals: 8,
            seconds_per_block: 60,
        };
        let obj = serde_json::json!({
            "satoshis": 0,
            "currencyvalues": {
                "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV": 0.5
            }
        });
        let map = obj.as_object().expect("delta object");

        let delta = extract_delta(map, &coin);
        assert_eq!(delta, 50_000_000.0);
    }

    #[test]
    fn extract_delta_uses_sent_outputs_for_native_identity_spends() {
        let coin = VrpcCoinContext {
            currency_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            decimals: 8,
            seconds_per_block: 60,
        };
        let obj = serde_json::json!({
            "address": "iIdentityAddress",
            "satoshis": 0,
            "spending": true,
            "sent": {
                "outputs": [
                    {
                        "addresses": "RRecipientAddress",
                        "amounts": {
                            "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV": 0.01001903
                        }
                    },
                    {
                        "addresses": "iIdentityAddress",
                        "amounts": {
                            "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV": 0.0
                        }
                    }
                ]
            }
        });
        let map = obj.as_object().expect("delta object");

        let delta = extract_delta(map, &coin);
        assert_eq!(delta, -1_001_903.0);
    }
}
