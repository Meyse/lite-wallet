import { invokeSessionBoundWalletCommand } from './invokeWalletCommand.js';
import type {
  WatchedCurrencyRate,
  WatchlistEntry,
  WatchlistEntrySnapshot,
  WatchlistRateRequest,
  WatchlistRefreshResult,
  WatchlistResolvedTarget,
} from '$lib/types/watchlist.js';

export function getWatchlistEntries(): Promise<WatchlistEntry[]> {
  return invokeSessionBoundWalletCommand<WatchlistEntry[]>('get_watchlist_entries');
}

export function resolveWatchlistTarget(query: string): Promise<WatchlistResolvedTarget> {
  return invokeSessionBoundWalletCommand<WatchlistResolvedTarget>('resolve_watchlist_target', {
    request: { query },
  });
}

export function addWatchlistEntry(query: string, name?: string): Promise<WatchlistEntrySnapshot> {
  return invokeSessionBoundWalletCommand<WatchlistEntrySnapshot>('add_watchlist_entry', {
    request: { query, name },
  });
}

export function removeWatchlistEntry(entryId: string): Promise<boolean> {
  return invokeSessionBoundWalletCommand<boolean>('remove_watchlist_entry', { entry_id: entryId });
}

export function refreshWatchlist(): Promise<WatchlistRefreshResult> {
  return invokeSessionBoundWalletCommand<WatchlistRefreshResult>('refresh_watchlist');
}

/** Prices watched currencies that the update engine does not track. */
export function getWatchlistRates(
  currencies: WatchlistRateRequest[]
): Promise<WatchedCurrencyRate[]> {
  return invokeSessionBoundWalletCommand<WatchedCurrencyRate[]>('get_watchlist_rates', {
    currencies,
  });
}
