// @vitest-environment jsdom
import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '$lib/i18n';
import type {
  WatchlistEntry,
  WatchlistEntrySnapshot,
  WatchlistRefreshResult,
} from '$lib/types/watchlist.js';
import Watchlist from './Watchlist.svelte';
import WatchlistLifecycleHarness from './test-fixtures/WatchlistLifecycleHarness.svelte';

const service = vi.hoisted(() => ({
  getWatchlistEntries: vi.fn(),
  resolveWatchlistTarget: vi.fn(),
  addWatchlistEntry: vi.fn(),
  removeWatchlistEntry: vi.fn(),
  refreshWatchlist: vi.fn(),
}));
vi.mock('$lib/services/watchlistService.js', () => service);

class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
vi.stubGlobal('ResizeObserver', ResizeObserverStub);
HTMLElement.prototype.hasPointerCapture = () => false;
HTMLElement.prototype.setPointerCapture = () => {};
HTMLElement.prototype.releasePointerCapture = () => {};

const entry: WatchlistEntry = {
  id: 'alice',
  targetKind: 'identity',
  displayName: 'Alice@',
  address: `R${'a'.repeat(33)}`,
  systemId: null,
  createdAt: 1,
  updatedAt: 1,
};

const snapshot: WatchlistEntrySnapshot = {
  entry,
  holdings: [
    {
      assetKey: 'vrsc',
      currencyId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
      systemId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
      systemTicker: 'VRSC',
      systemDisplayName: 'Verus',
      balance: '4.25',
      coin: {
        id: 'VRSC',
        currencyId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
        systemId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
        displayTicker: 'VRSC',
        displayName: 'Verus',
        proto: 'vrsc',
        compatibleChannels: ['vrpc'],
        decimals: 8,
        vrpcEndpoints: [],
        secondsPerBlock: 60,
        isTestnet: false,
      },
    },
  ],
  sources: [],
  availability: 'available',
  refreshedAt: 2,
};

const otherEntry: WatchlistEntry = {
  ...entry,
  id: 'bob',
  displayName: 'Bob@',
  address: `R${'b'.repeat(33)}`,
  createdAt: 2,
  updatedAt: 2,
};

const otherSnapshot: WatchlistEntrySnapshot = {
  ...snapshot,
  entry: otherEntry,
  holdings: snapshot.holdings.map((holding) => ({
    ...holding,
    assetKey: `bob-${holding.assetKey}`,
    balance: '2',
  })),
};

function refreshResult(entries = [snapshot]): WatchlistRefreshResult {
  return { network: 'mainnet', entries, refreshedAt: 2 };
}

let component: ReturnType<typeof mount>;

async function settle(): Promise<void> {
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 0));
  await tick();
}

function button(label: string, root: Document | Element | null = document.body): HTMLButtonElement {
  if (!root) throw new Error('Missing button container');
  const match = [...root.querySelectorAll<HTMLButtonElement>('button')].find(
    (element) =>
      element.textContent?.trim() === label || element.getAttribute('aria-label') === label
  );
  if (!match) throw new Error(`Missing button: ${label}`);
  return match;
}

async function setInput(selector: string, value: string): Promise<void> {
  const input = document.querySelector<HTMLInputElement>(selector);
  if (!input) throw new Error(`Missing input: ${selector}`);
  input.value = value;
  input.dispatchEvent(new Event('input', { bubbles: true }));
  await settle();
}

async function render(): Promise<void> {
  const target = document.createElement('div');
  document.body.append(target);
  component = mount(Watchlist, { target, props: { walletNetwork: 'mainnet' } });
  await settle();
}

beforeEach(() => {
  vi.clearAllMocks();
  setLocale('en');
  service.getWatchlistEntries.mockResolvedValue([]);
  service.refreshWatchlist.mockResolvedValue(refreshResult([]));
  service.removeWatchlistEntry.mockResolvedValue(true);
});

afterEach(async () => {
  if (component) await unmount(component);
  await settle();
  document.body.replaceChildren();
});

