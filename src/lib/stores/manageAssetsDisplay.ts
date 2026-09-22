import type { AssetDiscoveryResult, WalletNetwork } from '$lib/types/wallet.js';
import type { KnownAssetBalance } from './manageAssets.js';

interface ManageAssetsSnapshot {
  discovery: AssetDiscoveryResult | null;
  knownBalances: KnownAssetBalance[];
  discoveryStale: boolean;
}

// Display-only, in memory, for one unlock session. Preferences are always read
// afresh; cached discovery never authorizes a send or changes portfolio choices.
let owner = '';
let snapshot: ManageAssetsSnapshot | null = null;
let generation = 0;

export function resetManageAssetsDisplay(): void {
  owner = '';
  snapshot = null;
  generation += 1;
}

export function readManageAssetsDisplay(
  sessionId: string,
  network: WalletNetwork
): { snapshot: ManageAssetsSnapshot | null; generation: number } {
  const key = `${network}:${sessionId}`;
  if (owner !== key) {
    owner = key;
    snapshot = null;
    generation += 1;
  }
  return { snapshot: snapshot ? structuredClone(snapshot) : null, generation };
}

export function writeManageAssetsDisplay(
  sessionId: string,
  network: WalletNetwork,
  expectedGeneration: number,
  value: ManageAssetsSnapshot
): void {
  // A late response from an old view cannot repopulate a reset/replaced cache.
  if (owner !== `${network}:${sessionId}` || generation !== expectedGeneration) return;
  snapshot = structuredClone(value);
}
