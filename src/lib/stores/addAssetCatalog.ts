import verusCoinCatalog from '$lib/coins/verusCoinCatalog.generated.json';
import { resolveCoinPresentation, resolveCoinPresentationById } from '$lib/coins/presentation.js';
import { isWalletSupportedAsset } from '$lib/coins/supportedAssets.js';
import type { CoinDefinition, WalletNetwork } from '$lib/types/wallet.js';

export type AddAssetProto = CoinDefinition['proto'] | 'fiat';
export type AddAssetStatus = 'added' | 'available';
export type AddAssetAddStrategy = 'activate' | 'direct' | 'resolve_pbaas' | 'resolve_erc20';

interface CatalogCoin {
  id: string;
  currencyId: string;
  systemId: string;
  displayTicker: string;
  displayName: string;
  coinPaprikaId?: string | null;
  proto: AddAssetProto;
  mappedTo: string | null;
  isTestnet: boolean;
}

export interface AddAssetEntry {
  key: string;
  id: string;
  currencyId: string;
  systemId: string;
  displayTicker: string;
  displayName: string;
  coinPaprikaId: string | null;
  proto: AddAssetProto;
  mappedTo: string | null;
  isTestnet: boolean;
  source: 'runtime' | 'catalog' | 'both';
  status: AddAssetStatus;
  addStrategy: AddAssetAddStrategy;
}

export interface AddAssetCatalogView {
  addedEntries: AddAssetEntry[];
  availableEntries: AddAssetEntry[];
}

interface BuildAddAssetCatalogInput {
  coins: CoinDefinition[];
  network: WalletNetwork;
  query: string;
  activeCoinIds: string[];
}

const catalogCoins = verusCoinCatalog as CatalogCoin[];

const STATUS_ORDER: Record<AddAssetStatus, number> = {
  added: 0,
  available: 1,
};

const ETH_ZERO_ADDRESS = '0x0000000000000000000000000000000000000000';

function catalogLookupKey(value: string, isTestnet: boolean): string {
  return `${isTestnet ? 'testnet' : 'mainnet'}:${value.toLowerCase()}`;
}

const catalogById = new Map<string, CatalogCoin>();
const catalogByCurrencyId = new Map<string, CatalogCoin>();

for (const coin of catalogCoins) {
  catalogById.set(catalogLookupKey(coin.id, coin.isTestnet), coin);
  if (coin.currencyId.trim().length > 0) {
    catalogByCurrencyId.set(catalogLookupKey(coin.currencyId, coin.isTestnet), coin);
  }
}

function findCatalogCoinForDefinition(definition: CoinDefinition): CatalogCoin | null {
  const isTestnet = Boolean(definition.isTestnet);
  const idKey = catalogLookupKey(definition.id, isTestnet);
  const idMatch = catalogById.get(idKey);
  if (idMatch) return idMatch;

  const currencyId = definition.currencyId.trim();
  if (currencyId.length === 0) return null;
  const currencyKey = catalogLookupKey(currencyId, isTestnet);
  return catalogByCurrencyId.get(currencyKey) ?? null;
}

export function applyCatalogMetadataToCoinDefinition(definition: CoinDefinition): CoinDefinition {
  const catalogCoin = findCatalogCoinForDefinition(definition);
  if (!catalogCoin) return definition;
  const presentation = resolveCoinPresentationById(catalogCoin.id, catalogCoin.proto);

  return {
    ...definition,
    displayTicker:
      presentation?.displayTicker || catalogCoin.displayTicker || definition.displayTicker,
    displayName: presentation?.displayName || catalogCoin.displayName || definition.displayName,
    coinPaprikaId: catalogCoin.coinPaprikaId ?? definition.coinPaprikaId ?? null,
    mappedTo: catalogCoin.mappedTo ?? definition.mappedTo ?? null,
  };
}

function normalizeSearchValue(value: string): string {
  return value.trim().toLowerCase();
}

function entryKey(id: string, proto: AddAssetProto): string {
  return `${proto}:${id.toLowerCase()}`;
}

