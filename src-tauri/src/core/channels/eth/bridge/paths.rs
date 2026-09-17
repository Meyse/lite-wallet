//
// Bridge conversion-path discovery.
// Phase-1 scope: VRPC conversion path normalization for desktop bridge wizard.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::token_mapping::{
    get_currencies_mapped_to_eth, normalize_eth_address, CurrencyDefinitionRef, EthTokenMappings,
    ETH_ZERO_ADDRESS,
};
use crate::core::channels::eth::provider::EthNetworkProvider;
use crate::core::channels::vrpc::VrpcProvider;
use crate::types::bridge::{
    BridgeConversionEstimateRequest, BridgeConversionEstimateResult, BridgeConversionPathQuote,
    BridgeConversionPathRequest, BridgeConversionPathsResult,
};
use crate::types::WalletError;

const IS_FRACTIONAL_FLAG: u64 = 0x01;
const IS_GATEWAY_FLAG: u64 = 0x80;
const IS_GATEWAY_CONVERTER_FLAG: u64 = 0x200;

#[derive(Debug, Clone)]
struct CurrencyNode {
    currency_id: String,
    fully_qualified_name: Option<String>,
    name: Option<String>,
    symbol: Option<String>,
    options: u64,
    system_id: Option<String>,
    parent: Option<String>,
    launch_system_id: Option<String>,
    start_block: u64,
    max_preconversion_sum: Option<f64>,
    currencies: Vec<String>,
    best_prices: HashMap<String, f64>,
    reserve_states: HashMap<String, ReserveState>,
}

#[derive(Debug, Clone)]
struct ReserveState {
    weight: f64,
    reserves: f64,
}

impl CurrencyNode {
    fn display_name(&self) -> Option<String> {
        self.fully_qualified_name
            .clone()
            .or_else(|| self.name.clone())
    }

    fn display_ticker(&self) -> Option<String> {
        self.symbol
            .clone()
            .or_else(|| self.name.clone())
            .or_else(|| self.display_name())
    }

    fn matches_filter(&self, filter: &str) -> bool {
        equals_ignore_case(&self.currency_id, filter)
            || self
                .fully_qualified_name
                .as_deref()
                .map(|value| equals_ignore_case(value, filter))
                .unwrap_or(false)
            || self
                .name
                .as_deref()
                .map(|value| equals_ignore_case(value, filter))
                .unwrap_or(false)
            || self
                .symbol
                .as_deref()
                .map(|value| equals_ignore_case(value, filter))
                .unwrap_or(false)
    }

    fn reserve_state(&self, currency_id: &str) -> Option<&ReserveState> {
        self.reserve_states
            .iter()
            .find(|(key, _)| equals_ignore_case(key, currency_id))
            .map(|(_, state)| state)
    }
}

#[derive(Debug, Clone)]
struct RouteCandidate {
    destination_id: String,
    via_id: Option<String>,
    export_to_id: Option<String>,
    price: Option<f64>,
    via_price_in_root: Option<f64>,
    dest_price_in_via: Option<f64>,
    gateway: bool,
}

#[derive(Debug, Default)]
struct RouteBuildDebugStats {
    raw_candidate_count: usize,
    post_converter_gating_count: usize,
    post_via_recursion_count: usize,
    post_visibility_filter_count: usize,
}

pub async fn get_conversion_paths(
    request: &BridgeConversionPathRequest,
    vrpc_provider: &VrpcProvider,
    eth_provider: Option<&EthNetworkProvider>,
) -> Result<BridgeConversionPathsResult, WalletError> {
    if let Some(provider) = eth_provider {
        provider.validate_rpc_identity().await?;
    }
    let source_currency = request.source_currency.trim();
    if source_currency.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    if normalize_eth_address(source_currency).is_some() {
        return get_conversion_paths_from_evm_source(request, vrpc_provider, eth_provider).await;
    }

    let source_definition = vrpc_provider.getcurrency(source_currency).await?;
    let destination_definition = match request
        .destination_currency
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(destination_currency) => Some(vrpc_provider.getcurrency(destination_currency).await?),
        None => None,
    };

    let destination_currency_filter = destination_definition
        .as_ref()
        .and_then(extract_currency_id);
    let chain_currency_id = request
        .channel_id
        .rsplit_once('.')
        .map(|(_, system_id)| system_id.to_string());

    let mut normalized_paths = match vrpc_provider
        .getcurrencyconversionpaths(&source_definition, destination_definition.as_ref())
        .await
    {
        Ok(raw_paths) => parse_conversion_paths(raw_paths)?,
        Err(WalletError::BridgeNotImplemented) => {
            println!(
                    "[BRIDGE] VRPC conversion-path RPC unavailable; deriving conversion graph from listcurrencies"
                );
            derive_paths_from_listcurrencies(
                vrpc_provider,
                &source_definition,
                destination_currency_filter.as_deref(),
                chain_currency_id.as_deref(),
            )
            .await?
        }
        Err(err) => return Err(err),
    };

    match vrpc_provider.listcurrencies().await {
        Ok(list_payload) => {
            let currency_display_lookup = collect_currency_display_lookup(&list_payload);
            if !currency_display_lookup.is_empty() {
                enrich_quote_display_names(&mut normalized_paths, &currency_display_lookup);
            }
        }
        Err(err) => {
            println!(
                "[BRIDGE] listcurrencies lookup unavailable while enriching conversion display labels: {:?}",
                err
            );
        }
    }

    match vrpc_provider
        .listcurrencies_with_launchstate("prelaunch")
        .await
    {
        Ok(prelaunch_payload) => {
            let prelaunch_currency_refs = collect_prelaunch_currency_refs(&prelaunch_payload, true);
            if !prelaunch_currency_refs.is_empty() {
                mark_prelaunch_quotes(&mut normalized_paths, &prelaunch_currency_refs);
            }
        }
        Err(err) => {
            println!(
                "[BRIDGE] prelaunch listcurrencies lookup unavailable while enriching conversion paths: {:?}",
                err
            );
        }
    }

    append_eth_mappings_for_vrpc_source(
        &mut normalized_paths,
        &source_definition,
        request,
        vrpc_provider,
        eth_provider,
    )
    .await?;

    Ok(BridgeConversionPathsResult {
        source_currency: request.source_currency.clone(),
        paths: normalized_paths,
    })
}

pub async fn estimate_conversion(
    request: &BridgeConversionEstimateRequest,
    vrpc_provider: &VrpcProvider,
    eth_provider: Option<&EthNetworkProvider>,
) -> Result<BridgeConversionEstimateResult, WalletError> {
    if let Some(provider) = eth_provider {
        provider.validate_rpc_identity().await?;
    }
    let source_currency = request.source_currency.trim();
    let convert_to = request.convert_to.trim();
    if source_currency.is_empty() || convert_to.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    let amount = request
        .amount
        .trim()
        .parse::<f64>()
        .map_err(|_| WalletError::OperationFailed)?;
    if !amount.is_finite() || amount <= 0.0 {
        return Err(WalletError::OperationFailed);
    }

    if normalize_eth_address(source_currency).is_some() {
        return estimate_conversion_from_paths(request, vrpc_provider, eth_provider).await;
    }

    let raw_estimate = match vrpc_provider
        .estimateconversion(
            source_currency,
            convert_to,
            amount,
            request.via.as_deref(),
            request.preconvert,
        )
        .await
    {
        Ok(value) => value,
        Err(WalletError::BridgeNotImplemented) => {
            return estimate_conversion_from_paths(request, vrpc_provider, eth_provider).await;
        }
        Err(err) => return Err(err),
    };

    let estimated_currency_out = raw_estimate
        .get("estimatedcurrencyout")
        .and_then(extract_stringish);
    let price = raw_estimate.get("price").and_then(extract_stringish);

    Ok(BridgeConversionEstimateResult {
        estimated_currency_out,
        price,
    })
}

async fn estimate_conversion_from_paths(
    request: &BridgeConversionEstimateRequest,
    vrpc_provider: &VrpcProvider,
    eth_provider: Option<&EthNetworkProvider>,
) -> Result<BridgeConversionEstimateResult, WalletError> {
    let amount = request
        .amount
        .trim()
        .parse::<f64>()
        .map_err(|_| WalletError::OperationFailed)?;
    if !amount.is_finite() || amount <= 0.0 {
        return Err(WalletError::OperationFailed);
    }

    let paths = get_conversion_paths(
        &BridgeConversionPathRequest {
            coin_id: request.coin_id.clone(),
            channel_id: request.channel_id.clone(),
            source_currency: request.source_currency.clone(),
            destination_currency: None,
        },
        vrpc_provider,
        eth_provider,
    )
    .await?;

    let requested_convert_to = request.convert_to.trim();
    let requested_via = request
        .via
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let mut selected_quote: Option<&BridgeConversionPathQuote> = None;
    for quotes in paths.paths.values() {
        for quote in quotes {
            let quote_convert_to = quote
                .convert_to
                .as_deref()
                .unwrap_or(quote.destination_id.as_str());
            if !equals_ignore_case(quote_convert_to, requested_convert_to) {
                continue;
            }

            let quote_via = quote
                .via
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let via_matches = match (requested_via, quote_via) {
                (None, None) => true,
                (Some(left), Some(right)) => equals_ignore_case(left, right),
                (None, Some(_)) | (Some(_), None) => false,
            };
            if !via_matches {
                continue;
            }

            selected_quote = Some(quote);
            break;
        }
        if selected_quote.is_some() {
            break;
        }
    }

    let Some(quote) = selected_quote else {
        return Err(WalletError::BridgeRouteInvalid);
    };
    let Some(price_string) = quote.price.as_deref() else {
        return Err(WalletError::BridgeRouteInvalid);
    };
    let price_number = price_string
        .trim()
        .parse::<f64>()
        .map_err(|_| WalletError::BridgeRouteInvalid)?;
    if !price_number.is_finite() || price_number <= 0.0 {
        return Err(WalletError::BridgeRouteInvalid);
    }

    Ok(BridgeConversionEstimateResult {
        estimated_currency_out: Some(format_decimal(amount * price_number)),
        price: Some(format_decimal(price_number)),
    })
}

