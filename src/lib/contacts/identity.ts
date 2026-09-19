import type { AddressBookContact, ContactIdentity } from '$lib/types/addressBook';
import type { WalletNetwork } from '$lib/types/wallet';

export function contactChainId(network: WalletNetwork): string {
  return network === 'testnet'
    ? 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq'
    : 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
}

export function identityKey(identity: ContactIdentity): string {
  // Base58 identifiers are case-sensitive. Names are never membership/cache keys.
  return JSON.stringify([identity.network, identity.chainId, identity.identityAddress]);
}

export function contactProfile(contact: AddressBookContact): ContactIdentity | null {
  const identities = contact.identities ?? [];
  if (identities.length === 1) return identities[0];
  return (
    identities.find(
      (identity) =>
        contact.profileIdentity && identityKey(identity) === identityKey(contact.profileIdentity)
    ) ?? null
  );
}

export function contactName(contact: AddressBookContact): string {
  return contactProfile(contact)?.fullyQualifiedName ?? contact.displayName;
}

export function matchingContacts(
  contacts: AddressBookContact[],
  identity: ContactIdentity
): AddressBookContact[] {
  const key = identityKey(identity);
  return contacts.filter((contact) =>
    contact.identities?.some((candidate) => identityKey(candidate) === key)
  );
}
