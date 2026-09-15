use std::time::Duration;

use ethers::providers::{Http, Provider};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::core::channels::eth::config::EthChannelConfig;
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

const ETHEREUM_MAINNET_CHAIN_ID: u64 = 1;
const ETHEREUM_SEPOLIA_CHAIN_ID: u64 = 11155111;

#[derive(Debug, Clone)]
pub struct EthNetworkProvider {
    pub chain_id: u64,
    pub rpc_provider: Provider<Http>,
    pub history_provider: EtherscanHistoryClient,
}

pub struct EthProviderPool {
    mainnet: Option<EthNetworkProvider>,
    testnet: Option<EthNetworkProvider>,
    disabled_reason: Option<String>,
}

impl EthProviderPool {
    pub fn new() -> Self {
        let cfg = match EthChannelConfig::from_env() {
            Ok(cfg) => cfg,
            Err(_) => {
                return Self {
                    mainnet: None,
                    testnet: None,
                    disabled_reason: Some(
                        "Missing INFURA_PROJECT_ID or ETHERSCAN_API_KEY environment variables"
                            .to_string(),
                    ),
                }
            }
        };

        let mainnet_rpc = match Provider::<Http>::try_from(cfg.mainnet_rpc_url.as_str()) {
            Ok(provider) => provider.interval(Duration::from_millis(250)),
            Err(err) => {
                return Self {
                    mainnet: None,
                    testnet: None,
                    disabled_reason: Some(format!("Invalid ETH mainnet RPC URL: {}", err)),
                }
            }
        };

        let testnet_rpc = match Provider::<Http>::try_from(cfg.testnet_rpc_url.as_str()) {
            Ok(provider) => provider.interval(Duration::from_millis(250)),
            Err(err) => {
                return Self {
                    mainnet: None,
                    testnet: None,
                    disabled_reason: Some(format!("Invalid ETH testnet RPC URL: {}", err)),
                }
            }
        };

        let history_client = EtherscanHistoryClient::new(
            cfg.etherscan_api_key,
            cfg.etherscan_mainnet_url,
            cfg.etherscan_testnet_url,
        );

        Self {
            mainnet: Some(EthNetworkProvider {
                chain_id: ETHEREUM_MAINNET_CHAIN_ID,
                rpc_provider: mainnet_rpc,
                history_provider: history_client.clone(),
            }),
            testnet: Some(EthNetworkProvider {
                chain_id: ETHEREUM_SEPOLIA_CHAIN_ID,
                rpc_provider: testnet_rpc,
                history_provider: history_client,
            }),
            disabled_reason: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn for_tests(network: WalletNetwork, rpc_url: &str) -> Self {
        let provider = Provider::<Http>::try_from(rpc_url)
            .expect("valid test Ethereum RPC URL")
            .interval(Duration::from_millis(1));
        let network_provider = EthNetworkProvider {
            chain_id: match network {
                WalletNetwork::Mainnet => ETHEREUM_MAINNET_CHAIN_ID,
                WalletNetwork::Testnet => ETHEREUM_SEPOLIA_CHAIN_ID,
            },
            rpc_provider: provider,
            history_provider: EtherscanHistoryClient::new(
                String::new(),
                String::new(),
                String::new(),
            ),
        };
        match network {
            WalletNetwork::Mainnet => Self {
                mainnet: Some(network_provider),
                testnet: None,
                disabled_reason: None,
            },
            WalletNetwork::Testnet => Self {
                mainnet: None,
                testnet: Some(network_provider),
                disabled_reason: None,
            },
        }
    }

    #[cfg(test)]
    pub(crate) fn disabled_for_tests() -> Self {
        Self {
            mainnet: None,
            testnet: None,
            disabled_reason: Some("disabled for test".to_string()),
        }
    }

    pub fn is_enabled(&self) -> bool {
        #[cfg(test)]
        if self.disabled_reason.is_none() && (self.mainnet.is_some() || self.testnet.is_some()) {
            return true;
        }
        self.mainnet.is_some() && self.testnet.is_some()
    }

    pub fn disabled_reason(&self) -> Option<&str> {
        self.disabled_reason.as_deref()
    }

    pub fn for_network(&self, network: WalletNetwork) -> Result<&EthNetworkProvider, WalletError> {
        if !self.is_enabled() {
            return Err(WalletError::EthNotConfigured);
        }

        match network {
            WalletNetwork::Mainnet => self.mainnet.as_ref().ok_or(WalletError::EthNotConfigured),
            WalletNetwork::Testnet => self.testnet.as_ref().ok_or(WalletError::EthNotConfigured),
        }
    }
}

impl Default for EthProviderPool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct EtherscanHistoryClient {
    api_key: String,
    mainnet_url: String,
    testnet_url: String,
    client: Client,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct EtherscanTxRecord {
    #[serde(default, alias = "blockNumber")]
    pub block_number: String,
    #[serde(default, alias = "timeStamp", alias = "timestamp")]
    pub time_stamp: String,
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub nonce: String,
    #[serde(default, alias = "blockHash")]
    pub block_hash: String,
    #[serde(default, alias = "transactionIndex")]
    pub transaction_index: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub gas: String,
    #[serde(default, alias = "gasPrice")]
    pub gas_price: String,
    #[serde(default, alias = "gasUsed")]
    pub gas_used: String,
    #[serde(default, alias = "cumulativeGasUsed")]
    pub cumulative_gas_used: String,
    #[serde(default, alias = "contractAddress", alias = "contractaddress")]
    pub contract_address: String,
    #[serde(default)]
    pub confirmations: String,
}

impl EtherscanHistoryClient {
    pub fn new(api_key: String, mainnet_url: String, testnet_url: String) -> Self {
        Self {
            api_key,
            mainnet_url,
            testnet_url,
            client: Client::builder()
                .connect_timeout(Duration::from_secs(4))
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    pub async fn get_eth_history(
        &self,
        network: WalletNetwork,
        address: &str,
    ) -> Result<Vec<EtherscanTxRecord>, WalletError> {
        self.fetch_history(network, "txlist", address, None).await
    }

    pub async fn get_erc20_history(
        &self,
        network: WalletNetwork,
        address: &str,
        contract_address: &str,
    ) -> Result<Vec<EtherscanTxRecord>, WalletError> {
        self.fetch_history(network, "tokentx", address, Some(contract_address))
            .await
    }

    pub async fn get_eth_history_page(
        &self,
        network: WalletNetwork,
        address: &str,
        page: u32,
        offset: u32,
    ) -> Result<Vec<EtherscanTxRecord>, WalletError> {
        self.fetch_history_page(network, "txlist", address, None, page, offset)
            .await
    }

    pub async fn get_erc20_history_page(
        &self,
        network: WalletNetwork,
        address: &str,
        contract_address: &str,
        page: u32,
        offset: u32,
    ) -> Result<Vec<EtherscanTxRecord>, WalletError> {
        self.fetch_history_page(
            network,
            "tokentx",
            address,
            Some(contract_address),
            page,
            offset,
        )
        .await
    }

    async fn fetch_history(
        &self,
        network: WalletNetwork,
        action: &str,
        address: &str,
        contract_address: Option<&str>,
    ) -> Result<Vec<EtherscanTxRecord>, WalletError> {
        let url = match network {
            WalletNetwork::Mainnet => &self.mainnet_url,
            WalletNetwork::Testnet => &self.testnet_url,
        };

        let query = self.build_history_query(network, action, address, contract_address);

        let response = self
            .client
            .get(url)
            .query(&query)
            .send()
            .await
            .map_err(|_| WalletError::NetworkError)?;

        if !response.status().is_success() {
            return Err(WalletError::NetworkError);
        }

        let payload: Value = response
            .json()
            .await
            .map_err(|_| WalletError::OperationFailed)?;
        Self::parse_history_payload(network, action, payload)
    }

    async fn fetch_history_page(
        &self,
        network: WalletNetwork,
        action: &str,
        address: &str,
        contract_address: Option<&str>,
        page: u32,
        offset: u32,
    ) -> Result<Vec<EtherscanTxRecord>, WalletError> {
        let url = match network {
            WalletNetwork::Mainnet => &self.mainnet_url,
            WalletNetwork::Testnet => &self.testnet_url,
        };

        let query =
            self.build_history_page_query(network, action, address, contract_address, page, offset);

        let response = self
            .client
            .get(url)
            .query(&query)
            .send()
            .await
            .map_err(|_| WalletError::NetworkError)?;

        if !response.status().is_success() {
            return Err(WalletError::NetworkError);
        }

        let payload: Value = response
            .json()
            .await
            .map_err(|_| WalletError::OperationFailed)?;
        Self::parse_history_payload(network, action, payload)
    }

    fn build_history_query(
        &self,
        network: WalletNetwork,
        action: &str,
        address: &str,
        contract_address: Option<&str>,
    ) -> Vec<(String, String)> {
        let mut query = vec![
            (
                "chainid".to_string(),
                chain_id_for_network(network).to_string(),
            ),
            ("module".to_string(), "account".to_string()),
            ("action".to_string(), action.to_string()),
            ("address".to_string(), address.to_string()),
            ("startblock".to_string(), "0".to_string()),
            ("endblock".to_string(), "99999999".to_string()),
            ("sort".to_string(), "asc".to_string()),
            ("apikey".to_string(), self.api_key.clone()),
        ];

        if let Some(contract) = contract_address {
            // Match valu-mobile's ethers provider parameter naming.
            query.push(("contractAddress".to_string(), contract.to_string()));
        }

        query
    }

    fn build_history_page_query(
        &self,
        network: WalletNetwork,
        action: &str,
        address: &str,
        contract_address: Option<&str>,
        page: u32,
        offset: u32,
    ) -> Vec<(String, String)> {
        let safe_page = page.max(1);
        let safe_offset = offset.clamp(1, 100);
        let mut query = vec![
            (
                "chainid".to_string(),
                chain_id_for_network(network).to_string(),
            ),
            ("module".to_string(), "account".to_string()),
            ("action".to_string(), action.to_string()),
            ("address".to_string(), address.to_string()),
            ("startblock".to_string(), "0".to_string()),
            ("endblock".to_string(), "99999999".to_string()),
            ("page".to_string(), safe_page.to_string()),
            ("offset".to_string(), safe_offset.to_string()),
            ("sort".to_string(), "desc".to_string()),
            ("apikey".to_string(), self.api_key.clone()),
        ];

        if let Some(contract) = contract_address {
            query.push(("contractAddress".to_string(), contract.to_string()));
        }

        query
    }

    fn parse_history_payload(
        network: WalletNetwork,
        action: &str,
        payload: Value,
    ) -> Result<Vec<EtherscanTxRecord>, WalletError> {
        let status = payload
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or_default();
        let message = payload
            .get("message")
            .and_then(|s| s.as_str())
            .unwrap_or_default();
        let result = payload.get("result").cloned().unwrap_or(Value::Null);

        if status == "0"
            && (message.eq_ignore_ascii_case("No transactions found")
                || message.eq_ignore_ascii_case("No records found"))
        {
            return Ok(vec![]);
        }

        if status != "1" || !message.to_ascii_uppercase().starts_with("OK") {
            log_history_api_error(network, action, status, message, &result);
            if is_rate_limited_result(&result) {
                return Err(WalletError::NetworkError);
            }
            return Err(WalletError::OperationFailed);
        }

        let records = result.as_array().ok_or(WalletError::OperationFailed)?;
        let mut out = Vec::with_capacity(records.len());
        for entry in records {
            let tx: EtherscanTxRecord =
                serde_json::from_value(entry.clone()).map_err(|_| WalletError::OperationFailed)?;
            out.push(tx);
        }

        Ok(out)
    }
}

pub(crate) fn chain_id_for_network(network: WalletNetwork) -> u64 {
    match network {
        WalletNetwork::Mainnet => ETHEREUM_MAINNET_CHAIN_ID,
        WalletNetwork::Testnet => ETHEREUM_SEPOLIA_CHAIN_ID,
    }
}

fn is_rate_limited_result(result: &Value) -> bool {
    result
        .as_str()
        .map(|value| value.to_ascii_lowercase().contains("rate limit"))
        .unwrap_or(false)
}

fn log_history_api_error(
    network: WalletNetwork,
    action: &str,
    status: &str,
    message: &str,
    result: &Value,
) {
    let result_summary = match result {
        Value::String(value) => value.to_string(),
        Value::Array(value) => format!("array({})", value.len()),
        Value::Object(_) => "object".to_string(),
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
    };

    println!(
        "[ETH][HISTORY] Etherscan API response not usable network={:?} action={} status={} message={} result={}",
        network, action, status, message, result_summary
    );
}

#[cfg(test)]
mod tests {
    use super::EtherscanHistoryClient;
    use crate::types::wallet::WalletNetwork;
    use crate::types::WalletError;
    use serde_json::json;

    fn query_value(query: &[(String, String)], key: &str) -> Option<String> {
        query
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, value)| value.clone())
    }

    #[test]
    fn build_history_query_sets_mainnet_chain_id_for_eth_txlist() {
        let client = EtherscanHistoryClient::new(
            "api-key".to_string(),
            "https://api.etherscan.io/v2/api".to_string(),
            "https://api.etherscan.io/v2/api".to_string(),
        );
        let query = client.build_history_query(WalletNetwork::Mainnet, "txlist", "0xabc", None);

        assert_eq!(query_value(&query, "chainid").as_deref(), Some("1"));
        assert_eq!(query_value(&query, "module").as_deref(), Some("account"));
        assert_eq!(query_value(&query, "action").as_deref(), Some("txlist"));
        assert_eq!(query_value(&query, "address").as_deref(), Some("0xabc"));
        assert_eq!(query_value(&query, "startblock").as_deref(), Some("0"));
        assert_eq!(query_value(&query, "endblock").as_deref(), Some("99999999"));
        assert_eq!(query_value(&query, "sort").as_deref(), Some("asc"));
        assert_eq!(query_value(&query, "apikey").as_deref(), Some("api-key"));
    }

    #[test]
    fn build_history_query_sets_testnet_chain_id_and_contract_for_erc20() {
        let client = EtherscanHistoryClient::new(
            "api-key".to_string(),
            "https://api.etherscan.io/v2/api".to_string(),
            "https://api.etherscan.io/v2/api".to_string(),
        );
        let query = client.build_history_query(
            WalletNetwork::Testnet,
            "tokentx",
            "0xowner",
            Some("0xtoken"),
        );

        assert_eq!(query_value(&query, "chainid").as_deref(), Some("11155111"));
        assert_eq!(query_value(&query, "action").as_deref(), Some("tokentx"));
        assert_eq!(
            query_value(&query, "contractAddress").as_deref(),
            Some("0xtoken")
        );
    }

    #[test]
    fn build_history_page_query_sets_desc_sort_and_paging() {
        let client = EtherscanHistoryClient::new(
            "api-key".to_string(),
            "https://api.etherscan.io/v2/api".to_string(),
            "https://api.etherscan.io/v2/api".to_string(),
        );
        let query = client.build_history_page_query(
            WalletNetwork::Mainnet,
            "txlist",
            "0xowner",
            None,
            3,
            50,
        );

        assert_eq!(query_value(&query, "chainid").as_deref(), Some("1"));
        assert_eq!(query_value(&query, "page").as_deref(), Some("3"));
        assert_eq!(query_value(&query, "offset").as_deref(), Some("50"));
        assert_eq!(query_value(&query, "sort").as_deref(), Some("desc"));
    }

    #[test]
    fn parse_history_payload_returns_empty_for_no_transactions_found() {
        let payload = json!({
            "status": "0",
            "message": "No transactions found",
            "result": []
        });

        let parsed = EtherscanHistoryClient::parse_history_payload(
            WalletNetwork::Mainnet,
            "txlist",
            payload,
        )
        .expect("parse");
        assert!(parsed.is_empty());
    }

    #[test]
    fn parse_history_payload_returns_empty_for_no_records_found() {
        let payload = json!({
            "status": "0",
            "message": "No records found",
            "result": []
        });

        let parsed = EtherscanHistoryClient::parse_history_payload(
            WalletNetwork::Mainnet,
            "txlist",
            payload,
        )
        .expect("parse");
        assert!(parsed.is_empty());
    }

    #[test]
    fn parse_history_payload_maps_rate_limit_to_network_error() {
        let payload = json!({
            "status": "0",
            "message": "NOTOK",
            "result": "Max rate limit reached, please use API Key for higher rate limit"
        });

        let err = EtherscanHistoryClient::parse_history_payload(
            WalletNetwork::Mainnet,
            "txlist",
            payload,
        )
        .expect_err("rate-limit error");
        assert!(matches!(err, WalletError::NetworkError));
    }

    #[test]
    fn parse_history_payload_maps_notok_to_operation_failed() {
        let payload = json!({
            "status": "0",
            "message": "NOTOK",
            "result": "Free API access is temporarily unavailable"
        });

        let err = EtherscanHistoryClient::parse_history_payload(
            WalletNetwork::Mainnet,
            "txlist",
            payload,
        )
        .expect_err("notok error");
        assert!(matches!(err, WalletError::OperationFailed));
    }

    #[test]
    fn parse_history_payload_rejects_non_array_result_on_success() {
        let payload = json!({
            "status": "1",
            "message": "OK",
            "result": "unexpected shape"
        });

        let err = EtherscanHistoryClient::parse_history_payload(
            WalletNetwork::Mainnet,
            "txlist",
            payload,
        )
        .expect_err("shape error");
        assert!(matches!(err, WalletError::OperationFailed));
    }

    #[test]
    fn parse_history_payload_deserializes_camel_case_record_fields() {
        let payload = json!({
            "status": "1",
            "message": "OK",
            "result": [
                {
                    "blockNumber": "123",
                    "timeStamp": "1700000000",
                    "hash": "0xhash",
                    "nonce": "4",
                    "blockHash": "0xblock",
                    "transactionIndex": "9",
                    "from": "0xfrom",
                    "to": "0xto",
                    "value": "1000000",
                    "gas": "21000",
                    "gasPrice": "1000000000",
                    "gasUsed": "21000",
                    "cumulativeGasUsed": "22000",
                    "contractAddress": "0xtoken",
                    "confirmations": "42"
                }
            ]
        });

        let parsed = EtherscanHistoryClient::parse_history_payload(
            WalletNetwork::Mainnet,
            "tokentx",
            payload,
        )
        .expect("parse");
        assert_eq!(parsed.len(), 1);
        let first = &parsed[0];
        assert_eq!(first.block_number, "123");
        assert_eq!(first.time_stamp, "1700000000");
        assert_eq!(first.block_hash, "0xblock");
        assert_eq!(first.transaction_index, "9");
        assert_eq!(first.gas_price, "1000000000");
        assert_eq!(first.gas_used, "21000");
        assert_eq!(first.cumulative_gas_used, "22000");
        assert_eq!(first.contract_address, "0xtoken");
        assert_eq!(first.confirmations, "42");
    }
}
