// @vitest-environment jsdom
import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { setLocale } from '$lib/i18n';
import { setContactSession } from '$lib/contacts/session';
import { addressBookStore } from '$lib/stores/addressBook';
import type { AddressBookContact, ContactIdentity } from '$lib/types/addressBook';
import PreviewHarness from './test-fixtures/PreviewHarness.svelte';
import IdentityLookup from './IdentityLookup.svelte';
import RecipientHarness from './test-fixtures/RecipientHarness.svelte';
const invoke = vi.hoisted(() => vi.fn());
vi.mock('$lib/services/invokeWalletCommand', () => ({ invokeSessionBoundWalletCommand: invoke }));
vi.stubGlobal(
  'ResizeObserver',
  class {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
);
const identity: ContactIdentity = {
  identityAddress: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
  chainId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
  network: 'mainnet',
  fullyQualifiedName: 'alex.example@',
};
const contact: AddressBookContact = {
  id: 'saved',
  displayName: identity.fullyQualifiedName,
  note: 'Only in contact detail',
  identities: [identity],
  createdAt: 1,
  updatedAt: 1,
  endpoints: [
    {
      id: 'destination',
      kind: 'vrpc',
      address: identity.identityAddress,
      normalizedAddress: identity.identityAddress,
      label: 'VerusID',
      lastUsedAt: null,
      createdAt: 1,
      updatedAt: 1,
    },
  ],
};
const profile = {
  state: 'ready',
  description: { value: 'A published description.', source: {} },
  avatar: null,
  issues: [],
  revisionTxid: 'confirmed',
};
let components: ReturnType<typeof mount>[] = [];
const onView = vi.fn();
async function settle() {
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 10));
  await tick();
}
function button(text: string): HTMLButtonElement {
  const el = [...document.querySelectorAll<HTMLButtonElement>('button')].find(
    (el) => el.textContent?.trim() === text || el.getAttribute('aria-label') === text
  );
  if (!el) throw new Error(`Missing ${text}: ${document.body.textContent}`);
  return el;
}
async function render() {
  const target = document.createElement('div');
  document.body.append(target);
  components.push(mount(PreviewHarness, { target, props: { identity, onView } }));
  await settle();
}
beforeEach(() => {
  vi.clearAllMocks();
  setLocale('en');
  setContactSession(null);
  setContactSession({ sessionId: 'preview-session', network: 'mainnet' });
  invoke.mockImplementation((command: string) =>
    Promise.resolve(
      command === 'get_identity_profile'
        ? profile
        : command === 'list_address_book_contacts'
          ? []
          : contact
    )
  );
});
afterEach(async () => {
  for (const component of components) await unmount(component);
  components = [];
  setContactSession(null);
  await settle();
  document.body.replaceChildren();
});

