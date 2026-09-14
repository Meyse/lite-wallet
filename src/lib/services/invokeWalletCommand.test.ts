import { beforeEach, describe, expect, it, vi } from 'vitest';

const invokeMock = vi.hoisted(() => vi.fn());
const forceWalletToUnlockMock = vi.hoisted(() => vi.fn(async () => undefined));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));
vi.mock('./walletLockCoordinator.js', () => ({
  ForcedWalletLockError: class ForcedWalletLockError extends Error {},
  forceWalletToUnlock: forceWalletToUnlockMock,
  isForcedWalletLockError: () => false,
}));

import { invokeSessionBoundWalletCommand, invokeWalletCommand } from './invokeWalletCommand.js';

function deferred<T>() {
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((_resolve, rejectPromise) => {
    reject = rejectPromise;
  });
  return { promise, reject };
}

describe('invokeWalletCommand', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    forceWalletToUnlockMock.mockClear();
  });

  it('does not lock a replacement wallet for an old session cancellation', async () => {
    const oldSessionRequest = deferred<void>();
    let activeWallet = 'wallet-a';
    invokeMock.mockReturnValueOnce(oldSessionRequest.promise);

    const request = invokeWalletCommand<void>('build_and_sign_generic_response');
    activeWallet = 'wallet-b';
    oldSessionRequest.reject({ type: 'WalletSessionChanged' });

    await expect(request).rejects.toEqual({ type: 'WalletSessionChanged' });
    expect(activeWallet).toBe('wallet-b');
    expect(forceWalletToUnlockMock).not.toHaveBeenCalled();
  });

  it('propagates a session-bound WalletLocked cancellation without locking the replacement wallet', async () => {
    invokeMock.mockRejectedValueOnce({ type: 'WalletLocked' });

    await expect(
      invokeSessionBoundWalletCommand<void>('resume_pending_eth_submission', {
        recovery_id: 'recovery-1',
      })
    ).rejects.toEqual({ type: 'WalletLocked' });
    expect(forceWalletToUnlockMock).not.toHaveBeenCalled();
  });
});
