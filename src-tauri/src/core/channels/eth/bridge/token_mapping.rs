use std::collections::HashMap;
use std::sync::Arc;

use ethers::types::{Address, U256};
use serde_json::Value;

use crate::core::channels::eth::bridge::delegator::{
    validate_delegator_contract, VerusBridgeDelegatorContract,
};
use crate::core::channels::eth::provider::EthNetworkProvider;
use crate::core::channels::vrpc::VrpcProvider;
use crate::types::WalletError;

pub const ETH_ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
const DEST_ETH_TYPE: u64 = 9;
const I_ADDRESS_VERSION: u8 = 102;

#[derive(Debug, Clone)]
pub struct CurrencyDefinitionRef {
    pub currency_id: String,
    pub fully_qualified_name: Option<String>,
    pub name: Option<String>,
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EthTokenMappings {
    pub contract_to_currencies: HashMap<String, Vec<CurrencyDefinitionRef>>,
    pub currency_to_contracts: HashMap<String, Vec<String>>,
    pub currencies_by_id: HashMap<String, CurrencyDefinitionRef>,
}

impl EthTokenMappings {
    pub fn add_mapping(&mut self, contract_address: String, currency: CurrencyDefinitionRef) {
        let contract_key = normalize_eth_address(&contract_address).unwrap_or(contract_address);
        let currency_key = currency.currency_id.to_ascii_lowercase();
        self.currencies_by_id
            .entry(currency_key.clone())
            .or_insert_with(|| currency.clone());

        let mapped_currencies = self
            .contract_to_currencies
            .entry(contract_key.clone())
            .or_default();
        if !mapped_currencies.iter().any(|existing| {
            existing
                .currency_id
                .eq_ignore_ascii_case(&currency.currency_id)
        }) {
            mapped_currencies.push(currency.clone());
        }

        let mapped_contracts = self.currency_to_contracts.entry(currency_key).or_default();
        if !mapped_contracts
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&contract_key))
        {
            mapped_contracts.push(contract_key);
        }
    }
}

pub async fn get_currencies_mapped_to_eth(
    vrpc_provider: &VrpcProvider,
    eth_provider: Option<&EthNetworkProvider>,
) -> Result<EthTokenMappings, WalletError> {
    let mut list_payloads = Vec::<Value>::new();
    let mut used_partitioned_list = true;

    for systemtype in ["imported", "local", "pbaas"] {
        match vrpc_provider
            .listcurrencies_with_systemtype(systemtype)
            .await
        {
            Ok(payload) => list_payloads.push(payload),
            Err(_) => {
                used_partitioned_list = false;
                break;
            }
        }
    }

    if !used_partitioned_list || list_payloads.is_empty() {
        list_payloads.clear();
        list_payloads.push(vrpc_provider.listcurrencies().await?);
    }

    let mut all_currencies = HashMap::<String, CurrencyDefinitionRef>::new();
    let mut mapped = EthTokenMappings::default();

    for payload in &list_payloads {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for entry in entries {
            let definition = entry.get("currencydefinition").unwrap_or(entry);
            let Some(currency) = currency_definition_from_value(definition) else {
                continue;
            };

            all_currencies
                .entry(currency.currency_id.to_ascii_lowercase())
                .or_insert_with(|| currency.clone());
            mapped
                .currencies_by_id
                .entry(currency.currency_id.to_ascii_lowercase())
                .or_insert(currency.clone());
        }
    }

    // Imported-currency native ETH mappings from VRPC.
    for payload in &list_payloads {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for entry in entries {
            let definition = entry.get("currencydefinition").unwrap_or(entry);
            let Some(currency) = currency_definition_from_value(definition) else {
                continue;
            };

            let Some(native_currency_id) = definition.get("nativecurrencyid") else {
                continue;
            };
            let Some(native_type) = native_currency_id.get("type").and_then(extract_u64) else {
                continue;
            };
            if native_type != DEST_ETH_TYPE {
                continue;
            }
            let Some(contract_address) = native_currency_id
                .get("address")
                .and_then(extract_stringish)
                .and_then(|value| normalize_eth_address(&value))
            else {
                continue;
            };
            mapped.add_mapping(contract_address, currency);
        }
    }

    // Delegator token list mappings.
    if let Some(provider) = eth_provider {
        let (contract_address, _) = validate_delegator_contract(provider).await?;
        let delegator = VerusBridgeDelegatorContract::new(
            contract_address,
            Arc::new(provider.rpc_provider.clone()),
        );

        let tokens = delegator
            .get_token_list(U256::zero(), U256::zero())
            .call()
            .await
            .map_err(|_| WalletError::BridgeDeploymentUnavailable)?;
        for token in tokens {
            let contract = format!("{:#x}", token.erc_20_contract_address);
            let i_address = eth_address_to_iaddress(token.iaddress);
            let Some(currency) = all_currencies.get(&i_address.to_ascii_lowercase()).cloned()
            else {
                continue;
            };
            mapped.add_mapping(contract, currency);
        }
    }

    Ok(mapped)
}