async fn get_conversion_paths_from_evm_source(
    request: &BridgeConversionPathRequest,
    vrpc_provider: &VrpcProvider,
    eth_provider: Option<&EthNetworkProvider>,
) -> Result<BridgeConversionPathsResult, WalletError> {
    let source_contract = normalize_eth_address(request.source_currency.trim())
        .ok_or(WalletError::OperationFailed)?;
    let system_id = resolve_system_id_for_request(request, vrpc_provider).await?;
    let system_definition = vrpc_provider.getcurrency(&system_id).await?;
    let system_currency_id = extract_currency_id(&system_definition).unwrap_or(system_id.clone());
    let system_display_name = extract_currency_display_name(&system_definition)
        .or_else(|| extract_currency_id(&system_definition))
        .unwrap_or(system_currency_id.clone());

    let bridge_definition = match vrpc_provider.getcurrency("Bridge.vETH").await {
        Ok(value) => value,
        Err(_) => vrpc_provider.getcurrency("vETH").await?,
    };
    let bridge_currency_id =
        extract_currency_id(&bridge_definition).ok_or(WalletError::OperationFailed)?;
    let bridge_display_name = extract_currency_display_name(&bridge_definition)
        .or_else(|| extract_currency_id(&bridge_definition))
        .unwrap_or(bridge_currency_id.clone());
    let bridge_display_ticker = bridge_definition
        .get("symbol")
        .and_then(extract_stringish)
        .or_else(|| bridge_definition.get("name").and_then(extract_stringish))
        .or_else(|| Some(bridge_display_name.clone()));

    let convertable_currency_ids = bridge_definition
        .get("currencies")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(extract_currency_ref)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mappings = get_currencies_mapped_to_eth(vrpc_provider, eth_provider).await?;
    let source_mapped = mappings
        .contract_to_currencies
        .get(&source_contract)
        .cloned()
        .unwrap_or_default();

    let mut paths = HashMap::<String, Vec<BridgeConversionPathQuote>>::new();
    let is_eth_source = source_contract.eq_ignore_ascii_case(ETH_ZERO_ADDRESS);

    for source_currency in &source_mapped {
        let source_currency_id = source_currency.currency_id.clone();
        let source_is_bridge = equals_ignore_case(&source_currency_id, &bridge_currency_id);
        let is_convertable_source = convertable_currency_ids
            .iter()
            .any(|candidate| equals_ignore_case(candidate, &source_currency_id));

        if is_eth_source || is_convertable_source || source_is_bridge {
            if !source_is_bridge {
                if let Some(bridge_price) =
                    extract_bridge_best_price(&bridge_definition, &source_currency_id)
                {
                    let price = 1.0 / bridge_price;
                    add_path_quote(
                        &mut paths,
                        &bridge_currency_id,
                        BridgeConversionPathQuote {
                            destination_id: bridge_currency_id.clone(),
                            destination_display_name: Some(bridge_display_name.clone()),
                            destination_display_ticker: bridge_display_ticker.clone(),
                            convert_to: Some(bridge_currency_id.clone()),
                            convert_to_display_name: Some(bridge_display_name.clone()),
                            export_to: Some(system_currency_id.clone()),
                            export_to_display_name: Some(system_display_name.clone()),
                            via: None,
                            via_display_name: None,
                            map_to: None,
                            price: Some(format_decimal(price)),
                            via_price_in_root: None,
                            dest_price_in_via: None,
                            gateway: true,
                            mapping: false,
                            bounceback: false,
                            eth_destination: false,
                            prelaunch: false,
                        },
                    );
                }
            }

            for convertable_currency_id in &convertable_currency_ids {
                let Some(convertable_currency) =
                    resolve_currency_ref_by_id(vrpc_provider, &mappings, convertable_currency_id)
                        .await
                else {
                    continue;
                };

                let mapped_contracts = mappings
                    .currency_to_contracts
                    .get(&convertable_currency_id.to_ascii_lowercase())
                    .cloned()
                    .unwrap_or_default();

                if source_is_bridge {
                    let Some(dest_price_in_via) =
                        extract_bridge_best_price(&bridge_definition, convertable_currency_id)
                    else {
                        continue;
                    };

                    add_path_quote(
                        &mut paths,
                        convertable_currency_id,
                        BridgeConversionPathQuote {
                            destination_id: convertable_currency.currency_id.clone(),
                            destination_display_name: convertable_currency
                                .fully_qualified_name
                                .clone()
                                .or_else(|| convertable_currency.name.clone()),
                            destination_display_ticker: convertable_currency
                                .symbol
                                .clone()
                                .or_else(|| convertable_currency.name.clone()),
                            convert_to: Some(convertable_currency.currency_id.clone()),
                            convert_to_display_name: convertable_currency
                                .fully_qualified_name
                                .clone()
                                .or_else(|| convertable_currency.name.clone())
                                .or_else(|| Some(convertable_currency.currency_id.clone())),
                            export_to: Some(system_currency_id.clone()),
                            export_to_display_name: Some(system_display_name.clone()),
                            via: None,
                            via_display_name: None,
                            map_to: None,
                            price: Some(format_decimal(dest_price_in_via)),
                            via_price_in_root: None,
                            dest_price_in_via: None,
                            gateway: true,
                            mapping: false,
                            bounceback: false,
                            eth_destination: false,
                            prelaunch: false,
                        },
                    );

                    for contract_address in mapped_contracts {
                        let (destination_name, destination_ticker) =
                            contract_display(&contract_address, &mappings);
                        add_path_quote(
                            &mut paths,
                            &contract_address,
                            BridgeConversionPathQuote {
                                destination_id: contract_address.clone(),
                                destination_display_name: destination_name.clone(),
                                destination_display_ticker: destination_ticker.clone(),
                                convert_to: Some(convertable_currency.currency_id.clone()),
                                convert_to_display_name: convertable_currency
                                    .fully_qualified_name
                                    .clone()
                                    .or_else(|| convertable_currency.name.clone())
                                    .or_else(|| Some(convertable_currency.currency_id.clone())),
                                export_to: None,
                                export_to_display_name: None,
                                via: Some(convertable_currency.currency_id.clone()),
                                via_display_name: convertable_currency
                                    .fully_qualified_name
                                    .clone()
                                    .or_else(|| convertable_currency.name.clone()),
                                map_to: Some(convertable_currency.currency_id.clone()),
                                price: Some(format_decimal(dest_price_in_via)),
                                via_price_in_root: None,
                                dest_price_in_via: None,
                                gateway: true,
                                mapping: false,
                                bounceback: true,
                                eth_destination: true,
                                prelaunch: false,
                            },
                        );
                    }
                } else if !equals_ignore_case(convertable_currency_id, &source_currency_id) {
                    let Some(root_price) =
                        extract_bridge_best_price(&bridge_definition, &source_currency_id)
                    else {
                        continue;
                    };
                    let Some(dest_price_in_via) =
                        extract_bridge_best_price(&bridge_definition, convertable_currency_id)
                    else {
                        continue;
                    };

                    let via_price_in_root = 1.0 / root_price;
                    let price = via_price_in_root * dest_price_in_via;

                    add_path_quote(
                        &mut paths,
                        convertable_currency_id,
                        BridgeConversionPathQuote {
                            destination_id: convertable_currency.currency_id.clone(),
                            destination_display_name: convertable_currency
                                .fully_qualified_name
                                .clone()
                                .or_else(|| convertable_currency.name.clone()),
                            destination_display_ticker: convertable_currency
                                .symbol
                                .clone()
                                .or_else(|| convertable_currency.name.clone()),
                            convert_to: Some(convertable_currency.currency_id.clone()),
                            convert_to_display_name: convertable_currency
                                .fully_qualified_name
                                .clone()
                                .or_else(|| convertable_currency.name.clone())
                                .or_else(|| Some(convertable_currency.currency_id.clone())),
                            export_to: Some(system_currency_id.clone()),
                            export_to_display_name: Some(system_display_name.clone()),
                            via: Some(bridge_currency_id.clone()),
                            via_display_name: Some(bridge_display_name.clone()),
                            map_to: None,
                            price: Some(format_decimal(price)),
                            via_price_in_root: Some(format_decimal(via_price_in_root)),
                            dest_price_in_via: Some(format_decimal(dest_price_in_via)),
                            gateway: true,
                            mapping: false,
                            bounceback: false,
                            eth_destination: false,
                            prelaunch: false,
                        },
                    );

                    for contract_address in mapped_contracts {
                        let (destination_name, destination_ticker) =
                            contract_display(&contract_address, &mappings);
                        add_path_quote(
                            &mut paths,
                            &contract_address,
                            BridgeConversionPathQuote {
                                destination_id: contract_address.clone(),
                                destination_display_name: destination_name.clone(),
                                destination_display_ticker: destination_ticker.clone(),
                                convert_to: Some(convertable_currency.currency_id.clone()),
                                convert_to_display_name: convertable_currency
                                    .fully_qualified_name
                                    .clone()
                                    .or_else(|| convertable_currency.name.clone())
                                    .or_else(|| Some(convertable_currency.currency_id.clone())),
                                export_to: None,
                                export_to_display_name: None,
                                via: Some(bridge_currency_id.clone()),
                                via_display_name: Some(bridge_display_name.clone()),
                                map_to: Some(convertable_currency.currency_id.clone()),
                                price: Some(format_decimal(price)),
                                via_price_in_root: Some(format_decimal(via_price_in_root)),
                                dest_price_in_via: Some(format_decimal(dest_price_in_via)),
                                gateway: true,
                                mapping: false,
                                bounceback: true,
                                eth_destination: true,
                                prelaunch: false,
                            },
                        );
                    }
                }
            }
        }

        add_path_quote(
            &mut paths,
            &source_currency_id,
            BridgeConversionPathQuote {
                destination_id: source_currency_id.clone(),
                destination_display_name: source_currency
                    .fully_qualified_name
                    .clone()
                    .or_else(|| source_currency.name.clone()),
                destination_display_ticker: source_currency
                    .symbol
                    .clone()
                    .or_else(|| source_currency.name.clone()),
                convert_to: Some(source_currency_id.clone()),
                convert_to_display_name: source_currency
                    .fully_qualified_name
                    .clone()
                    .or_else(|| source_currency.name.clone())
                    .or_else(|| Some(source_currency_id.clone())),
                export_to: Some(system_currency_id.clone()),
                export_to_display_name: Some(system_display_name.clone()),
                via: None,
                via_display_name: None,
                map_to: Some(source_currency_id.clone()),
                price: Some("1".to_string()),
                via_price_in_root: None,
                dest_price_in_via: None,
                gateway: true,
                mapping: true,
                bounceback: false,
                eth_destination: false,
                prelaunch: false,
            },
        );
    }

    Ok(BridgeConversionPathsResult {
        source_currency: request.source_currency.clone(),
        paths,
    })
}

