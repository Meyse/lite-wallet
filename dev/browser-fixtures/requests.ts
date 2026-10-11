import '$lib/shims/node-globals';
import '../../src/app.css';
import { mount } from 'svelte';
import { setLocale, type Locale } from '$lib/i18n';
import { coinsStore } from '$lib/stores/coins';
import { walletChannelsStore } from '$lib/stores/walletChannels';
import type {
  AuthenticationDetailSession,
  DataPacketDetailSession,
  GenericRequestFlowSession,
  IdentityUpdateDetailSession,
  InvoiceDetailSession,
  ProvisionIdentityDetailSession,
  UserDataDetailSession,
} from '$lib/genericRequest/session';
import Fixture from './RequestsFixture.svelte';

// Synthetic requests for the guided flow. Nothing is signed, broadcast or sent:
// every wallet command below returns fixed data.
const params = new URLSearchParams(location.search);
const scenario = params.get('scenario') ?? 'update';
const fail = params.get('fail');
const stall = params.get('stall');
const TESTNET = 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq';
const REQUESTER = 'iGHXn8mvh9kutBNotJ2LCNz3zDxvemEb2i';
const PLAYER7 = 'i8KEYF3QeR1xPVVCL5BSpDXkA1CAfFDvmk';
const WALLET_ADDRESS = 'RFWcHcpnFh57ovrV1hmzN8o8wmMZjyqCSh';
const SHIELDED = 'zs13akhfdhypx9a8t2efkfgxpd5d9h3glz7d5pug2ys65mrzt4mw5vjqx0vw29gx82y3e0xjjuhrwn';
const NOTE_KEY = 'iNoteKeyFixture1111111111111111111';
const nowSeconds = Math.floor(Date.now() / 1000);

const auth: AuthenticationDetailSession = {
  index: 0,
  type: 'authentication',
  requestId: 'iAuthRequestFixture111111111111111',
  expiryTime: scenario === 'expired' ? nowSeconds - 60 : nowSeconds + 9 * 60,
  constraints:
    scenario === 'newid'
      ? [{ type: 3, identity: 'iParentFixture11111111111111111111' }]
      : [{ type: 2, identity: TESTNET }],
  raw: {} as AuthenticationDetailSession['raw'],
};
const update: IdentityUpdateDetailSession = {
  index: 1,
  type: 'identityUpdate',
  requestId: 'iUpdateRequestFixture1111111111111',
  targetIdentityAddress: PLAYER7,
  requestedIdentityJson: {
    name: 'player7',
    privateaddress: SHIELDED,
    contentmultimap: { [NOTE_KEY]: [{ text: 'Disposable QA note' }] },
  },
  sourceSystemId: TESTNET,
  expiryHeight: 1270000,
  raw: {} as IdentityUpdateDetailSession['raw'],
};
const provision: ProvisionIdentityDetailSession = {
  index: 1,
  type: 'provisionIdentity',
  requestedFqn: 'alex.lumen.VRSCTEST@',
  systemId: TESTNET,
  parent: 'iParentFixture11111111111111111111',
  webhook: 'https://lumen.example/provision',
  requestId: 'iProvisionFixture11111111111111111',
  raw: {} as ProvisionIdentityDetailSession['raw'],
};

const invoice: InvoiceDetailSession = {
  index: 1,
  type: 'invoice',
  acceptsAnyAmount: scenario === 'invoiceAny',
};

const dataPacket: DataPacketDetailSession = {
  index: 1,
  type: 'dataPacket',
  items: [
    {
      kind: 'text',
      label: null,
      mimeType: null,
      text: 'I am signing this disposable test message.',
      hex: '',
      size: 42,
      sha256: '13e625c7c70ab0dec07b148d009af34aad4b6d02b459345d3406236245c9083c',
    },
    {
      kind: 'data',
      label: 'Sample JSON',
      mimeType: 'application/json',
      text: '{"test":"mobile-review","approved":true}',
      hex: '7b2274657374223a226d6f62696c652d726576696577222c22617070726f766564223a747275657d',
      size: 40,
      sha256: 'f7c34af0f91afccef5e64be75ad16f9ddc25ec1fb4d4c458aabcb4b0062895a2',
    },
    {
      kind: 'data',
      label: 'Opaque test bytes',
      mimeType: 'application/octet-stream',
      text: null,
      hex: '00ff80010203',
      size: 6,
      sha256: '88c49f430623685ab229e73e9e555a2700f7801a40020e290b169f8265ac1888',
    },
  ],
  statements: ['These signatures are for a disposable test.'],
};

