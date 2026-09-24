import { get } from 'svelte/store';
import { invokeSessionBoundWalletCommand } from '$lib/services/invokeWalletCommand';
import {
  addressBookStore,
  setAddressBookContacts,
  upsertAddressBookContact,
} from '$lib/stores/addressBook';
import type {
  AddressBookContact,
  ContactIdentity,
  ResolvedContactIdentity,
} from '$lib/types/addressBook';
import {
  type ContactSession,
  contactSession,
  contactsLoadState,
  isContactSessionCurrent,
  requireContactSession,
} from './session';
import { identityKey, matchingContacts } from './identity';

let loading: Promise<AddressBookContact[]> | null = null;
let saves = new Map<string, Promise<AddressBookContact>>();
let revision = 0;
let cacheComplete = false;
let mutationTail: Promise<void> = Promise.resolve();
const pendingMutations = new Set<Promise<unknown>>();
contactSession.subscribe(() => {
  loading = null;
  saves = new Map();
  cacheComplete = false;
  mutationTail = Promise.resolve();
  pendingMutations.clear();
  revision++;
});

export async function resolveContactIdentity(value: string): Promise<ResolvedContactIdentity> {
  const session = requireContactSession();
  const identity = await invokeSessionBoundWalletCommand<ResolvedContactIdentity>(
    'resolve_contact_identity',
    { identity: value, expected_session_id: session.sessionId }
  );
  if (!isContactSessionCurrent(session)) throw new Error('Obsolete identity request');
  return identity;
}

export function loadContacts(refresh = false): Promise<AddressBookContact[]> {
  const session = requireContactSession();
  if (pendingMutations.size) {
    return Promise.allSettled([...pendingMutations]).then(() => {
      if (!isContactSessionCurrent(session)) throw new Error('Obsolete contacts request');
      return loadContacts(refresh);
    });
  }
  if (loading) return loading;
  if (!refresh && cacheComplete && get(contactsLoadState) === 'ready')
    return Promise.resolve(get(addressBookStore));
  const version = revision;
  contactsLoadState.set('loading');
  const request = invokeSessionBoundWalletCommand<AddressBookContact[]>(
    'list_address_book_contacts',
    { expected_session_id: session.sessionId }
  )
    .then(
      (contacts) => {
        if (!isContactSessionCurrent(session)) throw new Error('Obsolete contacts request');
        // An older read cannot replace a committed write or satisfy a post-write check.
        if (version !== revision) {
          if (loading === request) loading = null;
          return loadContacts(refresh);
        }
        setAddressBookContacts(contacts);
        cacheComplete = true;
        contactsLoadState.set('ready');
        return contacts;
      },
      (error: unknown) => {
        if (!isContactSessionCurrent(session)) throw new Error('Obsolete contacts request');
        if (version !== revision) {
          if (loading === request) loading = null;
          return loadContacts(refresh);
        }
        cacheComplete = false;
        contactsLoadState.set('error');
        throw error;
      }
    )
    .finally(() => {
      if (loading === request) loading = null;
    });
  loading = request;
  return request;
}

/** Serialize renderer mutations and publish only backend-confirmed results. */
export function runContactMutation<T>(
  session: ContactSession,
  invokeMutation: () => Promise<T>,
  publish: (result: T) => boolean
): Promise<T> {
  if (!isContactSessionCurrent(session))
    return Promise.reject(new Error('Obsolete contacts mutation'));
  revision++;
  loading = null;
  const previous = mutationTail;
  let release!: () => void;
  mutationTail = new Promise<void>((resolve) => {
    release = resolve;
  });
  const request = (async () => {
    try {
      await previous;
      if (!isContactSessionCurrent(session)) throw new Error('Obsolete contacts mutation');
      const result = await invokeMutation();
      if (!isContactSessionCurrent(session)) throw new Error('Obsolete contacts mutation');
      revision++;
      if (!publish(result)) cacheComplete = false;
      return result;
    } catch (error) {
      // A rejected command may have committed before its result was lost. Re-read
      // encrypted storage before trusting membership or retrying a duplicate.
      if (isContactSessionCurrent(session)) {
        revision++;
        cacheComplete = false;
      }
      throw error;
    } finally {
      release();
    }
  })();
  pendingMutations.add(request);
  contactsLoadState.set('idle');
  void request
    .finally(() => {
      pendingMutations.delete(request);
      if (isContactSessionCurrent(session) && pendingMutations.size === 0)
        contactsLoadState.set(cacheComplete ? 'ready' : 'idle');
    })
    .catch(() => {});
  return request;
}

export function addIdentityContact(identity: ContactIdentity): Promise<AddressBookContact> {
  const session = requireContactSession();
  const key = identityKey(identity);
  if (identity.network !== session.network)
    return Promise.reject(new Error('Wrong contact network'));
  const existing = saves.get(key);
  if (existing) return existing;
  const request = (async () => {
    // A warm complete cache avoids decrypting again. Failed/uncertain writes
    // invalidate it, so a retry still reconciles against encrypted storage.
    await loadContacts();
    if (!isContactSessionCurrent(session)) throw new Error('Obsolete contact save');
    const matches = matchingContacts(get(addressBookStore), identity);
    if (matches.length) return matches[0];
    const saved = await runContactMutation(
      session,
      () =>
        invokeSessionBoundWalletCommand<AddressBookContact>('save_address_book_contact', {
          request: {
            displayName: identity.fullyQualifiedName,
            identities: [identity],
            profileIdentity: identity,
            endpoints: [],
            expectedSessionId: session.sessionId,
            addIdentityIfMissing: true,
          },
        }),
      (contact) => {
        upsertAddressBookContact(contact);
        return true;
      }
    );
    if (!isContactSessionCurrent(session)) throw new Error('Obsolete contact save');
    return saved;
  })().finally(() => {
    if (saves.get(key) === request) saves.delete(key);
  });
  saves.set(key, request);
  return request;
}
