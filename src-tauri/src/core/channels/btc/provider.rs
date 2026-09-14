//
// Module 5d: BTC REST provider. Balance, UTXOs, tx history, broadcast.
// Security: Endpoint comes from backend runtime config; no sensitive data in logs.

use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::core::runtime_config;
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

#[derive(Debug, Deserialize)]
pub struct AddressInfo {
    pub chain_stats: ChainStats,
    pub mempool_stats: MempoolStats,
}

#[derive(Debug, Deserialize)]
pub struct ChainStats {
    pub funded_txo_sum: u64,
    pub spent_txo_sum: u64,
}

#[derive(Debug, Deserialize)]
pub struct MempoolStats {
    pub funded_txo_sum: u64,
    pub spent_txo_sum: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UtxoEntry {
    pub txid: String,
    pub vout: u32,
    pub value: u64,
    pub status: UtxoStatus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UtxoStatus {
    pub confirmed: bool,
    #[serde(default)]
    pub block_height: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct MempoolTx {
    pub txid: String,
    pub vin: Vec<Value>,
    pub vout: Vec<Value>,
    pub status: Option<MempoolTxStatus>,
    #[serde(default)]
    pub fee: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct MempoolTxStatus {
    pub confirmed: bool,
    #[serde(default)]
    pub block_height: Option<u32>,
    #[serde(default)]
    pub block_time: Option<u64>,
}

/// HTTP REST client for Bitcoin. Base URL comes from runtime config.
pub struct BtcProvider {
    client: Client,
    base_url: String,
}

impl BtcProvider {
    fn build_http_client() -> Client {
        Client::builder()
            .connect_timeout(Duration::from_secs(4))
            .timeout(Duration::from_secs(12))
            .build()
            .unwrap_or_else(|_| Client::new())
    }

    pub fn new_mainnet() -> Self {
        Self {
            client: Self::build_http_client(),
            base_url: runtime_config::btc_mainnet_api_url(),
        }
    }

    pub fn new_testnet() -> Self {
        Self {
            client: Self::build_http_client(),
            base_url: runtime_config::btc_testnet_api_url(),
        }
    }

    pub fn new() -> Self {
        Self::new_mainnet()
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    /// GET /address/:address -> balance info.
    pub async fn get_address_info(&self, address: &str) -> Result<AddressInfo, WalletError> {
        let url = self.url(&format!("address/{}", address));
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|_| WalletError::NetworkError)?;
        if !res.status().is_success() {
            return Err(WalletError::NetworkError);
        }
        let info: AddressInfo = res.json().await.map_err(|_| WalletError::OperationFailed)?;
        Ok(info)
    }

    /// GET /address/:address/utxo -> list of UTXOs.
    pub async fn get_utxos(&self, address: &str) -> Result<Vec<UtxoEntry>, WalletError> {
        let url = self.url(&format!("address/{}/utxo", address));
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|_| WalletError::NetworkError)?;
        if !res.status().is_success() {
            return Err(WalletError::NetworkError);
        }
        let list: Vec<UtxoEntry> = res.json().await.map_err(|_| WalletError::OperationFailed)?;
        Ok(list)
    }

    /// Fetch canonical previous transaction bytes. Callers must verify that
    /// the decoded transaction hashes to `txid` before trusting its outputs.
    pub async fn get_transaction_hex(&self, txid: &str) -> Result<String, WalletError> {
        let url = self.url(&format!("tx/{}/hex", txid));
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|_| WalletError::NetworkError)?;
        if !res.status().is_success() {
            return Err(WalletError::NetworkError);
        }
        let tx_hex = res.text().await.map_err(|_| WalletError::OperationFailed)?;
        let tx_hex = tx_hex.trim();
        if tx_hex.is_empty() {
            return Err(WalletError::OperationFailed);
        }
        Ok(tx_hex.to_string())
    }

    /// GET /address/:address/txs/chain -> confirmed txs (first page).
    pub async fn get_address_txs(&self, address: &str) -> Result<Vec<MempoolTx>, WalletError> {
        let url = self.url(&format!("address/{}/txs/chain", address));
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|_| WalletError::NetworkError)?;
        if !res.status().is_success() {
            return Err(WalletError::NetworkError);
        }
        let list: Vec<MempoolTx> = res.json().await.map_err(|_| WalletError::OperationFailed)?;
        Ok(list)
    }

    /// GET /address/:address/txs/chain/:last_seen_txid -> older confirmed txs page.
    pub async fn get_address_txs_chain_after(
        &self,
        address: &str,
        last_seen_txid: &str,
    ) -> Result<Vec<MempoolTx>, WalletError> {
        let url = self.url(&format!("address/{}/txs/chain/{}", address, last_seen_txid));
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|_| WalletError::NetworkError)?;
        if !res.status().is_success() {
            return Err(WalletError::NetworkError);
        }
        let list: Vec<MempoolTx> = res.json().await.map_err(|_| WalletError::OperationFailed)?;
        Ok(list)
    }

    /// POST /tx with raw tx hex body. Returns txid on success.
    pub async fn broadcast(&self, tx_hex: &str) -> Result<String, WalletError> {
        let url = self.url("tx");
        let body = tx_hex.trim_start_matches("0x");
        let res = self
            .client
            .post(&url)
            .body(body.to_string())
            .header("Content-Type", "text/plain")
            .send()
            .await
            .map_err(|_| WalletError::NetworkError)?;
        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            if status.as_u16() == 400 && text.contains("txid") {
                if let Some(start) = text.find('"') {
                    let rest = &text[start + 1..];
                    if let Some(end) = rest.find('"') {
                        return Ok(rest[..end].to_string());
                    }
                }
            }
            return Err(WalletError::OperationFailed);
        }
        let txid: String = res.text().await.map_err(|_| WalletError::OperationFailed)?;
        Ok(txid.trim().trim_matches('"').to_string())
    }
}

impl Default for BtcProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared mainnet/testnet BTC providers; select by active wallet network.
pub struct BtcProviderPool {
    mainnet: BtcProvider,
    testnet: BtcProvider,
}

impl BtcProviderPool {
    pub fn new() -> Self {
        Self {
            mainnet: BtcProvider::new_mainnet(),
            testnet: BtcProvider::new_testnet(),
        }
    }

    pub fn for_network(&self, network: WalletNetwork) -> &BtcProvider {
        match network {
            WalletNetwork::Mainnet => &self.mainnet,
            WalletNetwork::Testnet => &self.testnet,
        }
    }
}

impl Default for BtcProviderPool {
    fn default() -> Self {
        Self::new()
    }
}
