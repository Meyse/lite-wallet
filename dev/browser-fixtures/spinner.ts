import '../../src/app.css';
import { mount } from 'svelte';
import { setLocale } from '$lib/i18n';
import { setContactSession, contactsLoadState } from '$lib/contacts/session';
import { setWatchlistSession } from '$lib/watchlist/session';
import Fixture from './SpinnerFixture.svelte';

const params = new URLSearchParams(location.search);
const pending = () => new Promise<never>(() => {});
Object.assign(window, {
  __TAURI_INTERNALS__: {
    invoke: async (command: string) => {
      if (command === 'resolve_contact_identity' || command === 'refresh_watchlist')
        return pending();
      if (command === 'get_watchlist_entries') {
        if (!params.has('refresh')) return pending();
        return [
          {
            id: 'example',
            targetKind: 'identity',
            displayName: 'alex@',
            address: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
            systemId: null,
            createdAt: 1,
            updatedAt: 1,
          },
        ];
      }
      throw new Error(`Unsupported spinner fixture command: ${command}`);
    },
    transformCallback: () => 1,
    unregisterCallback: () => {},
  },
});
setContactSession({ sessionId: 'spinner-fixture', network: 'mainnet' });
setWatchlistSession({ sessionId: 'spinner-fixture', network: 'mainnet' });
contactsLoadState.set('ready');
document.documentElement.classList.toggle('dark', params.get('theme') === 'dark');
setLocale(params.get('locale') === 'nl' ? 'nl' : 'en');
mount(Fixture, {
  target: document.getElementById('fixture')!,
  props: { screen: params.get('screen') ?? 'examples' },
});
