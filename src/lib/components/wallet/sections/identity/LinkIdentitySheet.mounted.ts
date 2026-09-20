// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { LinkableIdentity } from '$lib/types/wallet';

class ResizeObserverStub {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}

vi.stubGlobal('ResizeObserver', ResizeObserverStub);

const mocks = vi.hoisted(() => ({
  discoverLinkableIdentities: vi.fn(),
  linkIdentity: vi.fn(),
}));

vi.mock('$lib/services/identityLinkService.js', () => ({
  discoverLinkableIdentities: mocks.discoverLinkableIdentities,
  linkIdentity: mocks.linkIdentity,
}));

vi.mock('$lib/services/walletLockCoordinator.js', () => ({
  isForcedWalletLockError: () => false,
}));

import { localeStore } from '$lib/i18n';
import LinkIdentitySheet from './LinkIdentitySheet.svelte';

const candidate: LinkableIdentity = {
  identityAddress: 'iCandidateIdentity',
  name: 'Alice',
  fullyQualifiedName: 'alice@',
  status: 'active',
  linked: false,
};

function deferred<T>(): {
  promise: Promise<T>;
  resolve: (value: T) => void;
} {
  let resolve: (value: T) => void = () => {};
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

async function settle(): Promise<void> {
  for (let index = 0; index < 4; index += 1) await Promise.resolve();
  await tick();
}

beforeEach(() => {
  document.body.innerHTML = '';
  localeStore.set('en');
  mocks.discoverLinkableIdentities.mockReset();
  mocks.linkIdentity.mockReset();
});

describe('mounted Link VerusID sheet', () => {
  it('uses the shared search treatment and three candidate-shaped loading rows', async () => {
    const discovery = deferred<LinkableIdentity[]>();
    mocks.discoverLinkableIdentities.mockReturnValue(discovery.promise);
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(LinkIdentitySheet, {
      target,
      props: { isOpen: true, allowManualLinkEntry: true },
    });

    try {
      await settle();
      const search = document.querySelector<HTMLInputElement>(
        'input[aria-label="Search by name or i-address"]'
      );
      expect(search?.classList.contains('h-9')).toBe(true);
      expect(search?.classList.contains('focus-visible:ring-[3px]')).toBe(true);
      const body = document.querySelector('[data-link-identity-sheet-body]');
      expect(body?.classList.contains('flex')).toBe(true);
      expect(body?.classList.contains('flex-col')).toBe(true);
      expect(body?.querySelector('[data-link-identity-scroll]')).not.toBeNull();
      expect(document.querySelectorAll('[data-link-identity-skeleton]')).toHaveLength(3);
      expect(document.body.textContent).not.toContain('Manual link');

      discovery.resolve([candidate]);
      await settle();
      expect(document.querySelectorAll('[data-link-identity-skeleton]')).toHaveLength(0);
      expect(document.body.textContent).toContain('alice@');
      expect(document.body.textContent).not.toContain('Manual link');

      const viewport = document.querySelector<HTMLElement>('[data-link-identity-scroll-viewport]');
      if (!viewport) throw new Error('Missing Link VerusID scroll viewport');
      Object.defineProperties(viewport, {
        clientHeight: { configurable: true, value: 200 },
        scrollHeight: { configurable: true, value: 600 },
      });
      viewport.scrollTop = 0;
      viewport.dispatchEvent(new Event('scroll'));
      await settle();
      expect(document.querySelector('[data-link-identity-scroll-fade="top"]')).toBeNull();
      expect(document.querySelector('[data-link-identity-scroll-fade="bottom"]')).not.toBeNull();

      viewport.scrollTop = 200;
      viewport.dispatchEvent(new Event('scroll'));
      await settle();
      expect(document.querySelector('[data-link-identity-scroll-fade="top"]')).not.toBeNull();
      expect(document.querySelector('[data-link-identity-scroll-fade="bottom"]')).not.toBeNull();

      viewport.scrollTop = 400;
      viewport.dispatchEvent(new Event('scroll'));
      await settle();
      expect(document.querySelector('[data-link-identity-scroll-fade="top"]')).not.toBeNull();
      expect(document.querySelector('[data-link-identity-scroll-fade="bottom"]')).toBeNull();

      if (!search) throw new Error('Missing Link VerusID search');
      search.value = 'missing';
      search.dispatchEvent(new Event('input', { bubbles: true }));
      await new Promise((resolve) => setTimeout(resolve, 175));
      await settle();
      expect(document.body.textContent).toContain('No identities match your search.');
      expect(document.body.textContent).not.toContain('Manual link');
      const clear = document.querySelector<HTMLButtonElement>('button[aria-label="Clear search"]');
      expect(clear).not.toBeNull();
      clear?.click();
      await settle();
      expect(search.value).toBe('');
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('reveals manual testnet entry only when automatic discovery finds no candidates', async () => {
    mocks.discoverLinkableIdentities.mockResolvedValue([]);
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(LinkIdentitySheet, {
      target,
      props: { isOpen: true, allowManualLinkEntry: true },
    });

    try {
      await settle();
      expect(document.body.textContent).toContain('Manual link');
      expect(document.body.textContent).toContain(
        'No VerusIDs were discovered automatically. Enter an exact handle or i-address.'
      );
      expect(document.body.textContent).not.toContain(
        'No linkable VerusIDs were found for this wallet address.'
      );
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('does not offer manual entry when that fallback is not allowed', async () => {
    mocks.discoverLinkableIdentities.mockResolvedValue([]);
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(LinkIdentitySheet, {
      target,
      props: { isOpen: true, allowManualLinkEntry: false },
    });

    try {
      await settle();
      expect(document.body.textContent).not.toContain('Manual link');
      expect(document.body.textContent).toContain(
        'No linkable VerusIDs were found for this wallet address.'
      );
    } finally {
      await unmount(component);
      target.remove();
    }
  });
});
