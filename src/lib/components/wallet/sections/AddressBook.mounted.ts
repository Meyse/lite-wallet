// @vitest-environment jsdom
import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { setLocale } from '$lib/i18n';
import { identityKey } from '$lib/contacts/identity';
import { identityProfiles } from '$lib/contacts/profiles';
import { addressBookStore } from '$lib/stores/addressBook';
import type { AddressBookContact, ContactIdentity } from '$lib/types/addressBook';
import AddressBook from './AddressBook.svelte';

const service = vi.hoisted(() => ({
  saveAddressBookContact: vi.fn(),
  deleteAddressBookContact: vi.fn(),
  validateDestinationAddress: vi.fn(),
}));
vi.mock('$lib/services/addressBookService', () => service);
vi.mock('$lib/services/walletLockCoordinator.js', () => ({
  isForcedWalletLockError: () => false,
}));

class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
vi.stubGlobal('ResizeObserver', ResizeObserverStub);

const contact: AddressBookContact = {
  id: 'mum',
  displayName: 'Mum',
  note: null,
  createdAt: 1,
  updatedAt: 1,
  endpoints: [
    {
      id: 'r',
      kind: 'vrpc',
      address: 'R' + 'a'.repeat(33),
      normalizedAddress: 'R' + 'a'.repeat(33),
      label: 'Default',
      lastUsedAt: null,
      createdAt: 1,
      updatedAt: 1,
    },
    {
      id: 'btc',
      kind: 'btc',
      address: 'bc1' + 'q'.repeat(59),
      normalizedAddress: 'bc1' + 'q'.repeat(59),
      label: 'Default 2',
      lastUsedAt: null,
      createdAt: 1,
      updatedAt: 1,
    },
  ],
};
let component: ReturnType<typeof mount>;
const copy = vi.fn().mockResolvedValue(undefined);
async function settle() {
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 0));
}
function button(label: string, root: Document | Element | null = document.body): HTMLButtonElement {
  if (!root) throw new Error('Missing button container');
  const match = [...root.querySelectorAll<HTMLButtonElement>('button')].find(
    (el) => el.textContent?.trim() === label || el.getAttribute('aria-label') === label
  );
  if (!match) throw new Error(`Missing button: ${label}`);
  return match;
}
async function input(selector: string, value: string) {
  const el = document.querySelector<HTMLInputElement>(selector);
  if (!el) throw new Error(`Missing input: ${selector}`);
  el.value = value;
  el.dispatchEvent(new Event('input', { bubbles: true }));
  await settle();
}
async function render(contacts = [structuredClone(contact)], requestedIdentity?: ContactIdentity) {
  addressBookStore.set(contacts);
  const target = document.createElement('div');
  document.body.append(target);
  component = mount(AddressBook, { target, props: { requestedIdentity } });
  await settle();
}

beforeEach(() => {
  vi.clearAllMocks();
  setLocale('en');
  service.validateDestinationAddress.mockResolvedValue({ valid: true });
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: copy } });
});
afterEach(async () => {
  if (component) await unmount(component);
  await settle();
  document.body.replaceChildren();
  addressBookStore.set([]);
  identityProfiles.set({});
});

