import { beforeEach, describe, expect, it, vi } from 'vitest';

const invokeMock = vi.hoisted(() => vi.fn());
const forceWalletToUnlockMock = vi.hoisted(() => vi.fn(async () => undefined));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));
vi.mock('./walletLockCoordinator.js', () => ({
  ForcedWalletLockError: class ForcedWalletLockError extends Error {},
  forceWalletToUnlock: forceWalletToUnlockMock,
  isForcedWalletLockError: () => false,
}));

import {
  addWatchlistEntry,
  getWatchlistEntries,
  refreshWatchlist,
  removeWatchlistEntry,
  resolveWatchlistTarget,
} from './watchlistService.js';

function deferred() {
  let reject!: (error: unknown) => void;
  const promise = new Promise<unknown>((_resolve, rejectPromise) => {
    reject = rejectPromise;
  });
  return { promise, reject };
}

describe('Watchlist session-bound invokes', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    forceWalletToUnlockMock.mockClear();
  });

  it.each([
    ['get_watchlist_entries', () => getWatchlistEntries()],
    ['resolve_watchlist_target', () => resolveWatchlistTarget('alice@')],
    ['add_watchlist_entry', () => addWatchlistEntry('RAddress', 'Local name')],
    ['remove_watchlist_entry', () => removeWatchlistEntry('entry-1')],
    ['refresh_watchlist', () => refreshWatchlist()],
  ])('keeps a replacement wallet open when %s rejects late', async (command, start) => {
    const oldRequest = deferred();
    invokeMock.mockReturnValueOnce(oldRequest.promise);

    let activeWallet = 'wallet-a';
    const request = start();
    activeWallet = 'wallet-b';
    oldRequest.reject({ type: 'WalletLocked' });

    await expect(request).rejects.toEqual({ type: 'WalletLocked' });
    expect(invokeMock.mock.calls[0]?.[0]).toBe(command);
    expect(activeWallet).toBe('wallet-b');
    expect(forceWalletToUnlockMock).not.toHaveBeenCalled();
  });
});
