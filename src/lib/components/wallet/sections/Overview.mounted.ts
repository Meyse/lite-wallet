import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { coinsStore } from '$lib/stores/coins.js';
import { balanceStore } from '$lib/stores/balances.js';
import { buildWalletChannels, walletChannelsStore } from '$lib/stores/walletChannels.js';
import { ratesStore } from '$lib/stores/rates.js';
import { networkStore } from '$lib/stores/network.js';
import { walletBootstrapStore } from '$lib/stores/walletBootstrap.js';
import { settingsStore } from '$lib/stores/settings.js';
import { setLocale } from '$lib/i18n';
import {
  readWalletOverviewPreferences,
  writeWalletOverviewPreferences,
} from '$lib/stores/walletOverviewPreferences.js';
import type { BalanceResult, CoinDefinition } from '$lib/types/wallet.js';
import OverviewHarness from './__tests__/OverviewHarness.svelte';

const services = vi.hoisted(() => ({
  getDisplayDlightSeedStatus: vi.fn(),
  getDisplayCoinScopes: vi.fn(),
  getDisplayBalance: vi.fn(),
}));
vi.mock('$lib/services/walletDisplayService.js', () => services);
vi.mock('$lib/services/walletLockCoordinator.js', () => ({ isForcedWalletLockError: () => false }));
class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
vi.stubGlobal('ResizeObserver', ResizeObserverStub);

const coins: CoinDefinition[] = [
  {
    id: 'VRSC',
    currencyId: 'VRSC',
    systemId: 'verus',
    displayName: 'Verus',
    displayTicker: 'VRSC',
    proto: 'vrsc',
    compatibleChannels: ['vrpc'],
  },
  {
    id: 'ETH',
    currencyId: 'ETH',
    systemId: '.eth',
    displayName: 'Ethereum',
    displayTicker: 'ETH',
    proto: 'eth',
    compatibleChannels: ['eth'],
  },
  {
    id: 'BTC',
    currencyId: 'BTC',
    systemId: '.btc',
    displayName: 'Bitcoin',
    displayTicker: 'BTC',
    proto: 'btc',
    compatibleChannels: ['btc'],
  },
  {
    id: 'usdc',
    currencyId: 'usdc',
    systemId: 'verus',
    displayName: 'USDC on Verus',
    displayTicker: 'USDC',
    proto: 'vrsc',
    compatibleChannels: ['vrpc'],
  },
].map(
  (coin) =>
    ({
      decimals: 8,
      isTestnet: false,
      secondsPerBlock: 60,
      vrpcEndpoints: [],
      ...coin,
    }) as CoinDefinition
);
const wallet = { name: 'Overview fixture', network: 'mainnet' as const, color: 'blue', emoji: '' };
const channels = buildWalletChannels(coins, 'Rfixture');
const balance = (amount: string): BalanceResult => ({
  total: amount,
  confirmed: amount,
  pending: '0',
});
let component: ReturnType<typeof createHarness> | undefined;
let balances: Record<string, Record<string, BalanceResult>>;
async function settle() {
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 10));
  await tick();
}
function createHarness(target: HTMLElement, onOpenAssetDetails: () => void) {
  return mount(OverviewHarness, { target, props: { initialWallet: wallet, onOpenAssetDetails } });
}
async function render() {
  const target = document.createElement('div');
  document.body.append(target);
  const open = vi.fn();
  component = createHarness(target, open);
  await settle();
  return open;
}
function required<T>(value: T | null | undefined): T {
  if (value === null || value === undefined) throw new Error('Expected test element or component');
  return value;
}
function search(): HTMLInputElement {
  return required(document.querySelector<HTMLInputElement>('input[type="search"]'));
}
async function query(value: string) {
  search().value = value;
  search().dispatchEvent(new InputEvent('input', { bubbles: true }));
  await settle();
}
function rowNames(): string[] {
  return [...document.querySelectorAll('ul li button p.truncate')].map((el) =>
    (el.textContent ?? '').trim()
  );
}
function banner(): string {
  return required(document.querySelector('.balance-banner')).textContent?.replace(/\s/g, '') ?? '';
}
async function openMenu() {
  required(document.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]')).click();
  await settle();
}
async function choose(label: string) {
  const item = [...document.querySelectorAll<HTMLElement>('[role="menuitemradio"]')].find(
    (el) => el.textContent?.trim() === label
  );
  expect(item).toBeDefined();
  required(item).click();
  await settle();
}

