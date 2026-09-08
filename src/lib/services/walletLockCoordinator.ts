import { goto } from '$app/navigation';
import { invoke } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';

export class ForcedWalletLockError extends Error {
  constructor() {
    super('');
    this.name = 'ForcedWalletLockError';
  }
}

export const walletUnlockRedirectingStore = writable(false);

let forcedWalletUnlockPromise: Promise<void> | null = null;

export function isForcedWalletLockError(error: unknown): error is ForcedWalletLockError {
  return error instanceof ForcedWalletLockError;
}

export async function forceWalletToUnlock(): Promise<void> {
  if (forcedWalletUnlockPromise) {
    return forcedWalletUnlockPromise;
  }

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
