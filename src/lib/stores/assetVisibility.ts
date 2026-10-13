import type { CoinDefinition, Protocol, WalletNetwork } from '$lib/types/wallet.js';

const ASSET_VISIBILITY_STORAGE_KEY = 'lite-wallet.asset-visibility.v1';

type VisibilitySnapshot = Record<WalletNetwork, string[]>;
type AssetVisibilityProto = Protocol | 'fiat';

const EMPTY_VISIBILITY: VisibilitySnapshot = {
  mainnet: [],
  testnet: [],
  privatetestnet: []
};

function normalizeAssetKey(key: string): string {
  return key.trim().toLowerCase();
}

function canUseStorage(): boolean {
  return typeof globalThis.localStorage !== 'undefined';
}

function readSnapshot(): VisibilitySnapshot {
  if (!canUseStorage()) return EMPTY_VISIBILITY;

  try {
    const raw = globalThis.localStorage.getItem(ASSET_VISIBILITY_STORAGE_KEY);
    if (!raw) return EMPTY_VISIBILITY;

    const parsed = JSON.parse(raw) as Partial<VisibilitySnapshot> | null;
    const mainnet = Array.isArray(parsed?.mainnet)
      ? parsed.mainnet.map((value) => normalizeAssetKey(String(value)))
      : [];
    const testnet = Array.isArray(parsed?.testnet)
      ? parsed.testnet.map((value) => normalizeAssetKey(String(value)))
      : [];
    const privatetestnet = Array.isArray(parsed?.privatetestnet)
      ? parsed.privatetestnet.map((value) => normalizeAssetKey(String(value)))
      : [];

    return { mainnet, testnet, privatetestnet };
  } catch {
    return EMPTY_VISIBILITY;
  }
}

function writeSnapshot(snapshot: VisibilitySnapshot): void {
  if (!canUseStorage()) return;

  try {
    globalThis.localStorage.setItem(ASSET_VISIBILITY_STORAGE_KEY, JSON.stringify(snapshot));
  } catch {
    // Ignore visibility persistence failures.
  }
}

function updateSnapshot(
  network: WalletNetwork,
  updater: (current: Set<string>) => void
): VisibilitySnapshot {
  const snapshot = readSnapshot();
  const nextSet = new Set(snapshot[network].map((key) => normalizeAssetKey(key)));
  updater(nextSet);
  const nextSnapshot: VisibilitySnapshot = {
    ...snapshot,
    [network]: Array.from(nextSet)
  };
  writeSnapshot(nextSnapshot);
  return nextSnapshot;
}

export function assetVisibilityKey(id: string, proto: AssetVisibilityProto): string {
  return `${proto}:${id.toLowerCase()}`;
}

export function isAssetHiddenByKey(key: string, network: WalletNetwork): boolean {
  const snapshot = readSnapshot();
  const normalized = normalizeAssetKey(key);
  return snapshot[network].includes(normalized);
}

export function hideAssetByKey(key: string, network: WalletNetwork): void {
  const normalized = normalizeAssetKey(key);
  updateSnapshot(network, (set) => {
    set.add(normalized);
  });
}

export function showAssetByKey(key: string, network: WalletNetwork): void {
  const normalized = normalizeAssetKey(key);
  updateSnapshot(network, (set) => {
    set.delete(normalized);
  });
}

export function filterVisibleAssets(
  coins: CoinDefinition[],
  network: WalletNetwork
): CoinDefinition[] {
  const hidden = new Set(readSnapshot()[network]);
  if (hidden.size === 0) return coins;

  return coins.filter((coin) => !hidden.has(assetVisibilityKey(coin.id, coin.proto)));
}
