import type { CoinDefinition, WalletNetwork } from './wallet.js';

export type WatchlistAvailability = 'available' | 'partial' | 'unavailable';
export type WatchlistTargetKind = 'identity' | 'address';

export interface WatchlistEntry {
  id: string;
  targetKind: WatchlistTargetKind;
  displayName: string;
  address: string;
  systemId?: string | null;
  createdAt: number;
  updatedAt: number;
}

export interface WatchlistResolvedTarget {
  targetKind: WatchlistTargetKind;
  displayName: string;
  address: string;
  systemId?: string | null;
  visibleCurrencyCount?: number | null;
  availability: WatchlistAvailability;
}

export interface WatchlistHolding {
  assetKey: string;
  currencyId: string;
  systemId: string;
  systemTicker: string;
  systemDisplayName: string;
  balance: string;
  coin?: CoinDefinition | null;
}

export interface WatchlistSource {
  systemId: string;
  systemTicker: string;
  systemDisplayName: string;
  status: WatchlistAvailability;
}

export interface WatchlistEntrySnapshot {
  entry: WatchlistEntry;
  holdings: WatchlistHolding[];
  sources: WatchlistSource[];
  availability: WatchlistAvailability;
  refreshedAt: number;
}

export interface WatchlistRefreshResult {
  network: WalletNetwork;
  entries: WatchlistEntrySnapshot[];
  refreshedAt: number;
}

/** A watched currency to price: the chain it is held on and its currency ID. */
export interface WatchlistRateRequest {
  systemId: string;
  currencyId: string;
}

/** A public rate for a watched currency that the update engine does not track. */
export interface WatchedCurrencyRate {
  coinId: string;
  rates: Record<string, number>;
  usdChange24hPct: number | null;
  /** Unix seconds when the source data was fetched. */
  fetchedAt: number;
}
