use ethers::types::U256;
use ethers::utils::format_units;

use crate::core::channels::eth::provider::{EthNetworkProvider, EtherscanTxRecord};
use crate::core::coins::CoinDefinition;
use crate::types::transaction::Transaction;
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

#[derive(Debug, Clone)]
pub struct EthTransactionsPage {
    pub transactions: Vec<Transaction>,
    pub next_page: u32,
    pub has_more: bool,
}

pub async fn get_eth_transactions(
    provider: &EthNetworkProvider,
    network: WalletNetwork,
    address: &str,
) -> Result<Vec<Transaction>, WalletError> {
    provider.validate_rpc_identity().await?;
    let records = provider
        .history_provider
        .get_eth_history(network, address)
        .await?;

    let mut txs = records
        .into_iter()
        .map(|record| map_record_to_transaction(record, address, 18, true))
        .collect::<Vec<_>>();

    sort_transactions(&mut txs);
    Ok(txs)
}

pub async fn get_eth_transactions_page(
    provider: &EthNetworkProvider,
    network: WalletNetwork,
    address: &str,
    page: u32,
    limit: usize,
) -> Result<EthTransactionsPage, WalletError> {
    provider.validate_rpc_identity().await?;
    let safe_limit = limit.clamp(1, 100) as u32;
    let safe_page = page.max(1);
    let records = provider
        .history_provider
        .get_eth_history_page(network, address, safe_page, safe_limit)
        .await?;

    let mut txs = records
        .into_iter()
        .map(|record| map_record_to_transaction(record, address, 18, true))
        .collect::<Vec<_>>();
    sort_transactions(&mut txs);

    let has_more = txs.len() >= safe_limit as usize;
    Ok(EthTransactionsPage {
        transactions: txs,
        next_page: safe_page.saturating_add(1),
        has_more,
    })
}

pub async fn get_erc20_transactions(
    provider: &EthNetworkProvider,
    network: WalletNetwork,
    address: &str,
    coin: &CoinDefinition,
) -> Result<Vec<Transaction>, WalletError> {
    provider.validate_rpc_identity().await?;
    let records = provider
        .history_provider
        .get_erc20_history(network, address, &coin.currency_id)
        .await?;

    let mut txs = records
        .into_iter()
        .map(|record| map_record_to_transaction(record, address, coin.decimals as usize, false))
        .collect::<Vec<_>>();

    sort_transactions(&mut txs);
    Ok(txs)
}

pub async fn get_erc20_transactions_page(
    provider: &EthNetworkProvider,
    network: WalletNetwork,
    address: &str,
    coin: &CoinDefinition,
    page: u32,
    limit: usize,
) -> Result<EthTransactionsPage, WalletError> {
    provider.validate_rpc_identity().await?;
    let safe_limit = limit.clamp(1, 100) as u32;
    let safe_page = page.max(1);
    let records = provider
        .history_provider
        .get_erc20_history_page(network, address, &coin.currency_id, safe_page, safe_limit)
        .await?;

    let mut txs = records
        .into_iter()
        .map(|record| map_record_to_transaction(record, address, coin.decimals as usize, false))
        .collect::<Vec<_>>();
    sort_transactions(&mut txs);

    let has_more = txs.len() >= safe_limit as usize;
    Ok(EthTransactionsPage {
        transactions: txs,
        next_page: safe_page.saturating_add(1),
        has_more,
    })
}

fn map_record_to_transaction(
    record: EtherscanTxRecord,
    owner_address: &str,
    decimals: usize,
    is_eth: bool,
) -> Transaction {
    let from_lower = record.from.to_ascii_lowercase();
    let to_lower = record.to.to_ascii_lowercase();
    let owner_lower = owner_address.to_ascii_lowercase();

    let is_self = !from_lower.is_empty() && from_lower == to_lower && from_lower == owner_lower;
    let is_sent = !is_self && !from_lower.is_empty() && from_lower == owner_lower;

    let value_raw = parse_u256_dec(&record.value).unwrap_or_else(U256::zero);
    let amount_display = format_units(value_raw, decimals).unwrap_or_else(|_| "0".to_string());

    let fee_display = fee_for_record(&record);

    let amount = if is_self {
        if is_eth {
            fee_display.clone()
        } else {
            "0".to_string()
        }
    } else if is_sent {
        if amount_display.starts_with('-') {
            amount_display
        } else {
            format!("-{}", amount_display)
        }
    } else {
        amount_display
    };

    let confirmations = record.confirmations.parse::<i64>().unwrap_or(0);
    let timestamp = record.time_stamp.parse::<u64>().ok();

    Transaction {
        txid: record.hash,
        amount,
        from_address: record.from,
        to_address: record.to,
        confirmations,
        timestamp,
        pending: confirmations <= 0,
    }
}

fn fee_for_record(record: &EtherscanTxRecord) -> String {
    let gas_price = parse_u256_dec(&record.gas_price).unwrap_or_else(U256::zero);
    let gas_used = parse_u256_dec(&record.gas_used).unwrap_or_else(U256::zero);
    let fee_wei = gas_price.saturating_mul(gas_used);
    format_units(fee_wei, 18).unwrap_or_else(|_| "0".to_string())
}

fn parse_u256_dec(value: &str) -> Option<U256> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    U256::from_dec_str(trimmed).ok()
}

fn sort_transactions(items: &mut [Transaction]) {
    items.sort_by(|a, b| {
        b.pending
            .cmp(&a.pending)
            .then(b.timestamp.unwrap_or(0).cmp(&a.timestamp.unwrap_or(0)))
    });
}