function availabilityForCatalogCoin(
  coin: CatalogCoin,
  network: WalletNetwork
): { status: AddAssetStatus; addStrategy: AddAssetAddStrategy } | null {
  if (!isWalletSupportedAsset(coin, network)) {
    return null;
  }

  if (coin.proto === 'vrsc') {
    return { status: 'available', addStrategy: 'resolve_pbaas' };
  }

  if (coin.proto === 'erc20' && coin.currencyId.startsWith('0x')) {
    return { status: 'available', addStrategy: 'resolve_erc20' };
  }

  if (coin.proto === 'eth') {
    return { status: 'available', addStrategy: 'direct' };
  }

  if (coin.proto === 'btc' && (coin.id === 'BTC' || coin.id === 'BTCTEST')) {
    return { status: 'available', addStrategy: 'direct' };
  }

  return null;
}

function searchRank(entry: AddAssetEntry, query: string): number | null {
  const normalized = normalizeSearchValue(query);
  if (!normalized) return 99;

  const ticker = entry.displayTicker.toLowerCase();
  const name = entry.displayName.toLowerCase();
  const id = entry.id.toLowerCase();
  const currencyId = entry.currencyId.toLowerCase();
  const systemId = entry.systemId.toLowerCase();

  if (ticker === normalized) return 0;
  if (ticker.startsWith(normalized)) return 1;
  if (name.includes(normalized)) return 2;
  if (id.includes(normalized) || currencyId.includes(normalized) || systemId.includes(normalized)) {
    return 3;
  }

  return null;
}

function compareEntries(a: AddAssetEntry, b: AddAssetEntry, query: string): number {
  const rankA = searchRank(a, query) ?? Number.MAX_SAFE_INTEGER;
  const rankB = searchRank(b, query) ?? Number.MAX_SAFE_INTEGER;

  if (rankA !== rankB) {
    return rankA - rankB;
  }

  const statusOrder = STATUS_ORDER[a.status] - STATUS_ORDER[b.status];
  if (statusOrder !== 0) {
    return statusOrder;
  }

  const tickerCompare = a.displayTicker.localeCompare(b.displayTicker, undefined, {
    sensitivity: 'base',
  });
  if (tickerCompare !== 0) {
    return tickerCompare;
  }

  return a.displayName.localeCompare(b.displayName, undefined, { sensitivity: 'base' });
}

