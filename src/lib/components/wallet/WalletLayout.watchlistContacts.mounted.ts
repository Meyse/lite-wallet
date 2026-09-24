// @vitest-environment jsdom
import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { navigationStub, emptyStub } = vi.hoisted(() => ({
  navigationStub: async () => ({
    default: (await import('./sections/test-fixtures/WalletLayoutNavigationStub.svelte')).default,
  }),
  emptyStub: async () => ({
    default: (await import('./sections/transfer-wizard/test-fixtures/EmptyWalletChild.svelte'))
      .default,
  }),
}));

vi.mock('$lib/components/wallet/AppSidebar.svelte', navigationStub);
vi.mock('$lib/components/wallet/sections/Overview.svelte', navigationStub);
vi.mock('$lib/components/wallet/sections/Watchlist.svelte', navigationStub);
vi.mock('$lib/components/wallet/sections/AddressBook.svelte', navigationStub);
vi.mock('$lib/components/wallet/sections/Identity.svelte', navigationStub);
vi.mock('$lib/components/wallet/sections/TransferWizard.svelte', navigationStub);
vi.mock('$lib/components/wallet/sections/AssetDetails.svelte', emptyStub);
vi.mock('$lib/components/wallet/sections/Receive.svelte', emptyStub);
vi.mock('$lib/components/wallet/sections/Apps.svelte', emptyStub);
vi.mock('$lib/components/wallet/sections/Activity.svelte', emptyStub);
vi.mock('$lib/components/wallet/sections/Settings.svelte', emptyStub);
vi.mock('$lib/components/flows/GenericRequest/GenericRequestImportSheet.svelte', emptyStub);
vi.mock('$lib/components/common/help/HelpCenterDialog.svelte', emptyStub);

import WalletLayout from './WalletLayout.svelte';

class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
vi.stubGlobal('ResizeObserver', ResizeObserverStub);

let component: ReturnType<typeof mount> | null = null;
let target: HTMLDivElement;

async function settle() {
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 0));
  await tick();
}

function button(label: string): HTMLButtonElement {
  const found = [...target.querySelectorAll<HTMLButtonElement>('button')].find(
    (element) => element.textContent?.trim() === label
  );
  if (!found) throw new Error(`Missing button: ${label}`);
  return found;
}

async function click(label: string) {
  button(label).click();
  await settle();
}

beforeEach(async () => {
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
  target = document.createElement('div');
  document.body.append(target);
  component = mount(WalletLayout, {
    target,
    props: {
      walletData: { name: 'Wallet A', emoji: 'A', color: '', network: 'mainnet', sessionId: 's1' },
    },
  });
  await settle();
});

afterEach(async () => {
  if (component) await unmount(component);
  component = null;
  target.remove();
});

describe('Watchlist shell navigation', () => {
  it('opens the VerusID public profile and returns to the selected Watchlist detail', async () => {
    await click('Watchlist');
    await click('View watched profile');
    expect(target.querySelector('[data-profile-kind]')?.getAttribute('data-profile-address')).toBe(
      `i${'b'.repeat(33)}`
    );
    await click('Back to Watchlist');
    expect(
      target.querySelector('[data-watchlist-detail]')?.getAttribute('data-watchlist-detail')
    ).toBe('watched-identity');
  });

  it('does not retain a Watchlist profile with a stale Back target after sidebar navigation', async () => {
    await click('Watchlist');
    await click('View watched profile');
    await click('VerusID');
    expect(target.querySelector('[data-profile-kind]')).not.toBeNull();
    await click('Apps');
    await click('VerusID');
    expect(target.querySelector('[data-profile-kind]')).toBeNull();
    expect(target.textContent).not.toContain('Back to Watchlist');
  });

  it('lets a new transfer replace the Watchlist profile return', async () => {
    await click('Watchlist');
    await click('View watched profile');
    await click('Send from profile');
    expect(target.querySelector('[data-transfer-stub]')).not.toBeNull();
    await click('View in contacts');
    expect(
      target.querySelector('[data-contact-request]')?.getAttribute('data-contact-request')
    ).toBe('');
    expect(button('Back to send')).toBeDefined();
    await click('Back to send');
    expect(target.querySelector('[data-transfer-stub]')).not.toBeNull();
  });

  it('keeps the direct watched-address cancel and save return', async () => {
    await click('Watchlist');
    await click('Open watched address');
    expect(
      target.querySelector('[data-contact-prefill]')?.getAttribute('data-contact-prefill')
    ).toBe(`R${'a'.repeat(33)}`);
    await click('Cancel watched address');
    expect(
      target.querySelector('[data-watchlist-detail]')?.getAttribute('data-watchlist-detail')
    ).toBe('watched-raw');
    await click('Open watched address');
    await click('Save watched address');
    expect(
      target.querySelector('[data-watchlist-detail]')?.getAttribute('data-watchlist-detail')
    ).toBe('watched-raw');
  });
});
