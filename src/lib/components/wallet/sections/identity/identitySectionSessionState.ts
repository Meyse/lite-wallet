import type {
  IdentityProfileLoadResult,
  LinkedIdentity,
  PendingIdentityProfileUpdate,
  ProvisioningJobRecord,
} from '$lib/types/wallet.js';
import {
  createVerusIdLookupState,
  type IdentitySectionTab,
  type VerusIdLookupState,
  type VerusIdProfileDestination,
} from './verusIdPublicProfile';

export type IdentitySectionSessionState = {
  linkedIdentities: LinkedIdentity[];
  linkedIdentityError: string;
  hasLoadedLinkedIdentitiesOnce: boolean;
  provisioningJobs: ProvisioningJobRecord[];
  provisioningError: string;
  hasLoadedProvisioningOnce: boolean;
  profilesByAddress: Record<string, IdentityProfileLoadResult>;
  pendingProfilesByAddress: Record<string, PendingIdentityProfileUpdate>;
  activeTab: IdentitySectionTab;
  linkedFilter: string;
  lookup: VerusIdLookupState;
  publicProfile: VerusIdProfileDestination | null;
};

export function createIdentitySectionSessionState(): IdentitySectionSessionState {
  return {
    linkedIdentities: [],
    linkedIdentityError: '',
    hasLoadedLinkedIdentitiesOnce: false,
    provisioningJobs: [],
    provisioningError: '',
    hasLoadedProvisioningOnce: false,
    profilesByAddress: {},
    pendingProfilesByAddress: {},
    activeTab: 'linked',
    linkedFilter: '',
    lookup: createVerusIdLookupState(),
    publicProfile: null,
  };
}