describe('interactive identity preview', () => {
  it.each([0, 100, 220])('accepts repeated hover entries after %i ms away', async (timeAway) => {
    await render();
    const trigger = document.querySelector('[data-popover-trigger]') as HTMLButtonElement;
    vi.useFakeTimers();
    try {
      trigger.dispatchEvent(new MouseEvent('pointerenter'));
      await tick();
      for (let cycle = 0; cycle < 3; cycle++) {
        expect(trigger.getAttribute('aria-expanded')).toBe('true');
        trigger.dispatchEvent(new MouseEvent('pointerleave', { relatedTarget: document.body }));
        // Move beyond both the trigger and the card to exercise the primitive's
        // real safe-polygon exit and delayed close, rather than closing by click.
        document.dispatchEvent(new MouseEvent('pointermove', { clientX: 800, clientY: 600 }));
        await vi.advanceTimersByTimeAsync(timeAway);
        expect(trigger.getAttribute('aria-expanded')).toBe(timeAway < 200 ? 'true' : 'false');
        trigger.dispatchEvent(new MouseEvent('pointerenter'));
        await tick();
        expect(trigger.getAttribute('aria-expanded')).toBe('true');
        // A cancelled close must not fire underneath the returned pointer.
        await vi.advanceTimersByTimeAsync(250);
        expect(trigger.getAttribute('aria-expanded')).toBe('true');
      }
    } finally {
      vi.useRealTimers();
    }
  });
  it.each(['ready', 'empty', 'unavailable'] as const)(
    'replaces the profile skeleton when loading completes as %s',
    async (state) => {
      let complete!: (value: unknown) => void;
      invoke.mockImplementation((command: string) =>
        command === 'get_identity_profile'
          ? new Promise((resolve) => {
              complete = resolve;
            })
          : Promise.resolve([])
      );
      await render();
      const trigger = document.querySelector('[data-popover-trigger]') as HTMLButtonElement;
      trigger.click();
      await settle();
      const preview = document.querySelector('[data-popover-content]');
      if (!preview) throw new Error('Missing profile preview');
      expect(preview.querySelector('[aria-busy="true"]')).not.toBeNull();
      expect(preview.querySelectorAll('[data-slot="skeleton"]')).toHaveLength(3);
      expect(preview.textContent).toContain(identity.fullyQualifiedName);
      expect(button('Add to contacts').getAttribute('aria-disabled')).toBe('false');
      complete(state === 'ready' ? profile : { state, issues: [], revisionTxid: null });
      await settle();
      expect(preview.querySelector('[aria-busy="true"]')).toBeNull();
      expect(preview.querySelector('[data-slot="skeleton"]')).toBeNull();
      expect(preview.textContent?.includes('A published description.')).toBe(state === 'ready');
      expect(preview.textContent?.includes('Profile unavailable')).toBe(state === 'unavailable');
    }
  );
  it('opens on hover without a delay and consumes Escape while the recipient input keeps focus', async () => {
    const input = document.createElement('input');
    document.body.append(input);
    input.focus();
    const closeSend = vi.fn();
    window.addEventListener('keydown', closeSend);
    try {
      await render();
      const trigger = document.querySelector('[data-popover-trigger]') as HTMLButtonElement;
      trigger.dispatchEvent(new MouseEvent('pointerenter'));
      await tick();
      expect(trigger.getAttribute('aria-expanded')).toBe('true');
      expect(document.activeElement).toBe(input);
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
      await settle();
      expect(closeSend).not.toHaveBeenCalled();
      expect(trigger.getAttribute('aria-expanded')).toBe('false');
      expect(document.activeElement).toBe(trigger);
      trigger.dispatchEvent(new MouseEvent('pointerleave', { relatedTarget: document.body }));
      trigger.dispatchEvent(new MouseEvent('pointerenter'));
      await tick();
      expect(trigger.getAttribute('aria-expanded')).toBe('true');
    } finally {
      window.removeEventListener('keydown', closeSend);
    }
  });
  it('saves once, waits for persistence, then navigates to Contacts without another detail dialog', async () => {
    let complete!: (contact: AddressBookContact) => void;
    invoke.mockImplementation((command: string) =>
      command === 'save_address_book_contact'
        ? new Promise((resolve) => {
            complete = resolve;
          })
        : Promise.resolve(command === 'get_identity_profile' ? profile : [])
    );
    await render();
    const trigger = document.querySelector<HTMLButtonElement>(
      '[data-popover-trigger]'
    ) as HTMLButtonElement;
    trigger.click();
    await settle();
    expect(document.body.textContent).toContain('A published description.');
    expect(document.body.textContent).not.toContain('Only in contact detail');
    const action = button('Add to contacts');
    action.focus();
    action.click();
    action.click();
    await settle();
    expect(button('Saving')).toBe(action);
    expect(action.getAttribute('aria-busy')).toBe('true');
    expect(trigger.getAttribute('aria-label')).toContain('not in Contacts');
    expect(get(addressBookStore)).toEqual([]);
    complete(contact);
    await settle();
    expect(button('Saved')).toBe(action);
    expect(document.activeElement).toBe(action);
    expect(trigger.getAttribute('aria-label')).not.toContain('not in Contacts');
    action.click();
    await settle();
    expect(document.querySelector('[data-contact-detail]')).toBeNull();
    await new Promise((resolve) => setTimeout(resolve, 1220));
    await settle();
    expect(button('View in contacts')).toBe(action);
    action.click();
    await settle();
    expect(onView).toHaveBeenCalledWith(identity, trigger);
    expect(document.querySelector('[data-contact-detail]')).toBeNull();
    expect(trigger.getAttribute('aria-expanded')).toBe('false');
    expect(
      invoke.mock.calls.filter(([command]) => command === 'save_address_book_contact')
    ).toHaveLength(1);
  });
  it('keeps failure unsaved, allows retry, and Escape restores focus without reopening', async () => {
    invoke.mockImplementation((command: string) =>
      command === 'save_address_book_contact'
        ? Promise.reject(new Error('Disk unavailable'))
        : Promise.resolve(command === 'get_identity_profile' ? profile : [])
    );
    await render();
    const trigger = document.querySelector<HTMLButtonElement>(
      '[data-popover-trigger]'
    ) as HTMLButtonElement;
    trigger.click();
    await settle();
    button('Add to contacts').click();
    await settle();
    expect(button('Try again')).not.toBeNull();
    expect(get(addressBookStore)).toEqual([]);
    const retry = button('Try again');
    retry.focus();
    const escapeOutside = vi.fn();
    window.addEventListener('keydown', escapeOutside);
    retry.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    window.removeEventListener('keydown', escapeOutside);
    expect(escapeOutside).not.toHaveBeenCalled();
    await settle();
    expect(trigger.getAttribute('aria-expanded')).toBe('false');
    expect(document.activeElement).toBe(trigger);
    trigger.click();
    await settle();
    expect(trigger.getAttribute('aria-expanded')).toBe('true');
  });
  it('dismisses the previous preview and clears presentation on lock', async () => {
    await render();
    await render();
    const triggers = document.querySelectorAll<HTMLButtonElement>('[data-popover-trigger]');
    triggers[0].click();
    await settle();
    triggers[1].click();
    await settle();
    expect(triggers[0].getAttribute('aria-expanded')).toBe('false');
    expect(triggers[1].getAttribute('aria-expanded')).toBe('true');
    setContactSession(null);
    await settle();
    expect(document.querySelector('[data-popover-content]')).toBeNull();
  });
});