async fn append_eth_mappings_for_vrpc_source(
    paths: &mut HashMap<String, Vec<BridgeConversionPathQuote>>,
    source_definition: &Value,
    request: &BridgeConversionPathRequest,
    vrpc_provider: &VrpcProvider,
    eth_provider: Option<&EthNetworkProvider>,
) -> Result<(), WalletError> {
    let Some(source_currency_id) = extract_currency_id(source_definition) else {
        return Ok(());
    };
    let mappings = get_currencies_mapped_to_eth(vrpc_provider, eth_provider).await?;
    let Some(mapped_contracts) = mappings
        .currency_to_contracts
        .get(&source_currency_id.to_ascii_lowercase())
        .cloned()
    else {
        return Ok(());
    };

    if mapped_contracts.is_empty() {
        return Ok(());
    }

    let veth_definition = match vrpc_provider.getcurrency("vETH").await {
        Ok(value) => Some(value),
        Err(_) => vrpc_provider.getcurrency("Bridge.vETH").await.ok(),
    };

    let export_to = veth_definition
        .as_ref()
        .and_then(extract_currency_id)
        .or_else(|| {
            request
                .channel_id
                .rsplit_once('.')
                .map(|(_, value)| value.to_string())
        });
    let export_to_label = veth_definition
        .as_ref()
        .and_then(extract_currency_display_name)
        .or(export_to.clone());

    for contract_address in mapped_contracts {
        let (destination_name, destination_ticker) = contract_display(&contract_address, &mappings);
        add_path_quote(
            paths,
            &contract_address,
            BridgeConversionPathQuote {
                destination_id: contract_address.clone(),
                destination_display_name: destination_name.clone(),
                destination_display_ticker: destination_ticker.clone(),
                convert_to: Some(contract_address.clone()),
                convert_to_display_name: destination_name
                    .or_else(|| Some(contract_address.clone())),
                export_to: export_to.clone(),
                export_to_display_name: export_to_label.clone(),
                via: None,
                via_display_name: None,
                map_to: Some(source_currency_id.clone()),
                price: Some("1".to_string()),
                via_price_in_root: None,
                dest_price_in_via: None,
                gateway: true,
                mapping: true,
                bounceback: false,
                eth_destination: false,
                prelaunch: false,
            },
        );
    }

    Ok(())
}

async fn resolve_currency_ref_by_id(
    vrpc_provider: &VrpcProvider,
    mappings: &EthTokenMappings,
    currency_id: &str,
) -> Option<CurrencyDefinitionRef> {
    if let Some(mapped) = mappings
        .currencies_by_id
        .get(&currency_id.to_ascii_lowercase())
        .cloned()
    {
        return Some(mapped);
    }

    let raw = vrpc_provider.getcurrency(currency_id).await.ok()?;
    Some(CurrencyDefinitionRef {
        currency_id: extract_currency_id(&raw)?,
        fully_qualified_name: raw.get("fullyqualifiedname").and_then(extract_stringish),
        name: raw.get("name").and_then(extract_stringish),
        symbol: raw.get("symbol").and_then(extract_stringish),
    })
}

fn extract_bridge_best_price(bridge_definition: &Value, currency_id: &str) -> Option<f64> {
    let best_state = bridge_definition.get("bestcurrencystate")?;
    let currencies = best_state.get("currencies")?.as_object()?;

    for (key, entry) in currencies {
        if !equals_ignore_case(key, currency_id) {
            continue;
        }
        let value = entry
            .get("lastconversionprice")
            .and_then(extract_f64)
            .or_else(|| extract_f64(entry))?;
        if value.is_finite() && value > 0.0 {
            return Some(value);
        }
    }

    None
}

fn resolve_system_id_from_chain_info(info: &Value) -> Option<String> {
    info.get("chainid").and_then(extract_stringish)
}

async fn resolve_system_id_for_request(
    request: &BridgeConversionPathRequest,
    vrpc_provider: &VrpcProvider,
) -> Result<String, WalletError> {
    if let Some((prefix, system_id)) = request.channel_id.split_once('.') {
        if prefix.eq_ignore_ascii_case("vrpc")
            && !system_id.trim().is_empty()
            && system_id.contains('.')
        {
            if let Some((_, parsed_system_id)) = request.channel_id.rsplit_once('.') {
                if !parsed_system_id.trim().is_empty() {
                    return Ok(parsed_system_id.to_string());
                }
            }
        } else if prefix.eq_ignore_ascii_case("vrpc") && !system_id.trim().is_empty() {
            return Ok(system_id.to_string());
        }
    }

    let info = vrpc_provider.getinfo().await?;
    resolve_system_id_from_chain_info(&info).ok_or(WalletError::OperationFailed)
}

fn contract_display(
    contract_address: &str,
    mappings: &EthTokenMappings,
) -> (Option<String>, Option<String>) {
    if contract_address.eq_ignore_ascii_case(ETH_ZERO_ADDRESS) {
        return (Some("Ethereum".to_string()), Some("ETH".to_string()));
    }

    let hit = mappings
        .contract_to_currencies
        .get(&contract_address.to_ascii_lowercase())
        .and_then(|currencies| currencies.first());
    if let Some(currency) = hit {
        let name = currency
            .fully_qualified_name
            .clone()
            .or_else(|| currency.name.clone())
            .or_else(|| Some(currency.currency_id.clone()));
        let ticker = currency
            .symbol
            .clone()
            .or_else(|| currency.name.clone())
            .or_else(|| name.clone());
        return (name, ticker);
    }

    (
        Some(contract_address.to_string()),
        Some(contract_address.to_string()),
    )
}

fn add_path_quote(
    paths: &mut HashMap<String, Vec<BridgeConversionPathQuote>>,
    destination_key: &str,
    quote: BridgeConversionPathQuote,
) {
    let entry = paths.entry(destination_key.to_string()).or_default();
    let duplicate = entry.iter().any(|existing| {
        existing
            .convert_to
            .as_deref()
            .eq(&quote.convert_to.as_deref())
            && existing
                .export_to
                .as_deref()
                .eq(&quote.export_to.as_deref())
            && existing.via.as_deref().eq(&quote.via.as_deref())
            && existing.map_to.as_deref().eq(&quote.map_to.as_deref())
            && existing.mapping == quote.mapping
            && existing.bounceback == quote.bounceback
            && existing.eth_destination == quote.eth_destination
    });
    if !duplicate {
        entry.push(quote);
    }
}

async fn derive_paths_from_listcurrencies(
    vrpc_provider: &VrpcProvider,
    source_definition: &Value,
    destination_currency_filter: Option<&str>,
    chain_currency_id: Option<&str>,
) -> Result<HashMap<String, Vec<BridgeConversionPathQuote>>, WalletError> {
    let mut list_payloads = Vec::new();
    let mut used_partitioned_list = true;

    for systemtype in ["local", "pbaas", "imported"] {
        match vrpc_provider
            .listcurrencies_with_systemtype(systemtype)
            .await
        {
            Ok(payload) => list_payloads.push(payload),
            Err(err) => {
                println!(
                    "[BRIDGE] listcurrencies systemtype={} unavailable in fallback: {:?}",
                    systemtype, err
                );
                used_partitioned_list = false;
                break;
            }
        }
    }

    if !used_partitioned_list || list_payloads.is_empty() {
        list_payloads.clear();
        list_payloads.push(vrpc_provider.listcurrencies().await?);
    }

    let chain_info = match vrpc_provider.getinfo().await {
        Ok(info) => Some(info),
        Err(err) => {
            println!(
                "[BRIDGE] getinfo unavailable in conversion-path fallback; using conservative defaults: {:?}",
                err
            );
            None
        }
    };
    let longest_chain = chain_info
        .as_ref()
        .and_then(extract_longest_chain)
        .unwrap_or(0);
    let chain_info_chain_id = chain_info
        .as_ref()
        .and_then(extract_chain_id)
        .or_else(|| chain_currency_id.map(ToString::to_string));

    let payload_refs = list_payloads.iter().collect::<Vec<_>>();
    let source_system_id = source_definition
        .get("systemid")
        .and_then(extract_stringish);
    let mut candidate_chain_ids: Vec<String> = Vec::new();

    for candidate in [
        chain_currency_id,
        chain_info_chain_id.as_deref(),
        source_system_id.as_deref(),
    ] {
        let Some(candidate_value) = candidate.map(str::trim).filter(|value| !value.is_empty())
        else {
            continue;
        };
        if candidate_chain_ids
            .iter()
            .any(|existing| equals_ignore_case(existing, candidate_value))
        {
            continue;
        }
        candidate_chain_ids.push(candidate_value.to_string());
    }

    if candidate_chain_ids.is_empty() {
        return derive_paths_from_list_payloads(
            &payload_refs,
            source_definition,
            destination_currency_filter,
            chain_currency_id,
            None,
            longest_chain,
            chain_info_chain_id.as_deref(),
        );
    }

    let mut last_empty_paths: Option<HashMap<String, Vec<BridgeConversionPathQuote>>> = None;
    for candidate_chain_id in candidate_chain_ids {
        let chain_definition = match vrpc_provider.getcurrency(&candidate_chain_id).await {
            Ok(definition) => Some(definition),
            Err(err) => {
                println!(
                    "[BRIDGE] chain currency lookup failed for {} during fallback: {:?}",
                    candidate_chain_id, err
                );
                None
            }
        };

        let derived_paths = derive_paths_from_list_payloads(
            &payload_refs,
            source_definition,
            destination_currency_filter,
            Some(candidate_chain_id.as_str()),
            chain_definition.as_ref(),
            longest_chain,
            chain_info_chain_id.as_deref(),
        )?;
        let route_count = count_quote_rows(&derived_paths);
        if parity_debug_enabled() {
            println!(
                "[BRIDGE][PARITY] fallback_chain={} routes={}",
                candidate_chain_id, route_count
            );
        }
        if route_count > 0 {
            return Ok(derived_paths);
        }
        last_empty_paths = Some(derived_paths);
    }

    Ok(last_empty_paths.unwrap_or_default())
}

