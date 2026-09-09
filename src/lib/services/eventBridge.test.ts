import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

const listeners = vi.hoisted(() => new Map<string, (event: { payload: unknown }) => void>());
const unlisteners = vi.hoisted(() => new Map<string, ReturnType<typeof vi.fn>>());
const listenMock = vi.hoisted(() => vi.fn());

vi.mock('@tauri-apps/api/event', () => ({ listen: listenMock }));

import { setupWalletEventBridge } from './eventBridge';
import { balanceStore } from '$lib/stores/balances.js';
import {
  clearWalletErrors,
  dismissWalletError,
  pushWalletError,
  walletErrorsStore,
} from '$lib/stores/walletErrors.js';
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
    clearWalletErrors();
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

  it('keeps update diagnostics in history while exposing structured presentation metadata', async () => {
    const cleanup = await setupWalletEventBridge();
    listeners.get('wallet://error')?.({
      payload: {
        dataType: 'balance',
        coinId: 'VRSC',
        channel: 'vrpc.RPublic.iSystem',
        message: 'Network error',
      },
    });

    const errors = get(walletErrorsStore);
    expect(errors.latest?.presentation).toEqual({
      kind: 'background-update',
      dataType: 'balance',
      coinId: 'VRSC',
      channel: 'vrpc.RPublic.iSystem',
    });
    expect(errors.history[0]).toContain('balance (vrpc.RPublic.iSystem): Network error');
    cleanup();
  });

  it('clears only the matching background alert after a successful refresh', async () => {
    const cleanup = await setupWalletEventBridge();
    const balanceSuccess = {
      coinId: 'VRSC',
      channel: 'vrpc.RPublic.iSystem',
      confirmed: '1',
      pending: '0',
      total: '1',
    };

    listeners.get('wallet://error')?.({
      payload: {
        dataType: 'balance',
        coinId: balanceSuccess.coinId,
        channel: balanceSuccess.channel,
        message: 'Network error',
      },
    });
    listeners.get('wallet://balances-updated')?.({ payload: balanceSuccess });
    expect(get(walletErrorsStore).latest).toBeNull();

    pushWalletError('Keep this actionable message');
    listeners.get('wallet://balances-updated')?.({ payload: balanceSuccess });
    expect(get(walletErrorsStore).latest?.presentation).toEqual({
      kind: 'message',
      message: 'Keep this actionable message',
    });
    cleanup();
  });

  it('tracks same-channel failures per asset and preserves an unrelated active failure', async () => {
    const cleanup = await setupWalletEventBridge();
    const channel = 'vrpc.RPublic.iSystem';
    const failBalance = (coinId: string) =>
      listeners.get('wallet://error')?.({
        payload: {
          dataType: 'balance',
          coinId,
          channel,
          message: 'Network error',
        },
      });
    const recoverBalance = (coinId: string) =>
      listeners.get('wallet://balances-updated')?.({
        payload: {
          coinId,
          channel,
          confirmed: '1',
          pending: '0',
          total: '1',
        },
      });

    failBalance('VRSC');
    failBalance('PURE');
    expect(get(walletErrorsStore).history).toHaveLength(2);
    expect(get(walletErrorsStore).latest?.presentation).toMatchObject({ coinId: 'PURE' });

    recoverBalance('VRSC');
    expect(get(walletErrorsStore).latest?.presentation).toMatchObject({ coinId: 'PURE' });

    failBalance('VRSC');
    expect(get(walletErrorsStore).history).toHaveLength(3);
    expect(get(walletErrorsStore).latest?.presentation).toMatchObject({ coinId: 'VRSC' });
    cleanup();
  });

  it('reveals the remaining failure when the latest asset recovers, then clears after all recover', async () => {
    const cleanup = await setupWalletEventBridge();
    const channel = 'vrpc.RPublic.iSystem';
    const failBalance = (coinId: string) =>
      listeners.get('wallet://error')?.({
        payload: { dataType: 'balance', coinId, channel, message: 'Network error' },
      });
    const recoverBalance = (coinId: string) =>
      listeners.get('wallet://balances-updated')?.({
        payload: {
          coinId,
          channel,
          confirmed: '1',
          pending: '0',
          total: '1',
        },
      });

    failBalance('VRSC');
    failBalance('PURE');
    recoverBalance('PURE');
    expect(get(walletErrorsStore).latest?.presentation).toMatchObject({ coinId: 'VRSC' });

    recoverBalance('VRSC');
    expect(get(walletErrorsStore).latest).toBeNull();
    cleanup();
  });

  it('does not cycle through pending background failures after explicit dismissal', async () => {
    const cleanup = await setupWalletEventBridge();
    const channel = 'vrpc.RPublic.iSystem';
    for (const coinId of ['VRSC', 'PURE']) {
      listeners.get('wallet://error')?.({
        payload: { dataType: 'balance', coinId, channel, message: 'Network error' },
      });
    }

    dismissWalletError();
    expect(get(walletErrorsStore).latest).toBeNull();

    listeners.get('wallet://balances-updated')?.({
      payload: {
        coinId: 'PURE',
        channel,
        confirmed: '1',
        pending: '0',
        total: '1',
      },
    });
    expect(get(walletErrorsStore).latest).toBeNull();
    cleanup();
  });
});
