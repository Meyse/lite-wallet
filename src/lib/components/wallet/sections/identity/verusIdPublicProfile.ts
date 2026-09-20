import type { ResolvedContactIdentity } from '$lib/types/addressBook';

export type IdentitySectionTab = 'linked' | 'lookup';

export type VerusIdLookupStatus = 'idle' | 'looking-up' | 'resolved' | 'not-found' | 'unavailable';

export type VerusIdLookupState = {
  query: string;
  submittedQuery: string;
  status: VerusIdLookupStatus;
  result: ResolvedContactIdentity | null;
  scrollTop: number;
};

export type VerusIdProfileOrigin = {
  kind: 'lookup';
};

export type VerusIdProfileDestination = {
  identity: ResolvedContactIdentity;
  origin: VerusIdProfileOrigin;
};

export function createVerusIdLookupState(): VerusIdLookupState {
  return {
    query: '',
    submittedQuery: '',
    status: 'idle',
    result: null,
    scrollTop: 0,
  };
}
