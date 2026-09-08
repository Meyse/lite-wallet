import { beforeEach, describe, expect, it, vi } from 'vitest';

const invokeWalletCommandMock = vi.hoisted(() => vi.fn());

vi.mock('./invokeWalletCommand.js', () => ({
  invokeWalletCommand: invokeWalletCommandMock,
}));

import {
  getDisplayBalance,
  getDisplayCoinScopes,
  getDisplayTransactionHistoryPage,
  invalidateWalletDisplayScopes,
  resetWalletDisplaySession,
  WalletDisplayRequestInvalidatedError,
} from './walletDisplayService.js';

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

describe('walletDisplayService', () => {
  beforeEach(() => {
    invokeWalletCommandMock.mockReset();
    resetWalletDisplaySession();
  });

  it('coalesces and reuses session-scoped coin scopes', async () => {
    const request = deferred<{ coinId: string; scopes: [] }>();
    invokeWalletCommandMock.mockReturnValueOnce(request.promise);

    const first = getDisplayCoinScopes('VRSC');
    const second = getDisplayCoinScopes('VRSC');
    expect(invokeWalletCommandMock).toHaveBeenCalledOnce();

    request.resolve({ coinId: 'VRSC', scopes: [] });
    await expect(first).resolves.toEqual({ coinId: 'VRSC', scopes: [] });
    await expect(second).resolves.toEqual({ coinId: 'VRSC', scopes: [] });
    await expect(getDisplayCoinScopes('VRSC')).resolves.toEqual({ coinId: 'VRSC', scopes: [] });
    expect(invokeWalletCommandMock).toHaveBeenCalledOnce();
  });

  it('does not resurrect a scope result after the wallet session resets', async () => {
    const staleRequest = deferred<{ coinId: string; scopes: [] }>();
    invokeWalletCommandMock.mockReturnValueOnce(staleRequest.promise);

    const stale = getDisplayCoinScopes('VRSC');
    resetWalletDisplaySession();
    staleRequest.resolve({ coinId: 'VRSC', scopes: [] });

    await expect(stale).rejects.toBeInstanceOf(WalletDisplayRequestInvalidatedError);

    invokeWalletCommandMock.mockResolvedValueOnce({ coinId: 'VRSC', scopes: [] });
    await expect(getDisplayCoinScopes('VRSC')).resolves.toEqual({ coinId: 'VRSC', scopes: [] });
    expect(invokeWalletCommandMock).toHaveBeenCalledTimes(2);
  });

  it('does not resurrect a scope result after scope data is invalidated', async () => {
    const staleRequest = deferred<{ coinId: string; scopes: [] }>();
    invokeWalletCommandMock.mockReturnValueOnce(staleRequest.promise);

    const stale = getDisplayCoinScopes('VRSC');
    invalidateWalletDisplayScopes();
    staleRequest.resolve({ coinId: 'VRSC', scopes: [] });

    await expect(stale).rejects.toBeInstanceOf(WalletDisplayRequestInvalidatedError);
  });

  it('coalesces concurrent display balance reads', async () => {
    const request = deferred<{ confirmed: string; pending: string; total: string }>();
    invokeWalletCommandMock.mockReturnValueOnce(request.promise);

    const first = getDisplayBalance('vrpc.R.iSystem', 'VRSC');
    const second = getDisplayBalance('vrpc.R.iSystem', 'VRSC');
    request.resolve({ confirmed: '1', pending: '0', total: '1' });

    await expect(first).resolves.toEqual({ confirmed: '1', pending: '0', total: '1' });
    await expect(second).resolves.toEqual({ confirmed: '1', pending: '0', total: '1' });
    expect(invokeWalletCommandMock).toHaveBeenCalledOnce();
  });

  it('coalesces identical history pages but keeps different limits independent', async () => {
    const firstRequest = deferred<{ transactions: []; nextCursor: null; hasMore: boolean }>();
    const secondRequest = deferred<{ transactions: []; nextCursor: null; hasMore: boolean }>();
    invokeWalletCommandMock
      .mockReturnValueOnce(firstRequest.promise)
      .mockReturnValueOnce(secondRequest.promise);

    const first = getDisplayTransactionHistoryPage('vrpc.R.iSystem', 'VRSC', undefined, 50);
    const joined = getDisplayTransactionHistoryPage('vrpc.R.iSystem', 'VRSC', undefined, 50);
    const differentLimit = getDisplayTransactionHistoryPage(
      'vrpc.R.iSystem',
      'VRSC',
      undefined,
      25
    );
    expect(invokeWalletCommandMock).toHaveBeenCalledTimes(2);

    const emptyPage = { transactions: [] as [], nextCursor: null, hasMore: false };
    firstRequest.resolve(emptyPage);
    secondRequest.resolve(emptyPage);

    await expect(first).resolves.toEqual(emptyPage);
    await expect(joined).resolves.toEqual(emptyPage);
    await expect(differentLimit).resolves.toEqual(emptyPage);
  });
});
