// @vitest-environment jsdom
import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { setLocale } from '$lib/i18n';
import { addressBookStore } from '$lib/stores/addressBook';
import type { AddressBookContact } from '$lib/types/addressBook';
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
async function render(contacts = [structuredClone(contact)]) {
  addressBookStore.set(contacts);
  const target = document.createElement('div');
  document.body.append(target);
  component = mount(AddressBook, { target });
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
});

describe('address book contact workflows', () => {
  it('copies the full address and keeps contact details available during a search with no matches', async () => {
    await render();
    expect(button('Add contact')).not.toBeNull();
    expect(document.body.textContent).toContain('Verus, Bitcoin');
    button('Copy').click();
    await settle();
    expect(copy).toHaveBeenCalledWith(contact.endpoints[0].address);
    await input('input[aria-label="Search contacts"]', 'not a contact');
    expect(document.body.textContent).toContain('No contacts found.');
    expect(document.body.textContent).toContain(contact.endpoints[1].address);
    button('Clear search').click();
    await settle();
    expect(document.body.textContent).toContain('Verus, Bitcoin');
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
      contact.endpoints[1].address
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
    ).toBe('Encrypted storage');
    expect(emptyState.querySelector('h3')?.textContent?.trim()).toBe('Save a trusted contact');
    expect(emptyState.textContent).not.toContain('Save trusted recipients in encrypted storage.');
    expect(document.querySelectorAll('header button')).toHaveLength(0);
    expect([...emptyState.children].some((child) => child.tagName === 'svg')).toBe(false);
    expect(emptyState.querySelector('button svg')).not.toBeNull();

    button('Add contact').click();
    await settle();
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
