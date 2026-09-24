import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { contactName, contactProfile, identityKey, matchingContacts } from './identity';
import { contactSession, contactsLoadState, setContactSession } from './session';
import { addIdentityContact, loadContacts, resolveContactIdentity } from './service';
import { identityProfiles, loadIdentityProfile } from './profiles';
import { addressBookStore } from '$lib/stores/addressBook';
import {
  type ContactDeleteRetry,
  type ContactSaveRetry,
  deleteAddressBookContact,
  markAddressBookEndpointUsed,
  saveAddressBookContact,
} from '$lib/services/addressBookService';
import type { AddressBookContact, ContactIdentity } from '$lib/types/addressBook';
const invoke = vi.hoisted(() => vi.fn());
vi.mock('$lib/services/invokeWalletCommand', () => ({
  invokeSessionBoundWalletCommand: invoke,
  invokeWalletCommand: invoke,
}));
const id: ContactIdentity = {
  identityAddress: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
  fullyQualifiedName: 'alex.example@',
  network: 'mainnet',
  chainId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
};
const contact: AddressBookContact = {
  id: 'one',
  displayName: 'Legacy Alex',
  legacyDisplayName: 'Legacy Alex',
  note: 'private note',
  createdAt: 1,
  updatedAt: 1,
  endpoints: [],
  identities: [id],
};
const addressContact: AddressBookContact = {
  ...contact,
  displayName: 'Mum',
  note: 'Private note',
  identities: [],
  profileIdentity: null,
  endpoints: [
    {
      id: 'endpoint',
      kind: 'vrpc',
      address: 'R' + 'a'.repeat(33),
      normalizedAddress: 'R' + 'a'.repeat(33),
      label: 'Main',
      lastUsedAt: null,
      createdAt: 1,
      updatedAt: 1,
    },
  ],
};
const addressRequest = {
  displayName: 'Mum',
  note: 'Private note',
  endpoints: [
    { kind: 'vrpc' as const, address: addressContact.endpoints[0].address, label: 'Main' },
  ],
  identities: [],
  profileIdentity: null,
};
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((a, b) => {
    resolve = a;
    reject = b;
  });
  return { promise, resolve, reject };
}
beforeEach(() => {
  vi.clearAllMocks();
  setContactSession(null);
  setContactSession({ sessionId: 'session-a', network: 'mainnet' });
});

describe('canonical contact associations', () => {
  it('preserves address-only labels, uses ID names, and never picks an ambiguous profile', () => {
    expect(contactName({ ...contact, identities: [] })).toBe('Legacy Alex');
    expect(contactName(contact)).toBe('alex.example@');
    const second = { ...id, identityAddress: 'second', fullyQualifiedName: 'bob@' };
    expect(contactProfile({ ...contact, identities: [id, second] })).toBeNull();
    expect(contactName({ ...contact, identities: [id, second], profileIdentity: second })).toBe(
      'bob@'
    );
  });
  it('matches exact canonical network and chain keys and retains duplicates', () => {
    expect(identityKey(id)).not.toBe(
      identityKey({ ...id, identityAddress: id.identityAddress.toLowerCase() })
    );
    expect(matchingContacts([contact, { ...contact, id: 'duplicate' }], id)).toHaveLength(2);
    expect(matchingContacts([contact], { ...id, network: 'testnet' })).toEqual([]);
    expect(matchingContacts([contact], { ...id, chainId: 'other-chain' })).toEqual([]);
  });
});

