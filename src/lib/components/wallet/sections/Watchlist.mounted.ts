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
    expect(document.querySelector('[data-testid="watchlist-empty"]')).not.toBeNull();
    button('Add address').click();
    await settle();
    await setInput('#watchlist-target', 'Alice@');
    button('Continue').click();
    await settle();
    expect(service.resolveWatchlistTarget).toHaveBeenCalledWith('Alice@');
    expect(document.querySelector('[data-testid="watchlist-resolved-preview"]')).not.toBeNull();
    button('Add to watchlist').click();
    await settle();
    expect(service.addWatchlistEntry).toHaveBeenCalledWith('Alice@');
    expect(document.querySelectorAll('[data-testid="watchlist-entry"]')).toHaveLength(0);
    expect(button('Adding…').disabled).toBe(true);

    finishAdd(snapshot);
    await settle();
    expect(document.querySelectorAll('[data-testid="watchlist-entry"]')).toHaveLength(1);
    expect(document.body.textContent).toContain('Alice@');
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
});
