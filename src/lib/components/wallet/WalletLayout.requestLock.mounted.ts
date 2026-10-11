// @vitest-environment jsdom
import { mount, tick, unmount } from 'svelte';
import { get } from 'svelte/store';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  clearQueuedGenericRequest,
  genericRequestQueueStore,
  queueGenericRequest,
} from '$lib/stores/genericRequest.js';

const { emptyStub, wallet } = vi.hoisted(() => ({
  emptyStub: async () => ({
    default: (await import('./sections/transfer-wizard/test-fixtures/EmptyWalletChild.svelte'))
      .default,
  }),
  wallet: { invoke: vi.fn() },
}));

vi.mock('$lib/services/invokeWalletCommand.js', () => ({
  invokeWalletCommand: wallet.invoke,
  invokeSessionBoundWalletCommand: wallet.invoke,
}));
vi.mock('$lib/components/wallet/AppSidebar.svelte', emptyStub);
vi.mock('$lib/components/wallet/sections/Overview.svelte', emptyStub);
vi.mock('$lib/components/wallet/sections/Watchlist.svelte', emptyStub);
vi.mock('$lib/components/wallet/sections/AddressBook.svelte', emptyStub);
vi.mock('$lib/components/wallet/sections/Identity.svelte', emptyStub);
vi.mock('$lib/components/wallet/sections/TransferWizard.svelte', emptyStub);
vi.mock('$lib/components/wallet/sections/AssetDetails.svelte', emptyStub);
vi.mock('$lib/components/wallet/sections/Receive.svelte', emptyStub);
vi.mock('$lib/components/wallet/sections/Apps.svelte', emptyStub);
vi.mock('$lib/components/wallet/sections/Activity.svelte', emptyStub);
vi.mock('$lib/components/wallet/sections/Settings.svelte', emptyStub);
vi.mock('$lib/components/flows/GenericRequest/GenericRequestImportSheet.svelte', emptyStub);
vi.mock('$lib/components/common/help/HelpCenterDialog.svelte', emptyStub);
// The request library rejects jsdom's Buffer, and session.test.ts covers parsing
// in Node. The layout only needs the request bytes.
vi.mock('$lib/genericRequest/session', () => ({
  parseGenericRequestSession: (input: string) => ({ requestHex: input.trim() }),
}));
vi.mock('$lib/components/flows/GenericRequest/GenericRequestFlowHost.svelte', async () => ({
  default: (await import('../flows/GenericRequest/test-fixtures/FlowHostStub.svelte')).default,
}));

import WalletLayout from './WalletLayout.svelte';

vi.stubGlobal(
  'ResizeObserver',
  class {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
);

const REQUEST = '0197001002050102';
let component: ReturnType<typeof mount> | null = null;
let target: HTMLDivElement;

async function settle() {
  for (let i = 0; i < 6; i += 1) {
    await tick();
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
}

beforeEach(async () => {
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      addEventListener() {},
      removeEventListener() {},
      addListener() {},
      removeListener() {},
    }),
  });
  wallet.invoke.mockReset();
  wallet.invoke.mockResolvedValue(undefined);
  clearQueuedGenericRequest();
  target = document.createElement('div');
  document.body.append(target);
  // A link that arrived while the wallet was open.
  queueGenericRequest({ input: REQUEST, passthroughAutoLinkFqn: null, source: 'deep-link' });
  component = mount(WalletLayout, {
    target,
    props: {
      walletData: { name: 'Wallet A', emoji: 'A', color: '', network: 'testnet', sessionId: 's1' },
    },
  });
  // The layout loads the flow lazily, then opens the waiting link.
  await vi.waitFor(() => expect(target.querySelector('[data-request-flow-stub]')).not.toBeNull(), {
    timeout: 5000,
  });
  expect(get(genericRequestQueueStore)).toBeNull();
});

afterEach(() => {
  if (component) void unmount(component);
  component = null;
  target.remove();
  clearQueuedGenericRequest();
});

/** A lock unmounts the wallet layout. */
async function lock() {
  if (component) await unmount(component);
  component = null;
}

describe('a lock with a request open', () => {
  it('keeps the request in memory before Approval, to open again after unlock', async () => {
    await lock();
    expect(get(genericRequestQueueStore)).toMatchObject({
      input: REQUEST,
      passthroughAutoLinkFqn: null,
      source: 'lock',
    });
  });

  it('drops the request after Approval, as on mobile', async () => {
    (target.querySelector('[data-request-flow-stub] button') as HTMLButtonElement).click();
    await settle();
    await lock();
    expect(get(genericRequestQueueStore)).toBeNull();
  });
});