describe('durable contact saves', () => {
  it('coalesces concurrent activation and publishes membership only after durable success', async () => {
    const stored = deferred<AddressBookContact>();
    invoke.mockImplementation((command: string) =>
      command === 'list_address_book_contacts' ? Promise.resolve([]) : stored.promise
    );
    const first = addIdentityContact(id);
    const second = addIdentityContact(id);
    expect(first).toBe(second);
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('save_address_book_contact', expect.anything())
    );
    expect(get(addressBookStore)).toEqual([]);
    stored.resolve(contact);
    await first;
    expect(get(addressBookStore)).toEqual([contact]);
    expect(
      invoke.mock.calls.filter(([command]) => command === 'save_address_book_contact')
    ).toHaveLength(1);
    expect(invoke.mock.calls[1][1].request.expectedSessionId).toBe('session-a');
  });
  it('reconciles uncertain saves and never overwrites existing notes or duplicate records', async () => {
    invoke.mockResolvedValue([contact, { ...contact, id: 'two' }]);
    expect(await addIdentityContact(id)).toEqual(contact);
    expect(get(addressBookStore)).toHaveLength(2);
    expect(invoke).toHaveBeenCalledTimes(1);
  });
  it('discards lock/account-switched reads, saves and resolution without replay', async () => {
    const read = deferred<AddressBookContact[]>();
    invoke.mockReturnValue(read.promise);
    const old = loadContacts();
    setContactSession({ sessionId: 'session-b', network: 'mainnet' });
    read.resolve([contact]);
    await expect(old).rejects.toThrow('Obsolete');
    expect(get(addressBookStore)).toEqual([]);
    expect(get(contactsLoadState)).toBe('idle');
    const resolution = deferred<ContactIdentity>();
    invoke.mockReturnValue(resolution.promise);
    const pending = resolveContactIdentity('alex@');
    setContactSession(null);
    resolution.resolve(id);
    await expect(pending).rejects.toThrow('Obsolete');
    expect(get(contactSession)).toBeNull();
  });
  it('keeps failed reads distinct from an empty book', async () => {
    invoke.mockRejectedValue(new Error('Storage unreadable'));
    await expect(loadContacts()).rejects.toThrow();
    expect(get(contactsLoadState)).toBe('error');
    await expect(addIdentityContact(id)).rejects.toThrow();
    expect(invoke.mock.calls.some(([command]) => command === 'save_address_book_contact')).toBe(
      false
    );
  });

  it('uses a complete warm cache for identity adds and keeps backend dedupe authoritative', async () => {
    invoke.mockImplementation((command: string) =>
      Promise.resolve(command === 'list_address_book_contacts' ? [] : contact)
    );
    await loadContacts();
    expect(await addIdentityContact(id)).toEqual(contact);
    expect(await loadContacts()).toEqual([contact]);
    expect(
      invoke.mock.calls.filter(([command]) => command === 'list_address_book_contacts')
    ).toHaveLength(1);
    expect(
      invoke.mock.calls.filter(([command]) => command === 'save_address_book_contact')
    ).toHaveLength(1);
  });

  it('publishes form saves and deletes only after durable acknowledgement, without a second read', async () => {
    const saved = deferred<AddressBookContact>();
    const deleted = deferred<boolean>();
    invoke.mockImplementation((command: string) => {
      if (command === 'list_address_book_contacts') return Promise.resolve([contact]);
      if (command === 'save_address_book_contact') return saved.promise;
      return deleted.promise;
    });
    await loadContacts();
    const edit = saveAddressBookContact({
      id: contact.id,
      displayName: 'Updated',
      endpoints: [],
      identities: [],
    });
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('save_address_book_contact', expect.anything())
    );
    const waitingRead = loadContacts();
    expect(get(addressBookStore)).toEqual([contact]);
    const updated = { ...contact, displayName: 'Updated', identities: [] };
    saved.resolve(updated);
    await edit;
    expect(await waitingRead).toEqual([updated]);
    const deletion = deleteAddressBookContact(contact.id);
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('delete_address_book_contact', expect.anything())
    );
    expect(get(addressBookStore)).toEqual([updated]);
    deleted.resolve(true);
    expect(await deletion).toBe(true);
    expect(await loadContacts()).toEqual([]);
    expect(
      invoke.mock.calls.filter(([command]) => command === 'list_address_book_contacts')
    ).toHaveLength(1);
  });

  it('re-reads after an uncertain delete or last-used update', async () => {
    let stored = [contact];
    invoke.mockImplementation((command: string) => {
      if (command === 'list_address_book_contacts') return Promise.resolve(stored);
      if (command === 'delete_address_book_contact') return Promise.reject(new Error('reply lost'));
      return Promise.resolve(true);
    });
    await loadContacts();
    await expect(deleteAddressBookContact(contact.id)).rejects.toThrow('reply lost');
    expect(get(addressBookStore)).toEqual([contact]);
    expect(get(contactsLoadState)).toBe('idle');
    stored = [];
    expect(await loadContacts()).toEqual([]);
    await markAddressBookEndpointUsed('endpoint');
    expect(get(contactsLoadState)).toBe('idle');
    await loadContacts();
    expect(
      invoke.mock.calls.filter(([command]) => command === 'list_address_book_contacts')
    ).toHaveLength(3);
  });

  it('does not let an older list response overwrite a confirmed form save', async () => {
    const oldRead = deferred<AddressBookContact[]>();
    const updated = { ...contact, displayName: 'Updated' };
    let reads = 0;
    invoke.mockImplementation((command: string) => {
      if (command === 'list_address_book_contacts')
        return ++reads === 1 ? oldRead.promise : Promise.resolve([updated]);
      return Promise.resolve(updated);
    });
    const pendingRead = loadContacts();
    await saveAddressBookContact({ displayName: 'Updated', endpoints: [], identities: [] });
    expect(get(addressBookStore)).toEqual([updated]);
    oldRead.resolve([contact]);
    expect(await pendingRead).toEqual([updated]);
    expect(get(addressBookStore)).toEqual([updated]);
    expect(reads).toBe(2);
  });

  it('serializes competing form mutations so a late save cannot resurrect a deleted contact', async () => {
    const saveReply = deferred<AddressBookContact>();
    const deleteReply = deferred<boolean>();
    const updated = { ...contact, displayName: 'Updated' };
    invoke.mockImplementation((command: string) => {
      if (command === 'list_address_book_contacts') return Promise.resolve([contact]);
      if (command === 'save_address_book_contact') return saveReply.promise;
      return deleteReply.promise;
    });
    await loadContacts();
    const save = saveAddressBookContact({ id: contact.id, displayName: 'Updated', endpoints: [] });
    const deletion = deleteAddressBookContact(contact.id);
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('save_address_book_contact', expect.anything())
    );
    expect(invoke.mock.calls.some(([command]) => command === 'delete_address_book_contact')).toBe(
      false
    );
    saveReply.resolve(updated);
    await save;
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('delete_address_book_contact', expect.anything())
    );
    deleteReply.resolve(true);
    await deletion;
    expect(get(addressBookStore)).toEqual([]);
  });

  it('cannot publish a form save into a replacement wallet session', async () => {
    const saveReply = deferred<AddressBookContact>();
    invoke.mockImplementation(() => saveReply.promise);
    const pending = saveAddressBookContact({ displayName: 'Updated', endpoints: [] });
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('save_address_book_contact', expect.anything())
    );
    setContactSession({ sessionId: 'session-b', network: 'mainnet' });
    saveReply.resolve(contact);
    await expect(pending).rejects.toThrow('Obsolete');
    expect(get(addressBookStore)).toEqual([]);
    expect(get(contactsLoadState)).toBe('idle');
  });

  it('refreshes after an uncertain create and stops at an existing endpoint without claiming success', async () => {
    const retry: ContactSaveRetry = {};
    const stored = { ...addressContact, id: 'possibly-preexisting', note: 'Other note' };
    invoke.mockImplementation((command: string) => {
      if (command === 'list_address_book_contacts') return Promise.resolve([stored]);
      if (command === 'validate_destination_address')
        return Promise.resolve({
          valid: true,
          normalizedAddress: stored.endpoints[0].normalizedAddress,
        });
      return Promise.reject(new Error('reply lost'));
    });
    await expect(saveAddressBookContact(addressRequest, retry)).rejects.toThrow('reply lost');
    await expect(saveAddressBookContact(addressRequest, retry)).rejects.toMatchObject({
      type: 'AddressBookDuplicate',
    });
    expect(
      invoke.mock.calls.filter(([command]) => command === 'save_address_book_contact')
    ).toHaveLength(1);
    expect(get(addressBookStore)).toEqual([stored]);
    expect(retry.uncertain).toBe(true);
  });

  it('stops an uncertain identity-only create when fresh storage has that identity', async () => {
    const retry: ContactSaveRetry = {};
    const request = {
      displayName: id.fullyQualifiedName,
      identities: [id],
      profileIdentity: id,
      endpoints: [],
    };
    invoke.mockImplementation((command: string) => {
      if (command === 'list_address_book_contacts') return Promise.resolve([contact]);
      return Promise.reject(new Error('reply lost'));
    });
    await expect(saveAddressBookContact(request, retry)).rejects.toThrow('reply lost');
    await expect(saveAddressBookContact(request, retry)).rejects.toMatchObject({
      type: 'AddressBookDuplicate',
    });
    expect(
      invoke.mock.calls.filter(([command]) => command === 'save_address_book_contact')
    ).toHaveLength(1);
    expect(get(addressBookStore)).toEqual([contact]);
  });

  it('retries a create only after fresh encrypted storage has no matching endpoint', async () => {
    const retry: ContactSaveRetry = {};
    let saves = 0;
    invoke.mockImplementation((command: string) => {
      if (command === 'list_address_book_contacts') return Promise.resolve([]);
      if (command === 'validate_destination_address')
        return Promise.resolve({
          valid: true,
          normalizedAddress: addressContact.endpoints[0].normalizedAddress,
        });
      return ++saves === 1
        ? Promise.reject(new Error('reply lost'))
        : Promise.resolve(addressContact);
    });
    await expect(saveAddressBookContact(addressRequest, retry)).rejects.toThrow();
    expect(await saveAddressBookContact(addressRequest, retry)).toEqual(addressContact);
    expect(saves).toBe(2);
    expect(get(addressBookStore)).toEqual([addressContact]);
  });

  it('refreshes before resubmitting an edit without silently calling readback a success', async () => {
    const retry: ContactSaveRetry = {};
    const updated = { ...addressContact, note: 'Updated note' };
    let saves = 0;
    invoke.mockImplementation((command: string) => {
      if (command === 'list_address_book_contacts') return Promise.resolve([updated]);
      return ++saves === 1 ? Promise.reject(new Error('reply lost')) : Promise.resolve(updated);
    });
    const request = { ...addressRequest, id: updated.id, note: 'Updated note' };
    await expect(saveAddressBookContact(request, retry)).rejects.toThrow();
    expect(await saveAddressBookContact(request, retry)).toEqual(updated);
    expect(saves).toBe(2);
  });

  it('reconciles a lost delete response before retry and does not report a false failure', async () => {
    const retry: ContactDeleteRetry = {};
    let stored: AddressBookContact[] = [addressContact];
    let deletes = 0;
    invoke.mockImplementation((command: string) => {
      if (command === 'list_address_book_contacts') return Promise.resolve(stored);
      deletes++;
      return Promise.reject(new Error('reply lost'));
    });
    await loadContacts();
    await expect(deleteAddressBookContact(addressContact.id, retry)).rejects.toThrow();
    stored = [];
    expect(await deleteAddressBookContact(addressContact.id, retry)).toBe(true);
    expect(deletes).toBe(1);
    expect(get(addressBookStore)).toEqual([]);
  });

  it('does not carry uncertain retry state into a replacement wallet session', async () => {
    const retry: ContactSaveRetry = {};
    invoke.mockRejectedValueOnce(new Error('reply lost'));
    await expect(saveAddressBookContact(addressRequest, retry)).rejects.toThrow();
    expect(retry.uncertain).toBe(true);
    setContactSession({ sessionId: 'session-b', network: 'mainnet' });
    invoke.mockResolvedValueOnce({ ...addressContact, id: 'new-wallet-contact' });
    await saveAddressBookContact(addressRequest, retry);
    const next = invoke.mock.calls[1][1].request;
    expect(next.expectedSessionId).toBe('session-b');
    expect(retry.uncertain).toBe(false);
    expect(
      invoke.mock.calls.filter(([command]) => command === 'list_address_book_contacts')
    ).toHaveLength(0);
  });
});