pub fn normalize_eth_address(value: &str) -> Option<String> {
    value
        .trim()
        .parse::<Address>()
        .ok()
        .map(|address| format!("{:#x}", address))
}

pub fn iaddress_to_eth_address(i_address: &str) -> Result<Address, WalletError> {
    let decoded = bs58::decode(i_address.trim())
        .with_check(None)
        .into_vec()
        .map_err(|_| WalletError::InvalidAddress)?;
    if decoded.len() != 21 || decoded[0] != I_ADDRESS_VERSION {
        return Err(WalletError::InvalidAddress);
    }

    let hash = &decoded[1..];
    let mut raw = [0u8; 20];
    raw.copy_from_slice(hash);
    Ok(Address::from(raw))
}

pub fn eth_address_to_iaddress(address: Address) -> String {
    let mut payload = Vec::with_capacity(21);
    payload.push(I_ADDRESS_VERSION);
    payload.extend_from_slice(address.as_bytes());
    bs58::encode(payload).with_check().into_string()
}

pub fn eth_address_to_iaddress_from_str(address: &str) -> Result<String, WalletError> {
    let parsed = address
        .trim()
        .parse::<Address>()
        .map_err(|_| WalletError::InvalidAddress)?;
    Ok(eth_address_to_iaddress(parsed))
}

fn currency_definition_from_value(value: &Value) -> Option<CurrencyDefinitionRef> {
    let currency_id = value.get("currencyid").and_then(extract_stringish)?;
    Some(CurrencyDefinitionRef {
        currency_id,
        fully_qualified_name: value.get("fullyqualifiedname").and_then(extract_stringish),
        name: value.get("name").and_then(extract_stringish),
        symbol: value.get("symbol").and_then(extract_stringish),
    })
}

fn extract_stringish(value: &Value) -> Option<String> {
    if let Some(raw) = value.as_str() {
        return Some(raw.to_string());
    }
    if let Some(raw) = value.as_i64() {
        return Some(raw.to_string());
    }
    if let Some(raw) = value.as_u64() {
        return Some(raw.to_string());
    }
    value.as_f64().map(|raw| raw.to_string())
}

fn extract_u64(value: &Value) -> Option<u64> {
    if let Some(raw) = value.as_u64() {
        return Some(raw);
    }
    if let Some(raw) = value.as_i64() {
        return u64::try_from(raw).ok();
    }
    value.as_str().and_then(|raw| raw.parse::<u64>().ok())
}

#[cfg(test)]
mod tests {
    use super::{eth_address_to_iaddress, iaddress_to_eth_address, normalize_eth_address};
    use ethers::types::Address;

    #[test]
    fn normalize_eth_address_returns_lowercase_checksumless_hex() {
        let normalized =
            normalize_eth_address("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").expect("address");
        assert_eq!(normalized, "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
    }

    #[test]
    fn i_address_roundtrip_to_eth_address() {
        let address: Address = "0x0000000000000000000000000000000000000001"
            .parse()
            .expect("address");
        let i_address = eth_address_to_iaddress(address);
        let parsed = iaddress_to_eth_address(&i_address).expect("parse i-address");
        assert_eq!(parsed, address);
    }
}
