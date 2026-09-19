import { get } from 'svelte/store';
import { contactSession, isContactSessionCurrent } from '$lib/contacts/session';
import { noteContactsMutation } from '$lib/contacts/service';
import { invokeSessionBoundWalletCommand, invokeWalletCommand } from './invokeWalletCommand.js';
import type {
  AddressBookContact,
  SaveAddressBookContactRequest,
  ValidateDestinationAddressRequest,
  ValidateDestinationAddressResult,
} from '$lib/types/addressBook';

export async function listAddressBookContacts(): Promise<AddressBookContact[]> {
  return invokeWalletCommand<AddressBookContact[]>('list_address_book_contacts');
}

export async function saveAddressBookContact(
  request: SaveAddressBookContactRequest
): Promise<AddressBookContact> {
  const session = get(contactSession);
  if (!session)
    return invokeWalletCommand<AddressBookContact>('save_address_book_contact', { request });
  const saved = await invokeSessionBoundWalletCommand<AddressBookContact>(
    'save_address_book_contact',
    { request: { ...request, expectedSessionId: session.sessionId } }
  );
  if (!isContactSessionCurrent(session)) throw new Error('Obsolete contact save');
  noteContactsMutation();
  return saved;
}

export async function deleteAddressBookContact(contactId: string): Promise<boolean> {
  const session = get(contactSession);
  if (!session)
    return invokeWalletCommand<boolean>('delete_address_book_contact', { contact_id: contactId });
  const result = await invokeSessionBoundWalletCommand<boolean>('delete_address_book_contact', {
    contact_id: contactId,
    expected_session_id: session.sessionId,
  });
  if (!isContactSessionCurrent(session)) throw new Error('Obsolete contacts mutation');
  noteContactsMutation();
  return result;
}

export async function markAddressBookEndpointUsed(endpointId: string): Promise<boolean> {
  const session = get(contactSession);
  if (!session)
    return invokeWalletCommand<boolean>('mark_address_book_endpoint_used', {
      endpoint_id: endpointId,
    });
  const result = await invokeSessionBoundWalletCommand<boolean>('mark_address_book_endpoint_used', {
    endpoint_id: endpointId,
    expected_session_id: session.sessionId,
  });
  if (!isContactSessionCurrent(session)) throw new Error('Obsolete contacts mutation');
  noteContactsMutation();
  return result;
}

export async function validateDestinationAddress(
  request: ValidateDestinationAddressRequest
): Promise<ValidateDestinationAddressResult> {
  return invokeWalletCommand<ValidateDestinationAddressResult>('validate_destination_address', {
    request,
  });
}
