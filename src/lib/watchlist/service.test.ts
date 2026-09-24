import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import type {
  WatchlistEntry,
  WatchlistEntrySnapshot,
  WatchlistRefreshResult,
} from '$lib/types/watchlist.js';
import {
  addWatchlistEntry,
  loadWatchlistEntries,
  refreshWatchlist,
  removeWatchlistEntry,
} from './service.js';
import {
  resetWatchlistSession,
  setWatchlistSession,
  watchlistEntriesStore,
  watchlistLoadState,
  watchlistSession,
  watchlistSessionEpoch,
} from './session.js';

const service = vi.hoisted(() => ({
  getWatchlistEntries: vi.fn(),
  resolveWatchlistTarget: vi.fn(),
  addWatchlistEntry: vi.fn(),
  removeWatchlistEntry: vi.fn(),
  refreshWatchlist: vi.fn(),
}));
vi.mock('$lib/services/watchlistService.js', () => service);

const entry: WatchlistEntry = {
  id: 'alice',
  targetKind: 'identity',
  displayName: 'Alice@',
  address: `R${'a'.repeat(33)}`,
  systemId: null,
  createdAt: 1,
  updatedAt: 1,
};

const otherEntry: WatchlistEntry = {
  ...entry,
  id: 'bob',
  displayName: 'Bob@',
  address: `R${'b'.repeat(33)}`,
  createdAt: 2,
  updatedAt: 2,
};

const snapshot: WatchlistEntrySnapshot = {
  entry,
  holdings: [],
  sources: [],
  availability: 'available',
  refreshedAt: 2,
};

const otherSnapshot: WatchlistEntrySnapshot = { ...snapshot, entry: otherEntry };

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

beforeEach(() => {
  vi.clearAllMocks();
  resetWatchlistSession();
  setWatchlistSession({ sessionId: 'session-a', network: 'mainnet' });
});

afterEach(() => {
  resetWatchlistSession();
});