describe('address book contact workflows', () => {
  const identity: ContactIdentity = {
    fullyQualifiedName: 'alice.example@',
    identityAddress: 'iQjVunnXvHswZhmNnqT4ucbRmvDkr5hBAg',
    chainId: 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq',
    network: 'testnet',
  };
  const identityContact = {
    ...contact,
    id: 'alice',
    displayName: identity.fullyQualifiedName,
    identities: [identity],
    endpoints: [
      {
        ...contact.endpoints[0],
        address: identity.identityAddress,
        normalizedAddress: identity.identityAddress,
      },
    ],
  };

  it('shows the VerusID name and identifier without its description and preserves it when editing', async () => {
    const description = 'A published profile description.';
    identityProfiles.set({
      [identityKey(identity)]: {
        profile: {
          state: 'ready',
          description: {
            value: description,
            source: {
              systemId: identity.chainId,
              txid: 'profile-transaction',
              vout: 0,
              height: 1,
              blockhash: 'profile-block',
              digest: 'profile-digest',
            },
          },
          avatar: null,
          issues: [],
          revisionTxid: 'confirmed',
        },
        loading: false,
        unavailable: false,
        checkedAt: Date.now(),
      },
    });
    await render([identityContact]);
    const detail = document.querySelector('[data-contact-detail]');
    expect(detail?.textContent).toContain(identity.fullyQualifiedName);
    expect(detail?.textContent).toContain('VerusID identifier');
    expect(detail?.textContent).toContain(identity.identityAddress);
    expect(detail?.textContent).not.toContain(description);
    button('Copy VerusID').click();
    await settle();
    expect(copy).toHaveBeenCalledWith(identity.fullyQualifiedName);
    button('Copy VerusID identifier').click();
    await settle();
    expect(copy).toHaveBeenCalledWith(identity.identityAddress);
    button('Edit').click();
    await settle();
    expect(document.querySelector('#endpoint-address-0')?.closest('[hidden]')).not.toBeNull();
    service.saveAddressBookContact.mockResolvedValue(identityContact);
    button('Save').click();
    await settle();
    expect(service.saveAddressBookContact).toHaveBeenCalledWith(
      expect.objectContaining({
        endpoints: [expect.objectContaining({ address: identity.identityAddress })],
        identities: [identity],
      })
    );
    expect(service.validateDestinationAddress).not.toHaveBeenCalled();
  });

  it('selects the requested contact and leaves existing duplicates for the user to choose', async () => {
    await render([contact, identityContact], identity);
    expect(document.querySelector('[data-contact-detail]')?.textContent).toContain(
      identity.fullyQualifiedName
    );
    await unmount(component);
    document.body.replaceChildren();
    await render(
      [contact, identityContact, { ...identityContact, id: 'alice-second', note: 'Private note' }],
      identity
    );
    expect(document.querySelector('[data-contact-detail]')).toBeNull();
    expect(document.body.textContent).not.toContain('Private note');
    const row = document.querySelector('aside li button') as HTMLButtonElement;
    row.click();
    await settle();
    expect(document.querySelector('[data-contact-detail]')).not.toBeNull();
  });

  it('selects the first contact for browsing and clears visible selection while adding', async () => {
    await render([
      structuredClone(contact),
      { ...structuredClone(contact), id: 'other', displayName: 'Other' },
    ]);
    const rows = [...document.querySelectorAll<HTMLButtonElement>('aside li button')];
    expect(rows[0].getAttribute('aria-current')).toBe('true');
    rows[1].click();
    await settle();
    expect(rows[1].getAttribute('aria-current')).toBe('true');

    button('Add contact').click();
    await settle();
    expect(rows.every((row) => row.getAttribute('aria-current') === null)).toBe(true);
    expect(rows.every((row) => !row.classList.contains('bg-settings-selection-surface'))).toBe(
      true
    );
    expect(rows.every((row) => !row.disabled)).toBe(true);

    rows[0].click();
    await settle();
    expect(document.querySelector('form')).toBeNull();
    expect(rows[0].getAttribute('aria-current')).toBe('true');
    expect(document.querySelector('[data-contact-detail]')?.textContent).toContain('Mum');

    button('Edit').click();
    await settle();
    expect(rows.every((row) => row.disabled)).toBe(true);
  });

  it('copies the full address and keeps contact details available during a search with no matches', async () => {
    await render();
    expect(document.querySelector('h2')).toBeNull();
    expect(button('Add contact').parentElement?.classList.contains('absolute')).toBe(true);
    expect(button('Add contact').parentElement?.classList.contains('top-5')).toBe(true);
    expect(
      document.querySelector('[data-address-book-divider] + section')?.classList.contains('pt-16')
    ).toBe(true);
    const contactRow = document.querySelector('aside li button');
    expect(contactRow?.textContent).toContain('Mum');
    expect(contactRow?.textContent).not.toContain('Verus');
    expect(contactRow?.textContent).not.toContain('Bitcoin');
    button('Copy').click();
    await settle();
    expect(copy).toHaveBeenCalledWith(contact.endpoints[0].address);
    await input('input[aria-label="Search contacts"]', 'not a contact');
    expect(document.body.textContent).toContain('No contacts found.');
    expect(document.body.textContent).toContain(contact.endpoints[1].address);
    button('Clear search').click();
    await settle();
    expect(document.querySelector('aside li button')?.textContent).toContain('Mum');
  });

  it('reveals notes on demand, cancels drafts, and reopens an existing note', async () => {
    await render();
    button('Edit').click();
    await settle();
    expect(document.querySelector('#address-book-note')).toBeNull();
    button('Add note').click();
    await settle();
    expect(document.activeElement?.id).toBe('address-book-note');
    await input('#address-book-note', 'Unsaved note');
    button('Cancel').click();
    await settle();
    expect(get(addressBookStore)[0].note).toBeNull();
    addressBookStore.set([{ ...contact, note: 'Saved note' }]);
    await settle();
    button('Edit').click();
    await settle();
    expect(document.querySelector<HTMLInputElement>('#address-book-note')?.value).toBe(
      'Saved note'
    );
  });

  it('keeps the edited contact and controls stable until saving completes', async () => {
    await render([
      structuredClone(contact),
      { ...structuredClone(contact), id: 'other', displayName: 'Other' },
    ]);
    button('Edit').click();
    await settle();
    await input('#address-book-name', 'Mother');
    let finish!: (value: AddressBookContact) => void;
    service.saveAddressBookContact.mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        })
    );
    button('Save').click();
    await settle();
    expect(service.saveAddressBookContact).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 'mum',
        displayName: 'Mother',
        endpoints: expect.arrayContaining([
          expect.objectContaining({ id: 'btc', address: contact.endpoints[1].address }),
        ]),
      })
    );
    expect(service.validateDestinationAddress).not.toHaveBeenCalled();
    expect(button('Cancel').disabled).toBe(true);
    expect(button('Saving').disabled).toBe(true);
    expect(button('Saving').getAttribute('aria-busy')).toBe('true');
    expect(button('Saving').querySelector('svg.animate-spin')).not.toBeNull();
    expect(
      document.querySelector<HTMLInputElement>('#address-book-name')?.matches(':disabled')
    ).toBe(true);
    expect(get(addressBookStore)[0].displayName).toBe('Mum');
    finish({ ...contact, displayName: 'Mother' });
    await settle();
    expect(get(addressBookStore)[0].displayName).toBe('Mother');
    expect(document.querySelector('form')).toBeNull();
  });

  it('preserves drafts after validation or save errors and never removes the final address', async () => {
    await render();
    button('Edit').click();
    await settle();
    button('Remove address').click();
    await settle();
    expect(button('Remove address').disabled).toBe(true);
    await input('#endpoint-address-0', contact.endpoints[1].address + 'q');
    service.validateDestinationAddress.mockResolvedValue({ valid: false });
    button('Save').click();
    await settle();
    expect(document.body.textContent).toContain('One or more addresses are invalid.');
    expect(service.saveAddressBookContact).not.toHaveBeenCalled();
    service.validateDestinationAddress.mockResolvedValue({ valid: true });
    service.saveAddressBookContact.mockRejectedValue(new Error('Synthetic save failure'));
    button('Save').click();
    await settle();
    expect(document.body.textContent).toContain('Synthetic save failure');
    expect(document.querySelector<HTMLInputElement>('#endpoint-address-0')?.value).toBe(
      contact.endpoints[1].address + 'q'
    );
    expect(get(addressBookStore)[0].endpoints).toHaveLength(2);
  });

  it('keeps failed deletion visible and only leaves editing after confirmed deletion', async () => {
    await render();
    button('Edit').click();
    await settle();
    button('Delete contact').click();
    await settle();
    service.deleteAddressBookContact.mockResolvedValue(false);
    button('Delete contact', document.querySelector('[role="dialog"]')).click();
    await settle();
    expect(document.querySelector('[role="dialog"]')?.textContent).toContain(
      'Could not delete this contact. Try again.'
    );
    expect(get(addressBookStore)).toHaveLength(1);
    service.deleteAddressBookContact.mockResolvedValue(true);
    button('Delete contact', document.querySelector('[role="dialog"]')).click();
    await settle();
    expect(service.deleteAddressBookContact).toHaveBeenLastCalledWith('mum');
    expect(get(addressBookStore)).toHaveLength(0);
    expect(document.querySelector('form')).toBeNull();
    expect(document.body.textContent).toContain('Save a trusted contact');
  });

  it('creates a contact from the empty state with an optional note', async () => {
    await render([]);

    const emptyState = document.querySelector('[data-testid="address-book-empty"]');
    expect(emptyState).not.toBeNull();
    if (!emptyState) throw new Error('Missing address book empty state');
    expect(
      emptyState.querySelector('[data-testid="wallet-empty-eyebrow"]')?.textContent?.trim()
    ).toBe('Encrypted locally');
    expect(emptyState.querySelector('[data-testid="wallet-empty-eyebrow"] svg')).not.toBeNull();
    expect(emptyState.querySelector('h3')?.textContent?.trim()).toBe('Save a trusted contact');
    expect(emptyState.textContent).not.toContain('Save trusted recipients in encrypted storage.');
    expect(document.querySelector('header')).toBeNull();
    expect([...emptyState.children].some((child) => child.tagName === 'svg')).toBe(false);
    expect(emptyState.querySelector('button svg')).not.toBeNull();

    button('Add contact').click();
    await settle();
    expect(document.querySelector('header h2')).toBeNull();
    const layout = document.querySelector('[data-address-book-layout]');
    expect(layout?.classList.contains('max-w-6xl')).toBe(true);
    expect(layout?.classList.contains('px-5')).toBe(true);
    expect(layout?.classList.contains('pt-5')).toBe(true);
    const divider = document.querySelector('[data-address-book-divider]');
    expect(divider).not.toBeNull();
    expect(divider?.classList.contains('mx-5')).toBe(true);
    const dividerLine = document.querySelector('[data-address-book-divider-line]');
    expect(dividerLine?.classList.contains('-top-5')).toBe(true);
    expect(dividerLine?.classList.contains('-bottom-6')).toBe(true);
    expect(
      document.querySelector('[data-address-book-divider] + section')?.classList.contains('pt-2')
    ).toBe(false);
    expect(document.querySelector('form h3')).toBeNull();
    expect(
      document.querySelector('[data-address-book-form-content]')?.classList.contains('px-1')
    ).toBe(true);
    const verusIdTab = button('VerusID');
    const addressTab = button('Receiving address');
    const findIdentity = button('Find VerusID');
    expect(verusIdTab.getAttribute('role')).toBe('tab');
    expect(verusIdTab.getAttribute('data-state')).toBe('active');
    expect(addressTab.getAttribute('data-state')).toBe('inactive');
    expect(findIdentity.classList.contains('bg-primary')).toBe(true);
    button('Receiving address').click();
    await settle();
    expect(verusIdTab.getAttribute('data-state')).toBe('inactive');
    expect(addressTab.getAttribute('data-state')).toBe('active');
    button('Save').click();
    await settle();
    expect(document.activeElement?.id).toBe('address-book-name');
    await input('#address-book-name', 'New contact');
    await input('#endpoint-address-0', contact.endpoints[0].address);
    button('Add note').click();
    await settle();
    await input('#address-book-note', 'A note');
    service.saveAddressBookContact.mockResolvedValue({
      ...contact,
      displayName: 'New contact',
      note: 'A note',
    });
    button('Save').click();
    await settle();
    expect(service.saveAddressBookContact).toHaveBeenCalledWith(
      expect.objectContaining({ id: undefined, displayName: 'New contact', note: 'A note' })
    );
    expect(document.body.textContent).toContain('A note');
  });
});

