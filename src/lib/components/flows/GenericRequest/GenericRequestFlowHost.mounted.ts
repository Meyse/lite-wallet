// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '$lib/i18n';
import { coinsStore } from '$lib/stores/coins';
import type {
  AuthenticationDetailSession,
  GenericRequestFlowSession,
  IdentityUpdateDetailSession,
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
    const signCall = wallet.invoke.mock.calls.find(
      ([command]) => command === 'build_and_sign_generic_response'
    );
    // Rust reads the request IDs and the txid itself; the UI names the responder.
    expect(signCall?.[1]).toEqual({
      request_hex: '00',
      signer: { systemId: TESTNET, identityId: TARGET },
    });
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
});