export function buildAddAssetCatalogView({
  coins,
  network,
  query,
  activeCoinIds,
}: BuildAddAssetCatalogInput): AddAssetCatalogView {
  const isTestnet = network === 'testnet';
  const activeSet = new Set(
    activeCoinIds.map((coinId) => coinId.trim().toLowerCase()).filter((coinId) => coinId.length > 0)
  );
  const entries = new Map<string, AddAssetEntry>();

  for (const runtimeCoin of coins) {
    if (!isWalletSupportedAsset(runtimeCoin, network)) continue;

    const presentation = resolveCoinPresentation(runtimeCoin);
    // Custom ERC20 registrations use erc20_<contract> IDs, while the bundled
    // catalog uses tickers. Merge by asset identity, as the rendered rows do.
    const key = entryKey(runtimeCoin.currencyId.trim() || runtimeCoin.id, runtimeCoin.proto);

    entries.set(key, {
      key,
      id: runtimeCoin.id,
      currencyId: runtimeCoin.currencyId,
      systemId: runtimeCoin.systemId,
      displayTicker: presentation.displayTicker,
      displayName: presentation.displayName,
      coinPaprikaId: runtimeCoin.coinPaprikaId ?? null,
      proto: runtimeCoin.proto,
      mappedTo: runtimeCoin.mappedTo ?? null,
      isTestnet: runtimeCoin.isTestnet,
      source: 'runtime',
      status: activeSet.has(runtimeCoin.id.toLowerCase()) ? 'added' : 'available',
      addStrategy: 'activate',
    });
  }

  for (const catalogCoin of catalogCoins) {
    if (catalogCoin.isTestnet !== isTestnet) {
      continue;
    }

    const key = entryKey(catalogCoin.currencyId.trim() || catalogCoin.id, catalogCoin.proto);
    const existing = entries.get(key);

    if (existing) {
      existing.source = existing.source === 'runtime' ? 'both' : existing.source;
      if (!existing.coinPaprikaId && catalogCoin.coinPaprikaId) {
        existing.coinPaprikaId = catalogCoin.coinPaprikaId;
      }
      if (!existing.mappedTo && catalogCoin.mappedTo) {
        existing.mappedTo = catalogCoin.mappedTo;
      }
      continue;
    }

    const availability = availabilityForCatalogCoin(catalogCoin, network);
    if (!availability) continue;
    const presentation = resolveCoinPresentationById(catalogCoin.id, catalogCoin.proto);

    entries.set(key, {
      key,
      id: catalogCoin.id,
      currencyId: catalogCoin.currencyId,
      systemId: catalogCoin.systemId,
      displayTicker: presentation?.displayTicker || catalogCoin.displayTicker,
      displayName: presentation?.displayName || catalogCoin.displayName,
      coinPaprikaId: catalogCoin.coinPaprikaId ?? null,
      proto: catalogCoin.proto,
      mappedTo: catalogCoin.mappedTo,
      isTestnet: catalogCoin.isTestnet,
      source: 'catalog',
      status: availability.status,
      addStrategy: availability.addStrategy,
    });
  }

  const filtered = Array.from(entries.values()).filter(
    (entry) => searchRank(entry, query) !== null
  );
  const sorted = filtered.sort((a, b) => compareEntries(a, b, query));

  return {
    addedEntries: sorted.filter((entry) => entry.status === 'added'),
    availableEntries: sorted.filter((entry) => entry.status === 'available'),
  };
}

export function catalogEntryToCoinDefinition(
  entry: AddAssetEntry,
  network: WalletNetwork
): CoinDefinition | null {
  const isTestnet = network === 'testnet';

  if (entry.proto === 'vrsc') {
    return {
      id: entry.id,
      currencyId: entry.currencyId,
      systemId: entry.systemId,
      displayTicker: entry.displayTicker,
      displayName: entry.displayName,
      coinPaprikaId: entry.coinPaprikaId,
      proto: 'vrsc',
      compatibleChannels: ['vrpc'],
      decimals: 8,
      vrpcEndpoints: [],
      electrumEndpoints: null,
      secondsPerBlock: 60,
      mappedTo: entry.mappedTo,
      isTestnet,
    };
  }

  if (entry.proto === 'eth') {
    return {
      id: entry.id,
      currencyId: entry.currencyId || ETH_ZERO_ADDRESS,
      systemId: entry.systemId,
      displayTicker: entry.displayTicker,
      displayName: entry.displayName,
      coinPaprikaId: entry.coinPaprikaId,
      proto: 'eth',
      compatibleChannels: ['eth'],
      decimals: 18,
      vrpcEndpoints: [],
      electrumEndpoints: null,
      secondsPerBlock: 12,
      mappedTo: entry.mappedTo,
      isTestnet,
    };
  }

  if (entry.proto === 'btc') {
    if (entry.id !== 'BTC' && entry.id !== 'BTCTEST') {
      return null;
    }

    return {
      id: entry.id,
      currencyId: entry.currencyId,
      systemId: entry.systemId,
      displayTicker: entry.displayTicker,
      displayName: entry.displayName,
      coinPaprikaId: entry.coinPaprikaId,
      proto: 'btc',
      compatibleChannels: ['btc'],
      decimals: 8,
      vrpcEndpoints: [],
      electrumEndpoints: null,
      secondsPerBlock: 600,
      mappedTo: entry.mappedTo,
      isTestnet,
    };
  }

  return null;
}

export function pbaasLookupValue(entry: AddAssetEntry): string {
  return entry.currencyId || entry.id;
}

export function erc20ContractValue(entry: AddAssetEntry): string {
  return entry.currencyId || entry.id;
}