it('withholds Add until local matches are known and routes duplicates to Contacts without choosing one', async () => {
  let finish!: (contacts: AddressBookContact[]) => void;
  invoke.mockImplementation((command: string) =>
    command === 'list_address_book_contacts'
      ? new Promise((resolve) => {
          finish = resolve;
        })
      : Promise.resolve(profile)
  );
  await render();
  const trigger = document.querySelector<HTMLButtonElement>(
    '[data-popover-trigger]'
  ) as HTMLButtonElement;
  trigger.click();
  await settle();
  expect(document.body.textContent).not.toContain('Add to contacts');
  finish([
    contact,
    {
      ...contact,
      id: 'second',
      endpoints: [
        { ...contact.endpoints[0], id: 'second-endpoint', address: 'Different saved destination' },
      ],
    },
  ]);
  await settle();
  button('View in contacts').click();
  await settle();
  expect(document.querySelector('[data-contact-detail]')).toBeNull();
  expect(document.body.textContent).not.toContain('Only in contact detail');
  expect(onView).toHaveBeenCalledWith(identity, trigger);
  expect(invoke.mock.calls.some(([command]) => command === 'save_address_book_contact')).toBe(
    false
  );
});

it('discards old recipient resolution immediately and never retargets a late result', async () => {
  const requests = new Map<string, (value: typeof identity) => void>();
  invoke.mockImplementation((command: string, args: { identity?: string }) =>
    command === 'resolve_contact_identity'
      ? new Promise((resolve) => {
          requests.set(args.identity ?? '', resolve);
        })
      : Promise.resolve(command === 'get_identity_profile' ? profile : [])
  );
  const target = document.createElement('div');
  document.body.append(target);
  const recipient = mount(RecipientHarness, { target });
  components.push(recipient);
  await settle();
  recipient.changeRecipient('bob@');
  await settle();
  requests.get('alex@')?.(identity);
  await settle();
  expect(document.querySelector('[data-popover-trigger]')).toBeNull();
  requests.get('bob@')?.({
    ...identity,
    identityAddress: 'iOtherCanonicalIdentity',
    fullyQualifiedName: 'bob@',
  });
  await settle();
  expect(document.querySelector('[data-popover-trigger]')?.textContent).toContain('bob@');
  recipient.changeRecipient('RordinaryAddress');
  await settle();
  expect(document.querySelector('[data-popover-trigger]')).toBeNull();
});

