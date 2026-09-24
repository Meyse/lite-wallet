import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

const service = vi.hoisted(() => ({ getActiveWallet: vi.fn() }));
vi.mock('./walletService.js', () => service);

import { contactSession, setContactSession } from '$lib/contacts/session';
import {
  resetWatchlistSession,
  watchlistSession,
  watchlistSessionEpoch,
} from '$lib/watchlist/session';
import { walletLockEpoch } from './walletLockCoordinator.js';
import { bindRecoveredWalletSession, recoverActiveWalletSession } from './walletSessionRecovery.js';

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

const activeWallet = {
  wallet_name: 'Wallet A',
  network: 'testnet' as const,
  emoji: '🦊',
  color: 'green',
  session_id: 'session-a',
};

beforeEach(() => {
  service.getActiveWallet.mockReset();
  setContactSession(null);
  resetWatchlistSession();
});

describe('wallet session recovery', () => {
  it('returns verified wallet metadata without binding until the caller applies it', async () => {
    service.getActiveWallet.mockResolvedValue({
      wallet_name: 'Wallet A',
      network: 'testnet',
      emoji: '🦊',
      color: 'green',
      session_id: 'session-a',
    });

    const recovered = await recoverActiveWalletSession();

    expect(recovered).toEqual({
      name: 'Wallet A',
      emoji: '🦊',
      color: 'green',
      network: 'testnet',
      sessionId: 'session-a',
    });
    expect(get(contactSession)).toBeNull();
    expect(get(watchlistSession)).toBeNull();
  });

  it('binds both route sessions only through the explicit bind step', () => {
    bindRecoveredWalletSession({
      name: 'Wallet A',
      emoji: '💰',
      color: 'blue',
      network: 'mainnet',
      sessionId: 'session-a',
    });

    expect(get(contactSession)).toEqual({ sessionId: 'session-a', network: 'mainnet' });
    expect(get(watchlistSession)).toEqual({ sessionId: 'session-a', network: 'mainnet' });
  });

  it('returns null and never binds a placeholder when no wallet is active', async () => {
    service.getActiveWallet.mockResolvedValue(null);

    expect(await recoverActiveWalletSession()).toBeNull();
    expect(get(contactSession)).toBeNull();
    expect(get(watchlistSession)).toBeNull();
  });

  it('propagates read failures without binding anything', async () => {
    service.getActiveWallet.mockRejectedValue(new Error('WalletLocked'));

    await expect(recoverActiveWalletSession()).rejects.toThrow('WalletLocked');
    expect(get(contactSession)).toBeNull();
    expect(get(watchlistSession)).toBeNull();
  });

  it('applies default emoji, color and network when the wallet omits them', async () => {
    service.getActiveWallet.mockResolvedValue({
      wallet_name: 'Wallet B',
      network: undefined,
      emoji: '',
      color: '',
      session_id: 'session-b',
    });

    await expect(recoverActiveWalletSession()).resolves.toEqual({
      name: 'Wallet B',
      emoji: '💰',
      color: 'blue',
      network: 'mainnet',
      sessionId: 'session-b',
    });
  });

  it('discards a verified wallet when a forced lock happens while reading', async () => {
    const read = deferred<typeof activeWallet>();
    service.getActiveWallet.mockReturnValue(read.promise);

    const request = recoverActiveWalletSession();
    walletLockEpoch.update((epoch) => epoch + 1);
    read.resolve(activeWallet);

    await expect(request).resolves.toBeNull();
    expect(get(contactSession)).toBeNull();
    expect(get(watchlistSession)).toBeNull();
  });

  it('discards a verified wallet when the Watchlist session resets while reading', async () => {
    const read = deferred<typeof activeWallet>();
    service.getActiveWallet.mockReturnValue(read.promise);

    const request = recoverActiveWalletSession();
    const epochBeforeReset = get(watchlistSessionEpoch);
    resetWatchlistSession();
    expect(get(watchlistSessionEpoch)).toBe(epochBeforeReset + 1);
    read.resolve(activeWallet);

    await expect(request).resolves.toBeNull();
    expect(get(contactSession)).toBeNull();
    expect(get(watchlistSession)).toBeNull();
  });
});
