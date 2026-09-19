import type { WalletNetwork } from './wallet';

export type ContactIdentity = {
  identityAddress: string;
  fullyQualifiedName: string;
  network: WalletNetwork;
  chainId: string;
};
export type ResolvedContactIdentity = ContactIdentity & { status?: string | null };

export type AddressEndpointKind = 'vrpc' | 'btc' | 'eth' | 'zs';

export type AddressBookEndpoint = {
  id: string;
  kind: AddressEndpointKind;
  address: string;
  normalizedAddress: string;
  label: string;
  lastUsedAt: number | null;
  createdAt: number;
  updatedAt: number;
};

export type AddressBookContact = {
  id: string;
  displayName: string;
  note: string | null;
  createdAt: number;
  updatedAt: number;
  endpoints: AddressBookEndpoint[];
  identities?: ContactIdentity[];
  profileIdentity?: ContactIdentity | null;
  legacyDisplayName?: string | null;
};

export type SaveAddressBookEndpointInput = {
  id?: string;
  kind: AddressEndpointKind;
  address: string;
  label: string;
};

export type SaveAddressBookContactRequest = {
  id?: string;
  displayName: string;
  note?: string | null;
  endpoints: SaveAddressBookEndpointInput[];
  identities?: ContactIdentity[];
  profileIdentity?: ContactIdentity | null;
  expectedSessionId?: string;
  addIdentityIfMissing?: boolean;
};

export type ValidateDestinationAddressRequest = {
  kind: AddressEndpointKind;
  address: string;
};

export type ValidateDestinationAddressResult = {
  valid: boolean;
  normalizedAddress: string | null;
  reason: string | null;
};