describe('session-scoped Watchlist membership', () => {
  it('loads membership once per session and reuses the cache', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry]);

    await expect(loadWatchlistEntries()).resolves.toEqual([entry]);
    await expect(loadWatchlistEntries()).resolves.toEqual([entry]);

    expect(service.getWatchlistEntries).toHaveBeenCalledTimes(1);
    expect(get(watchlistEntriesStore)).toEqual([entry]);
    expect(get(watchlistLoadState)).toBe('ready');
  });

  it('keeps a failed read distinct from a confirmed empty watchlist', async () => {
    service.getWatchlistEntries.mockRejectedValue(new Error('Storage unreadable'));

    await expect(loadWatchlistEntries()).rejects.toThrow('Storage unreadable');

    expect(get(watchlistEntriesStore)).toBeNull();
    expect(get(watchlistLoadState)).toBe('error');
  });

  it('keeps the last known membership when a later read fails', async () => {
    service.getWatchlistEntries.mockResolvedValueOnce([entry]);
    await loadWatchlistEntries();

    service.getWatchlistEntries.mockRejectedValueOnce(new Error('offline'));
    await expect(loadWatchlistEntries(true)).rejects.toThrow('offline');

    expect(get(watchlistEntriesStore)).toEqual([entry]);
    expect(get(watchlistLoadState)).toBe('error');
  });

  it('publishes an add only after durable success and wins against a slower membership read', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry]);
    await loadWatchlistEntries();

    const write = deferred<WatchlistEntrySnapshot>();
    service.addWatchlistEntry.mockReturnValue(write.promise);
    const addRequest = addWatchlistEntry('Bob@');
    await vi.waitFor(() => expect(service.addWatchlistEntry).toHaveBeenCalled());

    const slowRead = deferred<WatchlistEntry[]>();
    service.getWatchlistEntries.mockReturnValueOnce(slowRead.promise);
    const readRequest = loadWatchlistEntries(true);
    expect(get(watchlistEntriesStore)).toEqual([entry]);

    write.resolve(otherSnapshot);
    await expect(addRequest).resolves.toBe(otherSnapshot);
    expect(get(watchlistEntriesStore)).toEqual([otherEntry, entry]);

    slowRead.resolve([entry]);
    await readRequest;
    expect(get(watchlistEntriesStore)).toEqual([otherEntry, entry]);
  });

  it('coalesces concurrent identical adds and reports failures without touching membership', async () => {
    service.getWatchlistEntries.mockResolvedValue([]);
    const write = deferred<WatchlistEntrySnapshot>();
    service.addWatchlistEntry.mockReturnValue(write.promise);

    const first = addWatchlistEntry('Bob@');
    const second = addWatchlistEntry('Bob@');
    expect(second).toBe(first);
    await vi.waitFor(() => expect(service.addWatchlistEntry).toHaveBeenCalled());

    write.reject(new Error('WatchlistInvalidInput'));
    await expect(first).rejects.toThrow('WatchlistInvalidInput');
    expect(service.addWatchlistEntry).toHaveBeenCalledTimes(1);
    expect(get(watchlistEntriesStore)).toEqual([]);
  });

  it('publishes a removal only after the durable removal succeeds', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry, otherEntry]);
    await loadWatchlistEntries();

    const removal = deferred<boolean>();
    service.removeWatchlistEntry.mockReturnValue(removal.promise);
    const request = removeWatchlistEntry('alice');
    await vi.waitFor(() => expect(service.removeWatchlistEntry).toHaveBeenCalledWith('alice'));
    expect(get(watchlistEntriesStore)).toEqual([entry, otherEntry]);

    removal.resolve(true);
    await expect(request).resolves.toBe(true);
    expect(get(watchlistEntriesStore)).toEqual([otherEntry]);
  });

  it('keeps membership when the backend reports the removal was not durable', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry]);
    await loadWatchlistEntries();
    service.removeWatchlistEntry.mockResolvedValue(false);

    await expect(removeWatchlistEntry('alice')).resolves.toBe(false);
    expect(get(watchlistEntriesStore)).toEqual([entry]);
  });

  it('shares an in-flight balance refresh with the membership it actually started from', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry]);
    await loadWatchlistEntries();

    const refresh = deferred<WatchlistRefreshResult>();
    service.refreshWatchlist.mockReturnValue(refresh.promise);

    const first = refreshWatchlist();
    const second = refreshWatchlist();
    expect(second).toBe(first);

    // Bob is added while the shared request is in flight; joiners must keep the
    // original request scope instead of capturing the newer membership.
    service.addWatchlistEntry.mockResolvedValue(otherSnapshot);
    await addWatchlistEntry('Bob@');
    expect(refreshWatchlist()).toBe(first);

    const result: WatchlistRefreshResult = {
      network: 'mainnet',
      entries: [snapshot],
      refreshedAt: 2,
    };
    refresh.resolve(result);
    const envelope = await first;
    expect(envelope.requestedEntryIds).toEqual([entry.id]);
    expect(envelope.result).toMatchObject({ entries: [snapshot] });
    expect(service.refreshWatchlist).toHaveBeenCalledTimes(1);
  });

  it('discards reads, mutations and refreshes from a replaced session', async () => {
    const staleRead = deferred<WatchlistEntry[]>();
    service.getWatchlistEntries.mockReturnValueOnce(staleRead.promise).mockResolvedValue([]);
    const readRequest = loadWatchlistEntries();
    setWatchlistSession({ sessionId: 'session-b', network: 'testnet' });
    staleRead.resolve([entry]);
    await expect(readRequest).rejects.toThrow('Obsolete');
    expect(get(watchlistEntriesStore)).toBeNull();
    expect(get(watchlistLoadState)).toBe('idle');

    const staleSave = deferred<WatchlistEntrySnapshot>();
    service.addWatchlistEntry.mockReturnValueOnce(staleSave.promise);
    const addRequest = addWatchlistEntry('Bob@');
    await vi.waitFor(() => expect(service.addWatchlistEntry).toHaveBeenCalled());
    setWatchlistSession({ sessionId: 'session-c', network: 'mainnet' });
    staleSave.resolve(otherSnapshot);
    await expect(addRequest).rejects.toThrow('Obsolete');
    expect(get(watchlistEntriesStore)).toBeNull();

    const staleRefresh = deferred<WatchlistRefreshResult>();
    service.refreshWatchlist.mockReturnValueOnce(staleRefresh.promise);
    const refreshRequest = refreshWatchlist();
    setWatchlistSession({ sessionId: 'session-d', network: 'testnet' });
    staleRefresh.resolve({ network: 'testnet', entries: [snapshot], refreshedAt: 2 });
    await expect(refreshRequest).rejects.toThrow('Obsolete');
  });

  it('refuses every operation without a bound session and never caches unbound membership', async () => {
    resetWatchlistSession();

    expect(() => loadWatchlistEntries()).toThrow('Watchlist session unavailable');
    expect(() => refreshWatchlist()).toThrow('Watchlist session unavailable');
    expect(() => addWatchlistEntry('Bob@')).toThrow('Watchlist session unavailable');
    expect(() => removeWatchlistEntry('alice')).toThrow('Watchlist session unavailable');

    expect(service.getWatchlistEntries).not.toHaveBeenCalled();
    expect(service.refreshWatchlist).not.toHaveBeenCalled();
    expect(get(watchlistEntriesStore)).toBeNull();
    expect(get(watchlistLoadState)).toBe('idle');
  });

  it('clears and invalidates the cache on every reset, including repeated unbound resets', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry]);
    await loadWatchlistEntries();
    expect(get(watchlistEntriesStore)).toEqual([entry]);

    resetWatchlistSession();
    const epochAfterFirstReset = get(watchlistSessionEpoch);
    resetWatchlistSession();

    expect(get(watchlistSessionEpoch)).toBe(epochAfterFirstReset + 1);
    expect(get(watchlistSession)).toBeNull();
    expect(get(watchlistEntriesStore)).toBeNull();
    expect(get(watchlistLoadState)).toBe('idle');

    // Rebinding the same wallet session must start from unknown membership, not
    // from the previous wallet session's cache.
    service.getWatchlistEntries.mockResolvedValue([otherEntry]);
    setWatchlistSession({ sessionId: 'session-a', network: 'mainnet' });
    expect(get(watchlistEntriesStore)).toBeNull();
    await expect(loadWatchlistEntries()).resolves.toEqual([otherEntry]);
    expect(service.getWatchlistEntries).toHaveBeenCalledTimes(2);
  });

  it('discards in-flight work when the session is reset before it resolves', async () => {
    const staleRead = deferred<WatchlistEntry[]>();
    service.getWatchlistEntries.mockReturnValueOnce(staleRead.promise);
    const readRequest = loadWatchlistEntries();

    resetWatchlistSession();
    resetWatchlistSession();
    staleRead.resolve([entry]);

    await expect(readRequest).rejects.toThrow('Obsolete');
    expect(get(watchlistEntriesStore)).toBeNull();
    expect(get(watchlistLoadState)).toBe('idle');
  });
});
