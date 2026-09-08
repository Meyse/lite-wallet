import { describe, expect, it, vi } from 'vitest';
import { loadHistoryPageWithInvalidationRetry } from './transactionHistoryPages.js';

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
