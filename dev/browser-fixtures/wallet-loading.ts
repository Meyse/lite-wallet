import '../../src/app.css';
import { mount } from 'svelte';
import { setLocale } from '$lib/i18n';
import { coinsStore } from '$lib/stores/coins.js';
import { balanceStore } from '$lib/stores/balances.js';
import { ratesStore } from '$lib/stores/rates.js';
import { buildWalletChannels, walletChannelsStore } from '$lib/stores/walletChannels.js';
import { walletBootstrapStore } from '$lib/stores/walletBootstrap.js';
import type { CoinDefinition } from '$lib/types/wallet.js';
import Fixture from './WalletLoadingFixture.svelte';
import { setContactSession, contactsLoadState } from '$lib/contacts/session';
import { addressBookStore } from '$lib/stores/addressBook';

const params = new URLSearchParams(location.search);
const pending = () => new Promise<never>(() => {});
const coins: CoinDefinition[] = [
  {
    id: 'VRSC',
    currencyId: 'VRSC',
    systemId: 'verus',
    displayName: 'Verus',
    displayTicker: 'VRSC',
    proto: 'vrsc',
    compatibleChannels: ['vrpc'],
  },
  {
    id: 'ETH',
    currencyId: 'ETH',
    systemId: 'ETH',
    displayName: 'Ethereum',
    displayTicker: 'ETH',
    proto: 'eth',
    compatibleChannels: ['eth'],
  },
].map(
  (coin) =>
    ({
      decimals: 8,
      isTestnet: false,
      vrpcEndpoints: [],
      secondsPerBlock: 60,
      ...coin,
    }) as CoinDefinition
);
const channels = buildWalletChannels(coins, 'Rsynthetic');
const balance = (amount: string) => ({ total: amount, confirmed: amount, pending: '0' });
let balanceAttempts = 0;
let historyAttempts = 0;
const invoke = async (command: string, args: Record<string, unknown> = {}) => {
  if (command === 'delete_address_book_contact') return pending();
  if (command.startsWith('plugin:event|')) return 1;
  if (command === 'plugin:app|name') return 'Verus Express';
  if (command === 'plugin:app|identifier') return 'fixture.wallet';
  if (command === 'plugin:app|version') {
    if (params.get('wait') === 'version') return pending();
    if (params.has('failVersion')) throw new Error('Synthetic metadata failure');
    return '1.2.3';
  }
  if (command === 'get_dlight_seed_status') return { configured: false };
  if (command === 'get_coin_scopes') {
    if (params.get('wait') === 'scopes') return pending();
    if (params.has('failScopes')) throw new Error('Synthetic offline provider');
    const coinId = String(args.coin_id);
    return {
      coinId,
      scopes: [
        {
          coinId,
          channelId: channels.byCoinId[coinId],
          systemId: coinId === 'ETH' ? 'ETH' : 'verus',
          systemTicker: coinId,
          systemDisplayName: coinId === 'ETH' ? 'Ethereum' : 'Verus',
          address: '0x1111111111111111111111111111111111111111',
          addressLabel: 'My address',
          isPrimaryAddress: true,
          isReadOnly: false,
          scopeKind: 'transparent',
        },
      ],
    };
  }
  if (command === 'get_balances') {
    if (params.has('cold')) return pending();
    if (args.coin_id === 'ETH' && params.has('failBalance') && balanceAttempts++ === 0)
      throw new Error('Synthetic balance failure');
    if (args.coin_id === 'ETH' && (params.get('wait') === 'balances' || !params.get('screen')))
      return pending();
    return balance(args.coin_id === 'ETH' ? '2' : '24.5');
  }
  if (command === 'get_transaction_history_page') {
    if (params.has('failHistory') && historyAttempts++ === 0)
      throw new Error('Synthetic history failure');
    if (params.get('wait') === 'transactions') return pending();
    return { transactions: [], hasMore: false, nextCursor: null };
  }
  throw new Error(`Unsupported fixture command: ${command}`);
};
Object.assign(window, {
  __TAURI_INTERNALS__: { invoke, transformCallback: () => 1, unregisterCallback: () => {} },
  __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
});
coinsStore.set(coins);
setContactSession({ sessionId: 'loading-fixture', network: 'mainnet' });
addressBookStore.set([
  {
    id: 'example-contact',
    displayName: 'Example contact',
    note: null,
    createdAt: 1,
    updatedAt: 1,
    endpoints: [
      {
        id: 'example-address',
        kind: 'eth',
        address: '0x1111111111111111111111111111111111111111',
        normalizedAddress: '0x1111111111111111111111111111111111111111',
        label: '',
        lastUsedAt: null,
        createdAt: 1,
        updatedAt: 1,
      },
    ],
  },
]);
contactsLoadState.set('ready');
walletChannelsStore.set(channels);
balanceStore.set(
  params.has('cold')
    ? {}
    : {
        [channels.byCoinId.VRSC]: { VRSC: balance('24.5') },
        ...(params.has('cachedBalance') ? { [channels.byCoinId.ETH]: { ETH: balance('2') } } : {}),
      }
);
ratesStore.set({ VRSC: { rates: { EUR: 2, USD: 2.2 } } });
walletBootstrapStore.set(true);
document.documentElement.classList.toggle('dark', params.get('theme') === 'dark');
setLocale(params.get('locale') === 'nl' ? 'nl' : 'en');
mount(Fixture, {
  target: document.getElementById('fixture')!,
  props: { screen: params.get('screen') ?? 'overview' },
});
