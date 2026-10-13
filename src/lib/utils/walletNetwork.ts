import type { WalletNetwork } from '$lib/types/wallet';

// Testnet and private testnet follow VRSCTEST chain rules: the same coins, system
// and currency IDs, address encoding, and Sepolia ETH. Use this only for chain
// rules; endpoints, storage, and network names depend on the network itself.
const TEST_CHAIN: Record<WalletNetwork, boolean> = {
  mainnet: false,
  testnet: true,
  privatetestnet: true,
};

// Order of the network choice when a wallet is created or imported.
export const WALLET_NETWORKS: readonly WalletNetwork[] = ['mainnet', 'testnet', 'privatetestnet'];

export function isTestChain(network: WalletNetwork | null | undefined): boolean {
  return network ? TEST_CHAIN[network] : false;
}
