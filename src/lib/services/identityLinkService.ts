import { get } from 'svelte/store';
import { contactSession } from '$lib/contacts/session';
import { contactChainId } from '$lib/contacts/identity';
import { loadIdentityProfile } from '$lib/contacts/profiles';
/**
 * Thin invoke wrappers for identity discovery/link/detail commands.
 */

import { invokeSessionBoundWalletCommand, invokeWalletCommand } from './invokeWalletCommand.js';
import {
  invalidateWalletDisplayHistory,
  invalidateWalletDisplayScopes,
} from './walletDisplayService.js';
import type {
  IdentityDetails,
  IdentityProfileLoadResult,
  IdentityProfilePreflightRequest,
  IdentityProfilePreflightResult,
  LinkableIdentity,
  LinkedIdentity,
  LinkIdentityRequest,
  PendingIdentityProfileUpdate,
  ProfilePublicationState,
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

export async function getIdentityProfile(
  identityAddress: string,
  refresh = false,
  priority = false
): Promise<IdentityProfileLoadResult> {
  const session = get(contactSession);
  if (session)
    return loadIdentityProfile(
      {
        identityAddress,
        fullyQualifiedName: identityAddress,
        network: session.network,
        chainId: contactChainId(session.network),
      },
      refresh,
      priority
    );
  return invokeWalletCommand<IdentityProfileLoadResult>('get_identity_profile', {
    identity_address: identityAddress,
  });
}

export async function preflightIdentityProfileUpdate(
  request: IdentityProfilePreflightRequest
): Promise<IdentityProfilePreflightResult> {
  return invokeSessionBoundWalletCommand<IdentityProfilePreflightResult>(
    'preflight_identity_profile_update',
    {
      expected_session_id: get(contactSession)?.sessionId ?? null,
      request: {
        coinId: request.coinId,
        channelId: request.channelId,
        identityAddress: request.identityAddress,
        avatar: request.avatar,
        header: request.header ?? { action: 'keep' },
        smallerAvatar: request.smallerAvatar,
        smallerHeader: request.smallerHeader,
        description: request.description,
      },
    }
  );
}

export async function getPendingIdentityProfileUpdates(): Promise<PendingIdentityProfileUpdate[]> {
  return invokeWalletCommand<PendingIdentityProfileUpdate[]>(
    'get_pending_identity_profile_updates'
  );
}

export async function clearPendingIdentityProfileUpdate(
  identityAddress: string,
  txid: string
): Promise<boolean> {
  return invokeWalletCommand<boolean>('clear_pending_identity_profile_update', {
    identity_address: identityAddress,
    txid,
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

export async function getIdentityProfilePublication(
  identityAddress: string
): Promise<ProfilePublicationState | null> {
  return invokeSessionBoundWalletCommand('get_identity_profile_publication', {
    expected_session_id: get(contactSession)?.sessionId ?? '',
    identity_address: identityAddress,
  });
}
export async function reviewIdentityProfilePublication(
  identityAddress: string,
  planId: string,
  smallerField?: 'avatar' | 'header'
): Promise<IdentityProfilePreflightResult> {
  return invokeSessionBoundWalletCommand('review_identity_profile_publication', {
    expected_session_id: get(contactSession)?.sessionId ?? '',
    identity_address: identityAddress,
    plan_id: planId,
    smaller_field: smallerField ?? null,
  });
}
export async function discardIdentityProfilePublication(planId: string): Promise<void> {
  return invokeSessionBoundWalletCommand('discard_identity_profile_publication', {
    expected_session_id: get(contactSession)?.sessionId ?? '',
    plan_id: planId,
  });
}

/** Canonical revision + transaction/block confirmation, including all-field removals. */
export async function confirmIdentityProfileUpdate(
  identityAddress: string,
  txid: string
): Promise<IdentityProfileLoadResult | null> {
  return invokeSessionBoundWalletCommand('confirm_identity_profile_update', {
    expected_session_id: get(contactSession)?.sessionId ?? '',
    identity_address: identityAddress,
    txid,
  });
}