it('requires an explicit remaining profile, hides aliases, and requests a local name after removing all IDs', async () => {
  const identity = {
    identityAddress: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
    chainId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
    network: 'mainnet' as const,
    fullyQualifiedName: 'alex@',
  };
  const second = { ...identity, identityAddress: 'iSecondIdentity', fullyQualifiedName: 'bob@' };
  const third = { ...identity, identityAddress: 'iThirdIdentity', fullyQualifiedName: 'carol@' };
  await render([{ ...contact, identities: [identity, second, third], profileIdentity: identity }]);
  button('Edit').click();
  await settle();
  expect(document.querySelector('#address-book-name')).toBeNull();
  button('Remove alex@ from contact').click();
  await settle();
  button('Save').click();
  await settle();
  expect(service.saveAddressBookContact).not.toHaveBeenCalled();
  expect(document.querySelector('#address-book-name')).toBeNull();
  button('Remove bob@ from contact').click();
  await settle();
  button('Remove carol@ from contact').click();
  await settle();
  expect(document.querySelector<HTMLInputElement>('#address-book-name')?.value).toBe('');
  button('Save').click();
  await settle();
  expect(service.saveAddressBookContact).not.toHaveBeenCalled();
  expect(document.activeElement?.id).toBe('address-book-name');
});
