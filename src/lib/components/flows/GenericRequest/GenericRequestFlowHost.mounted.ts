// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '$lib/i18n';
import { coinsStore } from '$lib/stores/coins';
import type {
  AuthenticationDetailSession,
  DataPacketDetailSession,
  GenericRequestFlowSession,
  GiftCardDetailSession,
  IdentityUpdateDetailSession,
  InvoiceDetailSession,
  ProvisionIdentityDetailSession,
  UserDataDetailSession,
} from '$lib/genericRequest/session';
import GenericRequestFlowHost from './GenericRequestFlowHost.svelte';

// The wallet command boundary is the only mock: the shell, its steps and the
// approval stages run as in the app.
const wallet = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('$lib/services/invokeWalletCommand.js', () => ({
  invokeWalletCommand: wallet.invoke,
  invokeSessionBoundWalletCommand: wallet.invoke,
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
HTMLElement.prototype.scrollIntoView = () => {};

const TESTNET = 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq';
const TARGET = 'i8KEYF3QeR1xPVVCL5BSpDXkA1CAfFDvmk';
const ADDRESS = 'RFWcHcpnFh57ovrV1hmzN8o8wmMZjyqCSh';
const TXID = '08dc8363d710607560f48471bbfc4ea88c34e22ab3d5e9e42f66b545fa759bd4';

const auth: AuthenticationDetailSession = {
  index: 0,
  type: 'authentication',
  requestId: 'iAuthRequest',
  expiryTime: Math.floor(Date.now() / 1000) + 600,
  constraints: [],
  raw: {} as AuthenticationDetailSession['raw'],
};
const update: IdentityUpdateDetailSession = {
  index: 1,
  type: 'identityUpdate',
  requestId: 'iUpdateRequest',
  targetIdentityAddress: TARGET,
  requestedIdentityJson: { name: 'player7', contentmultimap: { iNoteKey: ['note'] } },
  sourceSystemId: TESTNET,
  expiryHeight: 100,
  raw: {} as IdentityUpdateDetailSession['raw'],
};
const session: GenericRequestFlowSession = {
  request: {} as GenericRequestFlowSession['request'],
  requestHex: '00',
  requestId: 'iEnvelope',
  testnet: true,
  responseUris: [{ mode: 'post', uri: 'https://lumen.example/response' }],
  signer: { systemId: TESTNET, identityId: 'iRequester', isSigned: true },
  details: [auth, update],
  passthroughAutoLinkFqn: null,
};
const review = {
  targetIdentity: TARGET,
  warnings: [],
  highRiskChanges: [],
  currentIdentity: { name: 'player7', contentmultimap: {} },
  requestedIdentity: update.requestedIdentityJson,
  fullyQualifiedName: 'player7.VRSCTEST@',
  friendlyNames: {},
  signerCmmKeyLabels: {},
  primaryAddressAfterUpdateInfo: { addresses: [], walletCount: 0, externalCount: 0 },
  currentAuthorities: {},
};

function commandsCalled(name: string) {
  return wallet.invoke.mock.calls.filter(([command]) => command === name).length;
}

async function settle() {
  for (let i = 0; i < 6; i += 1) {
    await tick();
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
  flushSync();
}

function button(label: string): HTMLButtonElement {
  const match = [...document.querySelectorAll('button')].find(
    (element) => element.textContent?.trim() === label
  );
  if (!match) throw new Error(`No button "${label}"`);
  return match;
}

async function press(label: string) {
  button(label).click();
  await settle();
}

describe('request flow approval', () => {
  let component: ReturnType<typeof mount> | null = null;
  let deliveryFailures = 0;

  beforeEach(() => {
    setLocale('en');
    deliveryFailures = 1;
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
    wallet.invoke.mockReset();
    wallet.invoke.mockImplementation(async (command: string) => {
      switch (command) {
        case 'get_linked_identities':
          return [
            {
              identityAddress: TARGET,
              fullyQualifiedName: 'player7.VRSCTEST@',
              systemId: TESTNET,
              favorite: false,
            },
          ];
        case 'get_identity_details':
          throw { type: 'NetworkError' };
        case 'review_generic_identity_update':
          return review;
        case 'preflight_generic_identity_update':
          return {
            ...review,
            preflightId: 'preflight-1',
            fromAddress: ADDRESS,
            fee: '0.0001',
            feeCurrency: TESTNET,
          };
        case 'get_coin_scopes':
          return {
            coinId: 'VRSCTEST',
            scopes: [
              {
                channelId: 'vrpc.primary',
                coinId: 'VRSCTEST',
                address: ADDRESS,
                addressLabel: '',
                systemId: TESTNET,
                systemTicker: 'VRSCTEST',
                systemDisplayName: 'Verus Testnet',
                isPrimaryAddress: true,
                isReadOnly: false,
                scopeKind: 'transparent',
              },
            ],
          };
        case 'get_balances':
          return { confirmed: '2', pending: '0', total: '2' };
        case 'send_generic_identity_update':
          return { txid: TXID };
        case 'build_and_sign_generic_response':
          return { signedResponseHex: '0001', sessionId: 'session-1' };
        case 'review_generic_invoice':
          return {
            amount: '0.01000000',
            currencyId: TESTNET,
            currencyName: 'VRSCTEST',
            destination: 'iGHXn8mvh9kutBNotJ2LCNz3zDxvemEb2i',
            destinationName: 'TestBob.antafri.VRSCTEST@',
            expiryHeight: 1270600,
            blocksLeft: 50,
            alreadyPaid: false,
          };
        case 'preflight_generic_invoice':
          return {
            preflightId: 'invoice-preflight',
            amount: '0.01000000',
            fee: '0.00010000',
            currencyId: TESTNET,
            feeCurrencyId: TESTNET,
            fromAddress: ADDRESS,
            toAddress: 'iGHXn8mvh9kutBNotJ2LCNz3zDxvemEb2i',
          };
        case 'send_generic_invoice_payment':
          return { txid: TXID };
        case 'pause_auto_lock_for_request':
        case 'resume_auto_lock':
          return undefined;
        case 'get_dlight_seed_status':
          return { configured: true };
        case 'review_gift_card':
          return {
            claimAddress: 'RGiftCard',
            currencies: [{ currencyId: TESTNET, name: 'VRSCTEST', amount: '0.06' }],
            identities: [],
          };
        case 'preflight_gift_card_claim':
          return {
            preflightId: 'gift',
            fee: '0.0001',
            feeCurrencyId: TESTNET,
            transactionCount: 1,
          };
        case 'send_gift_card_claim':
          return { txids: [TXID] };
        case 'review_generic_credentials':
          return {
            credentials: [
              {
                key: 'iProfileKey',
                label: 'Test profile',
                credential: { displayName: 'Test user' },
                scopes: ['iRequester'],
              },
              {
                key: 'iMembershipKey',
                label: 'Test membership',
                credential: { level: 'Sample' },
                scopes: ['iRequester'],
              },
            ],
            missingKeys: [],
          };
        case 'post_generic_response_callback':
          if (deliveryFailures > 0) {
            deliveryFailures -= 1;
            throw { type: 'NetworkError' };
          }
          return null;
        default:
          throw new Error(`Unexpected command ${command}`);
      }
    });
  });

  afterEach(() => {
    if (component) unmount(component);
    component = null;
    document.body.innerHTML = '';
  });

  it('broadcasts nothing before Approval and never twice when only delivery is retried', async () => {
    component = mount(GenericRequestFlowHost, {
      target: document.body,
      props: { isOpen: true, session },
    });
    await settle();

    await press('Continue'); // Sign in, with the only eligible VerusID chosen
    // The finished Sign in step names the VerusID that answers, once.
    const signInStep = [...document.querySelectorAll('ol li')].find((item) =>
      item.textContent?.includes('Sign in')
    );
    expect(signInStep?.textContent).toContain('as player7@');
    await press('Continue'); // Update identity: content only, no acknowledgement
    expect(document.body.textContent).toContain('Choose a source');
    // Nothing is preselected, so Continue waits for a choice.
    expect(button('Continue').disabled).toBe(true);
    [...document.querySelectorAll('button')]
      .find((element) => element.textContent?.includes('Choose a source'))
      ?.click();
    await settle();
    (document.querySelector('[role="dialog"] li button') as HTMLButtonElement).click();
    await settle();
    expect(document.body.textContent).toContain('0.0001 VRSCTEST');
    await press('Continue');
    expect(commandsCalled('send_generic_identity_update')).toBe(0);
    expect(commandsCalled('build_and_sign_generic_response')).toBe(0);

    await press('Sign in and update');
    expect(document.body.textContent).toContain('Signed, not delivered');
    await press('Try again');

    expect(document.body.textContent).toContain('player7@ is updated');
    expect(commandsCalled('send_generic_identity_update')).toBe(1);
    expect(commandsCalled('build_and_sign_generic_response')).toBe(1);
    expect(commandsCalled('post_generic_response_callback')).toBe(2);
    // Auto-lock pauses around each run of the stages and resumes after it.
    const order = wallet.invoke.mock.calls
      .map(([command]) => command)
      .filter((command) =>
        [
          'pause_auto_lock_for_request',
          'send_generic_identity_update',
          'post_generic_response_callback',
          'resume_auto_lock',
        ].includes(command)
      );
    expect(order).toEqual([
      'pause_auto_lock_for_request',
      'send_generic_identity_update',
      'post_generic_response_callback',
      'resume_auto_lock',
      'pause_auto_lock_for_request',
      'post_generic_response_callback',
      'resume_auto_lock',
    ]);
    const signCall = wallet.invoke.mock.calls.find(
      ([command]) => command === 'build_and_sign_generic_response'
    );
    // Rust reads the request IDs and the txid itself; the UI names the responder.
    expect(signCall?.[1]).toEqual({
      request_hex: '00',
      signer: { systemId: TESTNET, identityId: TARGET },
    });
  });

  it('names a new VerusID only when its name is known', async () => {
    const provision: ProvisionIdentityDetailSession = {
      index: 1,
      type: 'provisionIdentity',
      requestedFqn: null,
      systemId: TESTNET,
      parent: null,
      webhook: 'https://lumen.example/provision',
      requestId: null,
      raw: null,
    };
    component = mount(GenericRequestFlowHost, {
      target: document.body,
      props: { isOpen: true, session: { ...session, details: [auth, provision] } },
    });
    await settle();
    [...document.querySelectorAll('button')]
      .find((element) => element.textContent?.includes('player7@'))
      ?.click();
    await settle();
    await press('New VerusID');
    expect(document.body.textContent).toContain('A new VerusID answers this request.');

    await press('Continue'); // The name step, before a name is typed
    expect(document.body.textContent).not.toContain('can be provisioned');
  });

  it('signs and sends nothing when the request is declined', async () => {
    component = mount(GenericRequestFlowHost, {
      target: document.body,
      props: { isOpen: true, session },
    });
    await settle();
    await press('Decline');

    expect(document.body.textContent).toContain('Nothing was signed or sent.');
    for (const command of [
      'send_generic_identity_update',
      'build_and_sign_generic_response',
      'post_generic_response_callback',
    ]) {
      expect(commandsCalled(command)).toBe(0);
    }
  });

  it('pays an invoice once, with the fee on top, and sends no response for it alone', async () => {
    const invoice: InvoiceDetailSession = { index: 0, type: 'invoice', acceptsAnyAmount: false };
    component = mount(GenericRequestFlowHost, {
      target: document.body,
      props: {
        isOpen: true,
        session: { ...session, signer: { ...session.signer }, details: [invoice] },
      },
    });
    await settle();

    expect(document.body.textContent).toContain('0.01 VRSCTEST');
    expect(button('Continue').disabled).toBe(true);
    [...document.querySelectorAll('button')]
      .find((element) => element.textContent?.includes('Choose a source'))
      ?.click();
    await settle();
    (document.querySelector('[role="dialog"] li button') as HTMLButtonElement).click();
    await settle();
    expect(document.body.textContent).toContain('0.0101 VRSCTEST');
    await press('Continue');
    expect(commandsCalled('send_generic_invoice_payment')).toBe(0);

    await press('Pay 0.01 VRSCTEST');
    expect(document.body.textContent).toContain('You paid 0.01 VRSCTEST');
    expect(commandsCalled('send_generic_invoice_payment')).toBe(1);
    // An invoice alone adds no response detail, so nothing is signed or sent.
    expect(commandsCalled('build_and_sign_generic_response')).toBe(0);
    expect(commandsCalled('post_generic_response_callback')).toBe(0);
  });

  it('signs data only after every item and statement has consent and Approval', async () => {
    deliveryFailures = 0;
    const packet: DataPacketDetailSession = {
      index: 0,
      type: 'dataPacket',
      items: [
        {
          kind: 'text',
          label: null,
          mimeType: null,
          text: 'I agree to the test terms',
          hex: '',
          size: 25,
          sha256: '00',
        },
      ],
      statements: ['This is a test.'],
    };
    component = mount(GenericRequestFlowHost, {
      target: document.body,
      props: { isOpen: true, session: { ...session, details: [packet] } },
    });
    await settle();

    // One detail that signs as you: a single screen with the responder picker.
    expect(document.body.textContent).toContain('I agree to the test terms');
    expect(button('Sign').disabled).toBe(true);
    const boxes = [...document.querySelectorAll('[role="checkbox"]')] as HTMLButtonElement[];
    expect(boxes).toHaveLength(2);
    boxes[0].click();
    await settle();
    expect(button('Sign').disabled).toBe(true);
    boxes[1].click();
    await settle();
    expect(commandsCalled('build_and_sign_generic_response')).toBe(0);

    await press('Sign');
    expect(commandsCalled('build_and_sign_generic_response')).toBe(1);
    expect(document.body.textContent).toContain('You signed 2 items for');
  });

  it('shares credentials only after each has consent and Approval', async () => {
    deliveryFailures = 0;
    const userData: UserDataDetailSession = {
      index: 1,
      type: 'userData',
      keys: ['iProfileKey', 'iMembershipKey'],
      requiredIdentity: null,
    };
    component = mount(GenericRequestFlowHost, {
      target: document.body,
      props: { isOpen: true, session: { ...session, details: [auth, userData] } },
    });
    await settle();
    await press('Continue');

    // Rust reads the credentials of the chosen responder once.
    expect(wallet.invoke).toHaveBeenCalledWith('review_generic_credentials', {
      request_hex: '00',
      identity_id: TARGET,
    });
    expect(commandsCalled('review_generic_credentials')).toBe(1);
    expect(document.body.textContent).toContain('Test membership');
    const boxes = [...document.querySelectorAll('[role="checkbox"]')] as HTMLButtonElement[];
    expect(boxes).toHaveLength(2);
    boxes[0].click();
    await settle();
    expect(button('Continue').disabled).toBe(true);
    boxes[1].click();
    await settle();
    await press('Continue');

    expect(commandsCalled('build_and_sign_generic_response')).toBe(0);
    await press('Sign in and share');
    expect(commandsCalled('build_and_sign_generic_response')).toBe(1);
    expect(commandsCalled('review_generic_credentials')).toBe(1);
    expect(document.body.textContent).toContain('You shared 2 credentials with');
  });

  it('claims a gift card once, after its password, and sends nothing back', async () => {
    const giftCard: GiftCardDetailSession = { index: 0, type: 'giftCard', requiresPassword: true };
    component = mount(GenericRequestFlowHost, {
      target: document.body,
      props: {
        isOpen: true,
        session: {
          ...session,
          signer: { ...session.signer, isSigned: false },
          details: [giftCard],
        },
      },
    });
    await settle();

    // Nothing is read before the password.
    expect(commandsCalled('review_gift_card')).toBe(0);
    const field = document.querySelector('#request-gift-card-password') as HTMLInputElement;
    field.value = 'claim password';
    field.dispatchEvent(new Event('input', { bubbles: true }));
    await settle();
    await press('Open gift card');
    expect(wallet.invoke).toHaveBeenCalledWith('review_gift_card', {
      request_hex: '00',
      password: 'claim password',
    });
    expect(commandsCalled('preflight_gift_card_claim')).toBe(1);
    expect(document.body.textContent).toContain('0.06 VRSCTEST');

    await press('Continue');
    expect(commandsCalled('send_gift_card_claim')).toBe(0);
    await press('Claim');
    expect(commandsCalled('send_gift_card_claim')).toBe(1);
    expect(commandsCalled('build_and_sign_generic_response')).toBe(0);
    expect(commandsCalled('post_generic_response_callback')).toBe(0);
    expect(document.body.textContent).toContain('Gift card claimed');
  });
});