// Share data: Deep Link Sim's fixture keys (`data-lab-fixtures.ts`).
const userData: UserDataDetailSession = {
  index: 1,
  type: 'userData',
  keys: [
    'iEwa1Py7XEjJNvte13WogabJkTDVv7Vg5Z',
    'iRXFrvRaaVT7hKsQc9bqcHa1AAmtzXe5jb',
    'iNZn695ZGgyUJUBVQBFcVEpc8RBsbvuMcK',
  ],
  requiredIdentity: null,
};

const details =
  scenario === 'shareData' || scenario === 'shareDataNone' || scenario === 'shareDataError'
    ? [{ ...userData, index: 0 }]
    : scenario === 'authShareData'
      ? [auth, userData]
      : scenario === 'signData'
        ? [{ ...dataPacket, index: 0 }]
        : scenario === 'authSignData'
          ? [auth, dataPacket]
          : scenario === 'update'
            ? [auth, update]
            : scenario === 'updateOnly'
              ? [{ ...update, index: 0 }]
              : scenario === 'newid'
                ? [auth, provision]
                : scenario === 'invoice'
                  ? [auth, invoice]
                  : scenario === 'invoiceOnly' ||
                      scenario === 'invoiceAny' ||
                      scenario === 'invoicePaid'
                    ? [{ ...invoice, index: 0 }]
                    : [auth];
const session: GenericRequestFlowSession = {
  request: {} as GenericRequestFlowSession['request'],
  requestHex: '00',
  requestId: 'iEnvelopeFixture111111111111111111',
  testnet: true,
  responseUris: [
    { mode: 'post', uri: 'https://lumen.example/verus/response' },
    ...(params.has('redirect')
      ? [{ mode: 'redirect' as const, uri: 'https://lumen.example/verus/return' }]
      : []),
  ],
  signer: { systemId: TESTNET, identityId: REQUESTER, isSigned: true },
  details,
  passthroughAutoLinkFqn: null,
};

const review = {
  targetIdentity: PLAYER7,
  warnings: [],
  highRiskChanges: [],
  currentIdentity: { name: 'player7', contentmultimap: {} },
  requestedIdentity: update.requestedIdentityJson,
  fullyQualifiedName: 'player7.VRSCTEST@',
  friendlyNames: {},
  signerCmmKeyLabels: { [NOTE_KEY]: 'QA note' },
  primaryAddressAfterUpdateInfo: { addresses: [], walletCount: 0, externalCount: 0 },
  currentAuthorities: {},
};
const failures = new Set(fail ? [fail] : []);
const pending = () => new Promise<never>(() => {});
const once = async (stage: string) => {
  if (stall === stage) await pending();
  if (failures.has(stage)) {
    failures.delete(stage);
    // Wallet commands reject with a typed error object, not an Error.
    throw { type: 'NetworkError', message: 'Synthetic fixture failure' };
  }
};

