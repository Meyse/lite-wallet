import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

const listeners = vi.hoisted(() => new Map<string, (event: { payload: unknown }) => void>());
const unlisteners = vi.hoisted(() => new Map<string, ReturnType<typeof vi.fn>>());
const listenMock = vi.hoisted(() => vi.fn());

vi.mock('@tauri-apps/api/event', () => ({ listen: listenMock }));

import { setupWalletEventBridge } from './eventBridge';
import { balanceStore } from '$lib/stores/balances.js';
import {
  transactionHistoryPagesStore,
  updateTransactionHistoryPage,
} from '$lib/stores/transactionHistoryPages.js';

describe('setupWalletEventBridge', () => {
  beforeEach(() => {
    listeners.clear();
    unlisteners.clear();
    listenMock.mockReset();
    listenMock.mockImplementation(
      async (eventName: string, handler: (event: { payload: unknown }) => void) => {
        const unsubscribe = vi.fn();
        listeners.set(eventName, handler);
        unlisteners.set(eventName, unsubscribe);
        return unsubscribe;
      }
    );
    balanceStore.set({});
    transactionHistoryPagesStore.set({});
  });

  it('rolls back every registered listener when later registration fails', async () => {
    listenMock.mockImplementation(
      async (eventName: string, handler: (event: { payload: unknown }) => void) => {
        if (eventName === 'wallet://rates-updated') {
          throw new Error('listener registration failed');
        }
        const unsubscribe = vi.fn();
        listeners.set(eventName, handler);
        unlisteners.set(eventName, unsubscribe);
        return unsubscribe;
      }
    );

    await expect(setupWalletEventBridge()).rejects.toThrow('listener registration failed');

    expect([...unlisteners.values()]).toHaveLength(3);
    for (const unsubscribe of unlisteners.values()) {
      expect(unsubscribe).toHaveBeenCalledOnce();
    }
  });

  it('ignores a deferred event callback after cleanup', async () => {
    const onSessionExpired = vi.fn();
    const cleanup = await setupWalletEventBridge({ onSessionExpired });
    const deferredCallback = listeners.get('wallet://session-expired');

    cleanup();
    deferredCallback?.({ payload: null });

    expect(onSessionExpired).not.toHaveBeenCalled();
    expect([...unlisteners.values()]).toHaveLength(7);
    for (const unsubscribe of unlisteners.values()) {
      expect(unsubscribe).toHaveBeenCalledOnce();
    }
  });

  it('does not turn an incomplete balance event into a zero balance', async () => {
    const cleanup = await setupWalletEventBridge();
    listeners.get('wallet://balances-updated')?.({
      payload: {
        coinId: 'VRSC',
        channel: 'vrpc.R.iSystem',
      },
    });

    expect(get(balanceStore)).toEqual({});
    cleanup();
  });

  it('invalidates cached history when a transaction update arrives', async () => {
    const key = 'vrpc.R.iSystem::VRSC';
    updateTransactionHistoryPage(key, (state) => ({
      ...state,
      initialLoaded: true,
    }));
    const cleanup = await setupWalletEventBridge();

    listeners.get('wallet://transactions-updated')?.({
      payload: {
        coinId: 'VRSC',
        channel: 'vrpc.R.iSystem',
        transactions: [],
      },
    });

    expect(get(transactionHistoryPagesStore)[key]).toBeUndefined();
    cleanup();
  });
});
