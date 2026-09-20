import '../../src/app.css';
import { mount } from 'svelte';
import Fixture from './TransferNavigationFixture.svelte';
import { contactsLoadState, setContactSession } from '$lib/contacts/session';
import { setAddressBookContacts } from '$lib/stores/addressBook';
import { coinsStore } from '$lib/stores/coins';
import { buildWalletChannels, walletChannelsStore } from '$lib/stores/walletChannels';
import { get } from 'svelte/store';
import { balanceStore } from '$lib/stores/balances';
import { setLocale } from '$lib/i18n';
const chain = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
const source = 'RAutMoGh771ECTDbTq2qwwZo7MF5Tov3ka';
const channelId = `vrpc.${source}.${chain}`;
const identity = {
  identityAddress: chain,
  fullyQualifiedName: 'alex.example@',
  network: 'mainnet',
  chainId: chain,
};
const linkedIdentities = [
  {
    identityAddress: chain,
    name: 'alex.example',
    fullyQualifiedName: 'alex.example@',
    status: 'active',
    systemId: chain,
    favorite: false,
  },
  {
    identityAddress: 'iLinkedIdentityWithoutDescription',
    name: 'verusbuilder',
    fullyQualifiedName: 'verusbuilder@',
    status: 'active',
    systemId: chain,
    favorite: false,
  },
  {
    identityAddress: 'iSecondLinkedIdentityWithoutDescription',
    name: 'max',
    fullyQualifiedName: 'max@',
    status: 'active',
    systemId: chain,
    favorite: false,
  },
];
const make = (id, name, endpoints, extra = {}) => ({
  id,
  displayName: name,
  note: null,
  createdAt: 1,
  updatedAt: 1,
  endpoints: endpoints.map((address, i) => ({
    id: `${id}-${i}`,
    address,
    normalizedAddress: address,
    label: 'Default',
    kind: address.startsWith('0x') ? 'eth' : address.startsWith('bc1') ? 'btc' : 'vrpc',
    createdAt: 1,
    updatedAt: 1,
    lastUsedAt: null,
  })),
  ...extra,
});
const params = new URLSearchParams(location.search);
if (params.has('long'))
  identity.fullyQualifiedName = 'averylongcanonicalidentityname.withparent.namespace@';
let saveAttempts = 0;
let contacts =
  (params.get('screen') === 'send' && !params.has('saved')) || params.has('unsaved')
    ? []
    : [
        make('alex', 'alex.example@', [chain, `0x${'ab'.repeat(20)}`], {
          identities: [identity],
          profileIdentity: identity,
        }),
        make('mom', 'Mom', ['bc1qggqzj0uzun238nhzzs5wdz2en05s0d9ncwhxcf']),
        make('studio', 'Studio', [source]),
      ];
