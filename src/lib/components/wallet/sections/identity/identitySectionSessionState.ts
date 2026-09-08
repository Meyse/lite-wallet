import type { LinkedIdentity, ProvisioningJobRecord } from '$lib/types/wallet.js';

export type IdentitySectionSessionState = {
  linkedIdentities: LinkedIdentity[];
  linkedIdentityError: string;
  hasLoadedLinkedIdentitiesOnce: boolean;
  provisioningJobs: ProvisioningJobRecord[];
  provisioningError: string;
  hasLoadedProvisioningOnce: boolean;
};

export function createIdentitySectionSessionState(): IdentitySectionSessionState {
  return {
    linkedIdentities: [],
    linkedIdentityError: '',
    hasLoadedLinkedIdentitiesOnce: false,
    provisioningJobs: [],
    provisioningError: '',
    hasLoadedProvisioningOnce: false
  };
}