window.__PROFILE_FIXTURE_INVOKE__ = async (command, args) => {
  switch (command) {
    case 'get_linked_identities':
      return [
        {
          identityAddress: PLAYER7,
          name: 'player7',
          fullyQualifiedName: 'player7.VRSCTEST@',
          systemId: TESTNET,
          favorite: false,
        },
        {
          identityAddress: 'iBV3ZM3rqLpgEJKiZk4Pw4Lq3GAjEcGmA2',
          name: 'player6',
          fullyQualifiedName: 'player6.VRSCTEST@',
          systemId: TESTNET,
          favorite: false,
        },
      ];
    case 'get_identity_details':
      // Only the requester sits under the parent that the new VerusID scenario requires.
      return {
        identityAddress: String(args?.identity_address ?? REQUESTER),
        name: 'TestBob',
        fullyQualifiedName: 'TestBob.antafri.VRSCTEST@',
        parent:
          args?.identity_address === REQUESTER
            ? 'iParentFixture11111111111111111111'
            : 'iOtherParentFixture111111111111111',
        primaryAddresses: [],
        ownedByPrimaryAddress: false,
        minimumSignatures: 1,
        tokenizedControl: false,
        profileEditable: false,
        warnings: [],
      };
    case 'review_generic_identity_update':
      return review;
    case 'preflight_generic_identity_update':
      return {
        ...review,
        preflightId: 'fixture-preflight',
        fromAddress: WALLET_ADDRESS,
        fee: '0.0001',
        feeCurrency: TESTNET,
      };
    case 'get_coin_scopes':
      return {
        coinId: 'VRSCTEST',
        scopes: [
          {
            channelId: `vrpc.${WALLET_ADDRESS}.${TESTNET}`,
            coinId: 'VRSCTEST',
            address: WALLET_ADDRESS,
            addressLabel: '',
            systemId: TESTNET,
            systemTicker: 'VRSCTEST',
            systemDisplayName: 'Verus Testnet',
            isPrimaryAddress: true,
            isReadOnly: false,
            scopeKind: 'transparent',
          },
          {
            channelId: `vrpc.${PLAYER7}.${TESTNET}`,
            coinId: 'VRSCTEST',
            address: PLAYER7,
            addressLabel: 'player7@',
            systemId: TESTNET,
            systemTicker: 'VRSCTEST',
            systemDisplayName: 'Verus Testnet',
            isPrimaryAddress: false,
            isReadOnly: false,
            scopeKind: 'transparent',
          },
          {
            channelId: 'vrpc.watched',
            coinId: 'VRSCTEST',
            address: 'RWatchedFixture1111111111111111111',
            addressLabel: 'Watched',
            systemId: TESTNET,
            systemTicker: 'VRSCTEST',
            systemDisplayName: 'Verus Testnet',
            isPrimaryAddress: false,
            isReadOnly: true,
            scopeKind: 'transparent',
          },
        ],
      };
    case 'review_generic_invoice':
      return {
        amount: scenario === 'invoiceAny' ? null : '0.01000000',
        currencyId: TESTNET,
        currencyName: 'VRSCTEST',
        destination: REQUESTER,
        destinationName: 'TestBob.antafri.VRSCTEST@',
        expiryHeight: 1270600,
        blocksLeft: 55,
        alreadyPaid: scenario === 'invoicePaid',
      };
    case 'preflight_generic_invoice':
      return {
        preflightId: 'fixture-invoice-preflight',
        amount: String(args?.amount ?? '0.01000000'),
        fee: '0.00010000',
        currencyId: TESTNET,
        feeCurrencyId: TESTNET,
        fromAddress: WALLET_ADDRESS,
        toAddress: REQUESTER,
      };
    case 'send_generic_invoice_payment':
      await once('transaction');
      return { txid: '0600eba8a139873d6c3dba0a2dd6ebb8589ffb44a5ac9386102c123b776773f7' };
    case 'get_dlight_seed_status':
      return { configured: true };
    case 'review_generic_credentials':
      if (scenario === 'shareDataError') {
        throw { type: 'NetworkError', message: 'Synthetic fixture failure' };
      }
      if (scenario === 'shareDataNone') {
        return { credentials: [], missingKeys: userData.keys };
      }
      return {
        credentials: [
          {
            key: userData.keys[0],
            label: 'Disposable test profile',
            credential: { displayName: 'Deeplink test user', purpose: 'Mobile redesign QA only' },
            scopes: [REQUESTER],
          },
          {
            key: userData.keys[1],
            label: 'Disposable test membership',
            credential: { membership: 'Test group', level: 'Sample' },
            scopes: [REQUESTER],
          },
        ],
        missingKeys: [userData.keys[2]],
      };
    case 'get_balances':
      return { confirmed: '2.63337', pending: '0', total: '2.63337' };
    case 'send_generic_identity_update':
      await once('transaction');
      return { txid: '08dc8363d710607560f48471bbfc4ea88c34e22ab3d5e9e42f66b545fa759bd4' };
    case 'build_and_sign_generic_response':
      await once('signing');
      return { signedResponseHex: '0001fixture', sessionId: 'fixture-session' };
    case 'post_generic_response_callback':
    case 'open_generic_request_callback':
      await once('delivery');
      return null;
    default:
      throw new Error(`Unsupported requests fixture command: ${command}`);
  }
};

coinsStore.set([
  {
    id: 'VRSCTEST',
    currencyId: TESTNET,
    systemId: TESTNET,
    displayTicker: 'VRSCTEST',
    displayName: 'Verus Testnet',
    proto: 'vrsc',
    compatibleChannels: ['vrpc'],
    decimals: 8,
    vrpcEndpoints: [],
    secondsPerBlock: 60,
    isTestnet: true,
  },
]);
walletChannelsStore.set({
  byCoinId: {},
  channels: [],
  primaryChannelId: null,
  vrpcAddress: WALLET_ADDRESS,
});
document.documentElement.classList.toggle('dark', params.get('theme') === 'dark');
setLocale((params.get('locale') as Locale | null) ?? 'en');
mount(Fixture, { target: document.getElementById('fixture')!, props: { session } });
