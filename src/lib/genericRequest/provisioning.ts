import {
  LOGIN_CONSENT_PROVISIONING_RESULT_STATE_COMPLETE,
  LOGIN_CONSENT_PROVISIONING_RESULT_STATE_FAILED,
  LOGIN_CONSENT_PROVISIONING_RESULT_STATE_PENDINGAPPROVAL,
  LoginConsentProvisioningChallenge,
  LoginConsentProvisioningRequest,
  LoginConsentProvisioningResponse,
  VerusIDSignature,
} from 'verus-typescript-primitives';
import * as genericRequestService from '$lib/services/genericRequestService.js';
import type { ProvisioningJobRecord } from '$lib/types/wallet.js';
import { base64ToBytes, bytesToHex, readUint32LE } from '$lib/utils/bytes.js';
import type {
  AuthenticationDetailSession,
  GenericRequestFlowSession,
  ProvisionIdentityDetailSession,
} from './session';

const MAINNET_SYSTEM_ID = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
const TESTNET_SYSTEM_ID = 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq';

type SubmitGenericProvisioningRequestParams = {
  session: GenericRequestFlowSession;
  authenticationDetail: AuthenticationDetailSession;
  provisioningDetail: ProvisionIdentityDetailSession;
  signingAddress: string;
  /**
   * The full name the user chose, when the request offers none (Verus
   * Mobile's provisioning form). Rust checks the name the challenge carries.
   */
  chosenFqn?: string | null;
};

type ProvisioningResponseResultShape = {
  state?: string;
  error_desc?: string;
  identity_address?: string;
  fully_qualified_name?: string;
  info_uri?: string;
};

function normalizeRequestedName(requestedFqn: string): string {
  const match = requestedFqn.match(/^([^.@]+)/);
  return match?.[1] ?? requestedFqn;
}

function looksLikeIdentityAddress(value: string): boolean {
  const trimmed = value.trim();
  return trimmed.startsWith('i') && !trimmed.includes('.');
}

function extractSignatureBlockHeight(signatureBase64: string): {
  version: number;
  blockHeight: number;
} {
  const signatureBytes = base64ToBytes(signatureBase64);
  if (signatureBytes.length < 6) {
    throw new Error('genericRequest.provisioning.error.invalidResponse');
  }

  const version = signatureBytes[0];
  const blockHeightOffset = version >= 2 ? 2 : 1;

  if (signatureBytes.length < blockHeightOffset + 4) {
    throw new Error('genericRequest.provisioning.error.invalidResponse');
  }

  return {
    version,
    blockHeight: readUint32LE(signatureBytes, blockHeightOffset),
  };
}

