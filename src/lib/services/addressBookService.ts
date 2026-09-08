import { invokeWalletCommand } from './invokeWalletCommand.js';
import type {
  AddressBookContact,
  SaveAddressBookContactRequest,
  ValidateDestinationAddressRequest,
  ValidateDestinationAddressResult
} from '$lib/types/addressBook';

export async function listAddressBookContacts(): Promise<AddressBookContact[]> {
  return invokeWalletCommand<AddressBookContact[]>('list_address_book_contacts');
}

export async function saveAddressBookContact(
  request: SaveAddressBookContactRequest
): Promise<AddressBookContact> {
  return invokeWalletCommand<AddressBookContact>('save_address_book_contact', { request });
}

export async function deleteAddressBookContact(contactId: string): Promise<boolean> {
  return invokeWalletCommand<boolean>('delete_address_book_contact', { contact_id: contactId });
}

export async function markAddressBookEndpointUsed(endpointId: string): Promise<boolean> {
  return invokeWalletCommand<boolean>('mark_address_book_endpoint_used', { endpoint_id: endpointId });
}

export async function validateDestinationAddress(
  request: ValidateDestinationAddressRequest
): Promise<ValidateDestinationAddressResult> {
  return invokeWalletCommand<ValidateDestinationAddressResult>('validate_destination_address', { request });
}
