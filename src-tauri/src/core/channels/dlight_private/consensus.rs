use crate::types::wallet::WalletNetwork;
use serde::{Deserialize, Serialize};
use zcash_protocol::consensus::{BlockHeight, NetworkType, NetworkUpgrade, Parameters};

#[derive(Debug, Clone, Copy)]
pub(super) struct VerusConsensusParams {
    network: WalletNetwork,
}

impl Parameters for VerusConsensusParams {
    fn network_type(&self) -> NetworkType {
        match self.network {
            WalletNetwork::Mainnet => NetworkType::Main,
            WalletNetwork::Testnet => NetworkType::Test,
        }
    }
    fn activation_height(&self, upgrade: NetworkUpgrade) -> Option<BlockHeight> {
        match upgrade {
            NetworkUpgrade::Overwinter | NetworkUpgrade::Sapling => {
                Some(BlockHeight::from_u32(activation_height(self.network) as u32))
            }
            // Verus uses Sapling v4 and pre-ZIP-212 note encryption on both chains.
            // Zcash's later network upgrades must never activate implicitly here.
            _ => None,
        }
    }
}

pub(super) fn parameters(network: WalletNetwork) -> VerusConsensusParams {
    VerusConsensusParams { network }
}
pub(super) fn activation_height(network: WalletNetwork) -> u64 {
    match network {
        WalletNetwork::Mainnet => 227_520,
        WalletNetwork::Testnet => 1,
    }
}
pub(super) fn birthday_floor(network: WalletNetwork) -> u64 {
    activation_height(network) - 1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DlightBirthday {
    pub height: u64,
    /// Internal byte order, matching CompactBlock.hash (not a display hash).
    pub block_hash_hex: String,
    pub sapling_tree: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DlightSeedMetadata {
    #[serde(default)]
    pub birthday: Option<DlightBirthday>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use zcash_protocol::consensus::BranchId;
    #[test]
    fn both_chains_stay_on_sapling_at_future_heights() {
        for network in [WalletNetwork::Mainnet, WalletNetwork::Testnet] {
            let params = parameters(network);
            assert_eq!(
                BranchId::for_height(&params, BlockHeight::from_u32(10_000_000)),
                BranchId::Sapling
            );
            assert_eq!(params.activation_height(NetworkUpgrade::Canopy), None);
            assert_eq!(params.activation_height(NetworkUpgrade::Nu5), None);
        }
    }
}
