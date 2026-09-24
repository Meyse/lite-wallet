import { get, writable } from 'svelte/store';
import type { WalletNetwork } from '$lib/types/wallet';
import type { WatchlistEntry } from '$lib/types/watchlist';

export type WatchlistSession = { sessionId: string; network: WalletNetwork };

/**
 * Wallet binding for the cached Watchlist membership, owned by the wallet
 * route. Operations refuse to run without it, so membership can never be
 * cached outside a wallet session.
 */
export const watchlistSession = writable<WatchlistSession | null>(null);
/**
 * Known membership for the current session. `null` means membership is unknown
 * (never loaded, loading, or failed), so a failed read can never be mistaken
 * for a confirmed empty Watchlist.
 */
export const watchlistEntriesStore = writable<WatchlistEntry[] | null>(null);
export const watchlistLoadState = writable<'idle' | 'loading' | 'ready' | 'error'>('idle');
/**
 * Bumped whenever the binding is set or cleared, including repeated clears.
 * Session-scoped work and caches are invalidated through this even when the
 * session identity itself does not change.
 */
export const watchlistSessionEpoch = writable(0);

function clearWatchlistCache(): void {
  watchlistEntriesStore.set(null);
  watchlistLoadState.set('idle');
  watchlistSessionEpoch.update((epoch) => epoch + 1);
}

export function setWatchlistSession(session: WatchlistSession): void {
  const current = get(watchlistSession);
  if (current?.sessionId === session.sessionId && current.network === session.network) return;
  clearWatchlistCache();
  watchlistSession.set(session);
}

/** Clears the binding and cache, even when no session had been bound yet. */
export function resetWatchlistSession(): void {
  clearWatchlistCache();
  watchlistSession.set(null);
}

export function requireWatchlistSession(): WatchlistSession {
  const session = get(watchlistSession);
  if (!session) throw new Error('Watchlist session unavailable');
  return session;
}

export function isWatchlistSessionCurrent(session: WatchlistSession): boolean {
  return get(watchlistSession) === session;
}
