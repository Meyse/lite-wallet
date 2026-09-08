/**
 * Thin invoke wrappers for identity discovery/link/detail commands.
 */

import { invokeWalletCommand } from './invokeWalletCommand.js';
import {
  invalidateWalletDisplayHistory,
  invalidateWalletDisplayScopes,
} from './walletDisplayService.js';
import type {
  IdentityDetails,
  LinkableIdentity,
  LinkedIdentity,
  LinkIdentityRequest,
  SetLinkedIdentityFavoriteRequest,
  UnlinkIdentityRequest,
} from '$lib/types/wallet.js';

export async function discoverLinkableIdentities(): Promise<LinkableIdentity[]> {
  return invokeWalletCommand<LinkableIdentity[]>('discover_linkable_identities');
}

export async function getLinkedIdentities(): Promise<LinkedIdentity[]> {
  return invokeWalletCommand<LinkedIdentity[]>('get_linked_identities');
}

export async function linkIdentity(request: LinkIdentityRequest): Promise<LinkedIdentity[]> {
  const result = await invokeWalletCommand<LinkedIdentity[]>('link_identity', {
    request: {
      identityAddress: request.identityAddress,
    },
  });
  invalidateWalletDisplayScopes();
  invalidateWalletDisplayHistory();
  return result;
}

export async function unlinkIdentity(request: UnlinkIdentityRequest): Promise<LinkedIdentity[]> {
  const result = await invokeWalletCommand<LinkedIdentity[]>('unlink_identity', {
    request: {
      identityAddress: request.identityAddress,
    },
  });
  invalidateWalletDisplayScopes();
  invalidateWalletDisplayHistory();
  return result;
}

export async function getIdentityDetails(identityAddress: string): Promise<IdentityDetails> {
  return invokeWalletCommand<IdentityDetails>('get_identity_details', {
    identity_address: identityAddress,
  });
}

export async function setLinkedIdentityFavorite(
  request: SetLinkedIdentityFavoriteRequest
): Promise<LinkedIdentity[]> {
  const result = await invokeWalletCommand<LinkedIdentity[]>('set_linked_identity_favorite', {
    request: {
      identityAddress: request.identityAddress,
      favorite: request.favorite,
    },
  });
  invalidateWalletDisplayScopes();
  return result;
}