fn derive_paths_from_list_payloads(
    list_payloads: &[&Value],
    source_definition: &Value,
    destination_currency_filter: Option<&str>,
    chain_currency_id: Option<&str>,
    chain_definition: Option<&Value>,
    longest_chain: u64,
    chain_info_chain_id: Option<&str>,
) -> Result<HashMap<String, Vec<BridgeConversionPathQuote>>, WalletError> {
    let mut currencies: HashMap<String, CurrencyNode> = HashMap::new();
    let mut prelaunch_currency_refs = HashSet::new();
    for payload in list_payloads {
        merge_currency_nodes_from_list_payload(payload, &mut currencies)?;
        prelaunch_currency_refs.extend(collect_prelaunch_currency_refs(payload, false));
    }

    let source_currency = currency_node_from_definition(
        source_definition,
        source_definition.get("bestcurrencystate"),
    )
    .ok_or(WalletError::OperationFailed)?;
    currencies
        .entry(source_currency.currency_id.clone())
        .or_insert_with(|| source_currency.clone());

    let chain_currency = chain_definition.and_then(|definition| {
        currency_node_from_definition(definition, definition.get("bestcurrencystate"))
    });
    if let Some(chain_currency) = chain_currency.as_ref() {
        currencies
            .entry(chain_currency.currency_id.clone())
            .or_insert_with(|| chain_currency.clone());
    }

    let effective_chain_currency_id = chain_currency_id
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            chain_currency
                .as_ref()
                .map(|value| value.currency_id.clone())
        })
        .or_else(|| source_currency.system_id.clone())
        .unwrap_or_else(|| source_currency.currency_id.clone());

    let chain_currency_context = currencies
        .get(&effective_chain_currency_id)
        .cloned()
        .or(chain_currency);
    let root_currency_id = chain_currency_context
        .as_ref()
        .map(|currency| currency.currency_id.as_str())
        .unwrap_or(effective_chain_currency_id.as_str());
    let chain_context_id = chain_info_chain_id.unwrap_or(effective_chain_currency_id.as_str());
    let mut debug_stats = RouteBuildDebugStats::default();

    let mut routes = collect_routes_for_source(
        &currencies,
        &source_currency,
        &effective_chain_currency_id,
        chain_currency_context.as_ref(),
        root_currency_id,
        chain_context_id,
        longest_chain,
        true,
        &HashSet::new(),
        None,
        None,
        Some(&mut debug_stats),
    );

    if let Some(filter) = destination_currency_filter {
        routes.retain(|destination_id, _| {
            currencies
                .get(destination_id)
                .map(|currency| currency.matches_filter(filter))
                .unwrap_or_else(|| equals_ignore_case(destination_id, filter))
        });
    }

    let mut quotes = routes_to_quotes(&routes, &currencies);
    if !prelaunch_currency_refs.is_empty() {
        mark_prelaunch_quotes(&mut quotes, &prelaunch_currency_refs);
    }
    if parity_debug_enabled() {
        println!(
            "[BRIDGE][PARITY] raw={} post_converter={} post_via={} post_visibility={} post_prelaunch={}",
            debug_stats.raw_candidate_count,
            debug_stats.post_converter_gating_count,
            debug_stats.post_via_recursion_count,
            debug_stats.post_visibility_filter_count,
            count_quote_rows(&quotes),
        );
    }
    Ok(quotes)
}

fn collect_routes_for_source(
    currencies: &HashMap<String, CurrencyNode>,
    source: &CurrencyNode,
    chain_currency_id: &str,
    chain_currency: Option<&CurrencyNode>,
    root_currency_id: &str,
    chain_info_chain_id: &str,
    longest_chain: u64,
    include_via: bool,
    ignore_currencies: &HashSet<String>,
    via: Option<&CurrencyNode>,
    root: Option<&CurrencyNode>,
    mut debug_stats: Option<&mut RouteBuildDebugStats>,
) -> HashMap<String, Vec<RouteCandidate>> {
    let mut routes: HashMap<String, Vec<RouteCandidate>> = HashMap::new();
    let mut raw_candidate_count = 0usize;

    for destination in currencies.values() {
        if !contains_currency(destination, &source.currency_id) {
            continue;
        }
        raw_candidate_count += 1;

        if !passes_converter_gating(
            destination,
            &source.currency_id,
            root_currency_id,
            chain_currency_id,
            longest_chain,
        ) {
            continue;
        }

        let Some((price, via_price_in_root, dest_price_in_via)) =
            compute_destination_price(source, destination, via, root)
        else {
            continue;
        };

        let gateway = is_gateway(destination);
        let gateway_converter = is_gateway_converter(destination)
            || is_gateway_converter(source)
            || via.map(is_gateway_converter).unwrap_or(false);
        let fractional_converter = via.unwrap_or(destination);

        let export_to_id = if gateway || gateway_converter {
            if gateway {
                Some(destination.currency_id.clone())
            } else {
                resolve_export_to_currency_id(fractional_converter, chain_currency_id)
            }
        } else {
            None
        };

        add_route_candidate(
            &mut routes,
            RouteCandidate {
                destination_id: destination.currency_id.clone(),
                via_id: via.map(|currency| currency.currency_id.clone()),
                export_to_id: export_to_id.clone(),
                price: Some(price),
                via_price_in_root,
                dest_price_in_via,
                gateway,
            },
        );

        if gateway || gateway_converter {
            add_route_candidate(
                &mut routes,
                RouteCandidate {
                    destination_id: destination.currency_id.clone(),
                    via_id: via.map(|currency| currency.currency_id.clone()),
                    export_to_id: None,
                    price: Some(price),
                    via_price_in_root,
                    dest_price_in_via,
                    gateway: false,
                },
            );
        }
    }

    let fractional_source = is_fractional_source(source, chain_currency_id);
    if fractional_source {
        for reserve_currency_id in &source.currencies {
            if contains_ignore(ignore_currencies, reserve_currency_id) {
                continue;
            }

            let Some(destination) = currencies.get(reserve_currency_id) else {
                continue;
            };

            let Some((price, via_price_in_root, dest_price_in_via)) =
                compute_reserve_price(source, reserve_currency_id, via, root)
            else {
                continue;
            };

            let gateway = is_gateway(destination);
            let gateway_converter = is_gateway_converter(destination)
                || is_gateway_converter(source)
                || via.map(is_gateway_converter).unwrap_or(false);
            let fractional_converter = via.unwrap_or(source);

            let export_to_id = if gateway || gateway_converter {
                if gateway {
                    Some(destination.currency_id.clone())
                } else {
                    resolve_export_to_currency_id(fractional_converter, chain_currency_id)
                }
            } else {
                None
            };

            add_route_candidate(
                &mut routes,
                RouteCandidate {
                    destination_id: destination.currency_id.clone(),
                    via_id: via.map(|currency| currency.currency_id.clone()),
                    export_to_id: export_to_id.clone(),
                    price: Some(price),
                    via_price_in_root,
                    dest_price_in_via,
                    gateway,
                },
            );

            if gateway || gateway_converter {
                add_route_candidate(
                    &mut routes,
                    RouteCandidate {
                        destination_id: destination.currency_id.clone(),
                        via_id: via.map(|currency| currency.currency_id.clone()),
                        export_to_id: None,
                        price: Some(price),
                        via_price_in_root,
                        dest_price_in_via,
                        gateway: false,
                    },
                );
            }
        }
    }

    if let Some(stats) = debug_stats.as_deref_mut() {
        stats.raw_candidate_count += raw_candidate_count;
        stats.post_converter_gating_count += count_route_candidates(&routes);
    }

    if include_via {
        let candidate_destinations = routes.keys().cloned().collect::<Vec<_>>();
        for destination_id in candidate_destinations {
            let Some(destination_currency) = currencies.get(&destination_id) else {
                continue;
            };

            if !is_fractional(destination_currency)
                || !contains_currency(destination_currency, &source.currency_id)
                || contains_ignore(ignore_currencies, &destination_currency.currency_id)
                || !destination_is_started(destination_currency, longest_chain, chain_info_chain_id)
            {
                continue;
            }

            let via_routes = collect_routes_for_source(
                currencies,
                destination_currency,
                chain_currency_id,
                chain_currency,
                root_currency_id,
                chain_info_chain_id,
                longest_chain,
                false,
                ignore_currencies,
                Some(destination_currency),
                Some(source),
                None,
            );
            merge_route_maps(&mut routes, via_routes);
        }
    }

    if let Some(stats) = debug_stats.as_deref_mut() {
        stats.post_via_recursion_count += count_route_candidates(&routes);
    }

    apply_visibility_filters(
        &mut routes,
        currencies,
        source,
        chain_currency_id,
        chain_currency,
    );

    routes.retain(|destination_id, candidates| {
        !equals_ignore_case(destination_id, &source.currency_id) && !candidates.is_empty()
    });

    if let Some(stats) = debug_stats.as_deref_mut() {
        stats.post_visibility_filter_count += count_route_candidates(&routes);
    }

    routes
}

fn merge_route_maps(
    destination: &mut HashMap<String, Vec<RouteCandidate>>,
    source: HashMap<String, Vec<RouteCandidate>>,
) {
    for candidates in source.into_values() {
        for candidate in candidates {
            add_route_candidate(destination, candidate);
        }
    }
}

fn add_route_candidate(
    routes: &mut HashMap<String, Vec<RouteCandidate>>,
    candidate: RouteCandidate,
) {
    let entry = routes.entry(candidate.destination_id.clone()).or_default();
    if entry
        .iter()
        .any(|existing| route_identity_matches(existing, &candidate))
    {
        return;
    }
    entry.push(candidate);
}

