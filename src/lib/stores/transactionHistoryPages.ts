import { writable } from 'svelte/store';
import type { Transaction } from '$lib/types/wallet.js';

export type ScopeTransactionPageState = {
  items: Transaction[];
  nextCursor: string | null;
  hasMore: boolean;
  initialLoaded: boolean;
  loadingInitial: boolean;
  loadingMore: boolean;
  error: string;
  loadMoreError: string;
};

export const transactionHistoryPagesStore = writable<Record<string, ScopeTransactionPageState>>({});

type InvalidationRetryOptions<T> = {
  load: () => Promise<T>;
  isInvalidated: (error: unknown) => boolean;
  shouldRetry: () => boolean;
};

/**
 * Keep one consumer-owned request alive across display-cache invalidations.
 * The caller's in-flight guard remains held while a replacement request starts,
 * so reactive reruns cannot strand a selected scope between the two attempts.
 */
export async function loadHistoryPageWithInvalidationRetry<T>({
  load,
  isInvalidated,
  shouldRetry,
}: InvalidationRetryOptions<T>): Promise<T | null> {
  while (true) {
    try {
      return await load();
    } catch (error) {
      if (!isInvalidated(error) || !shouldRetry()) {
        if (isInvalidated(error)) return null;
        throw error;
      }
    }
  }
}

export function createEmptyTransactionPageState(): ScopeTransactionPageState {
  return {
    items: [],
    nextCursor: null,
    hasMore: false,
    initialLoaded: false,
    loadingInitial: false,
    loadingMore: false,
    error: '',
    loadMoreError: '',
  };
}

export function updateTransactionHistoryPage(
  key: string,
  updater: (state: ScopeTransactionPageState) => ScopeTransactionPageState
): void {
  transactionHistoryPagesStore.update((pages) => ({
    ...pages,
    [key]: updater(pages[key] ?? createEmptyTransactionPageState()),
  }));
}

export function resetTransactionHistoryPages(channelId?: string, coinId?: string): void {
  if (!channelId && !coinId) {
    transactionHistoryPagesStore.set({});
    return;
  }

  transactionHistoryPagesStore.update((pages) => {
    const next: Record<string, ScopeTransactionPageState> = {};
    for (const [key, page] of Object.entries(pages)) {
      const [pageChannelId, pageCoinId] = key.split('::');
      const channelMatches = !channelId || pageChannelId === channelId;
      const coinMatches = !coinId || pageCoinId === coinId;
      if (!channelMatches || !coinMatches) {
        next[key] = page;
      }
    }
    return next;
  });
}
