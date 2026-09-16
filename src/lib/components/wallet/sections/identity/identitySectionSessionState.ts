import type {
  IdentityProfileLoadResult,
  LinkedIdentity,
  PendingIdentityProfileUpdate,
  ProvisioningJobRecord,
} from '$lib/types/wallet.js';

export type IdentitySectionSessionState = {
  linkedIdentities: LinkedIdentity[];
  linkedIdentityError: string;
  hasLoadedLinkedIdentitiesOnce: boolean;
  provisioningJobs: ProvisioningJobRecord[];
  provisioningError: string;
  hasLoadedProvisioningOnce: boolean;
  profilesByAddress: Record<string, IdentityProfileLoadResult>;
  pendingProfilesByAddress: Record<string, PendingIdentityProfileUpdate>;
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
  };
}
