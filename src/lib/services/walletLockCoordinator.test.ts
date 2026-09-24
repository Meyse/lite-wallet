import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

const gotoMock = vi.hoisted(() => vi.fn(async () => undefined));
vi.mock('$app/navigation', () => ({ goto: gotoMock }));
const invokeMock = vi.hoisted(() => vi.fn(async () => undefined));
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { contactSession, setContactSession } from '$lib/contacts/session';
import {
  resetWatchlistSession,
  setWatchlistSession,
  watchlistEntriesStore,
  watchlistLoadState,
  watchlistSession,
} from '$lib/watchlist/session';
import type { WatchlistEntry } from '$lib/types/watchlist';
import {
  forceWalletToUnlock,
  walletLockEpoch,
  walletUnlockRedirectingStore,
} from './walletLockCoordinator';

const entry: WatchlistEntry = {
  id: 'alice',
  targetKind: 'identity',
  displayName: 'Alice@',
  address: `R${'a'.repeat(33)}`,
  systemId: null,
  createdAt: 1,
  updatedAt: 1,
};

beforeEach(() => {
  gotoMock.mockClear();
  invokeMock.mockClear();
  setContactSession(null);
  resetWatchlistSession();
});

describe('forced wallet lock session clearing', () => {
  it('clears the Watchlist binding and cache synchronously before lock navigation finishes', async () => {
    setContactSession({ sessionId: 'wallet-a', network: 'mainnet' });
    setWatchlistSession({ sessionId: 'wallet-a', network: 'mainnet' });
    watchlistEntriesStore.set([entry]);
    watchlistLoadState.set('ready');

    const unlock = forceWalletToUnlock();

    // No await: a failed or slow navigation must not leave membership cached.
    expect(get(contactSession)).toBeNull();
    expect(get(watchlistSession)).toBeNull();
    expect(get(watchlistEntriesStore)).toBeNull();
    expect(get(watchlistLoadState)).toBe('idle');
    expect(get(walletUnlockRedirectingStore)).toBe(true);

    await unlock;
    expect(gotoMock).toHaveBeenCalledWith('/', { replaceState: true });
    expect(get(walletUnlockRedirectingStore)).toBe(false);
  });

  it('clears an already unbound Watchlist instead of keeping stale membership', async () => {
    watchlistEntriesStore.set([entry]);
    watchlistLoadState.set('ready');

    const unlock = forceWalletToUnlock();

    expect(get(watchlistEntriesStore)).toBeNull();
    expect(get(watchlistLoadState)).toBe('idle');
    await unlock;
  });

  it('bumps the lock epoch on every request, including a coalesced one', async () => {
    let releaseLock!: () => void;
    invokeMock.mockImplementationOnce(
      () =>
        new Promise<undefined>((resolve) => {
          releaseLock = () => resolve(undefined);
        })
    );
    const before = get(walletLockEpoch);

    const first = forceWalletToUnlock();
    const second = forceWalletToUnlock();

    expect(get(walletLockEpoch)).toBe(before + 2);
    releaseLock();
    await Promise.all([first, second]);
  });
});
