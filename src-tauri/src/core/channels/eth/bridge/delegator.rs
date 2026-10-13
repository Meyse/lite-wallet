//
// Verus bridge delegator contract metadata and helpers.

use ethers::contract::abigen;
use ethers::providers::Middleware;
use ethers::types::Address;

use crate::core::channels::eth::config::metadata_for_network;
use crate::core::channels::eth::provider::EthNetworkProvider;
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

abigen!(
    VerusBridgeDelegatorContract,
    r#"[
      {
        "inputs": [],
        "name": "bridgeConverterActive",
        "outputs": [{"internalType": "bool", "name": "", "type": "bool"}],
        "stateMutability": "view",
        "type": "function"
      },
      {
        "inputs": [
          {"internalType": "uint256", "name": "start", "type": "uint256"},
          {"internalType": "uint256", "name": "end", "type": "uint256"}
        ],
        "name": "getTokenList",
        "outputs": [
          {
            "components": [
              {"internalType": "address", "name": "iaddress", "type": "address"},
              {"internalType": "address", "name": "erc20ContractAddress", "type": "address"},
              {"internalType": "address", "name": "launchSystemID", "type": "address"},
              {"internalType": "uint8", "name": "flags", "type": "uint8"},
              {"internalType": "string", "name": "name", "type": "string"},
              {"internalType": "string", "name": "ticker", "type": "string"},
              {"internalType": "uint256", "name": "tokenID", "type": "uint256"}
            ],
            "internalType": "struct VerusObjects.setupToken[]",
            "name": "",
            "type": "tuple[]"
          }
        ],
        "stateMutability": "nonpayable",
        "type": "function"
      },
      {
        "inputs": [
          {
            "components": [
              {"internalType": "uint32", "name": "version", "type": "uint32"},
              {
                "components": [
                  {"internalType": "address", "name": "currency", "type": "address"},
                  {"internalType": "uint64", "name": "amount", "type": "uint64"}
                ],
                "internalType": "struct VerusObjects.CCurrencyValueMap",
                "name": "currencyvalue",
                "type": "tuple"
              },
              {"internalType": "uint32", "name": "flags", "type": "uint32"},
              {"internalType": "address", "name": "feecurrencyid", "type": "address"},
              {"internalType": "uint64", "name": "fees", "type": "uint64"},
              {
                "components": [
                  {"internalType": "uint8", "name": "destinationtype", "type": "uint8"},
                  {"internalType": "bytes", "name": "destinationaddress", "type": "bytes"}
                ],
                "internalType": "struct VerusObjectsCommon.CTransferDestination",
                "name": "destination",
                "type": "tuple"
              },
              {"internalType": "address", "name": "destcurrencyid", "type": "address"},
              {"internalType": "address", "name": "destsystemid", "type": "address"},
              {"internalType": "address", "name": "secondreserveid", "type": "address"}
            ],
            "internalType": "struct VerusObjects.CReserveTransfer",
            "name": "_transfer",
            "type": "tuple"
          }
        ],
        "name": "sendTransfer",
        "outputs": [],
        "stateMutability": "payable",
        "type": "function"
      }
    ]"#,
);

pub fn delegator_contract_for_network(network: WalletNetwork) -> Result<Address, WalletError> {
    metadata_for_network(network)
        .bridge_delegator
        .parse::<Address>()
        .map_err(|_| WalletError::OperationFailed)
}

/// Testnet and private testnet share Sepolia but use different delegators, so the wallet
/// network selects the contract. A provider on a chain the network does not use fails.
pub fn delegator_contract_for(
    network: WalletNetwork,
    provider_chain_id: u64,
) -> Result<Address, WalletError> {
    if metadata_for_network(network).chain_id != provider_chain_id {
        return Err(WalletError::EthNetworkMismatch);
    }
    delegator_contract_for_network(network)
}

pub async fn validate_delegator_contract(
    provider: &EthNetworkProvider,
) -> Result<(Address, bool), WalletError> {
    provider.validate_rpc_identity().await?;
    let address = delegator_contract_for(provider.wallet_network, provider.chain_id)?;
    let code = provider
        .rpc_provider
        .get_code(address, None)
        .await
        .map_err(|_| WalletError::NetworkError)?;
    if code.as_ref().is_empty() {
        return Err(WalletError::BridgeDeploymentUnavailable);
    }

    let contract = VerusBridgeDelegatorContract::new(
        address,
        std::sync::Arc::new(provider.rpc_provider.clone()),
    );
    let converter_active = contract
        .bridge_converter_active()
        .call()
        .await
        .map_err(|_| WalletError::BridgeDeploymentUnavailable)?;
    Ok((address, converter_active))
}

#[cfg(test)]
mod tests {
    use super::{delegator_contract_for, delegator_contract_for_network};
    use crate::types::wallet::WalletNetwork;
    use ethers::types::Address;

    #[test]
    fn mainnet_delegator_is_valid_eth_address() {
        let addr = delegator_contract_for_network(WalletNetwork::Mainnet).expect("mainnet address");
        assert_ne!(addr, Address::zero());
    }

    #[test]
    fn testnet_delegator_is_valid_eth_address() {
        let addr = delegator_contract_for_network(WalletNetwork::Testnet).expect("testnet address");
        assert_ne!(addr, Address::zero());
    }

    #[test]
    fn wallet_network_selects_the_delegator_on_its_own_chain() {
        // Each delegator is the vETH `nativecurrencyid` of its Verus chain (2026-10-13).
        let sepolia_testnet: Address = "0xcaa98a4ec79dac8a06cb3bfdcf5351b6576d939f"
            .parse()
            .unwrap();
        let sepolia_private_testnet: Address = "0x98f58684b05e7d40719e61955015af91ef7e77d3"
            .parse()
            .unwrap();
        assert_eq!(
            delegator_contract_for(WalletNetwork::Testnet, 11155111).unwrap(),
            sepolia_testnet
        );
        assert_eq!(
            delegator_contract_for(WalletNetwork::PrivateTestnet, 11155111).unwrap(),
            sepolia_private_testnet
        );
        assert_ne!(
            delegator_contract_for(WalletNetwork::Mainnet, 1).unwrap(),
            sepolia_testnet
        );
    }

    #[test]
    fn a_provider_on_another_chain_is_rejected() {
        assert!(delegator_contract_for(WalletNetwork::Testnet, 5).is_err());
        assert!(delegator_contract_for(WalletNetwork::PrivateTestnet, 31337).is_err());
        assert!(delegator_contract_for(WalletNetwork::Mainnet, 11155111).is_err());
    }
}