describe('confirmed profile cache', () => {
  it('deduplicates requests, caches briefly, retains last good content on failure, and clears confirmed removal', async () => {
    const ready = {
      state: 'ready',
      avatar: null,
      description: { value: 'Published text', source: {} },
      issues: [],
      revisionTxid: 'tx',
    };
    invoke.mockResolvedValue(ready);
    const first = loadIdentityProfile(id);
    expect(loadIdentityProfile(id)).toBe(first);
    await first;
    await loadIdentityProfile(id);
    expect(invoke).toHaveBeenCalledTimes(1);
    invoke.mockRejectedValue(new Error('offline'));
    await expect(loadIdentityProfile(id, true)).rejects.toThrow();
    expect(get(identityProfiles)[identityKey(id)].profile?.description?.value).toBe(
      'Published text'
    );
    expect(get(identityProfiles)[identityKey(id)].unavailable).toBe(true);
    invoke.mockResolvedValue({ state: 'empty', issues: [], revisionTxid: 'removed' });
    await loadIdentityProfile(id, true);
    expect(get(identityProfiles)[identityKey(id)].profile?.description).toBeUndefined();
  });
  it('isolates network and session state including late responses', async () => {
    const response = deferred<never>();
    invoke.mockReturnValue(response.promise);
    const pending = loadIdentityProfile(id);
    setContactSession(null);
    response.reject(new Error('old failure'));
    await expect(pending).rejects.toThrow();
    expect(get(identityProfiles)).toEqual({});
    setContactSession({ sessionId: 'test-session', network: 'testnet' });
    expect((await loadIdentityProfile(id)).state).toBe('unavailable');
    expect(invoke).toHaveBeenCalledTimes(1);
  });
});

