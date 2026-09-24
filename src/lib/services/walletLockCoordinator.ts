import { goto } from '$app/navigation';
import { invoke } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';
import { setContactSession } from '$lib/contacts/session';
import { resetWatchlistSession } from '$lib/watchlist/session';

export class ForcedWalletLockError extends Error {
  constructor() {
    super('');
    this.name = 'ForcedWalletLockError';
  }
}

export const walletUnlockRedirectingStore = writable(false);
/**
 * Bumped on every forced-lock request, including coalesced ones. Async work
 * that started earlier must compare this before applying a result, because the
 * redirecting flag can return to false after a failed navigation while the
 * route is still mounted.
 */
export const walletLockEpoch = writable(0);

let forcedWalletUnlockPromise: Promise<void> | null = null;

export function isForcedWalletLockError(error: unknown): error is ForcedWalletLockError {
  return error instanceof ForcedWalletLockError;
}

export async function forceWalletToUnlock(): Promise<void> {
  walletLockEpoch.update((epoch) => epoch + 1);
  if (forcedWalletUnlockPromise) {
    return forcedWalletUnlockPromise;
  }

  setContactSession(null);
  resetWatchlistSession();
  walletUnlockRedirectingStore.set(true);

  forcedWalletUnlockPromise = (async () => {
    try {
      await invoke('lock_wallet');
    } catch {
      // Ignore backend lock failures and continue forcing the frontend to unlock.
    }

    try {
      await goto('/', { replaceState: true });
    } catch (error) {
      console.error('[WALLET_LOCK] Failed to navigate to unlock screen', error);
    }
  })().finally(() => {
    forcedWalletUnlockPromise = null;
    walletUnlockRedirectingStore.set(false);
  });

  return forcedWalletUnlockPromise;
}
