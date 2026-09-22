import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '$lib/i18n';
import { coinsStore } from '$lib/stores/coins.js';
import { balanceStore } from '$lib/stores/balances.js';
import { clearCoinScopes, selectedAddressByCoinId } from '$lib/stores/coinScopes.js';
import {
  invalidateTransactionHistoryPages,
  resetTransactionHistoryPages,
} from '$lib/stores/transactionHistoryPages.js';
import type { CoinDefinition, CoinScopesResult } from '$lib/types/wallet.js';
import AssetDetails from './AssetDetails.svelte';
import AssetDetailsHarness from './__tests__/AssetDetailsHarness.svelte';

const service = vi.hoisted(() => ({
  getDisplayCoinScopes: vi.fn(),
  getDisplayBalance: vi.fn(),
  getDisplayTransactionHistoryPage: vi.fn(),
  isWalletDisplayRequestInvalidated: () => false,
}));
vi.mock('$lib/services/walletDisplayService.js', () => service);
vi.mock('$lib/services/walletLockCoordinator.js', () => ({ isForcedWalletLockError: () => false }));
vi.stubGlobal(
  'ResizeObserver',
  class {
    observe() {}
    disconnect() {}
  }
);

const coin: CoinDefinition = {
  id: 'ETH',
  currencyId: 'ETH',
  systemId: 'ETH',
  displayName: 'Ethereum',
  displayTicker: 'ETH',
  proto: 'eth',
  compatibleChannels: ['eth'],
  decimals: 18,
  isTestnet: false,
  secondsPerBlock: 12,
  vrpcEndpoints: [],
};
const scopes: CoinScopesResult = {
  coinId: 'ETH',
  scopes: [
    {
      coinId: 'ETH',
      channelId: 'eth.fixture',
      systemId: 'ETH',
      systemTicker: 'ETH',
      systemDisplayName: 'Ethereum',
      address: '0x1111111111111111111111111111111111111111',
      addressLabel: 'My address',
      isPrimaryAddress: true,
      isReadOnly: false,
      scopeKind: 'transparent',
    },
  ],
};
let component: ReturnType<typeof mount> | undefined;
async function settle() {
  for (let i = 0; i < 6; i++) {
    await tick();
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
}
async function render() {
  const target = document.createElement('div');
  document.body.append(target);
  component = mount(AssetDetails, { target, props: { coinId: 'ETH' } });
  await settle();
}

describe('asset detail loading', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    setLocale('en');
    coinsStore.set([coin]);
    balanceStore.set({});
    clearCoinScopes();
    resetTransactionHistoryPages();
    service.getDisplayCoinScopes.mockResolvedValue(scopes);
    service.getDisplayBalance.mockResolvedValue({ total: '2', confirmed: '2', pending: '0' });
    service.getDisplayTransactionHistoryPage.mockResolvedValue({
      transactions: [],
      hasMore: false,
      nextCursor: null,
    });
  });
  afterEach(async () => {
    if (component) await unmount(component);
    component = undefined;
    document.body.replaceChildren();
    document.documentElement.classList.remove('dark');
  });

  it('keeps the asset header and controls in place while scopes and then history arrive', async () => {
    selectedAddressByCoinId.set({ ETH: 'previous-wallet-address' });
    let finishScopes!: (value: CoinScopesResult) => void;
    let finishHistory!: (value: unknown) => void;
    service.getDisplayCoinScopes.mockReturnValue(
      new Promise((resolve) => {
        finishScopes = resolve;
      })
    );
    service.getDisplayTransactionHistoryPage.mockReturnValue(
      new Promise((resolve) => {
        finishHistory = resolve;
      })
    );
    await render();
    expect(document.body.textContent).toContain('Ethereum');
    expect(document.body.textContent).not.toContain('previous-wallet-address');
    expect(document.querySelector<HTMLButtonElement>('button[aria-label="Copy"]')?.disabled).toBe(
      true
    );
    expect(document.querySelector('[data-transaction-loading]')).not.toBeNull();
    expect(document.querySelector<HTMLButtonElement>('button[aria-label="Send"]')?.disabled).toBe(
      true
    );
    expect(document.body.textContent).not.toContain('read-only');
    finishScopes(scopes);
    await settle();
    expect(document.body.textContent).toContain('2');
    expect(document.querySelector<HTMLButtonElement>('button[aria-label="Send"]')?.disabled).toBe(
      false
    );
    expect(document.querySelector('[data-transaction-loading]')).not.toBeNull();
    expect(document.body.textContent).not.toContain('No transactions');
    finishHistory({ transactions: [], hasMore: false, nextCursor: null });
    await settle();
    expect(document.body.textContent).toContain('No transactions');
    expect(document.querySelector('[data-slot="skeleton"]')).toBeNull();
  });

  it.each([false, true])(
    'ends a failed balance read with recovery, preserving cached values: %s',
    async (cached) => {
      if (cached)
        balanceStore.set({ 'eth.fixture': { ETH: { total: '7', confirmed: '7', pending: '0' } } });
      service.getDisplayBalance.mockRejectedValueOnce(new Error('Offline'));
      await render();
      expect(document.body.textContent).toContain(
        cached ? 'Some balances may be out of date.' : 'Some balances are unavailable.'
      );
      expect(document.querySelector('[data-slot="skeleton"]')).toBeNull();
      if (cached) expect(document.body.textContent).toContain('7.00');
      const retry = [...document.querySelectorAll('button')].find(
        (button) => button.textContent?.trim() === 'Try again'
      );
      retry?.click();
      await settle();
      expect(document.body.textContent).toContain('2.00');
      expect(document.body.textContent).not.toContain('Some balances');
      expect(service.getDisplayBalance).toHaveBeenCalledTimes(2);
    }
  );

  it('keeps a known balance visible during refresh and clears stale feedback on a newer event', async () => {
    const known = { total: '7', confirmed: '7', pending: '0' };
    balanceStore.set({ 'eth.fixture': { ETH: known } });
    let fail!: (error: Error) => void;
    service.getDisplayBalance.mockReturnValueOnce(
      new Promise((_resolve, reject) => {
        fail = reject;
      })
    );
    await render();
    expect(document.body.textContent).toContain('7.00');
    expect(document.querySelector('[data-slot="skeleton"]')).toBeNull();
    fail(new Error('Offline'));
    await settle();
    expect(document.body.textContent).toContain('Some balances may be out of date.');
    balanceStore.set({ 'eth.fixture': { ETH: { ...known } } });
    await settle();
    expect(document.body.textContent).not.toContain('Some balances may be out of date.');
  });

  it('starts a new coin balance read on the same channel while the previous coin is pending', async () => {
    const token = {
      ...coin,
      id: 'TOKEN',
      currencyId: 'TOKEN',
      displayName: 'Token',
      displayTicker: 'TOKEN',
    };
    coinsStore.set([coin, token]);
    service.getDisplayCoinScopes.mockImplementation(async (coinId: string) => ({
      coinId,
      scopes: scopes.scopes.map((scope) => ({ ...scope, coinId })),
    }));
    let finishEth!: (value: { total: string; confirmed: string; pending: string }) => void;
    service.getDisplayBalance.mockImplementation((_channelId: string, coinId: string) =>
      coinId === 'ETH'
        ? new Promise((resolve) => {
            finishEth = resolve;
          })
        : Promise.resolve({ total: '3', confirmed: '3', pending: '0' })
    );

    const target = document.createElement('div');
    document.body.append(target);
    const harness = mount(AssetDetailsHarness, { target, props: { initialCoinId: 'ETH' } });
    component = harness;
    await settle();
    expect(service.getDisplayBalance).toHaveBeenCalledWith('eth.fixture', 'ETH');

    harness.switchCoin('TOKEN');
    await settle();
    expect(service.getDisplayBalance).toHaveBeenCalledWith('eth.fixture', 'TOKEN');
    finishEth({ total: '2', confirmed: '2', pending: '0' });
    await settle();
    expect(document.body.textContent).toContain('Token');
    expect(document.body.textContent).toContain('3.00');
  });

  it('stops after a history error until the user retries', async () => {
    service.getDisplayTransactionHistoryPage.mockRejectedValueOnce(new Error('History offline'));
    await render();
    expect(document.body.textContent).toContain('History offline');
    expect(document.querySelector('[data-transaction-loading]')).toBeNull();
    expect(service.getDisplayTransactionHistoryPage).toHaveBeenCalledTimes(1);
    [...document.querySelectorAll('button')]
      .find((button) => button.textContent?.trim() === 'Try again')
      ?.click();
    await settle();
    expect(document.body.textContent).toContain('No transactions');
    expect(service.getDisplayTransactionHistoryPage).toHaveBeenCalledTimes(2);
  });

  it('retains history rows through refresh and failure until retry succeeds', async () => {
    const page = {
      transactions: [
        {
          txid: 'existing',
          amount: '1',
          fromAddress: 'Alice',
          toAddress: scopes.scopes[0].address,
          confirmations: 10,
          pending: false,
        },
      ],
      hasMore: false,
      nextCursor: null,
    };
    service.getDisplayTransactionHistoryPage.mockResolvedValueOnce(page);
    await render();
    const before = document.querySelector('ul li')?.textContent;
    expect(before).toBeTruthy();
    let fail!: (error: Error) => void;
    service.getDisplayTransactionHistoryPage.mockReturnValueOnce(
      new Promise((_resolve, reject) => {
        fail = reject;
      })
    );
    invalidateTransactionHistoryPages('eth.fixture', 'ETH');
    await settle();
    expect(document.querySelector('ul li')?.textContent).toBe(before);
    expect(document.querySelector('[data-transaction-loading]')).toBeNull();
    fail(new Error('History offline'));
    await settle();
    expect(document.querySelector('ul li')?.textContent).toBe(before);
    expect(document.body.textContent).toContain('Transaction history could not be updated.');
    [...document.querySelectorAll('button')]
      .find((button) => button.textContent?.trim() === 'Try again')
      ?.click();
    await settle();
    expect(document.body.textContent).toContain('No transactions');
    expect(document.body.textContent).not.toContain('Transaction history could not be updated.');
  });

  it('preserves the asset identity when scopes fail and offers a retry', async () => {
    service.getDisplayCoinScopes.mockRejectedValueOnce(new Error('Offline'));
    await render();
    expect(document.body.textContent).toContain('Ethereum');
    expect(document.body.textContent).toContain('Offline');
    const retry = [...document.querySelectorAll('button')].find(
      (button) => button.textContent?.trim() === 'Try again'
    );
    expect(retry).toBeDefined();
    retry?.click();
    await settle();
    expect(document.body.textContent).not.toContain('Offline');
    expect(document.querySelector<HTMLButtonElement>('button[aria-label="Send"]')?.disabled).toBe(
      false
    );
  });

  it.each([
    { locale: 'en', theme: 'light', message: 'Request timed out.' },
    { locale: 'nl', theme: 'dark', message: 'Het verzoek duurde te lang.' },
  ])('shows a localized timeout in $theme mode', async ({ locale, theme, message }) => {
    setLocale(locale);
    document.documentElement.classList.toggle('dark', theme === 'dark');
    service.getDisplayCoinScopes.mockRejectedValueOnce(
      new DOMException('Request timed out', 'TimeoutError')
    );
    await render();
    expect(document.body.textContent).toContain(message);
  });
});
