import { invoke } from '@tauri-apps/api/core';
import { isWalletLockedError } from '$lib/utils/walletErrors.js';
import {
  ForcedWalletLockError,
  forceWalletToUnlock,
  isForcedWalletLockError,
} from './walletLockCoordinator.js';

export async function invokeWalletCommand<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (isForcedWalletLockError(error)) {
      throw error;
    }

    if (!isWalletLockedError(error)) {
      throw error;
    }

    await forceWalletToUnlock();
    throw new ForcedWalletLockError();
  }
}

/**
 * Invokes a command whose backend work is already bound to the captured wallet
 * session. A late WalletLocked response is cancellation of that old session;
 * it must never force the currently active replacement wallet to lock.
 */
export function invokeSessionBoundWalletCommand<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> {
  return invoke<T>(command, args);
}
