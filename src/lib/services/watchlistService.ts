import { invokeWalletCommand } from './invokeWalletCommand.js';
import type {
  WatchlistEntry,
  WatchlistEntrySnapshot,
  WatchlistRefreshResult,
  WatchlistResolvedTarget,
} from '$lib/types/watchlist.js';

export function getWatchlistEntries(): Promise<WatchlistEntry[]> {
  return invokeWalletCommand<WatchlistEntry[]>('get_watchlist_entries');
}

export function resolveWatchlistTarget(query: string): Promise<WatchlistResolvedTarget> {
  return invokeWalletCommand<WatchlistResolvedTarget>('resolve_watchlist_target', {
    request: { query },
  });
}

export function addWatchlistEntry(query: string): Promise<WatchlistEntrySnapshot> {
  return invokeWalletCommand<WatchlistEntrySnapshot>('add_watchlist_entry', {
    request: { query },
  });
}

export function removeWatchlistEntry(entryId: string): Promise<boolean> {
  return invokeWalletCommand<boolean>('remove_watchlist_entry', { entry_id: entryId });
}

export function refreshWatchlist(): Promise<WatchlistRefreshResult> {
  return invokeWalletCommand<WatchlistRefreshResult>('refresh_watchlist');
}
