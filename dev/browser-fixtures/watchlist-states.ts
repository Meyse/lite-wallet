// Synthetic wallet data for visual inspection of the production Watchlist component.
import '../../src/app.css';
import { mount, tick } from 'svelte';
import { setLocale } from '$lib/i18n';
import { setContactSession } from '$lib/contacts/session';
import { setWatchlistSession } from '$lib/watchlist/session';
import { ratesStore } from '$lib/stores/rates';
import type { WatchlistEntry, WatchlistEntrySnapshot } from '$lib/types/watchlist';
import Fixture from './WatchlistStatesFixture.svelte';

const params = new URLSearchParams(location.search);
const state = params.get('state') ?? 'saved';
const pending = () => new Promise<never>(() => {});
const rawAddress = `R${'M'.repeat(33)}`;
const identityAddress = `i${'a'.repeat(33)}`;
const entries: WatchlistEntry[] = [
  {
    id: 'mom',
    targetKind: 'address',
    displayName: 'Mom',
    address: rawAddress,
    createdAt: 1,
    updatedAt: 1,
  },
  {
    id: 'mira',
    targetKind: 'identity',
    displayName: 'mira@',
    address: identityAddress,
    createdAt: 2,
    updatedAt: 2,
  },
];
const coin = {
  id: 'VRSC',
  currencyId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
  systemId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
  displayTicker: 'VRSC',
  displayName: 'Verus',
  proto: 'vrsc' as const,
  compatibleChannels: ['vrpc' as const],
  decimals: 8,
  vrpcEndpoints: [],
  secondsPerBlock: 60,
  isTestnet: false,
};
function snapshot(entry: WatchlistEntry): WatchlistEntrySnapshot {
  return {
    entry,
    holdings:
      state === 'detail-unavailable'
        ? []
        : [
            {
              assetKey: entry.id,
              currencyId: coin.currencyId,
              systemId: coin.systemId,
              systemTicker: 'VRSC',
              systemDisplayName: 'Verus',
              balance: entry.id === 'mom' ? '4.25' : '20',
              coin,
            },
          ],
    sources: [],
    availability:
      state === 'stale' || state === 'detail-stale'
        ? 'partial'
        : state === 'detail-unavailable'
          ? 'unavailable'
          : 'available',
    refreshedAt: 1,
  };
}
Object.assign(window, {
  __TAURI_INTERNALS__: {
    invoke: async (command: string, args: Record<string, unknown> = {}) => {
      if (command === 'get_watchlist_entries') {
        if (state === 'loading') return pending();
        if (state === 'error') throw new Error('Synthetic load failure');
        return state === 'empty' || state.startsWith('preview') || state === 'add' ? [] : entries;
      }
      if (command === 'refresh_watchlist' && state === 'detail-loading') return pending();
      if (command === 'refresh_watchlist')
        return { network: 'mainnet', entries: entries.map(snapshot), refreshedAt: 1 };
      if (command === 'validate_destination_address') {
        const request = args.request as { address: string };
        return { valid: true, normalizedAddress: request.address, reason: null };
      }
      if (command === 'list_address_book_contacts')
        return params.get('contact') === 'existing'
          ? [
              {
                id: 'mira-contact',
                displayName: 'mira@',
                note: null,
                createdAt: 1,
                updatedAt: 1,
                endpoints: [],
                identities: [
                  {
                    identityAddress,
                    fullyQualifiedName: 'mira@',
                    network: 'mainnet',
                    chainId: coin.currencyId,
                  },
                ],
              },
            ]
          : [];
      if (command === 'get_identity_profile')
        return { state: 'empty', issues: [], revisionTxid: null };
      if (command === 'resolve_watchlist_target') {
        if (state === 'lookup-error') throw { type: 'IdentityNotFound' };
        return {
          targetKind: state === 'preview-address' ? 'address' : 'identity',
          displayName: state === 'preview-address' ? rawAddress : 'mira@',
          address: state === 'preview-address' ? rawAddress : identityAddress,
          availability: 'available',
          visibleCurrencyCount: 1,
        };
      }
      if (command === 'add_watchlist_entry') {
        const request = args.request as { name?: string } | undefined;
        return snapshot({ ...entries[0], displayName: request?.name || entries[0].displayName });
      }
      if (command === 'remove_watchlist_entry') return true;
      throw new Error(`Unsupported Watchlist fixture command: ${command}`);
    },
    transformCallback: () => 1,
    unregisterCallback: () => {},
  },
});
setContactSession({ sessionId: 'watchlist-fixture', network: 'mainnet' });
setWatchlistSession({ sessionId: 'watchlist-fixture', network: 'mainnet' });
ratesStore.set({ VRSC: { rates: { USD: 2.2, EUR: 2 }, usdChange24hPct: null } });
document.documentElement.classList.toggle('dark', params.get('theme') === 'dark');
setLocale(params.get('locale') === 'nl' ? 'nl' : 'en');
const fixtureElement = document.getElementById('fixture');
if (!fixtureElement) throw new Error('Watchlist fixture host missing');
mount(Fixture, { target: fixtureElement });

async function advance(): Promise<void> {
  if (
    ![
      'add',
      'lookup-error',
      'preview-id',
      'preview-address',
      'detail',
      'detail-stale',
      'detail-unavailable',
      'detail-loading',
    ].includes(state)
  )
    return;
  for (let i = 0; i < 20; i++) {
    await new Promise((resolve) => setTimeout(resolve, 25));
    await tick();
    const row = document.querySelector<HTMLButtonElement>('[data-entry-id="mira"]');
    const add = [...document.querySelectorAll<HTMLButtonElement>('button')].find(
      (button) => button.textContent?.trim() === 'Add address'
    );
    if (state.startsWith('detail') && row) {
      row.click();
      return;
    }
    if (!state.startsWith('detail') && add) {
      add.click();
      if (state === 'add') return;
      await tick();
      const input = document.querySelector<HTMLInputElement>('#watchlist-target');
      if (!input) return;
      input.value = state === 'preview-address' ? rawAddress : 'mira@';
      input.dispatchEvent(new Event('input', { bubbles: true }));
      await tick();
      const cont = [...document.querySelectorAll<HTMLButtonElement>('button')].find(
        (button) => button.textContent?.trim() === 'Continue'
      );
      cont?.click();
      return;
    }
  }
}
void advance();