it('saves directly after a Contacts lookup and discards feedback when the query changes', async () => {
  let finish!: (value: AddressBookContact) => void;
  invoke.mockImplementation((command: string) =>
    command === 'resolve_contact_identity'
      ? Promise.resolve(identity)
      : command === 'get_identity_profile'
        ? Promise.resolve(profile)
        : command === 'save_address_book_contact'
          ? new Promise((resolve) => {
              finish = resolve;
            })
          : Promise.resolve([])
  );
  const target = document.createElement('div');
  document.body.append(target);
  const onSaved = vi.fn();
  components.push(mount(IdentityLookup, { target, props: { onSaved } }));
  await settle();
  const input = target.querySelector('input') as HTMLInputElement;
  input.value = 'alex@';
  input.dispatchEvent(new Event('input', { bubbles: true }));
  await settle();
  button('Find VerusID').click();
  await settle();
  const action = button('Add to contacts');
  action.focus();
  action.click();
  await settle();
  expect(button('Saving')).toBe(action);
  input.value = 'bob@';
  input.dispatchEvent(new Event('input', { bubbles: true }));
  await settle();
  finish(contact);
  await settle();
  expect(target.textContent).not.toContain('Saved');
  expect(onSaved).not.toHaveBeenCalled();
  expect(get(addressBookStore)[0].id).toBe(contact.id);
});

it('shows testnet lookup failures, retries, and clears the mention when the session changes', async () => {
  const chainId = 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq';
  const testIdentity = { ...identity, chainId, network: 'testnet' as const };
  setContactSession({ sessionId: 'testnet-session', network: 'testnet' });
  let fail = true;
  invoke.mockImplementation((command: string) =>
    command === 'resolve_contact_identity'
      ? fail
        ? Promise.reject(new Error('Provider unavailable'))
        : Promise.resolve(testIdentity)
      : Promise.resolve(command === 'get_identity_profile' ? profile : [])
  );
  const target = document.createElement('div');
  document.body.append(target);
  components.push(mount(RecipientHarness, { target, props: { chainId } }));
  await settle();
  expect(target.textContent).toContain('Couldn’t resolve this VerusID');
  expect(document.querySelector('[data-popover-trigger]')).toBeNull();
  fail = false;
  button('Try again').click();
  await settle();
  expect(document.querySelector('[data-popover-trigger]')?.getAttribute('aria-label')).toContain(
    'not in Contacts'
  );
  expect(invoke).toHaveBeenCalledWith('resolve_contact_identity', {
    identity: 'alex@',
    expected_session_id: 'testnet-session',
  });
  setContactSession({ sessionId: 'mainnet-session', network: 'mainnet' });
  await settle();
  expect(document.querySelector('[data-popover-trigger]')).toBeNull();
  expect(target.textContent).not.toContain('Couldn’t resolve');
});
