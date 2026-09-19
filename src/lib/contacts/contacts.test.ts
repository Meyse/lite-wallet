import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { contactName, contactProfile, identityKey, matchingContacts } from './identity';
import { contactSession, contactsLoadState, setContactSession } from './session';
import { addIdentityContact, loadContacts, resolveContactIdentity } from './service';
import { identityProfiles, loadIdentityProfile } from './profiles';
import { addressBookStore } from '$lib/stores/addressBook';
import type { AddressBookContact, ContactIdentity } from '$lib/types/addressBook';
const invoke = vi.hoisted(() => vi.fn());
vi.mock('$lib/services/invokeWalletCommand', () => ({ invokeSessionBoundWalletCommand: invoke }));
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
