// @vitest-environment jsdom
import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

const walletService = vi.hoisted(() => ({
  isUnlocked: vi.fn(),
  getActiveWallet: vi.fn(),
  getAddresses: vi.fn(),
  getActiveAssets: vi.fn(),
  setSessionTimeoutMinutes: vi.fn(),
  startUpdateEngine: vi.fn(),
}));
vi.mock('$lib/services/walletService.js', () => walletService);

vi.mock('$lib/services/coinsService.js', () => ({ getCoinRegistry: vi.fn(async () => []) }));
vi.mock('$lib/services/eventBridge.js', () => ({
  setupWalletEventBridge: vi.fn(async () => () => {}),
}));
vi.mock('$lib/services/walletActivityMonitor.js', () => ({
  startWalletActivityMonitor: vi.fn(() => () => {}),
}));
const contactsService = vi.hoisted(() => ({ loadContacts: vi.fn(async () => []) }));
vi.mock('$lib/contacts/service', () => contactsService);
const watchlistService = vi.hoisted(() => ({ loadWatchlistEntries: vi.fn(async () => []) }));
vi.mock('$lib/watchlist/service', () => watchlistService);
vi.mock('$lib/components/wallet/WalletLayout.svelte', async () => ({
  default: (await import('./__tests__/WalletLayoutStub.svelte')).default,
}));

const invokeMock = vi.hoisted(() => vi.fn(async () => undefined));
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import WalletPage from './+page.svelte';
import { navigation } from './__tests__/navigationStub';
import { contactSession, setContactSession } from '$lib/contacts/session';
import { resetWatchlistSession, watchlistSession } from '$lib/watchlist/session';
import { forceWalletToUnlock } from '$lib/services/walletLockCoordinator';

const activeWallet = {
  wallet_name: 'Wallet A',
  network: 'mainnet' as const,
  emoji: '🦊',
  color: 'blue',
  session_id: 'session-a',
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

let component: ReturnType<typeof mount>;

async function settle(): Promise<void> {
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 0));
  await tick();
}

async function render(): Promise<void> {
  const target = document.createElement('div');
  document.body.append(target);
  component = mount(WalletPage, { target });
  await settle();
}

beforeEach(() => {
  vi.clearAllMocks();
  navigation.calls = [];
  navigation.failNextGoto = false;
  invokeMock.mockResolvedValue(undefined);
  setContactSession(null);
  resetWatchlistSession();
  walletService.isUnlocked.mockResolvedValue(true);
  walletService.getActiveWallet
    .mockRejectedValueOnce(new Error('transient metadata failure'))
    .mockResolvedValue(activeWallet);
  walletService.getAddresses.mockResolvedValue({
    vrsc_address: '',
    eth_address: '',
    btc_address: '',
  });
  walletService.getActiveAssets.mockResolvedValue({ coinIds: [] });
  walletService.setSessionTimeoutMinutes.mockResolvedValue(0);
  walletService.startUpdateEngine.mockResolvedValue(undefined);
});

afterEach(async () => {
  if (component) await unmount(component);
  await settle();
  document.body.replaceChildren();
  setContactSession(null);
  resetWatchlistSession();
});