describe('mounted wallet overview controls', () => {
  beforeEach(() => {
    localStorage.clear();
    setLocale('en');
    settingsStore.set({ theme: 'light', displayCurrency: 'EUR', autoLockMinutes: 15 });
    coinsStore.set(coins);
    walletChannelsStore.set(channels);
    walletBootstrapStore.set(false);
    networkStore.set({});
    balances = {};
    for (const [i, coin] of coins.entries()) {
      const id = channels.byCoinId[coin.id];
      balances[id] ??= {};
      balances[id][coin.id] = balance(['10', '2', '0', '5'][i]);
    }
    balanceStore.set(balances);
    ratesStore.set(
      Object.fromEntries(
        coins.map((coin) => [
          coin.id,
          { rates: { EUR: coin.id === 'ETH' ? 100 : 2 }, usdChange24hPct: 1 },
        ])
      )
    );
    services.getDisplayDlightSeedStatus.mockResolvedValue({ configured: false });
    services.getDisplayCoinScopes.mockImplementation(async (coinId) => ({
      coinId,
      scopes: [{ channelId: channels.byCoinId[coinId], coinId, scopeKind: 'transparent' }],
    }));
    services.getDisplayBalance.mockImplementation(
      async (channelId, coinId) => balances[channelId]?.[coinId]
    );
  });
  afterEach(async () => {
    if (component) await unmount(component);
    component = undefined;
    document.body.replaceChildren();
    vi.clearAllMocks();
  });

  it('always exposes controls for four currencies, searches networks, clears and preserves row destinations and holdings', async () => {
    const open = await render();
    expect(rowNames()).toEqual(['Verus', 'USDC on Verus', 'Ethereum', 'Bitcoin']);
    const total = banner();
    required(
      document.querySelector<HTMLButtonElement>('button[aria-label="Hide holdings"]')
    ).click();
    await settle();
    await query('  VERUS  ');
    expect(rowNames()).toEqual(['Verus', 'USDC on Verus']);
    expect(banner()).toBe(total);
    expect(document.querySelector('.holdings-obscured')).not.toBeNull();
    required(document.querySelector<HTMLButtonElement>('ul li button')).click();
    expect(open).toHaveBeenCalledWith(
      expect.objectContaining({ coinId: 'VRSC', scopeFilterMode: 'transparent' })
    );
    await query('Solana');
    expect(document.body.textContent).toContain('No assets found');
    expect(banner()).toBe(total);
    required(
      document.querySelector<HTMLButtonElement>('button[aria-label="Clear search"]')
    ).click();
    await settle();
    expect(search().value).toBe('');
    expect(rowNames()).toHaveLength(4);
    expect(document.activeElement).toBe(search());
  });

  it('sorts globally, combines the balance filter with search, and persists preferences without the query', async () => {
    await render();
    const total = banner();
    await openMenu();
    await choose('Value: high to low');
    expect(rowNames()).toEqual(['Ethereum', 'Verus', 'USDC on Verus', 'Bitcoin']);
    await openMenu();
    expect(
      document.querySelector('[role="menuitemradio"][aria-checked="true"]')?.textContent
    ).toContain('Value: high to low');
    required(document.querySelector<HTMLElement>('[role="menuitemcheckbox"]')).click();
    await settle();
    expect(rowNames()).not.toContain('Bitcoin');
    expect(readWalletOverviewPreferences(wallet.name, 'mainnet')).toEqual({
      sort: 'value',
      reversed: false,
      withBalance: true,
    });
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await settle();
    await query('verus');
    expect(rowNames()).toEqual(['Verus', 'USDC on Verus']);
    expect(banner()).toBe(total);
    required(
      document.querySelector<HTMLButtonElement>('button[aria-label="Clear balance filter"]')
    ).click();
    await settle();
    expect(search().value).toBe('verus');
    expect(document.activeElement).toBe(search());
    expect(readWalletOverviewPreferences(wallet.name, 'mainnet')).toEqual({
      sort: 'value',
      reversed: false,
      withBalance: false,
    });
    await unmount(required(component));
    component = undefined;
    document.body.replaceChildren();
    await render();
    expect(search().value).toBe('');
    expect(rowNames()[0]).toBe('Ethereum');
  });

  it('reverses the selected sort repeatedly, preserves search/filter/total, and restores the direction', async () => {
    await render();
    const total = banner();
    await openMenu();
    await choose('Name: A–Z');
    const reverse = () =>
      required(
        document.querySelector<HTMLButtonElement>('button[aria-label="Reverse sort order"]')
      );
    reverse().click();
    await settle();
    expect(rowNames()).toEqual(['Verus', 'USDC on Verus', 'Ethereum', 'Bitcoin']);
    expect(document.querySelector('[aria-haspopup="menu"]')?.textContent).toContain('Name: Z–A');
    reverse().click();
    await settle();
    expect(rowNames()).toEqual(['Bitcoin', 'Ethereum', 'USDC on Verus', 'Verus']);
    await query('verus');
    reverse().click();
    await settle();
    expect(search().value).toBe('verus');
    expect(rowNames()).toEqual(['Verus', 'USDC on Verus']);
    expect(banner()).toBe(total);
    await unmount(required(component));
    component = undefined;
    document.body.replaceChildren();
    await render();
    expect(search().value).toBe('');
    expect(readWalletOverviewPreferences(wallet.name, 'mainnet')).toEqual({
      sort: 'name',
      reversed: true,
      withBalance: false,
    });
    await openMenu();
    expect(document.querySelector('[aria-checked="true"]')?.textContent).toContain('Name: Z–A');
    await choose('Value: high to low');
    expect(readWalletOverviewPreferences(wallet.name, 'mainnet').reversed).toBe(false);
    reverse().click();
    await settle();
    expect(rowNames()).toEqual(['Bitcoin', 'USDC on Verus', 'Verus', 'Ethereum']);
    expect(banner()).toBe(total);
  });

  it('switches wallet/network preferences and clears transient search', async () => {
    writeWalletOverviewPreferences('Other wallet', 'testnet', {
      sort: 'name',
      reversed: false,
      withBalance: true,
    });
    await render();
    await query('verus');
    required(component).switchWallet({ ...wallet, name: 'Other wallet', network: 'testnet' });
    await settle();
    expect(search().value).toBe('');
    expect(rowNames()[0]).toBe('Ethereum');
    expect(document.body.textContent).toContain('With balance');
    expect(readWalletOverviewPreferences(wallet.name, 'mainnet')).toEqual({
      sort: 'verus-first',
      reversed: false,
      withBalance: false,
    });
  });

  it('retains private syncing, pending, unknown and bootstrapping rows without changing the full total state', async () => {
    writeWalletOverviewPreferences(wallet.name, 'mainnet', {
      sort: 'verus-first',
      reversed: false,
      withBalance: true,
    });
    services.getDisplayDlightSeedStatus.mockResolvedValue({ configured: true });
    services.getDisplayCoinScopes.mockImplementation(async (coinId) => ({
      coinId,
      scopes: [
        { channelId: channels.byCoinId[coinId], coinId, scopeKind: 'transparent' },
        ...(coinId === 'VRSC'
          ? [{ channelId: 'private', coinId, systemId: 'verus', scopeKind: 'shielded' }]
          : []),
      ],
    }));
    balances.private = { VRSC: balance('0') };
    balances[channels.byCoinId.BTC].BTC = { total: '0', confirmed: '1', pending: '-1' };
    delete balances[channels.byCoinId.ETH].ETH;
    balanceStore.set(balances);
    networkStore.set({ private: { percent: 30, syncing: true } });
    await render();
    expect(document.querySelectorAll('ul li')).toHaveLength(5);
    const total = banner();
    expect(total).toContain('Partial');
    await query('no-match');
    expect(banner()).toBe(total);
    walletBootstrapStore.set(true);
    await settle();
    expect(document.querySelector('.balance-banner [aria-label="Loading…"]')).not.toBeNull();
  });

  it('offers filter-only recovery when every enabled balance is a loaded zero', async () => {
    for (const channel of Object.values(balances))
      for (const id of Object.keys(channel)) channel[id] = balance('0');
    balanceStore.set(balances);
    writeWalletOverviewPreferences(wallet.name, 'mainnet', {
      sort: 'verus-first',
      reversed: false,
      withBalance: true,
    });
    await render();
    expect(document.body.textContent).toContain('No assets found');
    const recovery = [...document.querySelectorAll<HTMLButtonElement>('button')].find(
      (el) => el.textContent?.trim() === 'Clear balance filter'
    );
    required(recovery).click();
    await settle();
    expect(rowNames()).toHaveLength(4);
  });
});
