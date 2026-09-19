// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { CoinDefinition, CoinScope, PreflightResult, SendResult } from '$lib/types/wallet';

const mocks = vi.hoisted(() => ({
  preflightSend: vi.fn(),
  sendTransaction: vi.fn(),
  forceWalletToUnlock: vi.fn(),
  getPendingEthSubmission: vi.fn(),
  getDisplayCoinScopes: vi.fn(),
  getAddresses: vi.fn(),
  getDlightRuntimeStatus: vi.fn(),
  getBridgeCapabilities: vi.fn(),
}));

vi.mock('$lib/services/txService.js', () => ({
  acknowledgePendingEthSubmission: vi.fn(),
  getPendingEthSubmission: mocks.getPendingEthSubmission,
  preflightSend: mocks.preflightSend,
  resumePendingEthSubmission: vi.fn(),
  sendTransaction: mocks.sendTransaction,
}));

vi.mock('$lib/services/walletDisplayService.js', () => ({
  getDisplayCoinScopes: mocks.getDisplayCoinScopes,
  invalidateWalletDisplayScopes: vi.fn(),
  isWalletDisplayRequestInvalidated: () => false,
}));

vi.mock('$lib/services/walletService.js', () => ({
  getAddresses: mocks.getAddresses,
  getDlightRuntimeStatus: mocks.getDlightRuntimeStatus,
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
  forceWalletToUnlock: mocks.forceWalletToUnlock,
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
import TransferWizard from '../TransferWizard.svelte';
import WalletLayoutLifecycleHarness from './test-fixtures/WalletLayoutLifecycleHarness.svelte';

const sourceAddress = `0x${'11'.repeat(20)}`;
const destinationAddress = `0x${'22'.repeat(20)}`;
const privateDestinationAddress = 'RAutMoGh771ECTDbTq2qwwZo7MF5Tov3ka';
const privateSourceAddress = 'zs1syntheticprivateaddressforrenderingonly';
const vrscSystemId = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
const privateChannelId = `dlight_private.${privateSourceAddress}.${vrscSystemId}`;

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

const vrscCoin: CoinDefinition = {
  id: 'VRSC',
  currencyId: vrscSystemId,
  systemId: vrscSystemId,
  displayTicker: 'VRSC',
  displayName: 'Verus',
  proto: 'vrsc',
  compatibleChannels: ['vrpc', 'dlight_private'],
  decimals: 8,
  vrpcEndpoints: [],
  secondsPerBlock: 60,
  isTestnet: false,
};

const privateScope: CoinScope = {
  channelId: privateChannelId,
  coinId: 'VRSC',
  address: privateSourceAddress,
  addressLabel: 'Private address',
  systemId: vrscSystemId,
  systemTicker: 'VRSC',
  systemDisplayName: 'Verus',
  isPrimaryAddress: false,
  isReadOnly: false,
  scopeKind: 'shielded',
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

function privatePreflightResult(): PreflightResult {
  return {
    preflightId: 'private-preflight',
    fee: '0.0001',
    feeCurrency: 'VRSC',
    value: '0.2499',
    amountSubmitted: '0.25',
    toAddress: privateDestinationAddress,
    fromAddress: privateSourceAddress,
    feeTakenFromAmount: true,
    warnings: [],
    feeMode: null,
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

function configurePrivateSend(): void {
  coinsStore.set([vrscCoin]);
  balanceStore.set({
    [privateChannelId]: {
      VRSC: { confirmed: '0.25', pending: '0', total: '0.25' },
    },
  });
  ratesStore.set({
    VRSC: { rates: { USD: 100 }, usdChange24hPct: null },
  });
  mocks.getDisplayCoinScopes.mockResolvedValue({ coinId: 'VRSC', scopes: [privateScope] });
  mocks.getAddresses.mockResolvedValue({
    vrsc_address: privateDestinationAddress,
    eth_address: '',
    btc_address: '',
  });
  mocks.getDlightRuntimeStatus.mockResolvedValue({
    channelId: privateChannelId,
    runtimeKey: 'synthetic-runtime',
    statusKind: 'synced',
    scannedHeight: 100,
    tipHeight: 100,
    syncing: false,
    lastUpdated: 1,
    consecutiveFailures: 0,
    stalled: false,
    spendCacheReady: true,
  });
}

async function mountPrivateSend(target: HTMLElement) {
  const component = mount(TransferWizard, {
    target,
    props: {
      entryIntent: 'send',
      entryContext: {
        coinId: 'VRSC',
        channelId: privateChannelId,
        scopeKind: 'shielded',
        readOnly: false,
      },
      walletNetwork: 'mainnet',
      walletKey: 'synthetic-wallet',
    },
  });
  await waitFor(() => target.querySelector('#transfer-amount') !== null, 'the private send form');
  enter(target, '#transfer-amount', '0.25');
  enter(target, '#transfer-recipient', privateDestinationAddress);
  await settle();
  return component;
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
  mocks.sendTransaction.mockReset();
  mocks.forceWalletToUnlock.mockReset().mockResolvedValue(undefined);
  mocks.getPendingEthSubmission.mockReset().mockResolvedValue(null);
  mocks.getDisplayCoinScopes.mockReset().mockResolvedValue({ coinId: 'ETH', scopes: [ethScope] });
  mocks.getAddresses.mockReset().mockResolvedValue({
    vrsc_address: '',
    eth_address: sourceAddress,
    btc_address: '',
  });
  mocks.getDlightRuntimeStatus.mockReset().mockResolvedValue(null);
  mocks.getBridgeCapabilities.mockReset().mockResolvedValue({
    conversionSupported: false,
    executionEngine: 'none',
    reasonCode: 'unsupported_channel',
  });
});

describe('transfer review refinements', () => {
  it('keeps immediate private-preflight feedback on Details and prevents duplicate activation', async () => {
    configurePrivateSend();
    const pendingPreflight = deferred<PreflightResult>();
    mocks.preflightSend.mockImplementation(() => pendingPreflight.promise);
    const consoleSpy = vi.spyOn(console, 'info').mockImplementation(() => {});
    const target = document.createElement('div');
    document.body.append(target);
    const component = await mountPrivateSend(target);

    try {
      clickReviewSend(target);
      await settle();

      const preparingButton = Array.from(target.querySelectorAll('button')).find((button) =>
        button.textContent?.includes('Preparing review…')
      );
      expect(preparingButton).toBeDefined();
      expect(preparingButton?.disabled).toBe(true);
      expect(preparingButton?.getAttribute('aria-busy')).toBe('true');
      expect(target.querySelector('[data-transfer-preflight-spinner]')).not.toBeNull();
      expect(target.querySelector('#transfer-amount')).not.toBeNull();
      expect(target.textContent).not.toContain('Review send');
      preparingButton?.click();
      await settle();
      expect(mocks.preflightSend).toHaveBeenCalledTimes(1);

      pendingPreflight.resolve(privatePreflightResult());
      await waitFor(() => target.textContent?.includes('Network fee') === true, 'private review');
    } finally {
      await unmount(component);
      target.remove();
      consoleSpy.mockRestore();
    }
  });

  it('renders one private network fee and places the Max adjustment beside the amount', async () => {
    configurePrivateSend();
    mocks.preflightSend.mockResolvedValue(privatePreflightResult());
    const consoleSpy = vi.spyOn(console, 'info').mockImplementation(() => {});
    const target = document.createElement('div');
    document.body.append(target);
    const component = await mountPrivateSend(target);

    try {
      clickReviewSend(target);
      await waitFor(
        () => target.querySelector('[data-transfer-fee-kind="network"]') !== null,
        'private review'
      );

      const networkFee = target.querySelector('[data-transfer-fee-kind="network"]');
      expect(networkFee?.textContent).toContain('0.0001 VRSC');
      expect(networkFee?.textContent).toContain('≈ $0.01');
      expect(target.querySelector('[data-transfer-fee-kind="total"]')).toBeNull();

      const amountNotice = target.querySelector('[data-transfer-amount-adjustment]');
      expect(amountNotice?.textContent).toContain('0.25 → 0.2499 VRSC');
      expect(amountNotice?.closest('[data-transfer-review-amount]')).not.toBeNull();
      expect(target.textContent).not.toContain('Warnings');

      const totalDebited = target.querySelector('[data-transfer-total-debited]');
      expect(totalDebited?.textContent).toContain('0.25 VRSC');
      expect(target.querySelector('[data-transfer-source-metadata]')?.className).not.toContain(
        'font-semibold'
      );
      expect(target.textContent).not.toContain('Change details');
      expect(target.textContent).toContain('Back');
    } finally {
      await unmount(component);
      target.remove();
      consoleSpy.mockRestore();
    }
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
        await settle();
        expect(oldAmountInput?.isConnected).toBe(false);
        expect(target.querySelector('#transfer-amount')).toBeNull();
        click(target, '[data-open-production-send]');
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

function buttonNamed(name: string, root: Document | Element = document): HTMLButtonElement {
  const button = [...root.querySelectorAll<HTMLButtonElement>('button')].find(
    (item) => item.textContent?.trim() === name
  );
  if (!button) throw new Error(`Missing button: ${name}`);
  return button;
}

async function mountNavigationDraft() {
  const target = document.createElement('div');
  document.body.append(target);
  const component = mount(WalletLayoutLifecycleHarness, { target });
  await settle();
  click(target, '[data-open-production-send]');
  await waitFor(() => target.querySelector('#transfer-amount') !== null, 'draft entry');
  return { target, component };
}

describe('sidebar transfer navigation', () => {
  it('retains exact inputs across sections, restores focus and isolates Escape', async () => {
    const { target, component } = await mountNavigationDraft();
    try {
      enter(target, '#transfer-amount', '0.125');
      enter(target, '#transfer-recipient', destinationAddress);
      await settle();
      const recipient = target.querySelector<HTMLInputElement>('#transfer-recipient');
      if (!recipient) throw new Error('Missing recipient field');
      recipient.focus();
      buttonNamed('Contacts', target).click();
      await settle();
      expect(recipient.closest('[hidden]')).not.toBeNull();
      expect((recipient.closest('[hidden]') as HTMLElement).inert).toBe(true);
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
      await settle();
      expect(document.body.textContent).not.toContain('Discard transfer?');
      buttonNamed('Activity', target).click();
      await settle();
      buttonNamed('Back to send', target).click();
      await settle();
      expect(target.querySelector('#transfer-recipient')).toBe(recipient);
      expect(recipient.value).toBe(destinationAddress);
      expect(target.querySelector<HTMLInputElement>('#transfer-amount')?.value).toBe('0.125');
      expect(document.activeElement).toBe(recipient);
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('requires explicit draft replacement when another entry point starts a payment', async () => {
    const { target, component } = await mountNavigationDraft();
    try {
      enter(target, '#transfer-amount', '0.125');
      await settle();
      const input = target.querySelector('#transfer-amount');
      buttonNamed('Wallet', target.querySelector('[data-sidebar="sidebar"]') ?? target).click();
      await settle();
      click(target, '[data-open-production-send]');
      await settle();
      expect(document.body.textContent).toContain('Resume your transfer?');
      buttonNamed('Resume transfer').click();
      await settle();
      expect(target.querySelector('#transfer-amount')).toBe(input);
      buttonNamed('Wallet', target.querySelector('[data-sidebar="sidebar"]') ?? target).click();
      await settle();
      click(target, '[data-open-production-send]');
      await settle();
      buttonNamed('Start new transfer').click();
      await settle();
      expect(input?.isConnected).toBe(false);
      expect(target.querySelector<HTMLInputElement>('#transfer-amount')?.value).toBe('');
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('uses the same discard protection for footer Cancel and Escape', async () => {
    const { target, component } = await mountNavigationDraft();
    try {
      enter(target, '#transfer-amount', '0.125');
      await settle();
      buttonNamed('Cancel', target).click();
      await settle();
      expect(document.body.textContent).toContain('Discard transfer?');
      const dialog = document.querySelector('[role="dialog"]');
      if (!dialog) throw new Error('Missing discard dialog');
      buttonNamed('Cancel', dialog).click();
      await settle();
      expect(target.querySelector<HTMLInputElement>('#transfer-amount')?.value).toBe('0.125');
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
      await settle();
      expect(document.body.textContent).toContain('Discard transfer?');
      buttonNamed('Discard transfer').click();
      await settle();
      expect(target.querySelector('#transfer-amount')).toBeNull();
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('requires a new review after returning, then keeps navigation visible but blocked during submission', async () => {
    mocks.preflightSend
      .mockResolvedValueOnce(preflightResult('first', '0.0001'))
      .mockResolvedValueOnce(preflightResult('refreshed', '0.0002'));
    const submission = deferred<SendResult>();
    mocks.sendTransaction.mockReturnValue(submission.promise);
    const { target, component } = await mountNavigationDraft();
    try {
      await fillAndStartPreflight(target);
      buttonNamed('Contacts', target).click();
      await settle();
      buttonNamed('Back to send', target).click();
      await settle();
      expect(target.textContent).toContain('Refresh the review before sending.');
      expect(target.querySelector('[data-transfer-review-amount]')).not.toBeNull();
      expect(mocks.sendTransaction).not.toHaveBeenCalled();
      buttonNamed('Refresh review', target).click();
      await settle();
      expect(target.textContent).toContain('0.0002 ETH');
      expect(mocks.sendTransaction).not.toHaveBeenCalled();
      const ack = target.querySelector<HTMLButtonElement>('#review-unsaved-recipient-footer');
      ack?.click();
      await settle();
      buttonNamed('Send now', target).click();
      await settle();
      expect(mocks.sendTransaction).toHaveBeenCalledExactlyOnceWith({ preflightId: 'refreshed' });
      const contacts = buttonNamed('Contacts', target);
      expect(contacts.getAttribute('aria-disabled')).toBe('true');
      contacts.click();
      await settle();
      expect(target.querySelector('[data-transfer-review-amount]')?.closest('[hidden]')).toBeNull();
      expect(buttonNamed('Lock', target).disabled).toBe(false);
      submission.resolve({
        txid: 'f'.repeat(64),
        fromAddress: sourceAddress,
        toAddress: destinationAddress,
        value: '0.25',
        fee: '0.0002',
      });
      await settle();
      expect(contacts.getAttribute('aria-disabled')).toBe('false');
      contacts.click();
      await settle();
      expect(target.textContent).toContain('Back to transfer result');
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('ignores a suspended preflight completion without settling the new request', async () => {
    const oldRequest = deferred<PreflightResult>();
    const newRequest = deferred<PreflightResult>();
    mocks.preflightSend
      .mockReturnValueOnce(oldRequest.promise)
      .mockReturnValueOnce(newRequest.promise);
    const { target, component } = await mountNavigationDraft();
    try {
      await fillAndStartPreflight(target);
      buttonNamed('Contacts', target).click();
      await settle();
      buttonNamed('Back to send', target).click();
      await settle();
      clickReviewSend(target);
      await settle();
      oldRequest.resolve(preflightResult('old', '0.9999'));
      await settle();
      expect(target.textContent).not.toContain('0.9999');
      expect(buttonNamed('Preparing review…', target).disabled).toBe(true);
      newRequest.resolve(preflightResult('new', '0.0003'));
      await settle();
      expect(target.textContent).toContain('0.0003 ETH');
    } finally {
      await unmount(component);
      target.remove();
    }
  });
});
