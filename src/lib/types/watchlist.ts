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
