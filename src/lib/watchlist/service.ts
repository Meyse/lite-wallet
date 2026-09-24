import { get } from 'svelte/store';
import * as watchlistService from '$lib/services/watchlistService.js';
import type {
  WatchlistEntry,
  WatchlistEntrySnapshot,
  WatchlistRefreshResult,
} from '$lib/types/watchlist.js';
import {
  isWatchlistSessionCurrent,
  requireWatchlistSession,
  watchlistEntriesStore,
  watchlistLoadState,
  watchlistSessionEpoch,
} from './session';

let loading: Promise<WatchlistEntry[]> | null = null;
let refreshRequest: Promise<WatchlistRefreshEnvelope> | null = null;
let saves = new Map<string, Promise<WatchlistEntrySnapshot>>();
let removals = new Map<string, Promise<boolean>>();
/**
 * Bumped on every session change and durable mutation. A membership read that
 * started before a newer mutation must not replace the mutation's result.
 */
let revision = 0;

watchlistSessionEpoch.subscribe(() => {
  loading = null;
  refreshRequest = null;
  saves = new Map();
  removals = new Map();
  revision++;
});

function obsolete(operation: string): Error {
  return new Error(`Obsolete watchlist ${operation}`);
}

function upsertEntry(entry: WatchlistEntry): void {
  const entries = get(watchlistEntriesStore);
  if (!entries) return;
  const index = entries.findIndex((existing) => existing.id === entry.id);
  if (index === -1) {
    watchlistEntriesStore.set([entry, ...entries]);
    return;
  }
  watchlistEntriesStore.set(entries.map((existing, at) => (at === index ? entry : existing)));
}

function removeEntry(entryId: string): void {
  const entries = get(watchlistEntriesStore);
  if (!entries) return;
  watchlistEntriesStore.set(entries.filter((entry) => entry.id !== entryId));
}

/**
 * Reads Watchlist membership once per session and publishes it only while it
 * is still current. A failed read keeps the last known list and marks the
 * state as `error` instead of becoming empty.
 */
export function loadWatchlistEntries(refresh = false): Promise<WatchlistEntry[]> {
  const session = requireWatchlistSession();
  if (loading) return loading;
  if (!refresh && get(watchlistLoadState) === 'ready') {
    return Promise.resolve(get(watchlistEntriesStore) ?? []);
  }
  const version = revision;
  watchlistLoadState.set('loading');
  const request = watchlistService
    .getWatchlistEntries()
    .then((entries) => {
      if (!isWatchlistSessionCurrent(session)) throw obsolete('membership request');
      // A durable mutation that landed while this read was in flight wins.
      if (version === revision) watchlistEntriesStore.set([...entries]);
      watchlistLoadState.set('ready');
      return get(watchlistEntriesStore) ?? [];
    })
    .catch((error: unknown) => {
      if (isWatchlistSessionCurrent(session)) watchlistLoadState.set('error');
      throw error;
    })
    .finally(() => {
      if (loading === request) loading = null;
    });
  loading = request;
  return request;
}

/**
 * A shared balance refresh plus the membership that existed when the request
 * was actually created. A component that joins an older request must settle
 * only the entries that request covered; entries added later stay pending and
 * are picked up by a follow-up refresh.
 */
export interface WatchlistRefreshEnvelope {
  result: WatchlistRefreshResult;
  requestedEntryIds: string[];
}

/**
 * Refreshes public balances for the current membership. Concurrent callers
 * during one visit share the same request, and a result from a replaced
 * session is rejected instead of being displayed.
 */
export function refreshWatchlist(): Promise<WatchlistRefreshEnvelope> {
  const session = requireWatchlistSession();
  if (refreshRequest) return refreshRequest;
  const requestedEntryIds = (get(watchlistEntriesStore) ?? []).map((entry) => entry.id);
  const request = watchlistService
    .refreshWatchlist()
    .then((result) => {
      if (!isWatchlistSessionCurrent(session)) throw obsolete('refresh request');
      return { result, requestedEntryIds };
    })
    .finally(() => {
      if (refreshRequest === request) refreshRequest = null;
    });
  refreshRequest = request;
  return request;
}

/**
 * Adds a Watchlist entry and publishes it only after the durable save
 * succeeds. Membership is reconciled first so the added entry cannot lose a
 * race against a slower membership read.
 */
export function addWatchlistEntry(query: string, name?: string): Promise<WatchlistEntrySnapshot> {
  const session = requireWatchlistSession();
  const key = `${query.trim()}::${name?.trim() ?? ''}`;
  const existing = saves.get(key);
  if (existing) return existing;
  const request = (async () => {
    await loadWatchlistEntries();
    if (!isWatchlistSessionCurrent(session)) throw obsolete('add request');
    const snapshot = await watchlistService.addWatchlistEntry(query, name);
    if (!isWatchlistSessionCurrent(session)) throw obsolete('add request');
    revision++;
    upsertEntry(snapshot.entry);
    return snapshot;
  })().finally(() => {
    if (saves.get(key) === request) saves.delete(key);
  });
  saves.set(key, request);
  return request;
}

/**
 * Removes a Watchlist entry and drops it from the cached membership only after
 * the durable removal succeeds.
 */
export function removeWatchlistEntry(entryId: string): Promise<boolean> {
  const session = requireWatchlistSession();
  const existing = removals.get(entryId);
  if (existing) return existing;
  const request = (async () => {
    await loadWatchlistEntries();
    if (!isWatchlistSessionCurrent(session)) throw obsolete('remove request');
    const removed = await watchlistService.removeWatchlistEntry(entryId);
    if (!isWatchlistSessionCurrent(session)) throw obsolete('remove request');
    if (removed) {
      revision++;
      removeEntry(entryId);
    }
    return removed;
  })().finally(() => {
    if (removals.get(entryId) === request) removals.delete(entryId);
  });
  removals.set(entryId, request);
  return request;
}
