// @vitest-environment jsdom
import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { writable } from 'svelte/store';
import { setLocale } from '$lib/i18n';
import { setContactSession } from '$lib/contacts/session';
import type { AddressBookContact } from '$lib/types/addressBook';
import { identityProfiles } from '$lib/contacts/profiles';
import { contactChainId, identityKey } from '$lib/contacts/identity';
import { ratesStore } from '$lib/stores/rates.js';
import { resetWatchlistSession, setWatchlistSession } from '$lib/watchlist/session.js';
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
const contactService = vi.hoisted(() => ({ loadContacts: vi.fn(), addIdentityContact: vi.fn() }));
vi.mock('$lib/contacts/service', () => contactService);
const addressService = vi.hoisted(() => ({ validateDestinationAddress: vi.fn() }));
vi.mock('$lib/services/addressBookService', () => addressService);
const profileMock = vi.hoisted(() => ({ loadIdentityProfile: vi.fn() }));
vi.mock('$lib/contacts/profiles', () => ({
  loadIdentityProfile: profileMock.loadIdentityProfile,
  identityProfiles: writable({}),
  profileImage: (profile: { avatar?: { value: { mimeType: string; base64: string } } } | null) =>
    profile?.avatar
      ? `data:${profile.avatar.value.mimeType};base64,${profile.avatar.value.base64}`
      : null,
}));

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

async function render(props: Record<string, unknown> = {}): Promise<void> {
  const target = document.createElement('div');
  document.body.append(target);
  component = mount(Watchlist, { target, props: { walletNetwork: 'mainnet', ...props } });
  await settle();
}

function retryButton(
  root: Document | Element | null = document.body
): HTMLButtonElement | undefined {
  if (!root) return undefined;
  return [...root.querySelectorAll<HTMLButtonElement>('button')].find(
    (element) => element.textContent?.trim() === 'Retry'
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  contactService.addIdentityContact.mockReset();
  contactService.loadContacts.mockReset();
  addressService.validateDestinationAddress.mockReset();
  profileMock.loadIdentityProfile.mockResolvedValue({
    state: 'unavailable',
    issues: [],
    revisionTxid: null,
  });
  setLocale('en');
  setContactSession(null);
  identityProfiles.set({});
  ratesStore.set({});
  resetWatchlistSession();
  setWatchlistSession({ sessionId: 'watchlist-test', network: 'mainnet' });
  service.getWatchlistEntries.mockResolvedValue([]);
  service.refreshWatchlist.mockResolvedValue(refreshResult([]));
  service.removeWatchlistEntry.mockResolvedValue(true);
  contactService.loadContacts.mockResolvedValue([]);
  addressService.validateDestinationAddress.mockImplementation(async ({ address }) => ({
    valid: true,
    normalizedAddress: address.trim(),
  }));
});

afterEach(async () => {
  if (component) await unmount(component);
  await settle();
  document.body.replaceChildren();
  setContactSession(null);
  identityProfiles.set({});
  ratesStore.set({});
  resetWatchlistSession();
});

