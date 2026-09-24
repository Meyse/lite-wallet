import '../../src/app.css';
import { mount } from 'svelte';
import { setLocale } from '$lib/i18n';
import { setContactSession } from '$lib/contacts/session';
import { ratesStore } from '$lib/stores/rates';
import { setWatchlistSession } from '$lib/watchlist/session';
import type { AddressBookContact, SaveAddressBookContactRequest } from '$lib/types/addressBook';
import type {
  WatchlistEntry,
  WatchlistEntrySnapshot,
  WatchlistResolvedTarget,
} from '$lib/types/watchlist';
import Fixture from './WatchlistFixture.svelte';

const params = new URLSearchParams(location.search);
const identityAddress = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
const rawAddress = 'RAutMoGh771ECTDbTq2qwwZo7MF5Tov3ka';
const verusCoin = {
  id: 'VRSC',
  currencyId: identityAddress,
  systemId: identityAddress,
  displayTicker: 'VRSC',
  displayName: 'Verus',
  proto: 'vrsc' as const,
  compatibleChannels: ['vrpc' as const],
  decimals: 8,
  vrpcEndpoints: [],
  secondsPerBlock: 60,
  isTestnet: false,
};

const targets: Record<string, WatchlistResolvedTarget> = {
  'alex@': {
    targetKind: 'identity',
    displayName: 'alex@',
    address: identityAddress,
    visibleCurrencyCount: 2,
    availability: 'available',
  },
  [rawAddress]: {
    targetKind: 'address',
    displayName: rawAddress,
    address: rawAddress,
    visibleCurrencyCount: 1,
    availability: 'available',
  },
};

const entries: WatchlistEntrySnapshot[] = [];
const contacts: AddressBookContact[] = [];

function snapshotFor(query: string): WatchlistEntrySnapshot {
  const target = targets[query];
  const entry: WatchlistEntry = {
    id: target.address,
    targetKind: target.targetKind,
    displayName: target.displayName,
    address: target.address,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  };
  const balances = query === 'alex@' ? ['20', '12.5'] : ['4.25'];
  return {
    entry,
    holdings: balances.map((balance, index) => ({
      assetKey: `${entry.id}-${index}`,
      currencyId: index === 0 ? identityAddress : 'fixture-pbaas-currency',
      systemId: identityAddress,
      systemTicker: 'VRSC',
      systemDisplayName: 'Verus',
      balance,
      coin:
        index === 0
          ? verusCoin
          : {
              ...verusCoin,
              id: 'fixture-pbaas-currency',
              currencyId: 'fixture-pbaas-currency',
              displayTicker: 'VUSD',
              displayName: 'Verus USD',
            },
    })),
    sources: [
      {
        systemId: identityAddress,
        systemTicker: 'VRSC',
        systemDisplayName: 'Verus',
        status: 'available',
      },
    ],
    availability: 'available',
    refreshedAt: Date.now(),
  };
}

window.__PROFILE_FIXTURE_INVOKE__ = async (command, args) => {
  if (command === 'validate_destination_address') {
    const request = args?.request as { address: string };
    return { valid: true, normalizedAddress: request.address, reason: null };
  }
  if (command === 'list_address_book_contacts') return structuredClone(contacts);
  if (command === 'save_address_book_contact') {
    const request = args?.request as SaveAddressBookContactRequest;
    const now = Date.now();
    const contact: AddressBookContact = {
      id: request.id ?? `fixture-contact-${contacts.length + 1}`,
      displayName: request.displayName,
      note: request.note ?? null,
      createdAt: now,
      updatedAt: now,
      endpoints: [],
      identities: request.identities ?? [],
      profileIdentity: request.profileIdentity ?? null,
    };
    contacts.push(contact);
    return structuredClone(contact);
  }
  if (command === 'get_watchlist_entries')
    return structuredClone(entries.map((snapshot) => snapshot.entry));
  if (command === 'resolve_watchlist_target') {
    const query = (args?.request as { query: string }).query.trim();
    if (!targets[query]) throw new Error('WatchlistInvalidInput');
    return structuredClone(targets[query]);
  }
  if (command === 'add_watchlist_entry') {
    const query = (args?.request as { query: string }).query.trim();
    if (!targets[query]) throw new Error('WatchlistInvalidInput');
    const snapshot = snapshotFor(query);
    entries.unshift(snapshot);
    return structuredClone(snapshot);
  }
  if (command === 'refresh_watchlist')
    return { network: 'mainnet', entries: structuredClone(entries), refreshedAt: Date.now() };
  if (command === 'remove_watchlist_entry') {
    const entryId = args?.entry_id;
    const index = entries.findIndex((snapshot) => snapshot.entry.id === entryId);
    if (index >= 0) entries.splice(index, 1);
    return true;
  }
  throw new Error(`Unsupported watchlist fixture command: ${command}`);
};

ratesStore.set({
  VRSC: { rates: { USD: 2.1 }, usdChange24hPct: null },
  'fixture-pbaas-currency': { rates: { USD: 1 }, usdChange24hPct: null },
});
setContactSession({ sessionId: 'watchlist-fixture', network: 'mainnet' });
setWatchlistSession({ sessionId: 'watchlist-fixture', network: 'mainnet' });
document.documentElement.classList.toggle('dark', params.get('theme') === 'dark');
setLocale(params.get('locale') === 'nl' ? 'nl' : 'en');
mount(Fixture, { target: document.getElementById('fixture')! });