export async function submitGenericProvisioningRequest(
  params: SubmitGenericProvisioningRequestParams
): Promise<ProvisioningJobRecord> {
  const { session, authenticationDetail, provisioningDetail, signingAddress } = params;

  const webhookUrl = provisioningDetail.webhook?.trim();
  if (!webhookUrl) {
    throw new Error('genericRequest.provisioning.error.noWebhook');
  }

  const requestedFqn = (provisioningDetail.requestedFqn ?? params.chosenFqn)?.trim();
  if (!requestedFqn) {
    throw new Error('genericRequest.provisioning.error.invalidRequest');
  }

  // Verus Mobile uses the request's own ID, else the sign-in detail's
  // (`AuthenticationRequestInfo.js`, `ProvisionIdentityConfirm.js`).
  const challengeId = session.requestId || authenticationDetail.requestId;
  if (!challengeId) {
    throw new Error('genericRequest.provisioning.error.invalidRequest');
  }

  const requestSignerId = session.signer.identityId?.trim();
  if (!requestSignerId) {
    throw new Error('genericRequest.provisioning.error.invalidRequest');
  }

  const requestSystemId =
    provisioningDetail.systemId?.trim() ||
    session.signer.systemId?.trim() ||
    (session.testnet ? TESTNET_SYSTEM_ID : MAINNET_SYSTEM_ID);

  const provisioningRequest = new LoginConsentProvisioningRequest({
    signing_address: signingAddress.trim(),
    challenge: new LoginConsentProvisioningChallenge({
      challenge_id: challengeId,
      created_at: Math.floor(Date.now() / 1000),
      name: normalizeRequestedName(requestedFqn),
      system_id: provisioningDetail.systemId?.trim() || undefined,
      parent: provisioningDetail.parent?.trim() || undefined,
    }),
  });

  const signingChallengeId = await genericRequestService.prepareProvisioningSignature(
    session.requestHex,
    bytesToHex(provisioningRequest.challenge.toBuffer()),
    signingAddress,
    requestSystemId
  );
  const requestSignature =
    await genericRequestService.confirmProvisioningSignature(signingChallengeId);
  provisioningRequest.signature = new VerusIDSignature({ signature: requestSignature });

  // Rust sends it to the request's provisioning address, with the 30-second
  // limit (spec "Delivery").
  const responseText = await genericRequestService
    .postProvisioningRequest(
      session.requestHex,
      webhookUrl,
      JSON.stringify(provisioningRequest.toJson())
    )
    .catch(() => {
      throw new Error('genericRequest.provisioning.error.webhook');
    });

  let responseJson: ConstructorParameters<typeof LoginConsentProvisioningResponse>[0];
  try {
    responseJson = JSON.parse(responseText);
  } catch {
    throw new Error('genericRequest.provisioning.error.webhook');
  }
  const provisioningResponse = new LoginConsentProvisioningResponse(responseJson);
  const responseSignature = provisioningResponse.signature?.signature;
  const responseSignerId = provisioningResponse.signing_id?.trim();
  const responseSystemId = provisioningResponse.system_id?.trim();

  if (!responseSignature || !responseSignerId || !responseSystemId) {
    throw new Error('genericRequest.provisioning.error.invalidResponse');
  }

  const { version, blockHeight } = extractSignatureBlockHeight(responseSignature);
  const responseHashHex = bytesToHex(provisioningResponse.getDecisionHash(blockHeight, version));
  const verified = await genericRequestService.verifyIdentitySignatureHash(
    responseHashHex,
    responseSignature,
    responseSignerId,
    responseSystemId
  );

  if (!verified) {
    throw new Error('genericRequest.provisioning.error.invalidResponseSignature');
  }

  const decisionWithResult = provisioningResponse.decision as {
    result?: ProvisioningResponseResultShape;
  } | null;
  const result = decisionWithResult?.result;
  if (!result?.state) {
    throw new Error('genericRequest.provisioning.error.invalidResponse');
  }

  if (result.state === LOGIN_CONSENT_PROVISIONING_RESULT_STATE_FAILED.vdxfid) {
    throw new Error(result.error_desc?.trim() || 'genericRequest.provisioning.error.failed');
  }

  const allowedStates = new Set([
    LOGIN_CONSENT_PROVISIONING_RESULT_STATE_PENDINGAPPROVAL.vdxfid,
    LOGIN_CONSENT_PROVISIONING_RESULT_STATE_COMPLETE.vdxfid,
  ]);
  if (!allowedStates.has(result.state)) {
    throw new Error('genericRequest.provisioning.error.invalidResponse');
  }

  const requestedIdentityAddress = looksLikeIdentityAddress(requestedFqn) ? requestedFqn : null;
  if (
    result.identity_address &&
    requestedIdentityAddress &&
    result.identity_address.toLowerCase() !== requestedIdentityAddress.toLowerCase()
  ) {
    throw new Error('genericRequest.provisioning.error.identityMismatch');
  }

  if (
    result.fully_qualified_name &&
    result.fully_qualified_name.toLowerCase() !== requestedFqn.toLowerCase()
  ) {
    throw new Error('genericRequest.provisioning.error.nameMismatch');
  }

  return genericRequestService.storeGenericProvisioningJob({
    requestHex: session.requestHex,
    requestedIdentityAddress: result.identity_address ?? requestedIdentityAddress,
    requestedFqn,
    signingId: requestSignerId,
    hasResponseUris: session.responseUris.length > 0,
    infoUri: result.info_uri ?? null,
    status: 'pending',
  });
}