describe('watchlist workflows', () => {
  const identityEntry: WatchlistEntry = {
    ...entry,
    address: `i${'a'.repeat(33)}`,
  };
  const addressEntry: WatchlistEntry = {
    ...entry,
    id: 'raw-address',
    targetKind: 'address',
    displayName: 'Mum',
    address: `R${'c'.repeat(33)}`,
  };

  it('checks membership and saves a VerusID from detail without removing the watch', async () => {
    setContactSession({ sessionId: 'watchlist-test', network: 'mainnet' });
    service.getWatchlistEntries.mockResolvedValue([identityEntry]);
    service.refreshWatchlist.mockResolvedValue(
      refreshResult([{ ...snapshot, entry: identityEntry }])
    );
    let finishSave!: (contact: AddressBookContact) => void;
    contactService.addIdentityContact.mockImplementation(
      () => new Promise((resolve) => (finishSave = resolve))
    );
    const saved = {
      id: 'saved-alice',
      displayName: identityEntry.displayName,
      note: null,
      endpoints: [],
      createdAt: 1,
      updatedAt: 1,
    };

    await render();
    document.querySelector<HTMLButtonElement>('[data-testid="watchlist-entry"]')?.click();
    await settle();
    expect(contactService.loadContacts).toHaveBeenCalled();
    button('Add to contacts').click();
    await settle();
    expect(contactService.addIdentityContact).toHaveBeenCalledWith({
      identityAddress: identityEntry.address,
      fullyQualifiedName: identityEntry.displayName,
      network: 'mainnet',
      chainId: contactChainId('mainnet'),
    });
    expect(button('Saving')).toBeDefined();
    expect(document.body.textContent).toContain('Remove from watchlist');
    finishSave(saved);
    await settle();
    const inContacts = button('In contacts');
    expect(inContacts.disabled).toBe(true);
    expect(inContacts.className).toContain('bg-contact-saved');
    expect(inContacts.querySelector('svg')).not.toBeNull();
    expect(button('View profile').disabled).toBe(false);
    inContacts.click();
    expect(contactService.addIdentityContact).toHaveBeenCalledTimes(1);
    expect(service.removeWatchlistEntry).not.toHaveBeenCalled();
  });

  it('does not report an old identity save as saved after the contact session is lost', async () => {
    setContactSession({ sessionId: 'watchlist-test', network: 'mainnet' });
    service.getWatchlistEntries.mockResolvedValue([identityEntry]);
    let finishSave!: (contact: AddressBookContact) => void;
    contactService.addIdentityContact.mockImplementation(
      () => new Promise((resolve) => (finishSave = resolve))
    );
    await render();
    document.querySelector<HTMLButtonElement>('[data-testid="watchlist-entry"]')?.click();
    await settle();
    button('Add to contacts').click();
    await settle();
    expect(button('Saving')).toBeDefined();

    setContactSession(null);
    finishSave({
      id: 'obsolete',
      displayName: 'Alice@',
      note: null,
      endpoints: [],
      createdAt: 1,
      updatedAt: 1,
    });
    await settle();
    expect(document.body.textContent).toContain('Couldn’t load contacts');
    expect(document.body.textContent).toContain('Remove from watchlist');
    expect(
      [...document.querySelectorAll('button')].some(
        (element) => element.textContent?.trim() === 'In contacts'
      )
    ).toBe(false);
  });

  it('recognizes an existing raw address by its normalized VRPC endpoint', async () => {
    setContactSession({ sessionId: 'watchlist-test', network: 'mainnet' });
    service.getWatchlistEntries.mockResolvedValue([addressEntry]);
    contactService.loadContacts.mockResolvedValue([
      {
        id: 'mum-contact',
        displayName: 'Mum',
        note: null,
        createdAt: 1,
        updatedAt: 1,
        endpoints: [
          {
            id: 'endpoint',
            kind: 'vrpc',
            address: ` ${addressEntry.address} `,
            normalizedAddress: addressEntry.address,
            label: 'Default',
            createdAt: 1,
            updatedAt: 1,
            lastUsedAt: null,
          },
        ],
      },
    ]);
    const onCreateAddressContact = vi.fn();
    await render({ onCreateAddressContact });
    document.querySelector<HTMLButtonElement>('[data-testid="watchlist-entry"]')?.click();
    await settle();
    expect(button('In contacts').disabled).toBe(true);
    expect(document.body.textContent).not.toContain('View profile');
    expect(onCreateAddressContact).not.toHaveBeenCalled();
  });

  it('shows a duplicate VerusID contact without issuing another save', async () => {
    setContactSession({ sessionId: 'watchlist-test', network: 'mainnet' });
    service.getWatchlistEntries.mockResolvedValue([identityEntry]);
    contactService.loadContacts.mockResolvedValue([
      {
        id: 'alice-contact',
        displayName: identityEntry.displayName,
        note: null,
        createdAt: 1,
        updatedAt: 1,
        identities: [
          {
            identityAddress: identityEntry.address,
            fullyQualifiedName: identityEntry.displayName,
            network: 'mainnet',
            chainId: contactChainId('mainnet'),
          },
        ],
        endpoints: [],
      },
    ]);
    const onViewIdentityProfile = vi.fn();
    await render({ onViewIdentityProfile });
    document.querySelector<HTMLButtonElement>('[data-testid="watchlist-entry"]')?.click();
    await settle();
    expect(button('In contacts').disabled).toBe(true);
    button('View profile').click();
    expect(onViewIdentityProfile).toHaveBeenCalledWith(identityEntry);
    expect(contactService.addIdentityContact).not.toHaveBeenCalled();
  });

  it('opens the full address contact flow and restores the selected detail on remount', async () => {
    setContactSession({ sessionId: 'watchlist-test', network: 'mainnet' });
    service.getWatchlistEntries.mockResolvedValue([addressEntry]);
    const onCreateAddressContact = vi.fn();
    await render({ onCreateAddressContact });
    document.querySelector<HTMLButtonElement>('[data-testid="watchlist-entry"]')?.click();
    await settle();
    button('Add to contacts').click();
    expect(onCreateAddressContact).toHaveBeenCalledWith(addressEntry);

    await unmount(component);
    document.body.replaceChildren();
    await render({ initialSelectedEntryId: addressEntry.id, onCreateAddressContact });
    expect(document.body.textContent).toContain('Mum');
    expect(document.body.textContent).toContain('Remove from watchlist');
  });

  it('keeps the detail and offers retry after contact checking or saving fails', async () => {
    setContactSession({ sessionId: 'watchlist-test', network: 'mainnet' });
    service.getWatchlistEntries.mockResolvedValue([identityEntry]);
    contactService.loadContacts.mockRejectedValueOnce(new Error('offline')).mockResolvedValue([]);
    contactService.addIdentityContact
      .mockRejectedValueOnce(new Error('save failed'))
      .mockResolvedValueOnce({
        id: 'saved-on-retry',
        displayName: identityEntry.displayName,
        note: null,
        endpoints: [],
        createdAt: 1,
        updatedAt: 1,
      });
    await render();
    document.querySelector<HTMLButtonElement>('[data-testid="watchlist-entry"]')?.click();
    await settle();
    expect(document.body.textContent).toContain('Couldn’t load contacts');
    button('Try again').click();
    await settle();
    button('Add to contacts').click();
    await settle();
    expect(document.body.textContent).toContain('Couldn’t save contact');
    expect(document.body.textContent).toContain('Remove from watchlist');
    button('Try again').click();
    await settle();
    expect(contactService.addIdentityContact).toHaveBeenCalledTimes(2);
  });
  it('holds a neutral canvas while membership is unknown and shows the empty state once it resolves', async () => {
    let finishLoad!: (entries: WatchlistEntry[]) => void;
    service.getWatchlistEntries.mockImplementation(
      () =>
        new Promise((resolve) => {
          finishLoad = resolve;
        })
    );

    await render();
    const pending = document.querySelector('[data-testid="watchlist-membership-pending"]');
    expect(pending).not.toBeNull();
    expect(pending?.textContent?.trim()).toBe('');
    expect(pending?.querySelector('[data-slot="skeleton"]')).toBeNull();
    expect(document.querySelector('[data-testid="watchlist-empty"]')).toBeNull();
    expect(document.querySelector('header')).toBeNull();

    finishLoad([]);
    await settle();
    expect(document.querySelector('[data-testid="watchlist-empty"]')).not.toBeNull();
    expect(document.querySelector('[data-testid="watchlist-membership-pending"]')).toBeNull();
    expect(document.querySelector('header')).toBeNull();
  });

  it('shows a concise encrypted empty state', async () => {
    await render();

    const emptyState = document.querySelector('[data-testid="watchlist-empty"]');
    expect(emptyState).not.toBeNull();
    if (!emptyState) throw new Error('Missing watchlist empty state');
    expect(
      emptyState.querySelector('[data-testid="wallet-empty-eyebrow"]')?.textContent?.trim()
    ).toBe('Encrypted locally');
    expect(emptyState.querySelector('[data-testid="wallet-empty-eyebrow"] svg')).not.toBeNull();
    expect(emptyState.querySelector('h3')?.textContent?.trim()).toBe('Watch a Verus address');
    expect(emptyState.textContent).not.toContain('Follow the public balances');
    expect(document.querySelector('header')).toBeNull();
    expect([...emptyState.children].some((child) => child.tagName === 'svg')).toBe(false);
    expect(emptyState.querySelector('button svg')).not.toBeNull();
  });

  it('keeps saved rows and shows restrained balance skeletons until the first refresh confirms values', async () => {
    ratesStore.set({ VRSC: { rates: { USD: 2 }, usdChange24hPct: null } });
    service.getWatchlistEntries.mockResolvedValue([entry]);
    let finishRefresh!: (result: WatchlistRefreshResult) => void;
    service.refreshWatchlist.mockImplementation(
      () =>
        new Promise((resolve) => {
          finishRefresh = resolve;
        })
    );

    await render();
    const row = document.querySelector<HTMLButtonElement>('[data-testid="watchlist-entry"]');
    expect(row?.textContent).toContain('Alice@');
    expect(document.body.textContent).not.toContain('0 currencies');
    expect(document.body.textContent).not.toContain('No public balances found');
    expect(document.body.textContent).not.toContain('Loading watchlist');
    // Settled failures offer Retry; a pending refresh must not.
    expect(retryButton()).toBeUndefined();

    await vi.waitFor(() => expect(row?.querySelector('[data-slot="skeleton"]')).not.toBeNull());
    expect(row?.querySelector('[aria-label="Balance pending"]')).not.toBeNull();

    finishRefresh(refreshResult());
    await settle();
    expect(row?.textContent).toContain('$8.50');
    expect(row?.textContent).toContain('1 currency');
    expect(row?.querySelector('[data-slot="skeleton"]')).toBeNull();
  });

  it('reuses cached membership on a revisit and refreshes balances again', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry]);
    service.refreshWatchlist.mockResolvedValue(refreshResult());

    await render();
    expect(service.getWatchlistEntries).toHaveBeenCalledTimes(1);
    expect(document.querySelector('[data-testid="watchlist-entry"]')).not.toBeNull();

    await unmount(component);
    document.body.replaceChildren();
    await render();

    expect(service.getWatchlistEntries).toHaveBeenCalledTimes(1);
    expect(service.refreshWatchlist).toHaveBeenCalledTimes(2);
    expect(document.querySelector('[data-testid="watchlist-entry"]')?.textContent).toContain(
      'Alice@'
    );
  });

  it('keeps saved metadata and lets Retry recover a failed balance refresh', async () => {
    ratesStore.set({ VRSC: { rates: { USD: 2 }, usdChange24hPct: null } });
    service.getWatchlistEntries.mockResolvedValue([entry]);
    service.refreshWatchlist
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce(refreshResult());

    await render();
    expect(document.body.textContent).toContain('Alice@');
    expect(document.body.textContent).toContain('Current balances unavailable');
    expect(document.body.textContent).not.toContain('0 currencies');
    expect(document.body.textContent).not.toContain('No public balances found');

    button('Retry').click();
    await settle();
    expect(document.body.textContent).toContain('$8.50');
    expect(document.body.textContent).toContain('1 currency');
  });

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
    expect(document.querySelector('header h2')).toBeNull();
    expect(document.querySelectorAll('[data-testid="watchlist-entry"]')).toHaveLength(1);
    button('Add address').click();
    await settle();
    await setInput('#watchlist-target', 'Alice@');
    button('Continue').click();
    await settle();
    expect(service.resolveWatchlistTarget).toHaveBeenCalledWith('Alice@');
    expect(document.querySelector('[data-testid="watchlist-resolved-preview"]')).not.toBeNull();
    button('Add to watchlist').click();
    await settle();
    expect(service.addWatchlistEntry).toHaveBeenCalledWith('Alice@', undefined);
    expect(document.querySelectorAll('[data-testid="watchlist-entry"]')).toHaveLength(0);
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

  it('discards a lookup result after the query changes', async () => {
    let finishLookup!: (value: unknown) => void;
    service.resolveWatchlistTarget.mockImplementation(
      () =>
        new Promise((resolve) => {
          finishLookup = resolve;
        })
    );
    await render();
    button('Add address').click();
    await settle();
    await setInput('#watchlist-target', 'Alice@');
    button('Continue').click();
    await settle();
    expect(document.body.textContent).toContain('Looking up…');
    await setInput('#watchlist-target', 'Bob@');
    finishLookup({ targetKind: 'identity', displayName: 'Alice@', address: entry.address });
    await settle();
    expect(document.querySelector('[data-testid="watchlist-resolved-preview"]')).toBeNull();
    expect(document.body.textContent).not.toContain('Alice@');
  });

  it('saves an optional local name for a raw R-address', async () => {
    service.resolveWatchlistTarget.mockResolvedValue({
      targetKind: 'address',
      displayName: entry.address,
      address: entry.address,
      availability: 'available',
      visibleCurrencyCount: 0,
    });
    service.addWatchlistEntry.mockResolvedValue({
      ...snapshot,
      entry: { ...entry, targetKind: 'address', displayName: 'Mom' },
    });
    await render();
    button('Add address').click();
    await settle();
    await setInput('#watchlist-target', entry.address);
    button('Continue').click();
    await settle();
    expect(document.querySelector('[data-testid="watchlist-address-avatar"]')).not.toBeNull();
    expect(document.body.textContent).toContain('R-address found');
    expect(document.body.textContent).toContain('balances are public');
    await setInput('#watchlist-name', 'Mom');
    button('Add to watchlist').click();
    await settle();
    expect(service.addWatchlistEntry).toHaveBeenCalledWith(entry.address, 'Mom');
    expect(document.querySelector('[data-testid="watchlist-entry"]')?.textContent).toContain('Mom');
  });

  it('keeps Retry available after an initial load error', async () => {
    service.getWatchlistEntries.mockRejectedValueOnce(new Error('unavailable'));
    await render();
    expect(document.querySelector('header')).toBeNull();
    expect(document.querySelector('[data-testid="watchlist-empty"]')).toBeNull();
    expect(document.body.textContent).toContain('Could not load your watchlist.');
    button('Retry').click();
    await settle();
    expect(document.querySelector('[data-testid="watchlist-empty"]')).not.toBeNull();
  });

  it('recovers the wallet session through the route before retrying membership', async () => {
    resetWatchlistSession();
    const retryWalletSession = vi.fn(async () => {
      setWatchlistSession({ sessionId: 'watchlist-test', network: 'mainnet' });
    });
    await render({ onRetryWalletSession: retryWalletSession });

    expect(retryWalletSession).not.toHaveBeenCalled();
    expect(service.getWatchlistEntries).not.toHaveBeenCalled();
    expect(document.querySelector('[data-testid="watchlist-empty"]')).toBeNull();
    expect(document.querySelector('[data-testid="watchlist-membership-pending"]')).toBeNull();
    expect(document.body.textContent).toContain('Could not load your watchlist.');

    service.getWatchlistEntries.mockResolvedValue([entry]);
    service.refreshWatchlist.mockResolvedValue(refreshResult());
    retryButton()?.click();
    await settle();

    expect(retryWalletSession).toHaveBeenCalledTimes(1);
    expect(service.getWatchlistEntries).toHaveBeenCalledTimes(1);
    expect(document.querySelector('[data-testid="watchlist-entry"]')?.textContent).toContain(
      'Alice@'
    );
  });

  it('keeps the guarded failure when route recovery cannot bind a session', async () => {
    resetWatchlistSession();
    const retryWalletSession = vi.fn(async () => undefined);
    await render({ onRetryWalletSession: retryWalletSession });

    retryButton()?.click();
    await settle();

    expect(retryWalletSession).toHaveBeenCalledTimes(1);
    expect(service.getWatchlistEntries).not.toHaveBeenCalled();
    expect(document.querySelector('[data-testid="watchlist-empty"]')).toBeNull();
    expect(document.body.textContent).toContain('Could not load your watchlist.');
    expect(retryButton()).toBeDefined();
  });

  it('resolves a newly added entry after leaving and re-entering during persistence', async () => {
    service.getWatchlistEntries.mockResolvedValue([]);
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
    service.refreshWatchlist.mockResolvedValue(refreshResult());

    await render();
    button('Add address').click();
    await settle();
    await setInput('#watchlist-target', 'Alice@');
    button('Continue').click();
    await settle();
    button('Add to watchlist').click();
    await settle();
    expect(service.addWatchlistEntry).toHaveBeenCalledTimes(1);

    // Leave and re-enter while persistence is still pending.
    await unmount(component);
    document.body.replaceChildren();
    await render();
    expect(document.querySelector('[data-testid="watchlist-empty"]')).not.toBeNull();
    expect(document.body.textContent).not.toContain('Alice@');

    finishAdd(snapshot);
    await settle();

    const row = document.querySelector('[data-testid="watchlist-entry"]');
    expect(row?.textContent).toContain('Alice@');
    expect(service.refreshWatchlist).toHaveBeenCalled();
    await vi.waitFor(() =>
      expect(document.querySelector('[data-testid="watchlist-entry"]')?.textContent).toContain(
        '1 currency'
      )
    );
    expect(document.querySelector('[data-slot="skeleton"]')).toBeNull();
  });

  it('keeps a newly added row pending when an older populated refresh resolves without it', async () => {
    ratesStore.set({ VRSC: { rates: { USD: 2 }, usdChange24hPct: null } });
    service.getWatchlistEntries.mockResolvedValue([otherEntry]);
    let finishOldRefresh!: (result: WatchlistRefreshResult) => void;
    let finishFollowUpRefresh!: (result: WatchlistRefreshResult) => void;
    service.refreshWatchlist
      .mockResolvedValueOnce(refreshResult([otherSnapshot]))
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finishOldRefresh = resolve;
          })
      )
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finishFollowUpRefresh = resolve;
          })
      );
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

    // Populated revisit: Bob has confirmed values.
    await render();
    expect(document.querySelector('[data-entry-id="bob"]')?.textContent).toContain('$4.00');

    // Start adding Alice, then leave and re-enter while persistence is pending.
    button('Add address').click();
    await settle();
    await setInput('#watchlist-target', 'Alice@');
    button('Continue').click();
    await settle();
    button('Add to watchlist').click();
    await settle();
    await unmount(component);
    document.body.replaceChildren();
    await render();
    expect(service.refreshWatchlist).toHaveBeenCalledTimes(2);

    // Add completes, then the in-flight refresh returns the older membership.
    finishAdd(snapshot);
    await settle();
    expect(document.querySelector('[data-entry-id="alice"]')).not.toBeNull();
    finishOldRefresh(refreshResult([otherSnapshot]));
    await settle();

    const aliceRow = document.querySelector<HTMLButtonElement>('[data-entry-id="alice"]');
    expect(aliceRow?.textContent).not.toContain('—');
    expect(service.refreshWatchlist).toHaveBeenCalledTimes(3);
    await vi.waitFor(() =>
      expect(aliceRow?.querySelector('[data-slot="skeleton"]')).not.toBeNull()
    );

    finishFollowUpRefresh(refreshResult([snapshot, otherSnapshot]));
    await settle();
    expect(document.querySelector('[data-entry-id="alice"]')?.textContent).toContain('$8.50');
    expect(document.querySelector('[data-entry-id="alice"] [data-slot="skeleton"]')).toBeNull();
  });

  it('keeps an entry added before remount pending when the shared old refresh predates it', async () => {
    ratesStore.set({ VRSC: { rates: { USD: 2 }, usdChange24hPct: null } });
    service.getWatchlistEntries.mockResolvedValue([entry]);
    let finishOldRefresh!: (result: WatchlistRefreshResult) => void;
    let finishFollowUpRefresh!: (result: WatchlistRefreshResult) => void;
    service.refreshWatchlist
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finishOldRefresh = resolve;
          })
      )
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finishFollowUpRefresh = resolve;
          })
      );
    service.resolveWatchlistTarget.mockResolvedValue({
      targetKind: 'identity',
      displayName: otherEntry.displayName,
      address: otherEntry.address,
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

    // The first component starts the shared refresh for the original membership.
    await render();
    expect(service.refreshWatchlist).toHaveBeenCalledTimes(1);

    // Add Bob while that shared refresh is still pending, then leave.
    button('Add address').click();
    await settle();
    await setInput('#watchlist-target', 'Bob@');
    button('Continue').click();
    await settle();
    button('Add to watchlist').click();
    await settle();
    await unmount(component);
    document.body.replaceChildren();

    // The add completes before the replacement component mounts.
    finishAdd(otherSnapshot);
    await settle();

    // The replacement joins the still-pending shared request instead of starting
    // a new one scoped to the current membership.
    await render();
    expect(service.refreshWatchlist).toHaveBeenCalledTimes(1);

    finishOldRefresh(refreshResult([snapshot]));
    await settle();

    const bobRow = document.querySelector<HTMLButtonElement>('[data-entry-id="bob"]');
    expect(bobRow).not.toBeNull();
    expect(bobRow?.textContent).not.toContain('—');
    expect(service.refreshWatchlist).toHaveBeenCalledTimes(2);
    await vi.waitFor(() => expect(bobRow?.querySelector('[data-slot="skeleton"]')).not.toBeNull());

    finishFollowUpRefresh(refreshResult([snapshot, otherSnapshot]));
    await settle();
    expect(document.querySelector('[data-entry-id="bob"]')?.textContent).toContain('$4.00');
    expect(document.querySelector('[data-entry-id="bob"] [data-slot="skeleton"]')).toBeNull();
  });

  it('refreshes an added entry when the joined old request only covered a removed entry', async () => {
    ratesStore.set({ VRSC: { rates: { USD: 2 }, usdChange24hPct: null } });
    service.getWatchlistEntries.mockResolvedValue([entry]);
    let finishOldRefresh!: (result: WatchlistRefreshResult) => void;
    let finishFollowUpRefresh!: (result: WatchlistRefreshResult) => void;
    service.refreshWatchlist
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finishOldRefresh = resolve;
          })
      )
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finishFollowUpRefresh = resolve;
          })
      );
    service.resolveWatchlistTarget.mockResolvedValue({
      targetKind: 'identity',
      displayName: otherEntry.displayName,
      address: otherEntry.address,
      systemId: null,
      visibleCurrencyCount: 1,
      availability: 'available',
    });
    service.addWatchlistEntry.mockResolvedValue(otherSnapshot);

    // Alice's refresh starts.
    await render();
    expect(service.refreshWatchlist).toHaveBeenCalledTimes(1);

    // Remove Alice while her refresh is still pending.
    document.querySelector<HTMLButtonElement>('[data-entry-id="alice"]')?.click();
    await settle();
    button('Remove from watchlist').click();
    await settle();
    button('Remove from watchlist', document.querySelector('[role="dialog"]')).click();
    await settle();
    expect(document.querySelector('[data-testid="watchlist-empty"]')).not.toBeNull();

    // Add Bob successfully before the old refresh resolves.
    button('Add address').click();
    await settle();
    await setInput('#watchlist-target', 'Bob@');
    button('Continue').click();
    await settle();
    button('Add to watchlist').click();
    await settle();
    expect(document.querySelector('[data-entry-id="bob"]')?.textContent).toContain('Bob@');

    // Leave and reenter while the old Alice-only request is still pending.
    await unmount(component);
    document.body.replaceChildren();
    await render();
    expect(service.refreshWatchlist).toHaveBeenCalledTimes(1);

    // The old result does not cover Bob, so he must get a follow-up refresh
    // instead of being marked as already attempted.
    finishOldRefresh(refreshResult([snapshot]));
    await settle();
    const bobRow = document.querySelector<HTMLButtonElement>('[data-entry-id="bob"]');
    expect(bobRow?.textContent).not.toContain('—');
    expect(service.refreshWatchlist).toHaveBeenCalledTimes(2);
    await vi.waitFor(() => expect(bobRow?.querySelector('[data-slot="skeleton"]')).not.toBeNull());

    finishFollowUpRefresh(refreshResult([otherSnapshot]));
    await settle();
    expect(document.querySelector('[data-entry-id="bob"]')?.textContent).toContain('$4.00');
    expect(document.querySelector('[data-entry-id="bob"] [data-slot="skeleton"]')).toBeNull();
  });

  it('stops after one failed follow-up for an uncovered entry and lets Retry recover', async () => {
    ratesStore.set({ VRSC: { rates: { USD: 2 }, usdChange24hPct: null } });
    service.getWatchlistEntries.mockResolvedValue([entry]);
    let finishOldRefresh!: (result: WatchlistRefreshResult) => void;
    service.refreshWatchlist
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finishOldRefresh = resolve;
          })
      )
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce(refreshResult([otherSnapshot, snapshot]));
    service.resolveWatchlistTarget.mockResolvedValue({
      targetKind: 'identity',
      displayName: otherEntry.displayName,
      address: otherEntry.address,
      systemId: null,
      visibleCurrencyCount: 1,
      availability: 'available',
    });
    service.addWatchlistEntry.mockResolvedValue(otherSnapshot);

    await render();
    button('Add address').click();
    await settle();
    await setInput('#watchlist-target', 'Bob@');
    button('Continue').click();
    await settle();
    button('Add to watchlist').click();
    await settle();
    await unmount(component);
    document.body.replaceChildren();
    await render();

    // The old request covers Alice only; Bob's follow-up fails.
    finishOldRefresh(refreshResult([snapshot]));
    await settle();
    expect(service.refreshWatchlist).toHaveBeenCalledTimes(2);

    const bobRow = document.querySelector<HTMLButtonElement>('[data-entry-id="bob"]');
    expect(bobRow?.textContent).toContain('—');
    expect(retryButton()).toBeDefined();

    // No failed-attempt provider loop.
    await settle();
    expect(service.refreshWatchlist).toHaveBeenCalledTimes(2);

    retryButton()?.click();
    await settle();
    expect(document.querySelector('[data-entry-id="bob"]')?.textContent).toContain('$4.00');
    expect(retryButton()).toBeUndefined();
  });

  it('settles an entry omitted by the joined shared request without an automatic refetch loop', async () => {
    ratesStore.set({ VRSC: { rates: { USD: 2 }, usdChange24hPct: null } });
    service.getWatchlistEntries.mockResolvedValue([entry]);
    let finishOldRefresh!: (result: WatchlistRefreshResult) => void;
    service.refreshWatchlist
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finishOldRefresh = resolve;
          })
      )
      .mockResolvedValueOnce(refreshResult([snapshot]));

    await render();
    expect(service.refreshWatchlist).toHaveBeenCalledTimes(1);
    await unmount(component);
    document.body.replaceChildren();
    await render();
    expect(service.refreshWatchlist).toHaveBeenCalledTimes(1);

    // The joined request covered Alice but reported nothing for her.
    finishOldRefresh({ network: 'mainnet', entries: [], refreshedAt: 5 });
    await settle();

    const row = document.querySelector<HTMLButtonElement>('[data-entry-id="alice"]');
    expect(row?.textContent).toContain('—');
    expect(row?.querySelector('[data-slot="skeleton"]')).toBeNull();
    expect(service.refreshWatchlist).toHaveBeenCalledTimes(1);
    expect(retryButton()).toBeDefined();

    retryButton()?.click();
    await settle();
    expect(document.querySelector('[data-entry-id="alice"]')?.textContent).toContain('$8.50');
  });

  it('offers Retry for a settled unavailable snapshot and clears it on success', async () => {
    ratesStore.set({ VRSC: { rates: { USD: 2 }, usdChange24hPct: null } });
    service.getWatchlistEntries.mockResolvedValue([entry]);
    service.refreshWatchlist
      .mockResolvedValueOnce(
        refreshResult([{ ...snapshot, holdings: [], availability: 'unavailable' }])
      )
      .mockResolvedValueOnce(refreshResult([snapshot]));

    await render();
    const row = document.querySelector<HTMLButtonElement>('[data-entry-id="alice"]');
    expect(row?.textContent).toContain('—');
    expect(document.body.textContent).toContain('Current balances unavailable');
    const listRetry = retryButton();
    expect(listRetry).toBeDefined();

    row?.click();
    await settle();
    expect(
      document.querySelector('[data-testid="watchlist-detail-status"]')?.textContent
    ).toContain('Current balances unavailable');
    const detailRetry = retryButton();
    expect(detailRetry).toBeDefined();

    detailRetry?.click();
    await settle();

    expect(document.querySelector('[data-testid="watchlist-detail-status"]')).toBeNull();
    expect(retryButton()).toBeUndefined();
    expect(document.querySelector('[data-testid="public-value-card"]')?.textContent).toContain(
      '$8.50'
    );
  });

  it('settles a requested but unreported entry instead of leaving it loading forever', async () => {
    ratesStore.set({ VRSC: { rates: { USD: 2 }, usdChange24hPct: null } });
    service.getWatchlistEntries.mockResolvedValue([entry]);
    service.refreshWatchlist
      .mockResolvedValueOnce({ network: 'mainnet', entries: [], refreshedAt: 5 })
      .mockResolvedValueOnce(refreshResult([snapshot]));

    await render();
    const row = document.querySelector<HTMLButtonElement>('[data-entry-id="alice"]');
    expect(row?.textContent).toContain('—');
    expect(row?.querySelector('[data-slot="skeleton"]')).toBeNull();
    expect(retryButton()).toBeDefined();

    retryButton()?.click();
    await settle();
    expect(document.querySelector('[data-entry-id="alice"]')?.textContent).toContain('$8.50');
    expect(retryButton()).toBeUndefined();
  });

  it('surfaces a failed refresh on a known-available detail and keeps its values', async () => {
    ratesStore.set({ VRSC: { rates: { USD: 2 }, usdChange24hPct: null } });
    service.getWatchlistEntries.mockResolvedValue([entry]);
    let rejectRefresh!: (error: unknown) => void;
    service.refreshWatchlist.mockImplementationOnce(
      () =>
        new Promise((_resolve, reject) => {
          rejectRefresh = reject;
        })
    );
    service.resolveWatchlistTarget.mockResolvedValue({
      targetKind: 'identity',
      displayName: otherEntry.displayName,
      address: otherEntry.address,
      systemId: null,
      visibleCurrencyCount: 1,
      availability: 'available',
    });
    service.addWatchlistEntry.mockResolvedValue(otherSnapshot);

    await render();
    button('Add address').click();
    await settle();
    await setInput('#watchlist-target', 'Bob@');
    button('Continue').click();
    await settle();
    button('Add to watchlist').click();
    await settle();
    expect(service.refreshWatchlist).toHaveBeenCalledTimes(1);

    // The first refresh fails while the added entry already has known values.
    rejectRefresh(new Error('offline'));
    await settle();
    document.querySelector<HTMLButtonElement>('[data-entry-id="bob"]')?.click();
    await settle();

    expect(
      document.querySelector('[data-testid="watchlist-detail-status"]')?.textContent
    ).toContain('Current balances unavailable · Last known values');
    expect(document.querySelector('[data-testid="public-value-card"]')?.textContent).toContain(
      '$4.00'
    );
    expect(document.querySelector('[data-testid="watchlist-detail-skeleton"]')).toBeNull();

    const retry = button('Retry', document.querySelector('[data-testid="watchlist-section"]'));
    service.refreshWatchlist.mockResolvedValueOnce(refreshResult([snapshot, otherSnapshot]));
    retry.click();
    await settle();

    expect(document.querySelector('[data-testid="watchlist-detail-status"]')).toBeNull();
    expect(document.querySelector('[data-testid="public-value-card"]')?.textContent).toContain(
      '$4.00'
    );
  });

  it('shows a published profile image for an identity and retains initials fallback', async () => {
    setContactSession({ sessionId: 'avatar-session', network: 'mainnet' });
    service.getWatchlistEntries.mockResolvedValue([entry]);
    service.refreshWatchlist.mockResolvedValue(refreshResult());
    await render();
    const avatar = document.querySelector('[data-testid="watchlist-avatar"]');
    expect(avatar?.textContent).toContain('AL');
    const identity = {
      network: 'mainnet' as const,
      chainId: contactChainId('mainnet'),
      identityAddress: entry.address,
      fullyQualifiedName: entry.displayName,
    };
    identityProfiles.set({
      [identityKey(identity)]: {
        profile: {
          state: 'ready',
          issues: [],
          revisionTxid: null,
          avatar: {
            value: {
              mimeType: 'image/png',
              base64: 'aGVsbG8=',
              width: 1,
              height: 1,
              byteLength: 5,
            },
            source: {
              systemId: contactChainId('mainnet'),
              txid: 'profile',
              vout: 0,
              height: 1,
              blockhash: 'block',
              digest: 'digest',
            },
          },
        },
        loading: false,
        unavailable: false,
        checkedAt: Date.now(),
      },
    });
    await settle();
    expect(avatar?.querySelector('img')?.getAttribute('src')).toBe(
      'data:image/png;base64,aGVsbG8='
    );
  });

  it('marks a partial snapshot stale while retaining its available balances', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry]);
    service.refreshWatchlist.mockResolvedValue(
      refreshResult([{ ...snapshot, availability: 'partial' }])
    );

    await render();
    expect(document.body.textContent).toContain('Current balances unavailable · Last known values');
    document.querySelector<HTMLButtonElement>('[data-testid="watchlist-entry"]')?.click();
    await settle();
    expect(document.body.textContent).toContain('4.25');
    expect(
      document.querySelector('[data-testid="watchlist-detail-status"]')?.textContent
    ).toContain('Current balances unavailable · Last known values');
  });

  it('distinguishes unavailable detail balances from a confirmed empty address', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry]);
    service.refreshWatchlist.mockResolvedValue(
      refreshResult([{ ...snapshot, holdings: [], availability: 'unavailable' }])
    );

    await render();
    expect(document.querySelector('[data-testid="watchlist-entry"]')?.textContent).not.toContain(
      '0 currencies'
    );
    document.querySelector<HTMLButtonElement>('[data-testid="watchlist-entry"]')?.click();
    await settle();
    expect(
      document.querySelector('[data-testid="watchlist-detail-status"]')?.textContent
    ).toContain('Current balances unavailable');
    expect(document.body.textContent).not.toContain('No public balances found');
    expect(document.querySelector('[data-testid="watchlist-detail-skeleton"]')).toBeNull();

    button('Back to watchlist').click();
    service.refreshWatchlist.mockResolvedValue(
      refreshResult([{ ...snapshot, holdings: [], availability: 'available' }])
    );
    // A fresh confirmed empty result has different copy from an unavailable response.
    // The service is called during the next mount rather than through a manual refresh control.
    await unmount(component);
    document.body.replaceChildren();
    await render();
    document.querySelector<HTMLButtonElement>('[data-testid="watchlist-entry"]')?.click();
    await settle();
    expect(document.body.textContent).toContain('No public balances found');
    expect(document.querySelector('[data-testid="watchlist-detail-status"]')).toBeNull();
  });

  it('shows passive detail loading while its first balance lookup is pending', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry]);
    service.refreshWatchlist.mockImplementation(() => new Promise(() => {}));

    await render();
    document.querySelector<HTMLButtonElement>('[data-testid="watchlist-entry"]')?.click();
    await settle();
    expect(
      document.querySelector('[data-testid="watchlist-detail-status"]')?.textContent
    ).toContain('Loading public balances…');
    expect(document.body.textContent).not.toContain('No public balances found');
    expect(document.querySelector('[data-testid="watchlist-detail-skeleton"]')).not.toBeNull();
    expect(document.querySelector('[data-slot="skeleton"]')).toBeNull();
    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-testid="watchlist-detail-skeleton"] [data-slot="skeleton"]')
      ).not.toBeNull()
    );
  });

  it('opens the VerusID detail with an initials avatar and restores row focus on Back', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry]);
    service.refreshWatchlist.mockResolvedValue(refreshResult());

    await render();
    const row = document.querySelector<HTMLButtonElement>('[data-testid="watchlist-entry"]');
    expect(row?.textContent).not.toContain(entry.address);
    row?.click();
    await settle();
    expect(document.querySelector('[data-testid="watchlist-avatar"]')).not.toBeNull();
    expect(document.querySelector('[data-testid="public-value-card"]')).not.toBeNull();
    expect(document.body.textContent).toContain('4.25');
    expect(document.body.textContent).toContain('VRSC');
    button('Back to watchlist').click();
    await settle();
    expect(document.activeElement).toBe(document.querySelector('[data-testid="watchlist-entry"]'));
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
    expect(button('Remove from watchlist').parentElement).toBe(
      button('Back to watchlist').parentElement
    );
    expect(document.querySelector('[aria-label="More actions"]')).toBeNull();
    button('Remove from watchlist').click();
    await settle();
    button('Remove from watchlist', document.querySelector('[role="dialog"]')).click();
    await settle();
    expect(document.body.textContent).toContain('Alice@');

    finishRemove(true);
    await settle();
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    expect(document.querySelector('[data-testid="watchlist-empty"]')).not.toBeNull();
  });

  it('keeps a durably removed entry removed when a slow balance refresh lands afterwards', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry, otherEntry]);
    let finishRefresh!: (result: WatchlistRefreshResult) => void;
    service.refreshWatchlist.mockImplementation(
      () =>
        new Promise((resolve) => {
          finishRefresh = resolve;
        })
    );

    await render();
    expect(document.querySelectorAll('[data-testid="watchlist-entry"]')).toHaveLength(2);

    document.querySelector<HTMLButtonElement>('[data-entry-id="alice"]')?.click();
    await settle();
    button('Remove from watchlist').click();
    await settle();
    button('Remove from watchlist', document.querySelector('[role="dialog"]')).click();
    await settle();
    expect(document.querySelectorAll('[data-testid="watchlist-entry"]')).toHaveLength(1);

    finishRefresh(refreshResult([snapshot, otherSnapshot]));
    await settle();
    const rows = [...document.querySelectorAll('[data-testid="watchlist-entry"]')];
    expect(rows).toHaveLength(1);
    expect(rows[0]?.textContent).toContain('Bob@');
  });

  it('uses removal copy when the persisted entry is no longer found', async () => {
    service.getWatchlistEntries.mockResolvedValue([entry]);
    service.refreshWatchlist.mockResolvedValue(refreshResult());
    service.removeWatchlistEntry.mockRejectedValue({ type: 'WatchlistEntryNotFound' });

    await render();
    document.querySelector<HTMLButtonElement>('[data-testid="watchlist-entry"]')?.click();
    await settle();
    button('Remove from watchlist').click();
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