fn route_identity_matches(left: &RouteCandidate, right: &RouteCandidate) -> bool {
    equals_ignore_case(&left.destination_id, &right.destination_id)
        && option_equals_ignore_case(left.via_id.as_deref(), right.via_id.as_deref())
        && option_equals_ignore_case(left.export_to_id.as_deref(), right.export_to_id.as_deref())
}

fn option_equals_ignore_case(left: Option<&str>, right: Option<&str>) -> bool {
    match (left, right) {
        (Some(l), Some(r)) => equals_ignore_case(l, r),
        (None, None) => true,
        _ => false,
    }
}

fn apply_visibility_filters(
    routes: &mut HashMap<String, Vec<RouteCandidate>>,
    currencies: &HashMap<String, CurrencyNode>,
    source: &CurrencyNode,
    chain_currency_id: &str,
    chain_currency: Option<&CurrencyNode>,
) {
    let fractional_source = is_fractional_source(source, chain_currency_id);

    for (destination_id, candidates) in routes.iter_mut() {
        let Some(destination) = currencies.get(destination_id) else {
            candidates.clear();
            continue;
        };

        candidates.retain(|candidate| {
            let via_currency = candidate
                .via_id
                .as_deref()
                .and_then(|currency_id| currencies.get(currency_id));

            let fractional_converter = if let Some(via_currency) = via_currency {
                Some(via_currency)
            } else {
                let fractional_destination = is_fractional(destination);
                if fractional_source && !fractional_destination {
                    Some(source)
                } else if !fractional_source && fractional_destination {
                    Some(destination)
                } else if !source.currencies.is_empty() && !destination.currencies.is_empty() {
                    if contains_currency(source, &destination.currency_id) {
                        Some(source)
                    } else {
                        Some(destination)
                    }
                } else {
                    None
                }
            };

            let Some(fractional_converter) = fractional_converter else {
                // Keep parity with mobile path derivation: ambiguous fractional context is invalid.
                return false;
            };

            if let Some(fractional_system_id) = fractional_converter.system_id.as_deref() {
                if !equals_ignore_case(fractional_system_id, chain_currency_id) {
                    let export_matches = candidate
                        .export_to_id
                        .as_deref()
                        .map(|export_to_id| equals_ignore_case(export_to_id, fractional_system_id))
                        .unwrap_or(false);
                    if !export_matches {
                        return false;
                    }
                }
            }

            let Some(export_to_id) = candidate.export_to_id.as_deref() else {
                return true;
            };
            let Some(chain_currency) = chain_currency else {
                return true;
            };
            let Some(export_to_currency) = currencies.get(export_to_id) else {
                return true;
            };

            let see_by_launch = export_to_currency
                .launch_system_id
                .as_deref()
                .map(|value| equals_ignore_case(value, &chain_currency.currency_id))
                .unwrap_or(false)
                || chain_currency
                    .launch_system_id
                    .as_deref()
                    .map(|value| equals_ignore_case(value, &export_to_currency.currency_id))
                    .unwrap_or(false);

            let see_by_parent = export_to_currency
                .parent
                .as_deref()
                .map(|value| equals_ignore_case(value, &chain_currency.currency_id))
                .unwrap_or(false)
                || chain_currency
                    .parent
                    .as_deref()
                    .map(|value| equals_ignore_case(value, &export_to_currency.currency_id))
                    .unwrap_or(false);

            see_by_launch || see_by_parent
        });
    }
}

fn compute_destination_price(
    source: &CurrencyNode,
    destination: &CurrencyNode,
    via: Option<&CurrencyNode>,
    root: Option<&CurrencyNode>,
) -> Option<(f64, Option<f64>, Option<f64>)> {
    if let Some(via_currency) = via {
        let root_currency = root?;
        let via_price_raw = via_currency
            .best_prices
            .get(&root_currency.currency_id)
            .copied()?;
        let dest_price_raw = via_currency
            .best_prices
            .get(&destination.currency_id)
            .copied()?;

        let via_price_in_root = normalize_positive(1.0 / via_price_raw)?;
        let dest_price_in_via = normalize_positive(dest_price_raw)?;
        let price = normalize_positive(via_price_in_root * dest_price_in_via)?;
        return Some((price, Some(via_price_in_root), Some(dest_price_in_via)));
    }

    let source_price_raw = destination.best_prices.get(&source.currency_id).copied()?;
    let price = normalize_positive(1.0 / source_price_raw)?;
    Some((price, None, None))
}

fn compute_reserve_price(
    source: &CurrencyNode,
    reserve_currency_id: &str,
    via: Option<&CurrencyNode>,
    root: Option<&CurrencyNode>,
) -> Option<(f64, Option<f64>, Option<f64>)> {
    if let Some(via_currency) = via {
        let root_currency = root?;
        let via_price_raw = via_currency
            .best_prices
            .get(&root_currency.currency_id)
            .copied()?;
        let dest_price_raw = via_currency.best_prices.get(reserve_currency_id).copied()?;

        let via_price_in_root = normalize_positive(1.0 / via_price_raw)?;
        let dest_price_in_via = normalize_positive(dest_price_raw)?;
        let price = normalize_positive(via_price_in_root * dest_price_in_via)?;
        return Some((price, Some(via_price_in_root), Some(dest_price_in_via)));
    }

    let price_raw = source.best_prices.get(reserve_currency_id).copied()?;
    let price = normalize_positive(price_raw)?;
    Some((price, None, None))
}

fn normalize_positive(value: f64) -> Option<f64> {
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        None
    }
}

fn resolve_export_to_currency_id(
    fractional_converter: &CurrencyNode,
    chain_currency_id: &str,
) -> Option<String> {
    let parent = fractional_converter
        .parent
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    let launch = fractional_converter
        .launch_system_id
        .as_deref()
        .filter(|value| !value.trim().is_empty());

    if parent
        .map(|parent_currency| equals_ignore_case(parent_currency, chain_currency_id))
        .unwrap_or(false)
    {
        launch.map(ToString::to_string)
    } else {
        parent.map(ToString::to_string)
    }
}

fn is_fractional_source(source: &CurrencyNode, chain_currency_id: &str) -> bool {
    is_fractional(source)
        && contains_currency(source, chain_currency_id)
        && (source
            .system_id
            .as_deref()
            .map(|system_id| equals_ignore_case(system_id, chain_currency_id))
            .unwrap_or(false)
            || is_gateway_converter(source))
}

fn is_fractional(currency: &CurrencyNode) -> bool {
    has_flag(currency.options, IS_FRACTIONAL_FLAG)
}

fn is_gateway(currency: &CurrencyNode) -> bool {
    has_flag(currency.options, IS_GATEWAY_FLAG)
}

fn is_gateway_converter(currency: &CurrencyNode) -> bool {
    has_flag(currency.options, IS_GATEWAY_CONVERTER_FLAG)
}

fn has_flag(value: u64, flag: u64) -> bool {
    (value & flag) == flag
}

fn contains_currency(currency: &CurrencyNode, currency_id: &str) -> bool {
    currency
        .currencies
        .iter()
        .any(|reserve_currency| equals_ignore_case(reserve_currency, currency_id))
}

fn contains_ignore(ignore_currencies: &HashSet<String>, currency_id: &str) -> bool {
    ignore_currencies
        .iter()
        .any(|ignored| equals_ignore_case(ignored, currency_id))
}

fn passes_converter_gating(
    destination: &CurrencyNode,
    source_currency_id: &str,
    root_currency_id: &str,
    chain_currency_id: &str,
    longest_chain: u64,
) -> bool {
    if !contains_currency(destination, source_currency_id) {
        return false;
    }

    if destination.start_block > longest_chain {
        return !destination
            .max_preconversion_sum
            .is_some_and(is_effectively_zero);
    }

    if !is_fractional(destination) {
        return false;
    }

    let target_reserve = destination.reserve_state(root_currency_id);
    let system_reserve = destination.reserve_state(chain_currency_id);
    match (target_reserve, system_reserve) {
        (Some(target), Some(system)) => target.weight > 0.1 && system.reserves > 1000.0,
        _ => false,
    }
}

fn destination_is_started(
    destination: &CurrencyNode,
    longest_chain: u64,
    chain_info_chain_id: &str,
) -> bool {
    destination.start_block <= longest_chain
        || destination
            .launch_system_id
            .as_deref()
            .map(|launch_id| !equals_ignore_case(launch_id, chain_info_chain_id))
            .unwrap_or(true)
}

fn is_effectively_zero(value: f64) -> bool {
    value.abs() <= 1e-12
}

fn count_route_candidates(routes: &HashMap<String, Vec<RouteCandidate>>) -> usize {
    routes.values().map(Vec::len).sum()
}

fn count_quote_rows(paths: &HashMap<String, Vec<BridgeConversionPathQuote>>) -> usize {
    paths.values().map(Vec::len).sum()
}

fn parity_debug_enabled() -> bool {
    std::env::var("BRIDGE_PATH_PARITY_DEBUG")
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            normalized == "1" || normalized == "true" || normalized == "yes"
        })
        .unwrap_or(false)
}

fn routes_to_quotes(
    routes: &HashMap<String, Vec<RouteCandidate>>,
    currencies: &HashMap<String, CurrencyNode>,
) -> HashMap<String, Vec<BridgeConversionPathQuote>> {
    let mut normalized = HashMap::new();

    for (destination_id, candidates) in routes {
        let Some(destination) = currencies.get(destination_id) else {
            continue;
        };

        let mut quotes = Vec::new();
        for candidate in candidates {
            let export_to = candidate.export_to_id.clone();
            let export_to_display_name = candidate.export_to_id.as_ref().map(|currency_id| {
                currencies
                    .get(currency_id)
                    .map(currency_display_ref)
                    .unwrap_or_else(|| currency_id.clone())
            });
            let via = candidate.via_id.clone();
            let via_display_name = candidate.via_id.as_ref().map(|currency_id| {
                currencies
                    .get(currency_id)
                    .map(currency_display_ref)
                    .unwrap_or_else(|| currency_id.clone())
            });

            quotes.push(BridgeConversionPathQuote {
                destination_id: destination.currency_id.clone(),
                destination_display_name: destination.display_name(),
                destination_display_ticker: destination.display_ticker(),
                convert_to: Some(destination.currency_id.clone()),
                convert_to_display_name: Some(currency_display_ref(destination)),
                export_to,
                export_to_display_name,
                via,
                via_display_name,
                map_to: None,
                price: candidate.price.map(format_decimal),
                via_price_in_root: candidate.via_price_in_root.map(format_decimal),
                dest_price_in_via: candidate.dest_price_in_via.map(format_decimal),
                gateway: candidate.gateway,
                mapping: false,
                bounceback: false,
                eth_destination: false,
                prelaunch: false,
            });
        }

        if !quotes.is_empty() {
            normalized.insert(destination_id.clone(), quotes);
        }
    }

    normalized
}

