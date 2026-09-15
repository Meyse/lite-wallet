// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { CoinDefinition, CoinScope, PreflightResult } from '$lib/types/wallet';

const mocks = vi.hoisted(() => ({
  preflightSend: vi.fn(),
  getPendingEthSubmission: vi.fn(),
  getDisplayCoinScopes: vi.fn(),
  getAddresses: vi.fn(),
  getBridgeCapabilities: vi.fn(),
}));

vi.mock('$lib/services/txService.js', () => ({
  acknowledgePendingEthSubmission: vi.fn(),
  getPendingEthSubmission: mocks.getPendingEthSubmission,
  preflightSend: mocks.preflightSend,
  resumePendingEthSubmission: vi.fn(),
  sendTransaction: vi.fn(),
}));

vi.mock('$lib/services/walletDisplayService.js', () => ({
  getDisplayCoinScopes: mocks.getDisplayCoinScopes,
  invalidateWalletDisplayScopes: vi.fn(),
  isWalletDisplayRequestInvalidated: () => false,
}));

vi.mock('$lib/services/walletService.js', () => ({
  getAddresses: mocks.getAddresses,
  getDlightRuntimeStatus: vi.fn(),
  getTransactionHistory: vi.fn().mockResolvedValue([]),
}));

vi.mock('$lib/services/bridgeTransferService.js', () => ({
  estimateBridgeConversion: vi.fn(),
  estimateBridgeExportFee: vi.fn(),
  getBridgeCapabilities: mocks.getBridgeCapabilities,
  getBridgeConversionPaths: vi.fn().mockResolvedValue({ paths: {} }),
  preflightBridgeTransfer: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock('$lib/services/walletLockCoordinator.js', () => ({
  isForcedWalletLockError: () => false,
}));

vi.mock('$lib/components/wallet/AppSidebar.svelte', async () => ({
  default: (await import('./test-fixtures/EmptyWalletChild.svelte')).default,
}));
vi.mock('$lib/components/wallet/sections/Overview.svelte', async () => ({
  default: (await import('./test-fixtures/OverviewNavigateStub.svelte')).default,
}));

vi.mock('$lib/components/wallet/sections/AssetDetails.svelte', async () => ({
  default: (await import('./test-fixtures/EmptyWalletChild.svelte')).default,
}));
vi.mock('$lib/components/wallet/sections/Receive.svelte', async () => ({
  default: (await import('./test-fixtures/EmptyWalletChild.svelte')).default,
}));
vi.mock('$lib/components/wallet/sections/Conversions.svelte', async () => ({
  default: (await import('./test-fixtures/EmptyWalletChild.svelte')).default,
}));
vi.mock('$lib/components/wallet/sections/Identity.svelte', async () => ({
  default: (await import('./test-fixtures/EmptyWalletChild.svelte')).default,
}));
vi.mock('$lib/components/wallet/sections/Apps.svelte', async () => ({
  default: (await import('./test-fixtures/EmptyWalletChild.svelte')).default,
}));
vi.mock('$lib/components/wallet/sections/Activity.svelte', async () => ({
  default: (await import('./test-fixtures/EmptyWalletChild.svelte')).default,
}));
vi.mock('$lib/components/wallet/sections/AddressBook.svelte', async () => ({
  default: (await import('./test-fixtures/EmptyWalletChild.svelte')).default,
}));
vi.mock('$lib/components/wallet/sections/Settings.svelte', async () => ({
  default: (await import('./test-fixtures/EmptyWalletChild.svelte')).default,
}));
vi.mock('$lib/components/flows/GenericRequest/GenericRequestImportSheet.svelte', async () => ({
  default: (await import('./test-fixtures/EmptyWalletChild.svelte')).default,
}));

import { addressBookStore } from '$lib/stores/addressBook';
import { balanceStore } from '$lib/stores/balances';
import { coinsStore } from '$lib/stores/coins';
import { networkStore } from '$lib/stores/network';
import { ratesStore } from '$lib/stores/rates';
import { localeStore } from '$lib/i18n';
import { PreflightRequestGuard } from './preflightRequestGuard';
import WalletLayoutLifecycleHarness from './test-fixtures/WalletLayoutLifecycleHarness.svelte';

const sourceAddress = `0x${'11'.repeat(20)}`;
const destinationAddress = `0x${'22'.repeat(20)}`;

const ethCoin: CoinDefinition = {
  id: 'ETH',
  currencyId: 'ETH',
  systemId: 'ethereum',
  displayTicker: 'ETH',
  displayName: 'Ethereum',
  proto: 'eth',
  compatibleChannels: ['eth'],
  decimals: 18,
  vrpcEndpoints: [],
  secondsPerBlock: 12,
  isTestnet: false,
};

const ethScope: CoinScope = {
  channelId: 'eth.ETH',
  coinId: 'ETH',
  address: sourceAddress,
  addressLabel: 'Ethereum address',
  systemId: 'ethereum',
  systemTicker: 'ETH',
  systemDisplayName: 'Ethereum',
  isPrimaryAddress: true,
  isReadOnly: false,
  scopeKind: 'transparent',
};

function preflightResult(preflightId: string, fee: string): PreflightResult {
  return {
    preflightId,
    fee,
    feeCurrency: 'ETH',
    value: '0.25',
    amountSubmitted: '0.25',
    toAddress: destinationAddress,
    fromAddress: sourceAddress,
    feeTakenFromAmount: false,
    warnings: [],
    feeMode: 'standard',
  };
}

function deferred<T>(): {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (error: unknown) => void;
} {
  let resolve: (value: T) => void = () => {};
  let reject: (error: unknown) => void = () => {};
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

async function settle(): Promise<void> {
  for (let index = 0; index < 4; index += 1) await Promise.resolve();
  await tick();
}

async function waitFor(
  predicate: () => boolean,
  description: string,
  details: () => string = () => ''
): Promise<void> {
  for (let attempt = 0; attempt < 30; attempt += 1) {
    await settle();
    if (predicate()) return;
  }
  throw new Error(`Timed out waiting for ${description}: ${details()}`);
}

function click(target: HTMLElement, selector: string): void {
  const element = target.querySelector(selector) as HTMLButtonElement | null;
  expect(element, selector).not.toBeNull();
  element?.click();
}

function enter(target: HTMLElement, selector: string, value: string): void {
  const input = target.querySelector(selector) as HTMLInputElement | null;
  expect(input, selector).not.toBeNull();
  if (!input) return;
  input.value = value;
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

function clickReviewSend(target: HTMLElement): void {
  const buttons = Array.from(target.querySelectorAll('button'));
  const button = buttons.find(
    (candidate) => candidate.textContent?.trim() === 'Review send' && !candidate.disabled
  );
  if (!button) {
    throw new Error(
      `Enabled Review send button missing: ${JSON.stringify(
        buttons.map((candidate) => ({
          text: candidate.textContent?.trim(),
          disabled: candidate.disabled,
        }))
      )}`
    );
  }
  button?.click();
}

async function fillAndStartPreflight(target: HTMLElement): Promise<void> {
  await waitFor(() => target.querySelector('#transfer-amount') !== null, 'the transfer form');
  if (target.textContent?.includes('Choose currency')) {
    const chooseCurrency = Array.from(target.querySelectorAll('button')).find(
      (candidate) => candidate.textContent?.trim() === 'Choose currency'
    );
    expect(chooseCurrency).toBeDefined();
    chooseCurrency?.click();
    await waitFor(
      () =>
        Array.from(document.querySelectorAll('button')).some((candidate) =>
          candidate.textContent?.includes('Ethereum')
        ),
      'the production source picker'
    );
    const source = Array.from(document.querySelectorAll('button')).find((candidate) =>
      candidate.textContent?.includes('Ethereum')
    );
    expect(source).toBeDefined();
    source?.click();
    await waitFor(
      () => !target.textContent?.includes('Choose currency'),
      'the production source selection'
    );
  }
  enter(target, '#transfer-amount', '0.25');
  enter(target, '#transfer-recipient', destinationAddress);
  await settle();
  clickReviewSend(target);
  await settle();
}

beforeEach(() => {
  class ResizeObserverStub {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  }
  Object.defineProperty(globalThis, 'ResizeObserver', {
    configurable: true,
    value: ResizeObserverStub,
  });
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
  localeStore.set('en');
  coinsStore.set([ethCoin]);
  balanceStore.set({
    'eth.ETH': {
      ETH: { confirmed: '1', pending: '0', total: '1' },
    },
  });
  networkStore.set({});
  ratesStore.set({});
  addressBookStore.set([]);
  mocks.preflightSend.mockReset();
  mocks.getPendingEthSubmission.mockReset().mockResolvedValue(null);
  mocks.getDisplayCoinScopes.mockReset().mockResolvedValue({ coinId: 'ETH', scopes: [ethScope] });
  mocks.getAddresses.mockReset().mockResolvedValue({
    vrsc_address: '',
    eth_address: sourceAddress,
    btc_address: '',
  });
  mocks.getBridgeCapabilities.mockReset().mockResolvedValue({
    conversionSupported: false,
    executionEngine: 'none',
    reasonCode: 'unsupported_channel',
  });
});

describe('production transfer preflight session lifecycle', () => {
  it.each([
    {
      context: 'wallet',
      replacementSelector: '[data-replace-wallet]',
      outcome: 'success',
      failOld: false,
    },
    {
      context: 'wallet',
      replacementSelector: '[data-replace-wallet]',
      outcome: 'failure',
      failOld: true,
    },
    {
      context: 'network',
      replacementSelector: '[data-replace-network]',
      outcome: 'success',
      failOld: false,
    },
    {
      context: 'network',
      replacementSelector: '[data-replace-network]',
      outcome: 'failure',
      failOld: true,
    },
    {
      context: 'same-wallet session',
      replacementSelector: '[data-replace-session]',
      outcome: 'success',
      failOld: false,
    },
    {
      context: 'same-wallet session',
      replacementSelector: '[data-replace-session]',
      outcome: 'failure',
      failOld: true,
    },
  ])(
    'keeps a late old-session $outcome stale after a $context change',
    async ({ replacementSelector, failOld }) => {
      const oldRequest = deferred<PreflightResult>();
      mocks.preflightSend
        .mockReset()
        .mockImplementationOnce(() => oldRequest.promise)
        .mockResolvedValueOnce(preflightResult('new-preflight', '0.00042'));

      const target = document.createElement('div');
      document.body.append(target);
      const component = mount(WalletLayoutLifecycleHarness, { target });
      const disposeSpy = vi.spyOn(PreflightRequestGuard.prototype, 'dispose');
      try {
        await settle();
        click(target, '[data-open-production-send]');
        await fillAndStartPreflight(target);
        const oldAmountInput = target.querySelector('#transfer-amount');
        expect(oldAmountInput).not.toBeNull();

        click(target, replacementSelector);
        await fillAndStartPreflight(target);
        expect(target.querySelector('#transfer-amount')).not.toBe(oldAmountInput);
        expect(disposeSpy).toHaveBeenCalledTimes(1);
        expect(target.textContent).toContain('0.00042 ETH');
        expect(mocks.preflightSend).toHaveBeenCalledTimes(2);
        expect(mocks.preflightSend.mock.calls[0]).toEqual(mocks.preflightSend.mock.calls[1]);

        if (failOld) oldRequest.reject(new Error('old provider failed'));
        else oldRequest.resolve(preflightResult('old-preflight', '0.00999999'));
        await settle();

        expect(target.textContent).toContain('0.00042 ETH');
        expect(target.textContent).not.toContain('0.00999999');
        expect(target.textContent).not.toContain('old provider failed');
      } finally {
        await unmount(component);
        target.remove();
        disposeSpy.mockRestore();
      }
    }
  );
});
