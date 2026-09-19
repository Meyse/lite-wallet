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
  contactSession,
  contactsLoadState,
  isContactSessionCurrent,
  requireContactSession,
} from './session';
import { identityKey, matchingContacts } from './identity';

let loading: Promise<AddressBookContact[]> | null = null;
let saves = new Map<string, Promise<AddressBookContact>>();
let revision = 0;
contactSession.subscribe(() => {
  loading = null;
  saves = new Map();
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
  if (loading) return loading;
  if (!refresh && get(contactsLoadState) === 'ready') return Promise.resolve(get(addressBookStore));
  const version = revision;
  contactsLoadState.set('loading');
  const request = invokeSessionBoundWalletCommand<AddressBookContact[]>(
    'list_address_book_contacts',
    { expected_session_id: session.sessionId }
  )
    .then((contacts) => {
      if (!isContactSessionCurrent(session)) throw new Error('Obsolete contacts request');
      // Do not replace a newer successful mutation with a slower read.
      if (version === revision) setAddressBookContacts(contacts);
      contactsLoadState.set('ready');
      return get(addressBookStore);
    })
    .catch((error: unknown) => {
      if (isContactSessionCurrent(session)) contactsLoadState.set('error');
      throw error;
    })
    .finally(() => {
      if (loading === request) loading = null;
    });
  loading = request;
  return request;
}

export function noteContactsMutation(): void {
  revision++;
}

export function addIdentityContact(identity: ContactIdentity): Promise<AddressBookContact> {
  const session = requireContactSession();
  const key = identityKey(identity);
  if (identity.network !== session.network)
    return Promise.reject(new Error('Wrong contact network'));
  const existing = saves.get(key);
  if (existing) return existing;
  const request = (async () => {
    // Reconcile first, including an uncertain previous commit, then dedupe again in encrypted storage.
    await loadContacts(true);
    if (!isContactSessionCurrent(session)) throw new Error('Obsolete contact save');
    const matches = matchingContacts(get(addressBookStore), identity);
    if (matches.length) return matches[0];
    const saved = await invokeSessionBoundWalletCommand<AddressBookContact>(
      'save_address_book_contact',
      {
        request: {
          displayName: identity.fullyQualifiedName,
          identities: [identity],
          profileIdentity: identity,
          endpoints: [],
          expectedSessionId: session.sessionId,
          addIdentityIfMissing: true,
        },
      }
    );
    if (!isContactSessionCurrent(session)) throw new Error('Obsolete contact save');
    revision++;
    upsertAddressBookContact(saved);
    return saved;
  })().finally(() => {
    if (saves.get(key) === request) saves.delete(key);
  });
  saves.set(key, request);
  return request;
}
