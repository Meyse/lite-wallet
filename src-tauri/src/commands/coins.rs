//
// Module 3: Tauri commands for coin registry and asset discovery.
// Includes registry reads/writes plus PBaaS/ERC20 resolver commands for add-asset UX.

use std::collections::HashSet;
use std::sync::Arc;

use ethers::abi::Abi;
use ethers::contract::Contract;
use ethers::providers::Middleware;
use ethers::types::Address;
use ethers::utils::to_checksum;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;
use tokio::sync::Mutex;

use crate::core::channels::eth::bridge::token_mapping::get_currencies_mapped_to_eth;
use crate::core::channels::eth::EthProviderPool;
use crate::core::channels::vrpc::VrpcProviderPool;
use crate::core::{Channel, CoinDefinition, CoinRegistry, Protocol, SessionManager};
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

const ERC20_METADATA_ABI: &str = r#"[
  {
    \"constant\": true,
    \"inputs\": [],
    \"name\": \"symbol\",
    \"outputs\": [{\"name\": \"\", \"type\": \"string\"}],
    \"type\": \"function\"
  },
  {
    \"constant\": true,
    \"inputs\": [],
    \"name\": \"name\",
    \"outputs\": [{\"name\": \"\", \"type\": \"string\"}],
    \"type\": \"function\"
  },
  {
    \"constant\": true,
    \"inputs\": [],
    \"name\": \"decimals\",
    \"outputs\": [{\"name\": \"\", \"type\": \"uint8\"}],
    \"type\": \"function\"
  }
]"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PbaasCandidate {
    pub currency_id: String,
    pub system_id: String,
    pub display_ticker: String,
    pub display_name: String,
    pub fully_qualified_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PbaasResolveResult {
    Resolved { coin: CoinDefinition },
    Ambiguous { candidates: Vec<PbaasCandidate> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Erc20ResolveResult {
    Resolved { coin: CoinDefinition },
}

fn normalize_query(input: &str) -> String {
    input.trim().to_ascii_lowercase()
}

fn value_as_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn as_currency_definition(value: &Value) -> &Value {
    value.get("currencydefinition").unwrap_or(value)
}

fn as_result_payload(value: &Value) -> &Value {
    value.get("result").unwrap_or(value)
}

fn pbaas_candidate_from_value(value: &Value) -> Option<PbaasCandidate> {
    let definition = as_currency_definition(value);
    let currency_id = value_as_string(
        definition
            .get("currencyid")
            .or(definition.get("currency_id")),
    )?;

    let system_id = value_as_string(definition.get("systemid").or(definition.get("system_id")))
        .or_else(|| value_as_string(definition.get("parent")))
        .unwrap_or_else(|| currency_id.clone());

    let fully_qualified_name = value_as_string(
        definition
            .get("fullyqualifiedname")
            .or(definition.get("fully_qualified_name")),
    );

    let display_ticker = value_as_string(definition.get("name"))
        .or_else(|| fully_qualified_name.clone())
        .unwrap_or_else(|| currency_id.clone());

    let display_name = fully_qualified_name
        .clone()
        .or_else(|| value_as_string(definition.get("name")))
        .unwrap_or_else(|| display_ticker.clone());

    Some(PbaasCandidate {
        currency_id,
        system_id,
        display_ticker,
        display_name,
        fully_qualified_name,
    })
}

fn collect_pbaas_candidates(payload: &Value) -> Vec<PbaasCandidate> {
    let root = as_result_payload(payload);
    let mut candidates = Vec::<PbaasCandidate>::new();

    if let Some(items) = root.as_array() {
        for item in items {
            if let Some(candidate) = pbaas_candidate_from_value(item) {
                candidates.push(candidate);
            }
        }
    } else if let Some(candidate) = pbaas_candidate_from_value(root) {
        candidates.push(candidate);
    }

    let mut seen = HashSet::<String>::new();
    candidates
        .into_iter()
        .filter(|candidate| seen.insert(candidate.currency_id.to_ascii_lowercase()))
        .collect()
}

fn candidate_matches_query(candidate: &PbaasCandidate, query_normalized: &str) -> bool {
    if query_normalized.is_empty() {
        return false;
    }

    [
        candidate.currency_id.as_str(),
        candidate.system_id.as_str(),
        candidate.display_ticker.as_str(),
        candidate.display_name.as_str(),
        candidate
            .fully_qualified_name
            .as_deref()
            .unwrap_or_default(),
    ]
    .iter()
    .any(|value| value.to_ascii_lowercase() == query_normalized)
}

fn build_pbaas_coin_definition(
    candidate: &PbaasCandidate,
    network: WalletNetwork,
    vrpc_endpoint: String,
) -> CoinDefinition {
    CoinDefinition {
        id: candidate.currency_id.clone(),
        currency_id: candidate.currency_id.clone(),
        system_id: candidate.system_id.clone(),
        display_ticker: candidate.display_ticker.clone(),
        display_name: candidate.display_name.clone(),
        coin_paprika_id: None,
        proto: Protocol::Vrsc,
        compatible_channels: vec![Channel::Vrpc],
        decimals: 8,
        vrpc_endpoints: vec![vrpc_endpoint],
        dlight_endpoints: None,
        electrum_endpoints: None,
        seconds_per_block: 60,
        mapped_to: None,
        is_testnet: matches!(network, WalletNetwork::Testnet),
    }
}

pub(crate) fn pbaas_coin_definition_from_payload(
    payload: &Value,
    network: WalletNetwork,
    vrpc_endpoint: String,
) -> Option<CoinDefinition> {
    let candidate = pbaas_candidate_from_value(as_result_payload(payload))?;
    Some(build_pbaas_coin_definition(
        &candidate,
        network,
        vrpc_endpoint,
    ))
}

fn parse_contract_address(contract: &str) -> Result<Address, WalletError> {
    contract
        .trim()
        .parse::<Address>()
        .map_err(|_| WalletError::InvalidContract)
}

fn build_erc20_coin_definition(
    contract_address: Address,
    symbol: &str,
    name: &str,
    decimals: u8,
    mapped_to: Option<String>,
    network: WalletNetwork,
) -> Result<CoinDefinition, WalletError> {
    let symbol_trimmed = symbol.trim();
    if symbol_trimmed.is_empty() {
        return Err(WalletError::InvalidContract);
    }

    let checksum = to_checksum(&contract_address, None);
    let lowercase_contract = format!("{:#x}", contract_address).to_ascii_lowercase();
    let display_ticker = symbol_trimmed.to_ascii_uppercase();
    let display_name = if name.trim().is_empty() {
        display_ticker.clone()
    } else {
        name.trim().to_string()
    };

    Ok(CoinDefinition {
        id: format!("erc20_{}", lowercase_contract),
        currency_id: checksum.clone(),
        system_id: checksum,
        display_ticker,
        display_name,
        coin_paprika_id: None,
        proto: Protocol::Erc20,
        compatible_channels: vec![Channel::Erc20],
        decimals,
        vrpc_endpoints: vec![],
        dlight_endpoints: None,
        electrum_endpoints: None,
        seconds_per_block: 12,
        mapped_to: mapped_to
            .and_then(|value| {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .or_else(|| {
                Some(
                    match network {
                        WalletNetwork::Mainnet => "ETH",
                        WalletNetwork::Testnet => "GETH",
                    }
                    .to_string(),
                )
            }),
        is_testnet: matches!(network, WalletNetwork::Testnet),
    })
}

async fn require_active_network(
    session_manager: &Arc<Mutex<SessionManager>>,
) -> Result<WalletNetwork, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    Ok(session.active_network().unwrap_or(WalletNetwork::Mainnet))
}

/// Returns all coins: static definitions plus dynamically added entries.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_coin_registry(
    registry: State<'_, Arc<CoinRegistry>>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<Vec<CoinDefinition>, WalletError> {
    let session = session_manager.lock().await;
    let active_account_id = if session.is_unlocked() {
        session.active_account_id().cloned()
    } else {
        None
    };
    drop(session);
    registry.set_active_account(active_account_id);

    println!("[COINS] Get coin registry requested");
    let coins = registry.get_all_for_active_account();
    println!("[COINS] Returning {} coins", coins.len());
    Ok(coins)
}

/// Adds a coin definition to the dynamic registry.
#[tauri::command(rename_all = "snake_case")]
pub async fn add_coin_definition(
    registry: State<'_, Arc<CoinRegistry>>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    definition: CoinDefinition,
    expected_session_id: String,
) -> Result<CoinDefinition, WalletError> {
    // Keep the wallet session stable through the synchronous registry write.
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    let account_id = session
        .active_account_id()
        .cloned()
        .ok_or(WalletError::WalletLocked)?;
    if session.active_session_id() != Some(expected_session_id.as_str()) {
        return Err(WalletError::WalletSessionChanged);
    }

    println!("[COINS] Add coin requested: {}", definition.id);
    let added = registry.add_coin_for_account(&account_id, definition)?;
    println!("[COINS] Coin added: {}", added.id);
    Ok(added)
}

/// Adds a PBaaS currency to the registry (compatibility wrapper).
#[tauri::command(rename_all = "snake_case")]
pub async fn add_pbaas_currency(
    registry: State<'_, Arc<CoinRegistry>>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    definition: CoinDefinition,
) -> Result<(), WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    let account_id = session
        .active_account_id()
        .cloned()
        .ok_or(WalletError::WalletLocked)?;
    drop(session);
    registry.set_active_account(Some(account_id));

    println!("[COINS] Add PBaaS currency requested: {}", definition.id);
    registry.add_pbaas(definition)?;
    println!("[COINS] PBaaS currency added");
    Ok(())
}

/// Resolves PBaaS by name or i-address against the active wallet network.
#[tauri::command(rename_all = "snake_case")]
pub async fn resolve_pbaas_currency(
    query: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<PbaasResolveResult, WalletError> {
    let normalized_query = normalize_query(&query);
    if normalized_query.is_empty() {
        return Err(WalletError::PbaasNotFound);
    }

    let network = require_active_network(session_manager.inner()).await?;
    let query_trimmed = query.trim();
    let provider_candidates = vrpc_provider_pool.provider_candidates(network, Some(query_trimmed));

    for provider in &provider_candidates {
        if let Ok(payload) = provider.getcurrency(query_trimmed).await {
            if let Some(candidate) = pbaas_candidate_from_value(as_result_payload(&payload)) {
                let endpoint =
                    vrpc_provider_pool.endpoint_url_for_system(network, &candidate.system_id);
                return Ok(PbaasResolveResult::Resolved {
                    coin: build_pbaas_coin_definition(&candidate, network, endpoint),
                });
            }
        }
    }

    let mut listcurrencies_success = false;
    let mut last_listcurrencies_error: Option<WalletError> = None;
    let mut matches = Vec::<PbaasCandidate>::new();
    for provider in &provider_candidates {
        match provider.listcurrencies().await {
            Ok(payload) => {
                listcurrencies_success = true;
                matches.extend(
                    collect_pbaas_candidates(&payload)
                        .into_iter()
                        .filter(|candidate| candidate_matches_query(candidate, &normalized_query)),
                );
            }
            Err(err) => {
                last_listcurrencies_error = Some(err);
            }
        }
    }
    let mut seen_currency_ids = HashSet::<String>::new();
    matches
        .retain(|candidate| seen_currency_ids.insert(candidate.currency_id.to_ascii_lowercase()));

    if matches.is_empty() {
        if !listcurrencies_success {
            return Err(last_listcurrencies_error.unwrap_or(WalletError::NetworkError));
        }
        return Err(WalletError::PbaasNotFound);
    }

    matches.sort_by(|a, b| {
        a.display_ticker
            .to_ascii_lowercase()
            .cmp(&b.display_ticker.to_ascii_lowercase())
            .then(
                a.currency_id
                    .to_ascii_lowercase()
                    .cmp(&b.currency_id.to_ascii_lowercase()),
            )
    });

    if matches.len() > 1 {
        return Ok(PbaasResolveResult::Ambiguous {
            candidates: matches,
        });
    }

    let candidate = matches
        .into_iter()
        .next()
        .ok_or(WalletError::PbaasNotFound)?;
    let endpoint = vrpc_provider_pool.endpoint_url_for_system(network, &candidate.system_id);
    Ok(PbaasResolveResult::Resolved {
        coin: build_pbaas_coin_definition(&candidate, network, endpoint),
    })
}

/// Resolves ERC20 metadata by contract on the active Ethereum network.
#[tauri::command(rename_all = "snake_case")]
pub async fn resolve_erc20_contract(
    contract: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    eth_provider_pool: State<'_, Arc<EthProviderPool>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<Erc20ResolveResult, WalletError> {
    let network = require_active_network(session_manager.inner()).await?;
    let provider = eth_provider_pool.for_network(network)?;
    provider.validate_rpc_identity().await?;
    let contract_address = parse_contract_address(&contract)?;

    let bytecode = provider
        .rpc_provider
        .get_code(contract_address, None)
        .await
        .map_err(|_| WalletError::NetworkError)?;
    if bytecode.as_ref().is_empty() {
        return Err(WalletError::InvalidContract);
    }

    let abi: Abi =
        serde_json::from_str(ERC20_METADATA_ABI).map_err(|_| WalletError::OperationFailed)?;
    let contract_client = Arc::new(provider.rpc_provider.clone());
    let metadata_contract = Contract::new(contract_address, abi, contract_client);

    let symbol = metadata_contract
        .method::<_, String>("symbol", ())
        .map_err(|_| WalletError::InvalidContract)?
        .call()
        .await
        .map_err(|_| WalletError::InvalidContract)?;

    let name = metadata_contract
        .method::<_, String>("name", ())
        .map_err(|_| WalletError::InvalidContract)?
        .call()
        .await
        .map_err(|_| WalletError::InvalidContract)?;

    let decimals = metadata_contract
        .method::<_, u8>("decimals", ())
        .map_err(|_| WalletError::InvalidContract)?
        .call()
        .await
        .map_err(|_| WalletError::InvalidContract)?;

    let mapped_to =
        match get_currencies_mapped_to_eth(vrpc_provider_pool.for_network(network), Some(provider))
            .await
        {
            Ok(mappings) => {
                let contract_key = format!("{:#x}", contract_address).to_ascii_lowercase();
                mappings
                    .contract_to_currencies
                    .get(&contract_key)
                    .and_then(|currencies| currencies.first())
                    .map(|currency| currency.currency_id.clone())
            }
            Err(_) => None,
        };

    let coin = build_erc20_coin_definition(
        contract_address,
        &symbol,
        &name,
        decimals,
        mapped_to,
        network,
    )?;

    Ok(Erc20ResolveResult::Resolved { coin })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_contract_address_rejects_invalid_input() {
        let result = parse_contract_address("not-an-address");
        assert!(matches!(result, Err(WalletError::InvalidContract)));
    }

    #[test]
    fn build_erc20_coin_definition_uses_deterministic_id() {
        let contract = parse_contract_address("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")
            .expect("valid contract");
        let coin = build_erc20_coin_definition(
            contract,
            "usdc",
            "USD Coin",
            6,
            None,
            WalletNetwork::Mainnet,
        )
        .expect("coin definition");

        assert_eq!(coin.id, "erc20_0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
        assert_eq!(coin.display_ticker, "USDC");
        assert_eq!(coin.decimals, 6);
        assert_eq!(coin.proto, Protocol::Erc20);
        assert!(!coin.is_testnet);
        assert_eq!(coin.mapped_to.as_deref(), Some("ETH"));
    }

    #[test]
    fn build_erc20_coin_definition_scopes_sepolia_assets_to_testnet() {
        let contract = parse_contract_address("0x1111111111111111111111111111111111111111")
            .expect("valid contract");
        let coin = build_erc20_coin_definition(
            contract,
            "test",
            "Test token",
            18,
            None,
            WalletNetwork::Testnet,
        )
        .expect("coin definition");

        assert!(coin.is_testnet);
        assert_eq!(coin.mapped_to.as_deref(), Some("GETH"));
    }

    #[test]
    fn collect_pbaas_candidates_reads_currency_definition_entries() {
        let payload = serde_json::json!({
            "result": [
                {
                    "currencydefinition": {
                        "currencyid": "iExampleOne",
                        "systemid": "iSystem",
                        "name": "ONE",
                        "fullyqualifiedname": "One currency"
                    }
                },
                {
                    "currencydefinition": {
                        "currencyid": "iExampleTwo",
                        "systemid": "iSystem",
                        "name": "TWO"
                    }
                }
            ]
        });

        let candidates = collect_pbaas_candidates(&payload);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].currency_id, "iExampleOne");
        assert_eq!(candidates[1].currency_id, "iExampleTwo");
    }

    #[test]
    fn candidate_matches_query_checks_expected_fields() {
        let candidate = PbaasCandidate {
            currency_id: "iCurrency".to_string(),
            system_id: "iSystem".to_string(),
            display_ticker: "TICK".to_string(),
            display_name: "Ticker Name".to_string(),
            fully_qualified_name: Some("Ticker.Name".to_string()),
        };

        assert!(candidate_matches_query(
            &candidate,
            &normalize_query("iCurrency")
        ));
        assert!(candidate_matches_query(
            &candidate,
            &normalize_query("iSystem")
        ));
        assert!(candidate_matches_query(
            &candidate,
            &normalize_query("TICK")
        ));
        assert!(candidate_matches_query(
            &candidate,
            &normalize_query("Ticker.Name")
        ));
        assert!(!candidate_matches_query(
            &candidate,
            &normalize_query("missing")
        ));
    }
}
