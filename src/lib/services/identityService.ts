/**
 * Thin invoke wrappers for identity update/revoke/recover commands.
 * Security: send accepts only preflight_id.
 */

import type {
  IdentityPreflightParams,
  IdentityPreflightResult,
  IdentitySendRequest,
  IdentitySendResult,
} from '$lib/types/wallet.js';
import { invokeWalletCommand } from './invokeWalletCommand.js';
import {
  invalidateWalletDisplayHistory,
  invalidateWalletDisplayScopes,
} from './walletDisplayService.js';

export async function preflightIdentityUpdate(
  params: IdentityPreflightParams
): Promise<IdentityPreflightResult> {
  return invokeWalletCommand<IdentityPreflightResult>('preflight_identity_update', {
    params: {
      coinId: params.coinId,
      channelId: params.channelId,
      operation: params.operation,
      targetIdentity: params.targetIdentity,
      patch: params.patch
        ? {
            primaryAddresses: params.patch.primaryAddresses ?? null,
            recoveryAuthority: params.patch.recoveryAuthority ?? null,
            revocationAuthority: params.patch.revocationAuthority ?? null,
            privateAddress: params.patch.privateAddress ?? null,
          }
        : null,
      memo: params.memo ?? null,
    },
  });
}

export async function sendIdentityUpdate(
  request: IdentitySendRequest
): Promise<IdentitySendResult> {
  const result = await invokeWalletCommand<IdentitySendResult>('send_identity_update', {
    request: { preflightId: request.preflightId },
  });
  invalidateWalletDisplayScopes();
  invalidateWalletDisplayHistory();
  return result;
}
