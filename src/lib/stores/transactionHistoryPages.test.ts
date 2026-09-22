import { describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import {
  createEmptyTransactionPageState,
  invalidateTransactionHistoryPages,
  loadHistoryPageWithInvalidationRetry,
  resetTransactionHistoryPages,
  transactionHistoryPagesStore,
} from './transactionHistoryPages.js';

class InvalidatedRequestError extends Error {}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

describe('loadHistoryPageWithInvalidationRetry', () => {
  it('retains current-session rows on scoped invalidation but clears them on session reset', () => {
    const page = {
      ...createEmptyTransactionPageState(),
      initialLoaded: true,
      hasMore: true,
      nextCursor: 'older',
      items: [
        {
          txid: 'existing',
          amount: '1',
          fromAddress: 'A',
          toAddress: 'B',
          confirmations: 1,
          pending: false,
        },
      ],
    };
    transactionHistoryPagesStore.set({ 'eth.one::ETH': page, 'eth.two::ETH': page });
    invalidateTransactionHistoryPages('eth.one', 'ETH');
    expect(get(transactionHistoryPagesStore)['eth.one::ETH']).toEqual({
      ...createEmptyTransactionPageState(),
      items: page.items,
    });
    expect(get(transactionHistoryPagesStore)['eth.two::ETH']).toEqual(page);
    resetTransactionHistoryPages();
    expect(get(transactionHistoryPagesStore)).toEqual({});
  });
  it('replaces an invalidated deferred initial page while the scope remains selected', async () => {
    const first = deferred<string>();
    const replacement = deferred<string>();
    const load = vi
      .fn()
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(replacement.promise);

    const result = loadHistoryPageWithInvalidationRetry({
      load,
      isInvalidated: (error) => error instanceof InvalidatedRequestError,
      shouldRetry: () => true,
    });
    expect(load).toHaveBeenCalledOnce();

    first.reject(new InvalidatedRequestError());
    await vi.waitFor(() => expect(load).toHaveBeenCalledTimes(2));
    replacement.resolve('replacement page');

    await expect(result).resolves.toBe('replacement page');
  });
});
