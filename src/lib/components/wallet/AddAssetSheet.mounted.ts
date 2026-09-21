// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '$lib/i18n';
import Component from './AddAssetSheet.svelte';

const service = vi.hoisted(() => ({
  getCoinRegistry: vi.fn(),
  getAssetPreferences: vi.fn(),
  getActiveWallet: vi.fn(),
  discoverVrpcAssets: vi.fn(),
  getCoinScopes: vi.fn(),
  getBalances: vi.fn(),
  setAssetPreferences: vi.fn(),
  startUpdateEngine: vi.fn(),
  resolveErc20Contract: vi.fn(),
  resolvePbaasCurrency: vi.fn(),
  addCoinDefinition: vi.fn(),
}));

vi.mock('$lib/services/coinsService.js', () => service);
vi.mock('$lib/services/walletService.js', () => service);
vi.mock('$lib/services/walletLockCoordinator.js', () => ({
  isForcedWalletLockError: () => false,
}));

vi.stubGlobal(
  'ResizeObserver',
  class {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
);
vi.stubGlobal(
  'IntersectionObserver',
  class {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
);
HTMLElement.prototype.scrollIntoView = vi.fn();
HTMLElement.prototype.hasPointerCapture = () => false;
HTMLElement.prototype.setPointerCapture = () => {};
HTMLElement.prototype.releasePointerCapture = () => {};

const sessionId = 'manage-assets-session';
const verusSystemId = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
const vrsc = {
  id: 'VRSC',
  currencyId: verusSystemId,
  systemId: verusSystemId,
  displayTicker: 'VRSC',
  displayName: 'Verus',
  proto: 'vrsc' as const,
  compatibleChannels: ['vrpc' as const],
  decimals: 8,
  vrpcEndpoints: [],
  secondsPerBlock: 60,
  isTestnet: false,
};
const usdc = {
  ...vrsc,
  id: 'USDC',
  currencyId: '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48',
  systemId: 'ETH',
  displayTicker: 'USDC',
  displayName: 'USD Coin',
  proto: 'erc20' as const,
  compatibleChannels: ['erc20' as const],
  decimals: 6,
};

function source(
  id = verusSystemId,
  name = 'Verus',
  status: 'available' | 'partial' | 'unavailable' = 'available'
) {
  return {
    systemId: id,
    systemTicker: name,
    systemDisplayName: name,
    status,
    checkedScopeCount: status === 'unavailable' ? 0 : 1,
    uncheckedScopeCount: status === 'available' ? 0 : 1,
    includesReadOnly: false,
    privateScopeChecked: false,
  };
}

function holding(coin = vrsc, id = verusSystemId, name = 'Verus', balance = '24.5') {
  return {
    assetKey: `${coin.proto}:${coin.currencyId.toLowerCase()}`,
    currencyId: coin.currencyId,
    systemId: id,
    systemTicker: name,
    systemDisplayName: name,
    balance,
    balanceStatus: 'available' as const,
    includesReadOnly: false,
    coin,
  };
}

function initialDiscovery() {
  return {
    network: 'mainnet' as const,
    scopeMetadataComplete: true,
    sources: [source()],
    holdings: [holding()],
  };
}

function discoveryWithUnknown(status: 'available' | 'partial' = 'available') {
  return {
    ...initialDiscovery(),
    sources: status === 'partial' ? [source(), source('iDEX', 'vDEX', 'unavailable')] : [source()],
    holdings: [
      ...initialDiscovery().holdings,
      {
        assetKey: 'vrsc:iunknowncurrency',
        currencyId: 'iUnknownCurrency',
        systemId: verusSystemId,
        systemTicker: 'VRSC',
        systemDisplayName: 'Verus',
        balance: '3',
        balanceStatus: 'available' as const,
        includesReadOnly: false,
        coin: null,
      },
    ],
  };
}

let component: ReturnType<typeof mount> | null = null;
let onClose: () => void;

async function settle(): Promise<void> {
  for (let index = 0; index < 6; index += 1) {
    await tick();
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
}

async function render(): Promise<void> {
  const target = document.createElement('div');
  document.body.append(target);
  component = mount(Component, {
    target,
    props: { isOpen: true, network: 'mainnet', onClose },
  });
  await settle();
}

function button(label: string): HTMLButtonElement {
  const found = [...document.querySelectorAll<HTMLButtonElement>('button')].find(
    (candidate) =>
      candidate.textContent?.trim() === label || candidate.getAttribute('aria-label') === label
  );
  if (!found) throw new Error(`Missing button ${label}`);
  return found;
}

function row(label: string): HTMLElement | undefined {
  return [...document.querySelectorAll<HTMLElement>('.asset-row')].find((candidate) =>
    candidate.textContent?.includes(label)
  );
}

function requiredElement<T extends Element>(selector: string): T {
  const element = document.querySelector<T>(selector);
  if (!element) throw new Error(`Missing element ${selector}`);
  return element;
}

async function unmountRenderedComponent(): Promise<void> {
  if (!component) throw new Error('Expected a mounted component');
  await unmount(component);
  component = null;
}

describe('mounted Manage assets', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    setLocale('en');
    onClose = vi.fn();
    service.getCoinRegistry.mockResolvedValue([vrsc, usdc]);
    service.getAssetPreferences.mockResolvedValue({
      network: 'mainnet',
      sessionId,
      portfolioCoinIds: ['VRSC'],
      hiddenAssetKeys: [],
    });
    service.getActiveWallet.mockResolvedValue({
      wallet_name: 'Test wallet',
      network: 'mainnet',
      emoji: 'wallet',
      color: 'blue',
      session_id: sessionId,
    });
    service.discoverVrpcAssets.mockResolvedValue(initialDiscovery());
    service.getCoinScopes.mockImplementation(async (coinId: string) => ({
      coinId,
      scopes: [
        {
          channelId: `erc20.${coinId}`,
          coinId,
          systemId: 'ETH',
          systemTicker: 'ETH',
          systemDisplayName: 'Ethereum',
          address: 'example',
          addressLabel: 'example',
          isPrimaryAddress: true,
          isReadOnly: false,
          scopeKind: 'transparent',
        },
      ],
    }));
    service.getBalances.mockResolvedValue({ total: '125', confirmed: '125', pending: '0' });
    service.setAssetPreferences.mockImplementation(
      async (expectedSessionId: string, portfolioCoinIds: string[], hiddenAssetKeys: string[]) => ({
        network: 'mainnet',
        sessionId: expectedSessionId,
        portfolioCoinIds,
        hiddenAssetKeys,
      })
    );
    service.startUpdateEngine.mockResolvedValue(undefined);
    service.addCoinDefinition.mockImplementation(async (coin) => coin);
  });

  afterEach(async () => {
    if (component) await unmount(component);
    component = null;
    await settle();
    vi.useRealTimers();
    document.body.replaceChildren();
    document.documentElement.classList.remove('dark');
  });

  it('shows a positive inactive ERC20 holding in Found in your wallet', async () => {
    await render();
    expect(service.getBalances).toHaveBeenCalled();
    expect(row('USD Coin')).toBeDefined();
  });

  it('shows asset-shaped skeletons until the initial lookup finishes', async () => {
    let finishRegistry: (coins: (typeof vrsc | typeof usdc)[]) => void = () => {};
    service.getCoinRegistry.mockImplementation(
      () => new Promise((resolve) => (finishRegistry = resolve))
    );
    await render();
    expect(document.querySelector('[role="tabpanel"]')?.getAttribute('aria-busy')).toBe('true');
    expect(document.querySelectorAll('[data-slot="skeleton"]')).toHaveLength(12);
    finishRegistry([vrsc, usdc]);
    await settle();
    expect(document.querySelector('[role="tabpanel"]')?.getAttribute('aria-busy')).toBe('false');
    expect(document.querySelector('[data-slot="skeleton"]')).toBeNull();
  });

  it('keeps the divider between Found and shown assets but omits the last one', async () => {
    await render();
    expect(row('USD Coin')?.firstElementChild?.className).toContain('border-b');
    const rows = [...document.querySelectorAll<HTMLElement>('.asset-row')];
    expect(rows.at(-1)?.firstElementChild?.className).not.toContain('border-b');
  });

  it('shows top and bottom fades only while content remains in that direction', async () => {
    await render();
    const viewport = requiredElement<HTMLElement>('[data-slot="scroll-area-viewport"]');
    Object.defineProperties(viewport, {
      clientHeight: { configurable: true, value: 300 },
      scrollHeight: { configurable: true, value: 600 },
      scrollTop: { configurable: true, writable: true, value: 0 },
    });

    viewport.dispatchEvent(new Event('scroll'));
    await settle();
    expect(document.querySelector('[data-manage-assets-scroll-fade="top"]')).toBeNull();
    expect(document.querySelector('[data-manage-assets-scroll-fade="bottom"]')).not.toBeNull();

    viewport.scrollTop = 150;
    viewport.dispatchEvent(new Event('scroll'));
    await settle();
    expect(document.querySelector('[data-manage-assets-scroll-fade="top"]')).not.toBeNull();
    expect(document.querySelector('[data-manage-assets-scroll-fade="bottom"]')).not.toBeNull();

    viewport.scrollTop = 300;
    viewport.dispatchEvent(new Event('scroll'));
    await settle();
    expect(document.querySelector('[data-manage-assets-scroll-fade="top"]')).not.toBeNull();
    expect(document.querySelector('[data-manage-assets-scroll-fade="bottom"]')).toBeNull();
  });

  it('uses the standard secondary treatment for both custom-asset actions', async () => {
    await render();
    const addCustom = button('Add custom asset');
    expect(addCustom.className).toContain('bg-secondary');
    expect(addCustom.className).toContain('text-secondary-foreground');

    addCustom.click();
    await settle();
    const findAsset = button('Find asset');
    expect(findAsset.className).toContain('bg-secondary');
    expect(findAsset.className).toContain('text-secondary-foreground');
    expect(findAsset.className).not.toContain('text-primary');
  });

  it('does not invent zero when a relevant VRPC provider failed', async () => {
    service.discoverVrpcAssets.mockResolvedValue({
      network: 'mainnet',
      scopeMetadataComplete: true,
      sources: [source(verusSystemId, 'Verus', 'unavailable'), source('iOther', 'vDEX')],
      holdings: [],
    });
    await render();
    expect(row('Verus')?.textContent).toMatch(/Unavailable|Partial/);
    expect(document.body.textContent).toContain('Some holding networks could not be checked.');
  });

  it('retries only incomplete VRPC sources without rescanning healthy chains', async () => {
    service.discoverVrpcAssets
      .mockResolvedValueOnce({
        network: 'mainnet',
        scopeMetadataComplete: true,
        sources: [source(verusSystemId, 'Verus', 'unavailable'), source('iOther', 'vDEX')],
        holdings: [],
      })
      .mockResolvedValueOnce({
        network: 'mainnet',
        scopeMetadataComplete: true,
        sources: [source(verusSystemId, 'Verus', 'unavailable')],
        holdings: [],
      });
    await render();
    button('Try again').click();
    await settle();
    expect(service.discoverVrpcAssets).toHaveBeenLastCalledWith([verusSystemId]);
    expect(service.getBalances).toHaveBeenCalledTimes(1);
  });

  it('keeps a failed hide visible with an actionable retry', async () => {
    service.setAssetPreferences.mockRejectedValue(new Error('Preference save failed'));
    await render();
    row('Verus')?.querySelector<HTMLButtonElement>('[role="switch"]')?.click();
    await settle();
    expect(document.body.textContent).toContain('Preference save failed');
    expect(row('Verus')).toBeDefined();
    expect(row('Verus')?.textContent).toContain('Try again');
  });

  it('places a persisted hidden noncatalog discovery only in Hidden after reopening', async () => {
    service.getAssetPreferences.mockResolvedValue({
      network: 'mainnet',
      sessionId,
      portfolioCoinIds: ['VRSC'],
      hiddenAssetKeys: ['vrsc:iunknowncurrency'],
    });
    service.discoverVrpcAssets.mockResolvedValue(discoveryWithUnknown());
    await render();
    expect(row('iUnknownCurrency')).toBeUndefined();
    button('Hidden').click();
    await settle();
    button('Other assets found').click();
    await settle();
    expect(row('iUnknownCurrency')).toBeDefined();
    expect(
      [...document.querySelectorAll<HTMLElement>('.asset-row')].filter((candidate) =>
        candidate.textContent?.includes('iUnknownCurrency')
      )
    ).toHaveLength(1);
    expect(row('iUnknownCurrency')?.textContent).toContain('Review');
  });

  it('promotes a newly positive known balance into Found when reopening', async () => {
    service.getBalances.mockResolvedValueOnce({ total: '0', confirmed: '0', pending: '0' });
    await render();
    expect(row('USD Coin')).toBeUndefined();
    service.getBalances.mockResolvedValue({ total: '125', confirmed: '125', pending: '0' });
    await unmountRenderedComponent();
    await render();
    expect(service.getBalances).toHaveBeenCalledTimes(2);
    expect(row('USD Coin')).toBeDefined();
    expect(row('USD Coin')?.closest('section')?.querySelector('h2')?.textContent).toContain(
      'Found in your wallet'
    );
  });

  it('shows an actionable save error for a failed noncatalog dismissal', async () => {
    service.discoverVrpcAssets.mockResolvedValue(discoveryWithUnknown());
    service.setAssetPreferences.mockRejectedValue(new Error('Preference save failed'));
    await render();
    button('Other assets found').click();
    await settle();
    const unknown = row('iUnknownCurrency');
    const dismiss = unknown?.querySelector<HTMLButtonElement>('button[title]');
    expect(dismiss).toBeDefined();
    dismiss?.click();
    await settle();
    expect(service.setAssetPreferences).toHaveBeenCalledTimes(1);
    expect(row('iUnknownCurrency')?.textContent).toContain('Preference save failed');
    expect(row('iUnknownCurrency')?.textContent).toContain('Try again');
    expect(row('iUnknownCurrency')?.textContent).not.toContain('Hidden');
  });

  it('marks a noncatalog aggregate partial while keeping a healthy selected network accurate', async () => {
    service.discoverVrpcAssets.mockResolvedValue(discoveryWithUnknown('partial'));
    await render();
    button('Other assets found').click();
    await settle();
    expect(row('iUnknownCurrency')?.textContent).toContain('Partial');

    const trigger = requiredElement<HTMLButtonElement>('[aria-haspopup="menu"]');
    trigger.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await settle();
    const option = [...document.querySelectorAll<HTMLElement>('[role="menuitemradio"]')].find(
      (candidate) => candidate.textContent?.trim() === 'Verus'
    );
    option?.click();
    await settle();
    expect(row('iUnknownCurrency')?.textContent).toContain('3 iUnknownCurrency');
    expect(row('iUnknownCurrency')?.textContent).not.toContain('Partial');
  });

  it('keeps a long fallback identifier out of the balance cell and available in Review', async () => {
    const longCurrencyId = 'i123456789012345678901234567890123';
    service.resolvePbaasCurrency.mockResolvedValue({
      status: 'resolved',
      coin: {
        ...vrsc,
        id: longCurrencyId,
        currencyId: longCurrencyId,
        displayTicker: longCurrencyId,
        displayName: longCurrencyId,
      },
    });
    service.discoverVrpcAssets.mockResolvedValue({
      ...initialDiscovery(),
      holdings: [
        ...initialDiscovery().holdings,
        {
          assetKey: `vrsc:${longCurrencyId.toLowerCase()}`,
          currencyId: longCurrencyId,
          systemId: verusSystemId,
          systemTicker: 'VRSC',
          systemDisplayName: 'Verus',
          balance: '3',
          balanceStatus: 'available' as const,
          includesReadOnly: false,
          coin: null,
        },
      ],
    });
    await render();
    button('Other assets found').click();
    await settle();

    const unknown = row(longCurrencyId);
    expect(
      unknown?.querySelector(`[data-manage-assets-balance="vrsc:${longCurrencyId.toLowerCase()}"]`)
        ?.textContent
    ).toContain('3');
    expect(
      unknown?.querySelector(`[data-manage-assets-balance="vrsc:${longCurrencyId.toLowerCase()}"]`)
        ?.textContent
    ).not.toContain(longCurrencyId);

    unknown?.querySelector<HTMLButtonElement>('button[data-manage-assets-return-focus]')?.click();
    await settle();
    expect(requiredElement<HTMLInputElement>('#custom-asset-input').value).toBe(longCurrencyId);
    button('Find asset').click();
    await settle();
    expect(document.querySelector('dl')?.textContent).toContain(longCurrencyId);
    expect(button('Copy')).toBeDefined();
  });

  it('marks an unqueried custom contract balance unavailable instead of zero', async () => {
    service.resolveErc20Contract.mockResolvedValue({
      status: 'resolved',
      coin: {
        ...usdc,
        id: 'OTHER',
        currencyId: '0x1111111111111111111111111111111111111111',
        displayTicker: 'OTHER',
        displayName: 'Other token',
      },
    });
    await render();
    button('Add custom asset').click();
    await settle();
    const input = requiredElement<HTMLInputElement>('#custom-asset-input');
    expect(document.activeElement).toBe(input);
    input.value = '0x1111111111111111111111111111111111111111';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await settle();
    button('Find asset').click();
    await settle();
    expect(document.querySelector('dl')?.textContent).toContain('Unavailable');
    expect(document.querySelector('dl')?.textContent).not.toContain('0 OTHER');
    expect(document.querySelector('dl')?.textContent).toContain(
      '0x1111111111111111111111111111111111111111'
    );
    input.value = 'another asset';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await settle();
    expect(document.querySelector('dl')).toBeNull();
    expect(document.body.textContent).not.toContain('Show in portfolio');
  });

  it('shows the resolved UNI logo in the custom asset result in both themes', async () => {
    const contract = '0x1f9840a85d5af5bf1d1762f925bdaddc4201f984';
    service.resolveErc20Contract.mockResolvedValue({
      status: 'resolved',
      coin: {
        ...usdc,
        id: `erc20_${contract}`,
        currencyId: contract,
        systemId: contract,
        displayTicker: 'UNI',
        displayName: 'Uniswap',
      },
    });

    for (const theme of ['light', 'dark']) {
      document.documentElement.classList.toggle('dark', theme === 'dark');
      await render();
      button('Add custom asset').click();
      await settle();
      const input = requiredElement<HTMLInputElement>('#custom-asset-input');
      input.value = contract;
      input.dispatchEvent(new Event('input', { bubbles: true }));
      await settle();
      button('Find asset').click();
      await settle();

      expect(
        requiredElement<HTMLImageElement>('section img.coin-icon-surface').getAttribute('src')
      ).toBe('/images/coin-logos/web3/uni_dark.svg');
      await unmountRenderedComponent();
      document.body.replaceChildren();
    }
  });

  it('keeps the acted-on row connected while changing its portfolio state', async () => {
    const veth = {
      ...vrsc,
      id: 'i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X',
      currencyId: 'i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X',
      displayTicker: 'vETH',
      displayName: 'Ethereum on Verus',
    };
    service.getCoinRegistry.mockResolvedValue([vrsc, usdc, veth]);
    service.discoverVrpcAssets.mockResolvedValue({
      ...initialDiscovery(),
      holdings: [holding(), holding(veth, verusSystemId, 'Verus', '0.8')],
    });
    await render();
    const before = row('Ethereum on Verus');
    before?.querySelector<HTMLButtonElement>('[role="switch"]')?.click();
    await settle();
    expect(before?.isConnected).toBe(true);
  });

  it('shows the selected network balance instead of the all-network total', async () => {
    service.discoverVrpcAssets.mockResolvedValue({
      ...initialDiscovery(),
      sources: [source(), source('iDEX', 'vDEX')],
      holdings: [holding(), holding(vrsc, 'iDEX', 'vDEX', '2')],
    });
    await render();
    const trigger = document.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]');
    trigger?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await settle();
    const option = [...document.querySelectorAll<HTMLElement>('[role="menuitemradio"]')].find(
      (candidate) => candidate.textContent?.trim() === 'Verus'
    );
    option?.click();
    await settle();
    expect(row('Verus')?.textContent).toContain('24.5 VRSC');
    expect(row('Verus')?.textContent).not.toContain('26.5 VRSC');
  });

  it('does not dispatch a second preference write while the first save is in flight', async () => {
    service.getAssetPreferences.mockResolvedValue({
      network: 'mainnet',
      sessionId,
      portfolioCoinIds: ['VRSC', 'USDC'],
      hiddenAssetKeys: [],
    });
    let releaseFirst = () => {};
    service.setAssetPreferences.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          releaseFirst = () =>
            resolve({
              network: 'mainnet',
              sessionId,
              portfolioCoinIds: ['USDC'],
              hiddenAssetKeys: [],
            });
        })
    );
    await render();
    row('Verus')?.querySelector<HTMLButtonElement>('[role="switch"]')?.click();
    await settle();
    row('USD Coin')?.querySelector<HTMLButtonElement>('[role="switch"]')?.click();
    expect(service.setAssetPreferences).toHaveBeenCalledTimes(1);
    await unmountRenderedComponent();
    releaseFirst();
    await settle();
    expect(service.setAssetPreferences).toHaveBeenCalledTimes(1);
  });

  it('stops hydration when the active wallet session changes', async () => {
    service.getAssetPreferences
      .mockResolvedValueOnce({
        network: 'mainnet',
        sessionId,
        portfolioCoinIds: ['VRSC'],
        hiddenAssetKeys: [],
      })
      .mockResolvedValueOnce({
        network: 'mainnet',
        sessionId: 'replacement-session',
        portfolioCoinIds: ['USDC'],
        hiddenAssetKeys: [],
      });
    service.getActiveWallet.mockResolvedValue({
      wallet_name: 'Replacement wallet',
      network: 'mainnet',
      emoji: 'wallet',
      color: 'blue',
      session_id: 'replacement-session',
    });
    await render();
    expect(service.discoverVrpcAssets).not.toHaveBeenCalled();
  });

  it('does not continue delayed hydration after teardown', async () => {
    let resolvePreferences: (preferences: unknown) => void = () => {};
    service.getAssetPreferences.mockImplementation(
      () => new Promise((resolve) => (resolvePreferences = resolve))
    );
    const target = document.createElement('div');
    document.body.append(target);
    component = mount(Component, {
      target,
      props: { isOpen: true, network: 'mainnet', onClose },
    });
    await settle();
    await unmount(component);
    component = null;
    resolvePreferences({
      network: 'mainnet',
      sessionId,
      portfolioCoinIds: ['VRSC'],
      hiddenAssetKeys: [],
    });
    await settle();
    expect(service.getCoinRegistry).not.toHaveBeenCalled();
    expect(service.discoverVrpcAssets).not.toHaveBeenCalled();
  });

  it('shows hidden non-catalog discoveries in global search', async () => {
    const hiddenKey = 'vrsc:iunknowncurrency';
    service.getAssetPreferences.mockResolvedValue({
      network: 'mainnet',
      sessionId,
      portfolioCoinIds: ['VRSC'],
      hiddenAssetKeys: [hiddenKey],
    });
    service.discoverVrpcAssets.mockResolvedValue({
      ...initialDiscovery(),
      holdings: [
        ...initialDiscovery().holdings,
        {
          assetKey: hiddenKey,
          currencyId: 'iUnknownCurrency',
          systemId: verusSystemId,
          systemTicker: 'VRSC',
          systemDisplayName: 'Verus',
          balance: '3',
          balanceStatus: 'available',
          includesReadOnly: false,
          coin: null,
        },
      ],
    });
    await render();
    const search = requiredElement<HTMLInputElement>('input[type="search"]');
    search.value = 'iUnknownCurrency';
    search.dispatchEvent(new Event('input', { bubbles: true }));
    await settle();
    expect(row('iUnknownCurrency')?.textContent).toContain('Hidden');
  });

  it('implements arrow-key tab selection and focus', async () => {
    await render();
    const yours = button('Your assets');
    expect(yours.className).toContain('border-primary');
    expect(yours.className).not.toContain('border-transparent');
    yours.focus();
    yours.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
    await settle();
    const browse = button('Browse');
    expect(browse.getAttribute('aria-selected')).toBe('true');
    expect(browse.className).toContain('border-primary');
    expect(browse.className).not.toContain('border-transparent');
    expect(document.activeElement).toBe(browse);
  });

  it('lets Escape close the network menu without closing Manage assets', async () => {
    await render();
    const trigger = document.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]');
    trigger?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await settle();
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await settle();
    expect(onClose).not.toHaveBeenCalled();
  });

  it('restores focus to the custom-asset trigger after returning', async () => {
    await render();
    const trigger = button('Add custom asset');
    trigger.focus();
    trigger.click();
    await settle();
    expect(document.activeElement).toBe(
      document.querySelector<HTMLInputElement>('#custom-asset-input')
    );
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await settle();
    expect(document.activeElement).toBe(button('Add custom asset'));
  });

  it('places a newly added custom asset in Shown when returning to Manage assets', async () => {
    const custom = {
      ...usdc,
      id: 'OTHER',
      currencyId: '0x1111111111111111111111111111111111111111',
      displayTicker: 'OTHER',
      displayName: 'Other token',
    };
    service.resolveErc20Contract.mockResolvedValue({ status: 'resolved', coin: custom });
    service.addCoinDefinition.mockImplementation(async () => {
      service.getCoinRegistry.mockResolvedValue([vrsc, usdc, custom]);
      return custom;
    });
    service.setAssetPreferences.mockImplementation(
      async (expectedSessionId: string, portfolioCoinIds: string[], hiddenAssetKeys: string[]) => {
        const preferences = {
          network: 'mainnet',
          sessionId: expectedSessionId,
          portfolioCoinIds,
          hiddenAssetKeys,
        };
        service.getAssetPreferences.mockResolvedValue(preferences);
        return preferences;
      }
    );
    await render();
    button('Add custom asset').click();
    await settle();
    const input = requiredElement<HTMLInputElement>('#custom-asset-input');
    input.value = custom.currencyId;
    input.dispatchEvent(new Event('input', { bubbles: true }));
    button('Find asset').click();
    await settle();
    button('Show in portfolio').click();
    await settle();
    const success = requiredElement<HTMLButtonElement>('[data-manage-assets-success]');
    expect(success.textContent).toContain('Shown in portfolio');
    expect(success.className).toContain('bg-contact-saved');
    expect(success.className).toContain('text-contact-saved-foreground');
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await settle();
    expect(row('Other token')).toBeDefined();
    expect(row('Other token')?.querySelector('[role="switch"]')?.getAttribute('aria-checked')).toBe(
      'true'
    );
    expect(row('Other token')?.textContent).toContain('Unavailable');
    expect(row('Other token')?.textContent).not.toContain('0 OTHER');

    service.getBalances.mockImplementation(async (_channelId: string, coinId?: string) => {
      const total = coinId === 'OTHER' ? '0' : '125';
      return { total, confirmed: total, pending: '0' };
    });
    await unmountRenderedComponent();
    await render();
    expect(row('Other token')?.textContent).toContain('0 OTHER');
    expect(row('Other token')?.textContent).not.toContain('Unavailable');
  });

  it('ignores a custom lookup that resolves after teardown', async () => {
    let resolveContract: (result: unknown) => void = () => {};
    service.resolveErc20Contract.mockImplementation(
      () => new Promise((resolve) => (resolveContract = resolve))
    );
    await render();
    button('Add custom asset').click();
    await settle();
    const input = requiredElement<HTMLInputElement>('#custom-asset-input');
    input.value = '0x1111111111111111111111111111111111111111';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    button('Find asset').click();
    await settle();
    await unmountRenderedComponent();
    resolveContract({ status: 'resolved', coin: usdc });
    await settle();
    expect(service.addCoinDefinition).not.toHaveBeenCalled();
  });
});
