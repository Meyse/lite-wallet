import { get } from 'svelte/store';
import { matchingContacts } from '$lib/contacts/identity';
import {
  type ContactSession,
  contactSession,
  isContactSessionCurrent,
} from '$lib/contacts/session';
import { loadContacts, runContactMutation } from '$lib/contacts/service';
import { removeAddressBookContact, upsertAddressBookContact } from '$lib/stores/addressBook';
import { invokeSessionBoundWalletCommand, invokeWalletCommand } from './invokeWalletCommand.js';
import type {
  AddressBookContact,
  SaveAddressBookContactRequest,
  ValidateDestinationAddressRequest,
  ValidateDestinationAddressResult,
} from '$lib/types/addressBook';

export type ContactSaveRetry = {
  session?: ContactSession;
  uncertain?: boolean;
};

export type ContactDeleteRetry = {
  session?: ContactSession;
  failedId?: string;
};

export async function listAddressBookContacts(): Promise<AddressBookContact[]> {
  return invokeWalletCommand<AddressBookContact[]>('list_address_book_contacts');
}

export async function saveAddressBookContact(
  request: SaveAddressBookContactRequest,
  retry?: ContactSaveRetry
): Promise<AddressBookContact> {
  const session = get(contactSession);
  if (!session) throw new Error('Contact session unavailable');
  if (retry && retry.session !== session) {
    retry.session = session;
    retry.uncertain = false;
  }
  if (retry?.uncertain) {
    // A rejected command may have committed. Refresh before replaying a create;
    // a matching endpoint might be ours or a preexisting contact, so never
    // silently call either one a successful save.
    const contacts = await loadContacts(true);
    if (!isContactSessionCurrent(session)) throw new Error('Obsolete contact save');
    if (!request.id) {
      if (request.identities?.some((identity) => matchingContacts(contacts, identity).length)) {
        throw Object.assign(new Error('AddressBookDuplicate'), { type: 'AddressBookDuplicate' });
      }
      for (const endpoint of request.endpoints) {
        const result = await validateDestinationAddress({
          kind: endpoint.kind,
          address: endpoint.address,
        });
        if (!isContactSessionCurrent(session)) throw new Error('Obsolete contact save');
        if (!result.valid || !result.normalizedAddress) continue;
        if (
          contacts.some((contact) =>
            contact.endpoints.some(
              (saved) =>
                saved.kind === endpoint.kind && saved.normalizedAddress === result.normalizedAddress
            )
          )
        ) {
          throw Object.assign(new Error('AddressBookDuplicate'), { type: 'AddressBookDuplicate' });
        }
      }
    }
  }
  try {
    const saved = await runContactMutation(
      session,
      () =>
        invokeSessionBoundWalletCommand<AddressBookContact>('save_address_book_contact', {
          request: { ...request, expectedSessionId: session.sessionId },
        }),
      (saved) => {
        upsertAddressBookContact(saved);
        return true;
      }
    );
    if (retry) retry.uncertain = false;
    return saved;
  } catch (error) {
    if (retry && isContactSessionCurrent(session)) retry.uncertain = true;
    throw error;
  }
}

export async function deleteAddressBookContact(
  contactId: string,
  retry?: ContactDeleteRetry
): Promise<boolean> {
  const session = get(contactSession);
  if (!session) throw new Error('Contact session unavailable');
  if (retry && retry.session !== session) {
    retry.session = session;
    retry.failedId = undefined;
  }
  if (retry?.failedId === contactId) {
    const contacts = await loadContacts(true);
    if (!isContactSessionCurrent(session)) throw new Error('Obsolete contacts mutation');
    if (!contacts.some((contact) => contact.id === contactId)) {
      retry.failedId = undefined;
      return true;
    }
  }
  try {
    const deleted = await runContactMutation(
      session,
      () =>
        invokeSessionBoundWalletCommand<boolean>('delete_address_book_contact', {
          contact_id: contactId,
          expected_session_id: session.sessionId,
        }),
      (deleted) => {
        if (deleted) removeAddressBookContact(contactId);
        return deleted;
      }
    );
    if (retry) retry.failedId = deleted ? undefined : contactId;
    return deleted;
  } catch (error) {
    if (retry && isContactSessionCurrent(session)) retry.failedId = contactId;
    throw error;
  }
}

export async function markAddressBookEndpointUsed(endpointId: string): Promise<boolean> {
  const session = get(contactSession);
  if (!session) throw new Error('Contact session unavailable');
  return runContactMutation(
    session,
    () =>
      invokeSessionBoundWalletCommand<boolean>('mark_address_book_endpoint_used', {
        endpoint_id: endpointId,
        expected_session_id: session.sessionId,
      }),
    // The command returns no canonical timestamp; refresh on the next read.
    () => false
  );
}

export async function validateDestinationAddress(
  request: ValidateDestinationAddressRequest
): Promise<ValidateDestinationAddressResult> {
  return invokeWalletCommand<ValidateDestinationAddressResult>('validate_destination_address', {
    request,
  });
}