describe('wallet route fallback session retry', () => {
  it('recovers the real wallet session and route bindings from the fallback Retry', async () => {
    await render();

    // Fallback metadata never binds a placeholder session.
    expect(walletService.getActiveWallet).toHaveBeenCalledTimes(1);
    expect(document.querySelector('[data-testid="fallback-wallet-session"]')?.textContent).toBe(
      'unavailable'
    );
    expect(get(contactSession)).toBeNull();
    expect(get(watchlistSession)).toBeNull();
    expect(contactsService.loadContacts).not.toHaveBeenCalled();
    expect(watchlistService.loadWatchlistEntries).not.toHaveBeenCalled();

    document.querySelector<HTMLButtonElement>('[data-testid="fallback-retry-session"]')?.click();
    await settle();

    expect(walletService.getActiveWallet).toHaveBeenCalledTimes(2);
    expect(document.querySelector('[data-testid="fallback-wallet-session"]')?.textContent).toBe(
      'session-a'
    );
    expect(get(contactSession)).toEqual({ sessionId: 'session-a', network: 'mainnet' });
    expect(get(watchlistSession)).toEqual({ sessionId: 'session-a', network: 'mainnet' });
    expect(contactsService.loadContacts).toHaveBeenCalledTimes(1);
  });

  it('keeps the fallback sessionless state when no wallet can be verified', async () => {
    walletService.getActiveWallet.mockReset();
    walletService.getActiveWallet.mockResolvedValue(null);

    await render();
    document.querySelector<HTMLButtonElement>('[data-testid="fallback-retry-session"]')?.click();
    await settle();

    expect(walletService.getActiveWallet).toHaveBeenCalledTimes(2);
    expect(document.querySelector('[data-testid="fallback-wallet-session"]')?.textContent).toBe(
      'unavailable'
    );
    expect(get(contactSession)).toBeNull();
    expect(get(watchlistSession)).toBeNull();
    expect(contactsService.loadContacts).not.toHaveBeenCalled();
  });

  it('does not bind a session when recovery fails', async () => {
    walletService.getActiveWallet.mockReset();
    walletService.getActiveWallet
      .mockRejectedValueOnce(new Error('transient metadata failure'))
      .mockRejectedValueOnce(new Error('still unavailable'));

    await render();
    document.querySelector<HTMLButtonElement>('[data-testid="fallback-retry-session"]')?.click();
    await settle();

    expect(document.querySelector('[data-testid="fallback-wallet-session"]')?.textContent).toBe(
      'unavailable'
    );
    expect(get(contactSession)).toBeNull();
    expect(get(watchlistSession)).toBeNull();
  });

  it('does not rebind a recovery that resolves after a forced lock with failed navigation', async () => {
    const recovery = deferred<typeof activeWallet>();
    walletService.getActiveWallet.mockReset();
    walletService.getActiveWallet
      .mockRejectedValueOnce(new Error('transient metadata failure'))
      .mockReturnValueOnce(recovery.promise);

    await render();
    document.querySelector<HTMLButtonElement>('[data-testid="fallback-retry-session"]')?.click();
    await settle();
    expect(walletService.getActiveWallet).toHaveBeenCalledTimes(2);

    // Force a lock whose navigation fails, so the route stays mounted and the
    // redirecting flag returns to false before the old recovery resolves.
    navigation.failNextGoto = true;
    await forceWalletToUnlock();
    expect(navigation.calls).toHaveLength(1);

    recovery.resolve(activeWallet);
    await settle();

    expect(document.querySelector('[data-testid="fallback-wallet-session"]')?.textContent).toBe(
      'unavailable'
    );
    expect(get(contactSession)).toBeNull();
    expect(get(watchlistSession)).toBeNull();
    expect(contactsService.loadContacts).not.toHaveBeenCalled();
  });

  it('does not bind boot metadata that resolves after a forced lock with failed navigation', async () => {
    const boot = deferred<typeof activeWallet>();
    walletService.getActiveWallet.mockReset();
    walletService.getActiveWallet.mockReturnValueOnce(boot.promise);

    await render();
    navigation.failNextGoto = true;
    await forceWalletToUnlock();

    boot.resolve(activeWallet);
    await settle();

    expect(document.querySelector('[data-testid="fallback-wallet-session"]')?.textContent).toBe(
      'unavailable'
    );
    expect(get(contactSession)).toBeNull();
    expect(get(watchlistSession)).toBeNull();
    expect(contactsService.loadContacts).not.toHaveBeenCalled();
    expect(walletService.startUpdateEngine).not.toHaveBeenCalled();
  });

  it('recovers a new verified wallet after an earlier forced lock', async () => {
    walletService.getActiveWallet.mockReset();
    walletService.getActiveWallet
      .mockRejectedValueOnce(new Error('transient metadata failure'))
      .mockResolvedValueOnce({ ...activeWallet, session_id: 'session-b' });

    await render();
    navigation.failNextGoto = true;
    await forceWalletToUnlock();

    // A later unlock makes the backend report a new verified session.
    document.querySelector<HTMLButtonElement>('[data-testid="fallback-retry-session"]')?.click();
    await settle();

    expect(document.querySelector('[data-testid="fallback-wallet-session"]')?.textContent).toBe(
      'session-b'
    );
    expect(get(contactSession)).toEqual({ sessionId: 'session-b', network: 'mainnet' });
    expect(get(watchlistSession)).toEqual({ sessionId: 'session-b', network: 'mainnet' });
    expect(contactsService.loadContacts).toHaveBeenCalledTimes(1);
  });
});