const avatar = document.createElement('canvas');
avatar.width = 64;
avatar.height = 64;
const ctx = avatar.getContext('2d');
if (!ctx) throw new Error('Canvas context unavailable');
const gradient = ctx.createLinearGradient(0, 0, 64, 64);
gradient.addColorStop(0, '#007baf');
gradient.addColorStop(1, '#83dabd');
ctx.fillStyle = gradient;
ctx.fillRect(0, 0, 64, 64);
ctx.fillStyle = '#d7edb9';
ctx.beginPath();
ctx.ellipse(26, 28, 8, 14, -0.8, 0, Math.PI * 2);
ctx.fill();
ctx.beginPath();
ctx.ellipse(39, 23, 7, 13, 0.8, 0, Math.PI * 2);
ctx.fill();
ctx.strokeStyle = '#245d50';
ctx.lineWidth = 3;
ctx.beginPath();
ctx.moveTo(32, 52);
ctx.lineTo(32, 28);
ctx.stroke();
window.fixture = {
  delay: Number(params.get('delay') || 0),
  fail: params.has('fail'),
  calls: [],
  identity,
  setContacts: (value) => {
    contacts = value;
    setAddressBookContacts(value);
  },
  theme: (theme) => document.documentElement.classList.toggle('dark', theme === 'dark'),
  locale: setLocale,
  clear: () => {
    contacts = [];
    setAddressBookContacts([]);
  },
};
window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
window.__TAURI_INTERNALS__ = {
  transformCallback: () => 1,
  unregisterCallback: () => {},
  invoke: async (command, args) => {
    window.fixture.calls.push({ command, args });
    if (command === 'get_linked_identities')
      return structuredClone(
        params.has('single') ? linkedIdentities.slice(1, 2) : linkedIdentities
      );
    if (command === 'get_pending_identity_profile_updates') return [];
    if (command === 'list_identity_provisioning_jobs')
      return params.has('provisioning')
        ? Array.from({ length: 6 }, (_, index) => ({
            jobId: `fixture-provisioning-${index}`,
            requestType: 'identity_provisioning',
            requestHex: `fixture-request-${index}`,
            requestedIdentityAddress: null,
            requestedFqn: `pending-${index + 1}.example@`,
            signingId: `provisioning.service.${index + 1}@`,
            hasResponseUris: index % 2 === 0,
            infoUri: null,
            status: index % 3 === 0 ? 'ready' : 'pending',
            createdAt: index + 1,
            error: null,
          }))
        : [];
    if (command === 'discover_linkable_identities') return [];
    if (command === 'get_identity_profile' && params.has('empty'))
      return { state: 'empty', issues: [], revisionTxid: 'empty' };
    if (command === 'get_identity_profile' && params.has('unavailable'))
      return { state: 'unavailable', issues: [], revisionTxid: null };
    if (
      command === 'get_identity_profile' &&
      args.identity_address &&
      args.identity_address !== chain
    )
      return { state: 'empty', issues: [], revisionTxid: 'empty' };
    if (command === 'get_identity_profile')
      return {
        state: 'ready',
        avatar: {
          value: {
            base64: params.has('broken')
              ? 'invalid-image'
              : avatar.toDataURL('image/png').split(',')[1],
            mimeType: 'image/png',
            width: 64,
            height: 64,
            byteLength: 100,
          },
          source: {},
        },
        description: {
          value: params.has('lines')
            ? 'A\n'.repeat(75)
            : params.has('long')
              ? 'A long public description with a full parent identity name. This text wraps naturally and keeps the contact action visible on a compact desktop. 🌱'
              : 'Building things. Growing a little every day.',
          source: {},
        },
        issues: [],
        revisionTxid: 'fixture-confirmed',
      };
    if (command === 'resolve_contact_identity')
      return args.identity === 'bob@'
        ? {
            ...identity,
            identityAddress: 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq',
            fullyQualifiedName: 'bob@',
          }
        : identity;
    if (command === 'list_address_book_contacts') return structuredClone(contacts);
    if (command === 'save_address_book_contact') {
      await new Promise((r) => setTimeout(r, window.fixture.delay));
      if (window.fixture.fail || (params.has('failOnce') && saveAttempts++ === 0))
        throw new Error('Synthetic storage failure');
      const savedIdentity = args.request.identities?.[0] || identity;
      const saved = make(
        savedIdentity.identityAddress,
        savedIdentity.fullyQualifiedName,
        [savedIdentity.identityAddress],
        { identities: [savedIdentity], profileIdentity: savedIdentity }
      );
      contacts.push(saved);
      return saved;
    }
    if (command === 'validate_destination_address')
      return { valid: true, normalizedAddress: args.request.address };
    if (command === 'get_addresses')
      return { vrsc_address: source, eth_address: `0x${'11'.repeat(20)}`, btc_address: '' };
    if (command === 'get_coin_scopes')
      return {
        coinId: 'VRSC',
        scopes: [
          {
            channelId,
            coinId: 'VRSC',
            address: source,
            addressLabel: 'Public address',
            systemId: chain,
            systemTicker: 'VRSC',
            systemDisplayName: 'Verus',
            isPrimaryAddress: true,
            isReadOnly: false,
            scopeKind: 'transparent',
          },
        ],
      };
    if (command === 'get_bridge_capabilities')
      return { conversionSupported: true, executionEngine: 'vrpc' };
    if (command === 'get_bridge_conversion_paths')
      return {
        sourceCurrency: chain,
        paths: {
          ['iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq']: [
            {
              destinationId: 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq',
              destinationDisplayName: 'Verus USD',
              destinationDisplayTicker: 'vUSDC',
              convertTo: 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq',
              convertToDisplayName: 'Verus USD',
              via: chain,
              viaDisplayName: 'Bridge.vETH',
              price: '1.25',
              gateway: false,
              mapping: false,
              bounceback: false,
              ethDestination: false,
            },
          ],
        },
      };
    if (command === 'get_dlight_seed_status') return { configured: false };
    if (command === 'estimate_bridge_conversion')
      return { estimatedCurrencyOut: '1.25', price: '1.25' };
    if (command === 'get_transaction_history_page')
      return { transactions: [], hasMore: false, nextCursor: null };
    if (command === 'get_balances') return { confirmed: '100', pending: '0', total: '100' };
    if (command === 'preflight_send')
      return {
        preflightId: 'fixture-preflight',
        fee: '0.0001',
        feeCurrency: 'VRSC',
        value: args.params.amount,
        amountSubmitted: args.params.amount,
        toAddress: chain,
        fromAddress: source,
        feeTakenFromAmount: false,
        warnings: [{ warningType: 'resolved_destination', message: 'fixture' }],
        memo: null,
      };
    if (command === 'get_pending_eth_submission' || command === 'get_dlight_runtime_status')
      return null;
    if (command === 'plugin:os|type') return 'macos';
    if (command === 'plugin:event|listen') return 1;
    if (command === 'plugin:event|unlisten' || command === 'mark_address_book_endpoint_used')
      return true;
    if (command === 'get_transaction_history') return [];
    if (command === 'send_transaction' && params.has('simulateReceipt'))
      return {
        txid: 'f'.repeat(64),
        fromAddress: source,
        toAddress: identity.identityAddress,
        value: '1',
        fee: '0.0001',
        feeCurrency: 'VRSC',
      };
    if (command === 'send_transaction') throw new Error('Signing disabled in fixture');
    return null;
  },
};
if (params.get('theme') === 'dark') document.documentElement.classList.add('dark');
setLocale(params.get('locale') === 'nl' ? 'nl' : 'en');
setContactSession({ sessionId: 'fixture-session', network: 'mainnet' });
setAddressBookContacts(contacts);
contactsLoadState.set('ready');
coinsStore.set([
  {
    id: 'VRSC',
    currencyId: chain,
    systemId: chain,
    displayTicker: 'VRSC',
    displayName: 'Verus',
    proto: 'vrsc',
    compatibleChannels: ['vrpc'],
    decimals: 8,
    vrpcEndpoints: [],
    secondsPerBlock: 60,
    isTestnet: false,
  },
]);
balanceStore.set({ [channelId]: { VRSC: { confirmed: '100', pending: '0', total: '100' } } });
walletChannelsStore.set(buildWalletChannels(get(coinsStore), source));
const fixtureTarget = document.getElementById('fixture');
if (!fixtureTarget) throw new Error('Fixture target unavailable');
mount(Fixture, { target: fixtureTarget });
