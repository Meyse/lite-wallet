use std::env;

use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

const DEFAULT_MAINNET_INFURA_URL: &str = "https://mainnet.infura.io/v3/{project_id}";
const DEFAULT_TESTNET_INFURA_URL: &str = "https://sepolia.infura.io/v3/{project_id}";
const DEFAULT_ETHERSCAN_MAINNET_URL: &str = "https://api.etherscan.io/v2/api";
const DEFAULT_ETHERSCAN_TESTNET_URL: &str = "https://api.etherscan.io/v2/api";

pub const ETHEREUM_MAINNET_CHAIN_ID: u64 = 1;
pub const ETHEREUM_SEPOLIA_CHAIN_ID: u64 = 11155111;
pub const VERUS_BRIDGE_DELEGATOR_MAINNET_CONTRACT: &str =
    "0x71518580f36FeCEFfE0721F06bA4703218cD7F63";
pub const VERUS_BRIDGE_DELEGATOR_SEPOLIA_CONTRACT: &str =
    "0xCaA98A4eC79dAC8A06Cb3BfDcF5351b6576d939f";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EthNetworkMetadata {
    pub wallet_network: WalletNetwork,
    pub chain_id: u64,
    pub network_name: &'static str,
    pub native_asset_name: &'static str,
    pub bridge_delegator: &'static str,
}

pub fn metadata_for_network(network: WalletNetwork) -> EthNetworkMetadata {
    match network {
        WalletNetwork::Mainnet => EthNetworkMetadata {
            wallet_network: network,
            chain_id: ETHEREUM_MAINNET_CHAIN_ID,
            network_name: "Ethereum",
            native_asset_name: "Ethereum",
            bridge_delegator: VERUS_BRIDGE_DELEGATOR_MAINNET_CONTRACT,
        },
        WalletNetwork::Testnet => EthNetworkMetadata {
            wallet_network: network,
            chain_id: ETHEREUM_SEPOLIA_CHAIN_ID,
            network_name: "Sepolia",
            native_asset_name: "Sepolia ETH",
            bridge_delegator: VERUS_BRIDGE_DELEGATOR_SEPOLIA_CONTRACT,
        },
    }
}

pub fn metadata_for_chain_id(chain_id: u64) -> Result<EthNetworkMetadata, WalletError> {
    match chain_id {
        ETHEREUM_MAINNET_CHAIN_ID => Ok(metadata_for_network(WalletNetwork::Mainnet)),
        ETHEREUM_SEPOLIA_CHAIN_ID => Ok(metadata_for_network(WalletNetwork::Testnet)),
        _ => Err(WalletError::UnsupportedNetwork),
    }
}

#[derive(Debug, Clone)]
pub struct EthChannelConfig {
    pub etherscan_api_key: String,
    pub mainnet_rpc_url: String,
    pub testnet_rpc_url: String,
    pub etherscan_mainnet_url: String,
    pub etherscan_testnet_url: String,
}

impl EthChannelConfig {
    pub fn from_env() -> Result<Self, WalletError> {
        let infura_project_id = read_required_env("INFURA_PROJECT_ID")?;
        let etherscan_api_key = read_required_env("ETHERSCAN_API_KEY")?;

        let mainnet_rpc_url = read_optional_env("ETH_MAINNET_RPC_URL").unwrap_or_else(|| {
            DEFAULT_MAINNET_INFURA_URL.replace("{project_id}", &infura_project_id)
        });
        let testnet_rpc_url = read_optional_env("ETH_TESTNET_RPC_URL").unwrap_or_else(|| {
            DEFAULT_TESTNET_INFURA_URL.replace("{project_id}", &infura_project_id)
        });

        let etherscan_mainnet_url = read_optional_env("ETHERSCAN_MAINNET_URL")
            .unwrap_or_else(|| DEFAULT_ETHERSCAN_MAINNET_URL.to_string());
        let etherscan_testnet_url = read_optional_env("ETHERSCAN_TESTNET_URL")
            .unwrap_or_else(|| DEFAULT_ETHERSCAN_TESTNET_URL.to_string());

        Ok(Self {
            etherscan_api_key,
            mainnet_rpc_url,
            testnet_rpc_url,
            etherscan_mainnet_url,
            etherscan_testnet_url,
        })
    }
}

fn read_optional_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn read_required_env(key: &str) -> Result<String, WalletError> {
    read_optional_env(key).ok_or(WalletError::EthNotConfigured)
}