describe('watchlist workflows', () => {
  it('shows the resolved preview and only adds the entry after persistence succeeds', async () => {
    service.getWatchlistEntries.mockResolvedValue([otherEntry]);
    service.refreshWatchlist.mockResolvedValue(refreshResult([otherSnapshot]));
    service.resolveWatchlistTarget.mockResolvedValue({
      targetKind: 'identity',
      displayName: entry.displayName,
      address: entry.address,
      systemId: null,
      visibleCurrencyCount: 1,
      availability: 'available',
    });
    let finishAdd!: (value: WatchlistEntrySnapshot) => void;
    service.addWatchlistEntry.mockImplementation(
      () =>
        new Promise((resolve) => {
          finishAdd = resolve;
        })
    );

    await render();
    expect(document.querySelectorAll('[data-testid="watchlist-entry"]')).toHaveLength(1);
    button('Add').click();
    await settle();
    await setInput('#watchlist-target', 'Alice@');
    button('Continue').click();
    await settle();
    expect(service.resolveWatchlistTarget).toHaveBeenCalledWith('Alice@');
    expect(document.querySelector('[data-testid="watchlist-resolved-preview"]')).not.toBeNull();
    button('Add to watchlist').click();
    await settle();
    expect(service.addWatchlistEntry).toHaveBeenCalledWith('Alice@');
    expect(document.querySelectorAll('[data-testid="watchlist-entry"]')).toHaveLength(1);
    expect(button('Adding…').disabled).toBe(true);

    finishAdd(snapshot);
    await settle();
    const entries = document.querySelectorAll('[data-testid="watchlist-entry"]');
    expect(entries).toHaveLength(2);
    expect(entries[0]?.textContent).toContain('Alice@');
    expect(entries[1]?.textContent).toContain('Bob@');
  });

  it('maps identity lookup failures to the VerusID not-found message', async () => {
    service.resolveWatchlistTarget.mockRejectedValue({ type: 'IdentityNotFound' });

    await render();
    button('Add address').click();
    await settle();
    await setInput('#watchlist-target', 'Missing@');
    button('Continue').click();
    await settle();

    expect(document.querySelector('[role="alert"]')?.textContent).toContain(
      'That VerusID could not be found.'
    );
  });

  it('keeps the last successful balances visible when a later refresh fails', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry]);
    service.refreshWatchlist
      .mockResolvedValueOnce(refreshResult())
      .mockRejectedValueOnce(new Error('offline'));

    await render();
    expect(document.body.textContent).toContain('4.25');
    button('Refresh').click();
    await settle();
    expect(document.body.textContent).toContain('Update unavailable');
    expect(document.body.textContent).toContain('4.25');
  });

  it('opens a restrained detail view without an identity avatar', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry]);
    service.refreshWatchlist.mockResolvedValue(refreshResult());

    await render();
    document.querySelector<HTMLButtonElement>('[data-testid="watchlist-entry"]')?.click();
    await settle();
    expect(document.body.textContent).toContain('Read only');
    expect(document.querySelector('[data-testid="public-value-card"]')).not.toBeNull();
    expect(document.body.textContent).toContain('4.25');
    expect(document.body.textContent).toContain('VRSC');
    expect(document.querySelector('[data-testid="identity-avatar"]')).toBeNull();
  });

  it('keeps the entry until confirmed removal finishes', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry]);
    service.refreshWatchlist.mockResolvedValue(refreshResult());
    let finishRemove!: (value: boolean) => void;
    service.removeWatchlistEntry.mockImplementation(
      () =>
        new Promise((resolve) => {
          finishRemove = resolve;
        })
    );

    await render();
    document.querySelector<HTMLButtonElement>('[data-testid="watchlist-entry"]')?.click();
    await settle();
    button('More actions').click();
    await settle();
    const removeItem = document.querySelector<HTMLElement>('[role="menuitem"]');
    expect(removeItem?.textContent).toContain('Remove from watchlist');
    removeItem?.click();
    await settle();
    button('Remove from watchlist', document.querySelector('[role="dialog"]')).click();
    await settle();
    expect(document.body.textContent).toContain('Alice@');

    finishRemove(true);
    await settle();
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    expect(document.querySelector('[data-testid="watchlist-empty"]')).not.toBeNull();
  });

  it('uses removal copy when the persisted entry is no longer found', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry]);
    service.refreshWatchlist.mockResolvedValue(refreshResult());
    service.removeWatchlistEntry.mockRejectedValue({ type: 'WatchlistEntryNotFound' });

    await render();
    document.querySelector<HTMLButtonElement>('[data-testid="watchlist-entry"]')?.click();
    await settle();
    button('More actions').click();
    await settle();
    document.querySelector<HTMLElement>('[role="menuitem"]')?.click();
    await settle();
    button('Remove from watchlist', document.querySelector('[role="dialog"]')).click();
    await settle();

    const dialogText = document.querySelector('[role="dialog"]')?.textContent ?? '';
    expect(dialogText).toContain('Could not remove this address right now.');
    expect(dialogText).not.toContain('That VerusID could not be found.');
    expect(document.body.textContent).toContain('Alice@');
  });

  it('remounts for a new wallet session and ignores the old hydration result', async () => {
    let finishOldHydration!: (value: WatchlistEntry[]) => void;
    service.getWatchlistEntries
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finishOldHydration = resolve;
          })
      )
      .mockResolvedValueOnce([otherEntry]);
    service.refreshWatchlist.mockResolvedValue({
      network: 'testnet',
      entries: [otherSnapshot],
      refreshedAt: 3,
    });

    const target = document.createElement('div');
    document.body.append(target);
    component = mount(WatchlistLifecycleHarness, { target });
    await settle();
    expect(service.getWatchlistEntries).toHaveBeenCalledTimes(1);

    document.querySelector<HTMLButtonElement>('[data-testid="switch-wallet"]')?.click();
    await settle();
    expect(service.getWatchlistEntries).toHaveBeenCalledTimes(2);
    expect(document.body.textContent).toContain('Bob@');
    expect(document.querySelector('[data-testid="wallet-key"]')?.textContent).toBe(
      'wallet b::testnet::session-b'
    );

    finishOldHydration([entry]);
    await settle();
    expect(document.body.textContent).toContain('Bob@');
    expect(document.body.textContent).not.toContain('Alice@');
  });
});