fn currency_display_ref(currency: &CurrencyNode) -> String {
    currency
        .fully_qualified_name
        .clone()
        .or_else(|| currency.name.clone())
        .unwrap_or_else(|| currency.currency_id.clone())
}

fn merge_currency_nodes_from_list_payload(
    list_payload: &Value,
    currencies: &mut HashMap<String, CurrencyNode>,
) -> Result<(), WalletError> {
    let entries = list_payload
        .as_array()
        .ok_or(WalletError::OperationFailed)?;
    for entry in entries {
        if let Some(currency) = currency_node_from_list_entry(entry) {
            currencies
                .entry(currency.currency_id.clone())
                .or_insert(currency);
        }
    }
    Ok(())
}

fn currency_node_from_list_entry(entry: &Value) -> Option<CurrencyNode> {
    let definition = entry.get("currencydefinition")?;
    currency_node_from_definition(definition, entry.get("bestcurrencystate"))
}

fn currency_node_from_definition(
    definition: &Value,
    best_currency_state: Option<&Value>,
) -> Option<CurrencyNode> {
    let currency_id = definition.get("currencyid").and_then(extract_stringish)?;

    let reserve_currencies = definition
        .get("currencies")
        .and_then(Value::as_array)
        .map(|currencies| {
            currencies
                .iter()
                .filter_map(extract_currency_ref)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some(CurrencyNode {
        currency_id,
        fully_qualified_name: definition
            .get("fullyqualifiedname")
            .and_then(extract_stringish),
        name: definition.get("name").and_then(extract_stringish),
        symbol: definition.get("symbol").and_then(extract_stringish),
        options: definition.get("options").and_then(extract_u64).unwrap_or(0),
        system_id: definition.get("systemid").and_then(extract_stringish),
        parent: definition.get("parent").and_then(extract_stringish),
        launch_system_id: definition.get("launchsystemid").and_then(extract_stringish),
        start_block: definition
            .get("startblock")
            .and_then(extract_u64)
            .unwrap_or(0),
        max_preconversion_sum: extract_max_preconversion_sum(definition),
        currencies: reserve_currencies,
        best_prices: extract_best_prices(best_currency_state),
        reserve_states: extract_reserve_states(best_currency_state),
    })
}

fn extract_best_prices(best_currency_state: Option<&Value>) -> HashMap<String, f64> {
    let mut prices = HashMap::new();
    let Some(currencies) = best_currency_state
        .and_then(|state| state.get("currencies"))
        .and_then(Value::as_object)
    else {
        return prices;
    };

    for (currency_id, value) in currencies {
        let price = value
            .get("lastconversionprice")
            .and_then(extract_f64)
            .or_else(|| extract_f64(value));
        if let Some(price) = price.and_then(normalize_positive) {
            prices.insert(currency_id.clone(), price);
        }
    }

    prices
}

fn extract_max_preconversion_sum(definition: &Value) -> Option<f64> {
    let values = definition.get("maxpreconversion")?.as_array()?;
    let mut sum = 0.0;
    for value in values {
        sum += extract_f64(value).unwrap_or(0.0);
    }
    Some(sum)
}

fn extract_reserve_states(best_currency_state: Option<&Value>) -> HashMap<String, ReserveState> {
    let mut reserve_states = HashMap::new();
    let Some(reserves) = best_currency_state
        .and_then(|state| state.get("reservecurrencies"))
        .and_then(Value::as_array)
    else {
        return reserve_states;
    };

    for reserve in reserves {
        let Some(object) = reserve.as_object() else {
            continue;
        };
        let Some(currency_id) = object.get("currencyid").and_then(extract_stringish) else {
            continue;
        };
        let weight = object.get("weight").and_then(extract_f64).unwrap_or(0.0);
        let reserves_value = object.get("reserves").and_then(extract_f64).unwrap_or(0.0);
        reserve_states.insert(
            currency_id,
            ReserveState {
                weight,
                reserves: reserves_value,
            },
        );
    }

    reserve_states
}

fn format_decimal(value: f64) -> String {
    let mut output = format!("{value:.16}");
    while output.contains('.') && output.ends_with('0') {
        output.pop();
    }
    if output.ends_with('.') {
        output.pop();
    }
    if output.is_empty() {
        return "0".to_string();
    }
    output
}

fn equals_ignore_case(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn parse_conversion_paths(
    raw_paths: Value,
) -> Result<HashMap<String, Vec<BridgeConversionPathQuote>>, WalletError> {
    let raw_paths_object = raw_paths.as_object().ok_or(WalletError::OperationFailed)?;
    let mut normalized = HashMap::new();

    for (destination_key, raw_quotes) in raw_paths_object {
        let mut quotes = Vec::new();
        let Some(raw_quotes_array) = raw_quotes.as_array() else {
            normalized.insert(destination_key.clone(), quotes);
            continue;
        };

        for raw_quote in raw_quotes_array {
            let Some(raw_quote_object) = raw_quote.as_object() else {
                continue;
            };

            let destination = raw_quote_object.get("destination");
            let destination_id = extract_destination_id(destination, destination_key);

            let quote = BridgeConversionPathQuote {
                destination_id: destination_id.clone(),
                destination_display_name: extract_destination_display_name(destination),
                destination_display_ticker: extract_destination_display_ticker(destination),
                convert_to: raw_quote_object
                    .get("convertto")
                    .and_then(extract_currency_ref)
                    .or(Some(destination_id)),
                convert_to_display_name: raw_quote_object
                    .get("convertto")
                    .and_then(extract_currency_display_name)
                    .or_else(|| extract_destination_display_name(destination))
                    .or_else(|| {
                        raw_quote_object
                            .get("destination")
                            .and_then(extract_currency_ref)
                    }),
                export_to: raw_quote_object
                    .get("exportto")
                    .and_then(extract_currency_ref),
                export_to_display_name: raw_quote_object
                    .get("exportto")
                    .and_then(extract_currency_display_name),
                via: raw_quote_object.get("via").and_then(extract_currency_ref),
                via_display_name: raw_quote_object
                    .get("via")
                    .and_then(extract_currency_display_name),
                map_to: raw_quote_object
                    .get("mapto")
                    .and_then(extract_currency_ref)
                    .or_else(|| {
                        destination
                            .and_then(|value| value.get("mapto"))
                            .and_then(extract_currency_ref)
                    }),
                price: raw_quote_object.get("price").and_then(extract_stringish),
                via_price_in_root: raw_quote_object
                    .get("viapriceinroot")
                    .and_then(extract_stringish),
                dest_price_in_via: raw_quote_object
                    .get("destpriceinvia")
                    .and_then(extract_stringish),
                gateway: extract_bool(raw_quote_object.get("gateway")),
                mapping: extract_bool(raw_quote_object.get("mapping")),
                bounceback: extract_bool(raw_quote_object.get("bounceback")),
                eth_destination: extract_bool(raw_quote_object.get("ethdest")),
                prelaunch: extract_bool(raw_quote_object.get("prelaunch")),
            };

            quotes.push(quote);
        }

        normalized.insert(destination_key.clone(), quotes);
    }

    Ok(normalized)
}

fn collect_prelaunch_currency_refs(
    list_payload: &Value,
    assume_all_entries_prelaunch: bool,
) -> HashSet<String> {
    let mut refs = HashSet::new();
    let Some(entries) = list_payload.as_array() else {
        return refs;
    };

    for entry in entries {
        if !entry_is_prelaunch(entry, assume_all_entries_prelaunch) {
            continue;
        }
        collect_currency_refs_from_entry(entry, &mut refs);
    }

    refs
}

fn entry_is_prelaunch(entry: &Value, assume_all_entries_prelaunch: bool) -> bool {
    if assume_all_entries_prelaunch {
        return true;
    }

    extract_launchstate(entry).is_some_and(|value| equals_ignore_case(value, "prelaunch"))
}

fn extract_launchstate(entry: &Value) -> Option<&str> {
    entry
        .get("launchstate")
        .and_then(Value::as_str)
        .or_else(|| {
            entry
                .get("currencydefinition")
                .and_then(Value::as_object)
                .and_then(|definition| definition.get("launchstate"))
                .and_then(Value::as_str)
        })
}

fn collect_currency_refs_from_entry(entry: &Value, refs: &mut HashSet<String>) {
    let Some(definition) = entry.get("currencydefinition") else {
        return;
    };

    for key in ["currencyid", "fullyqualifiedname", "name"] {
        if let Some(value) = definition.get(key).and_then(extract_stringish) {
            let normalized = normalize_currency_ref(&value);
            if !normalized.is_empty() {
                refs.insert(normalized);
            }
        }
    }
}

fn normalize_currency_ref(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn collect_currency_display_lookup(list_payload: &Value) -> HashMap<String, String> {
    let mut lookup = HashMap::new();
    let Some(entries) = list_payload.as_array() else {
        return lookup;
    };

    for entry in entries {
        let definition = entry.get("currencydefinition").unwrap_or(entry);
        insert_currency_display_lookup_entry(&mut lookup, definition);
    }

    lookup
}

fn insert_currency_display_lookup_entry(lookup: &mut HashMap<String, String>, definition: &Value) {
    let Some(display_label) = extract_currency_display_name(definition) else {
        return;
    };

    for key in [
        "currencyid",
        "address",
        "fullyqualifiedname",
        "name",
        "symbol",
    ] {
        let Some(value) = definition.get(key).and_then(extract_stringish) else {
            continue;
        };

        let normalized = normalize_currency_ref(&value);
        if normalized.is_empty() || lookup.contains_key(&normalized) {
            continue;
        }
        lookup.insert(normalized, display_label.clone());
    }
}

fn enrich_quote_display_names(
    paths: &mut HashMap<String, Vec<BridgeConversionPathQuote>>,
    lookup: &HashMap<String, String>,
) {
    if lookup.is_empty() {
        return;
    }

    for quotes in paths.values_mut() {
        for quote in quotes {
            maybe_enrich_display_name(
                &mut quote.destination_display_name,
                Some(&quote.destination_id),
                lookup,
            );
            maybe_enrich_display_name(
                &mut quote.convert_to_display_name,
                quote.convert_to.as_deref(),
                lookup,
            );
            maybe_enrich_display_name(
                &mut quote.export_to_display_name,
                quote.export_to.as_deref(),
                lookup,
            );
            maybe_enrich_display_name(&mut quote.via_display_name, quote.via.as_deref(), lookup);
        }
    }
}

fn maybe_enrich_display_name(
    display_name: &mut Option<String>,
    identity: Option<&str>,
    lookup: &HashMap<String, String>,
) {
    let Some(identity_value) = identity else {
        return;
    };

    let identity_normalized = normalize_currency_ref(identity_value);
    if identity_normalized.is_empty() {
        return;
    }

    let should_enrich = match display_name.as_deref() {
        None => true,
        Some(current) => current.trim().is_empty() || equals_ignore_case(current, identity_value),
    };
    if !should_enrich {
        return;
    }

    let Some(enriched_value) = lookup.get(&identity_normalized) else {
        return;
    };
    *display_name = Some(enriched_value.clone());
}

fn mark_prelaunch_quotes(
    paths: &mut HashMap<String, Vec<BridgeConversionPathQuote>>,
    prelaunch_refs: &HashSet<String>,
) {
    if prelaunch_refs.is_empty() {
        return;
    }

    for (destination_key, quotes) in paths {
        let destination_key_normalized = normalize_currency_ref(destination_key);
        for quote in quotes {
            quote.prelaunch = quote.prelaunch
                || quote_matches_prelaunch(quote, &destination_key_normalized, prelaunch_refs);
        }
    }
}

fn quote_matches_prelaunch(
    quote: &BridgeConversionPathQuote,
    destination_key_normalized: &str,
    prelaunch_refs: &HashSet<String>,
) -> bool {
    if prelaunch_refs.contains(destination_key_normalized) {
        return true;
    }

    if prelaunch_refs.contains(&normalize_currency_ref(&quote.destination_id)) {
        return true;
    }

    if let Some(convert_to) = quote.convert_to.as_deref() {
        if prelaunch_refs.contains(&normalize_currency_ref(convert_to)) {
            return true;
        }
    }

    false
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

fn extract_f64(value: &Value) -> Option<f64> {
    if let Some(raw) = value.as_f64() {
        return Some(raw);
    }
    if let Some(raw) = value.as_i64() {
        return Some(raw as f64);
    }
    if let Some(raw) = value.as_u64() {
        return Some(raw as f64);
    }
    value.as_str().and_then(|raw| raw.parse::<f64>().ok())
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

fn extract_longest_chain(info: &Value) -> Option<u64> {
    info.get("longestchain").and_then(extract_u64)
}

fn extract_chain_id(info: &Value) -> Option<String> {
    info.get("chainid").and_then(extract_stringish)
}

fn extract_currency_id(definition: &Value) -> Option<String> {
    definition
        .get("currencyid")
        .and_then(extract_stringish)
        .or_else(|| {
            definition
                .get("fullyqualifiedname")
                .and_then(extract_stringish)
        })
        .or_else(|| definition.get("name").and_then(extract_stringish))
}

fn extract_currency_ref(value: &Value) -> Option<String> {
    if let Some(raw) = value.as_str() {
        return Some(raw.to_string());
    }

    let object = value.as_object()?;
    object
        .get("currencyid")
        .and_then(extract_stringish)
        .or_else(|| object.get("address").and_then(extract_stringish))
        .or_else(|| object.get("fullyqualifiedname").and_then(extract_stringish))
        .or_else(|| object.get("name").and_then(extract_stringish))
}

fn extract_currency_display_name(value: &Value) -> Option<String> {
    if let Some(raw) = value.as_str() {
        return Some(raw.to_string());
    }

    let object = value.as_object()?;
    object
        .get("fullyqualifiedname")
        .and_then(extract_stringish)
        .or_else(|| object.get("name").and_then(extract_stringish))
        .or_else(|| object.get("symbol").and_then(extract_stringish))
        .or_else(|| object.get("currencyid").and_then(extract_stringish))
        .or_else(|| object.get("address").and_then(extract_stringish))
}

fn extract_destination_id(destination: Option<&Value>, fallback: &str) -> String {
    destination
        .and_then(extract_currency_ref)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn extract_destination_display_name(destination: Option<&Value>) -> Option<String> {
    let object = destination?.as_object()?;
    object
        .get("fullyqualifiedname")
        .and_then(extract_stringish)
        .or_else(|| object.get("name").and_then(extract_stringish))
}

fn extract_destination_display_ticker(destination: Option<&Value>) -> Option<String> {
    destination
        .and_then(Value::as_object)
        .and_then(|object| object.get("symbol"))
        .and_then(extract_stringish)
}

fn extract_bool(value: Option<&Value>) -> bool {
    value.and_then(Value::as_bool).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{
        collect_currency_display_lookup, derive_paths_from_list_payloads,
        enrich_quote_display_names, parse_conversion_paths,
    };
    use serde_json::json;

    #[test]
    fn parse_conversion_paths_normalizes_common_vrpc_shape() {
        let input = json!({
            "iDest": [
                {
                    "destination": {
                        "currencyid": "iDest",
                        "fullyqualifiedname": "Bridge.DEST",
                        "symbol": "DEST"
                    },
                    "convertto": {
                        "currencyid": "iDest",
                        "fullyqualifiedname": "Bridge.DEST"
                    },
                    "exportto": {
                        "currencyid": "iExport",
                        "fullyqualifiedname": "DEST"
                    },
                    "via": {
                        "currencyid": "iVia",
                        "fullyqualifiedname": "Bridge.vETH"
                    },
                    "price": 1.25,
                    "viapriceinroot": "1.5",
                    "destpriceinvia": 0.83,
                    "gateway": true,
                    "mapping": false,
                    "bounceback": false,
                    "ethdest": false
                }
            ]
        });

        let parsed = parse_conversion_paths(input).expect("parse paths");
        let quotes = parsed.get("iDest").expect("destination key");
        assert_eq!(quotes.len(), 1);

        let quote = &quotes[0];
        assert_eq!(quote.destination_id, "iDest");
        assert_eq!(
            quote.destination_display_name.as_deref(),
            Some("Bridge.DEST")
        );
        assert_eq!(quote.destination_display_ticker.as_deref(), Some("DEST"));
        assert_eq!(quote.convert_to.as_deref(), Some("iDest"));
        assert_eq!(quote.export_to.as_deref(), Some("iExport"));
        assert_eq!(quote.via.as_deref(), Some("iVia"));
        assert_eq!(quote.price.as_deref(), Some("1.25"));
        assert_eq!(quote.via_price_in_root.as_deref(), Some("1.5"));
        assert_eq!(quote.dest_price_in_via.as_deref(), Some("0.83"));
        assert!(quote.gateway);
        assert!(!quote.mapping);
        assert!(!quote.prelaunch);
    }

    #[test]
    fn parse_conversion_paths_rejects_non_object_payload() {
        let parsed = parse_conversion_paths(json!([]));
        assert!(parsed.is_err());
    }

    #[test]
    fn parse_conversion_paths_reads_prelaunch_flag() {
        let input = json!({
            "iDest": [
                {
                    "destination": {
                        "currencyid": "iDest",
                        "fullyqualifiedname": "Bridge.DEST",
                        "symbol": "DEST"
                    },
                    "price": 1.25,
                    "prelaunch": true
                }
            ]
        });

        let parsed = parse_conversion_paths(input).expect("parse paths");
        let quotes = parsed.get("iDest").expect("destination key");
        assert_eq!(quotes.len(), 1);
        assert!(quotes[0].prelaunch);
    }

    #[test]
    fn parse_conversion_paths_enriches_display_labels_from_currency_lookup() {
        let input = json!({
            "iDest": [
                {
                    "destination": {
                        "currencyid": "iDest",
                        "fullyqualifiedname": "vUSDC.vETH",
                        "symbol": "vUSDC"
                    },
                    "convertto": "iDest",
                    "exportto": "iExport",
                    "via": "iVia",
                    "price": 1.25
                }
            ]
        });
        let list_payload = json!([
            {
                "currencydefinition": {
                    "currencyid": "iDest",
                    "fullyqualifiedname": "vUSDC.vETH",
                    "name": "vUSDC",
                    "symbol": "vUSDC"
                }
            },
            {
                "currencydefinition": {
                    "currencyid": "iVia",
                    "fullyqualifiedname": "Bridge.vETH",
                    "name": "Bridge",
                    "symbol": "BRIDGE"
                }
            },
            {
                "currencydefinition": {
                    "currencyid": "iExport",
                    "fullyqualifiedname": "Ethereum",
                    "name": "Ethereum",
                    "symbol": "ETH"
                }
            }
        ]);

        let mut parsed = parse_conversion_paths(input).expect("parse paths");
        let lookup = collect_currency_display_lookup(&list_payload);
        enrich_quote_display_names(&mut parsed, &lookup);

        let quotes = parsed.get("iDest").expect("destination key");
        assert_eq!(quotes.len(), 1);
        let quote = &quotes[0];
        assert_eq!(quote.convert_to_display_name.as_deref(), Some("vUSDC.vETH"));
        assert_eq!(quote.export_to_display_name.as_deref(), Some("Ethereum"));
        assert_eq!(quote.via_display_name.as_deref(), Some("Bridge.vETH"));
    }

    #[test]
    fn fallback_derives_direct_path_quote() {
        let source_definition = json!({
            "currencyid": "iSource",
            "name": "SRC",
            "systemid": "iChain"
        });
        let list_payload = json!([
            {
                "currencydefinition": {
                    "currencyid": "iDest",
                    "fullyqualifiedname": "Bridge.DEST",
                    "name": "DEST",
                    "options": 129,
                    "startblock": 500,
                    "maxpreconversion": [1],
                    "currencies": ["iSource"]
                },
                "bestcurrencystate": {
                    "currencies": {
                        "iSource": {
                            "lastconversionprice": 2.0
                        }
                    }
                }
            }
        ]);

        let paths = derive_paths_from_list_payloads(
            &[&list_payload],
            &source_definition,
            None,
            Some("iChain"),
            None,
            100,
            Some("iChain"),
        )
        .expect("paths");

        let quotes = paths.get("iDest").expect("destination");
        assert!(!quotes.is_empty());
        assert!(quotes.iter().any(|quote| quote.export_to.is_none()));

        let bridge_route = quotes
            .iter()
            .find(|quote| quote.export_to.as_deref() == Some("iDest"))
            .expect("bridge route");
        assert_eq!(bridge_route.convert_to.as_deref(), Some("iDest"));
        assert_eq!(bridge_route.price.as_deref(), Some("0.5"));
        assert!(bridge_route.gateway);
        assert!(!bridge_route.prelaunch);
    }

    #[test]
    fn fallback_derives_via_paths_from_partitioned_payloads() {
        let source_definition = json!({
            "currencyid": "iSource",
            "name": "VRSC",
            "symbol": "VRSC",
            "systemid": "iChain",
            "options": 0
        });

        let local_payload = json!([
            {
                "currencydefinition": {
                    "currencyid": "iSource",
                    "name": "VRSC",
                    "symbol": "VRSC",
                    "systemid": "iChain",
                    "options": 0
                },
                "bestcurrencystate": {
                    "currencies": {}
                }
            }
        ]);

        let pbaas_payload = json!([
            {
                "currencydefinition": {
                    "currencyid": "iDest",
                    "fullyqualifiedname": "vUSDC.vETH",
                    "name": "vUSDC",
                    "symbol": "vUSDC",
                    "systemid": "iChain",
                    "options": 0,
                    "startblock": 500,
                    "maxpreconversion": [1],
                    "currencies": ["iVia"]
                },
                "bestcurrencystate": {
                    "currencies": {
                        "iVia": {
                            "lastconversionprice": 0.25
                        }
                    }
                }
            }
        ]);

        let imported_payload = json!([
            {
                "currencydefinition": {
                    "currencyid": "iVia",
                    "fullyqualifiedname": "Bridge.vETH",
                    "name": "vETH",
                    "symbol": "vETH",
                    "systemid": "iChain",
                    "options": 1,
                    "currencies": ["iSource", "iDest", "iChain"]
                },
                "bestcurrencystate": {
                    "currencies": {
                        "iSource": {
                            "lastconversionprice": 2.0
                        },
                        "iDest": {
                            "lastconversionprice": 4.0
                        },
                        "iChain": {
                            "lastconversionprice": 1.0
                        }
                    },
                    "reservecurrencies": [
                        {
                            "currencyid": "iChain",
                            "weight": 0.2,
                            "reserves": 2000
                        }
                    ]
                }
            }
        ]);

        let chain_definition = json!({
            "currencyid": "iChain",
            "name": "VRSC",
            "launchsystemid": "iChain",
            "parent": "iParent"
        });

        let paths = derive_paths_from_list_payloads(
            &[&local_payload, &pbaas_payload, &imported_payload],
            &source_definition,
            None,
            Some("iChain"),
            Some(&chain_definition),
            100,
            Some("iChain"),
        )
        .expect("paths");

        let quotes = paths.get("iDest").expect("destination");
        let via_quote = quotes
            .iter()
            .find(|quote| quote.via.as_deref() == Some("iVia"))
            .expect("via route");

        assert_eq!(via_quote.convert_to.as_deref(), Some("iDest"));
        assert_eq!(via_quote.price.as_deref(), Some("2"));
        assert_eq!(via_quote.via_price_in_root.as_deref(), Some("0.5"));
        assert_eq!(via_quote.dest_price_in_via.as_deref(), Some("4"));
        assert!(!via_quote.prelaunch);
    }

    #[test]
    fn fallback_marks_prelaunch_routes_from_launchstate() {
        let source_definition = json!({
            "currencyid": "iSource",
            "name": "SRC",
            "systemid": "iChain"
        });

        let list_payload = json!([
            {
                "launchstate": "prelaunch",
                "currencydefinition": {
                    "currencyid": "iPrelaunchDest",
                    "fullyqualifiedname": "Bridge.PRE",
                    "name": "PRE",
                    "options": 129,
                    "startblock": 500,
                    "maxpreconversion": [1],
                    "currencies": ["iSource"]
                },
                "bestcurrencystate": {
                    "currencies": {
                        "iSource": {
                            "lastconversionprice": 2.0
                        }
                    }
                }
            }
        ]);

        let paths = derive_paths_from_list_payloads(
            &[&list_payload],
            &source_definition,
            None,
            Some("iChain"),
            None,
            100,
            Some("iChain"),
        )
        .expect("paths");

        let quotes = paths.get("iPrelaunchDest").expect("prelaunch destination");
        assert!(!quotes.is_empty());
        assert!(quotes.iter().all(|quote| quote.prelaunch));
    }

    #[test]
    fn fallback_excludes_non_fractional_launched_candidates() {
        let source_definition = json!({
            "currencyid": "iSource",
            "name": "SRC",
            "systemid": "iChain",
            "options": 0
        });

        let list_payload = json!([
            {
                "currencydefinition": {
                    "currencyid": "iNonFractionalDest",
                    "name": "NFD",
                    "options": 0,
                    "currencies": ["iSource"]
                },
                "bestcurrencystate": {
                    "currencies": {
                        "iSource": {
                            "lastconversionprice": 2.0
                        }
                    }
                }
            }
        ]);

        let paths = derive_paths_from_list_payloads(
            &[&list_payload],
            &source_definition,
            None,
            Some("iChain"),
            None,
            100,
            Some("iChain"),
        )
        .expect("paths");

        assert!(!paths.contains_key("iNonFractionalDest"));
    }

    #[test]
    fn fallback_excludes_low_liquidity_fractional_candidates() {
        let source_definition = json!({
            "currencyid": "iSource",
            "name": "SRC",
            "systemid": "iChain",
            "options": 0
        });

        let list_payload = json!([
            {
                "currencydefinition": {
                    "currencyid": "iLowLiquidity",
                    "name": "LOW",
                    "options": 1,
                    "currencies": ["iSource"]
                },
                "bestcurrencystate": {
                    "currencies": {
                        "iSource": {
                            "lastconversionprice": 2.0
                        }
                    },
                    "reservecurrencies": [
                        {
                            "currencyid": "iChain",
                            "weight": 0.05,
                            "reserves": 500
                        }
                    ]
                }
            }
        ]);

        let paths = derive_paths_from_list_payloads(
            &[&list_payload],
            &source_definition,
            None,
            Some("iChain"),
            None,
            100,
            Some("iChain"),
        )
        .expect("paths");

        assert!(!paths.contains_key("iLowLiquidity"));
    }

    #[test]
    fn fallback_via_recursion_requires_started_destination() {
        let source_definition = json!({
            "currencyid": "iSource",
            "name": "SRC",
            "systemid": "iChain",
            "options": 0
        });

        let local_payload = json!([
            {
                "currencydefinition": {
                    "currencyid": "iSource",
                    "name": "SRC",
                    "systemid": "iChain",
                    "options": 0
                },
                "bestcurrencystate": {
                    "currencies": {}
                }
            }
        ]);

        let fractional_via_payload = json!([
            {
                "currencydefinition": {
                    "currencyid": "iViaFractional",
                    "name": "VIA",
                    "options": 1,
                    "systemid": "iChain",
                    "launchsystemid": "iChain",
                    "startblock": 500,
                    "maxpreconversion": [1],
                    "currencies": ["iSource", "iDestViaOnly", "iChain"]
                },
                "bestcurrencystate": {
                    "currencies": {
                        "iSource": {
                            "lastconversionprice": 2.0
                        },
                        "iDestViaOnly": {
                            "lastconversionprice": 4.0
                        },
                        "iChain": {
                            "lastconversionprice": 1.0
                        }
                    },
                    "reservecurrencies": [
                        {
                            "currencyid": "iChain",
                            "weight": 0.2,
                            "reserves": 2000
                        }
                    ]
                }
            },
            {
                "currencydefinition": {
                    "currencyid": "iDestViaOnly",
                    "name": "DEST",
                    "options": 1,
                    "systemid": "iChain",
                    "currencies": ["iViaFractional"]
                },
                "bestcurrencystate": {
                    "currencies": {
                        "iViaFractional": {
                            "lastconversionprice": 0.25
                        }
                    },
                    "reservecurrencies": [
                        {
                            "currencyid": "iChain",
                            "weight": 0.2,
                            "reserves": 2000
                        }
                    ]
                }
            }
        ]);

        let paths = derive_paths_from_list_payloads(
            &[&local_payload, &fractional_via_payload],
            &source_definition,
            None,
            Some("iChain"),
            None,
            100,
            Some("iChain"),
        )
        .expect("paths");

        assert!(paths.contains_key("iViaFractional"));
        assert!(!paths.contains_key("iDestViaOnly"));
    }

    #[test]
    fn fallback_drops_routes_with_ambiguous_fractional_context() {
        let source_definition = json!({
            "currencyid": "iSource",
            "name": "SRC",
            "systemid": "iChain",
            "options": 0
        });

        let list_payload = json!([
            {
                "currencydefinition": {
                    "currencyid": "iAmbiguousDest",
                    "name": "AMB",
                    "options": 0,
                    "startblock": 500,
                    "maxpreconversion": [1],
                    "currencies": ["iSource"]
                },
                "bestcurrencystate": {
                    "currencies": {
                        "iSource": {
                            "lastconversionprice": 2.0
                        }
                    }
                }
            }
        ]);

        let paths = derive_paths_from_list_payloads(
            &[&list_payload],
            &source_definition,
            None,
            Some("iChain"),
            None,
            100,
            Some("iChain"),
        )
        .expect("paths");

        assert!(!paths.contains_key("iAmbiguousDest"));
    }
}