describe('contact and profile race boundaries', () => {
  it('never applies a completed old-session save to a replacement wallet', async () => {
    const write = deferred<AddressBookContact>();
    invoke.mockImplementation((command: string) =>
      command === 'list_address_book_contacts' ? Promise.resolve([]) : write.promise
    );
    const pending = addIdentityContact(id);
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('save_address_book_contact', expect.anything())
    );
    setContactSession({ sessionId: 'replacement', network: 'mainnet' });
    write.resolve(contact);
    await expect(pending).rejects.toThrow('Obsolete');
    expect(get(addressBookStore)).toEqual([]);
    expect(
      invoke.mock.calls.filter(([command]) => command === 'save_address_book_contact')
    ).toHaveLength(1);
  });

  it('bounds profile concurrency and prioritizes a deliberate preview over queued avatars', async () => {
    const responses = new Map<
      string,
      ReturnType<typeof deferred<{ state: 'empty'; issues: []; revisionTxid: null }>>
    >();
    invoke.mockImplementation((_command: string, args: { identity_address: string }) => {
      const response = deferred<{ state: 'empty'; issues: []; revisionTxid: null }>();
      responses.set(args.identity_address, response);
      return response.promise;
    });
    const identities = Array.from({ length: 6 }, (_, index) => ({
      ...id,
      identityAddress: `identity-${index}`,
    }));
    const jobs = identities.map((identity) => loadIdentityProfile(identity));
    expect(responses.size).toBe(3);
    expect(loadIdentityProfile(identities[5], false, true)).toBe(jobs[5]);
    responses.get('identity-0')?.resolve({ state: 'empty', issues: [], revisionTxid: null });
    await vi.waitFor(() => expect(responses.has('identity-5')).toBe(true));
    expect(responses.has('identity-3')).toBe(false);
    for (let index = 0; index < 6; index++) {
      for (const response of responses.values())
        response.resolve({ state: 'empty', issues: [], revisionTxid: null });
      await Promise.resolve();
      await Promise.resolve();
    }
    await Promise.all(jobs);
    expect(invoke).toHaveBeenCalledTimes(6);
  });
});
