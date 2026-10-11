import type {
  BuildAndSignGenericResponseResult,
  GenericCredentialReview,
  GenericIdentityUpdatePreflightResult,
  GenericIdentityUpdateReviewResult,
  GenericInvoicePreflightResult,
  GenericInvoiceReview,
  GenericRequestTransactionResult,
  GenericResponseSignerInput,
  GiftCardClaimPreflight,
  GiftCardClaimResult,
  GiftCardReview,
  LinkReadyProvisioningJobResult,
  OpenedGenericRequest,
  ProvisioningJobRecord,
  StoreGenericProvisioningJobRequest,
} from '$lib/types/wallet.js';
import { invokeWalletCommand } from './invokeWalletCommand.js';
import {
  invalidateWalletDisplayHistory,
  invalidateWalletDisplayScopes,
} from './walletDisplayService.js';

/**
 * Rust checks the request (order, signature, network, expiry) and holds it as
 * the one open request. Later commands act only on that request.
 */
export async function openGenericRequest(requestHex: string): Promise<OpenedGenericRequest> {
  return invokeWalletCommand<OpenedGenericRequest>('open_generic_request', {
    request_hex: requestHex,
  });
}

export async function closeGenericRequest(): Promise<void> {
  return invokeWalletCommand<void>('close_generic_request');
}

/** Rust builds the response from the open request; the UI names only the responder. */
export async function buildAndSignGenericResponse(
  requestHex: string,
  signer: GenericResponseSignerInput
): Promise<BuildAndSignGenericResponseResult> {
  return invokeWalletCommand<BuildAndSignGenericResponseResult>('build_and_sign_generic_response', {
    request_hex: requestHex,
    signer,
  });
}

export async function sendGenericIdentityUpdate(
  requestHex: string,
  preflightId: string
): Promise<GenericRequestTransactionResult> {
  return invokeWalletCommand<GenericRequestTransactionResult>('send_generic_identity_update', {
    request_hex: requestHex,
    preflight_id: preflightId,
  });
}

export async function reviewGenericInvoice(requestHex: string): Promise<GenericInvoiceReview> {
  return invokeWalletCommand<GenericInvoiceReview>('review_generic_invoice', {
    request_hex: requestHex,
  });
}

/**
 * Rust reads and decrypts the credentials of `identityId` that the open
 * request asks for. The response shares exactly the credentials returned here.
 */
export async function reviewGenericCredentials(
  requestHex: string,
  identityId: string
): Promise<GenericCredentialReview> {
  return invokeWalletCommand<GenericCredentialReview>('review_generic_credentials', {
    request_hex: requestHex,
    identity_id: identityId,
  });
}

/**
 * Rust reads the gift card's key, with `password` when the card has one, and
 * what the key holds. The password is not stored.
 */
export async function reviewGiftCard(
  requestHex: string,
  password: string | null
): Promise<GiftCardReview> {
  return invokeWalletCommand<GiftCardReview>('review_gift_card', {
    request_hex: requestHex,
    password,
  });
}

export async function preflightGiftCardClaim(requestHex: string): Promise<GiftCardClaimPreflight> {
  return invokeWalletCommand<GiftCardClaimPreflight>('preflight_gift_card_claim', {
    request_hex: requestHex,
  });
}

/** Broadcasts the claim; after a failure, the next call sends what is left. */
export async function sendGiftCardClaim(
  requestHex: string,
  preflightId: string
): Promise<GiftCardClaimResult> {
  return invokeWalletCommand<GiftCardClaimResult>('send_gift_card_claim', {
    request_hex: requestHex,
    preflight_id: preflightId,
  });
}

/** Rust pays exactly the invoice amount, or `amount` when the invoice accepts any amount. */
export async function preflightGenericInvoice(
  requestHex: string,
  sourceChannelId: string,
  amount: string | null
): Promise<GenericInvoicePreflightResult> {
  return invokeWalletCommand<GenericInvoicePreflightResult>('preflight_generic_invoice', {
    request_hex: requestHex,
    source_channel_id: sourceChannelId,
    amount,
  });
}

export async function sendGenericInvoicePayment(
  requestHex: string,
  preflightId: string
): Promise<GenericRequestTransactionResult> {
  return invokeWalletCommand<GenericRequestTransactionResult>('send_generic_invoice_payment', {
    request_hex: requestHex,
    preflight_id: preflightId,
  });
}

/**
 * Delivers the signed response by POST. A legacy login's webhook takes
 * `bodyJson`, the response object as JSON; Rust checks it against the
 * response.
 */
export async function postGenericResponseCallback(
  requestHex: string,
  callbackUri: string,
  responseHex: string,
  sessionId: string,
  bodyJson: string | null = null
): Promise<void> {
  return invokeWalletCommand<void>('post_generic_response_callback', {
    request_hex: requestHex,
    callback_uri: callbackUri,
    response_hex: responseHex,
    session_id: sessionId,
    body_json: bodyJson,
  });
}

/**
 * Rust sends the signed provisioning request to the open request's
 * provisioning address and returns the service's JSON answer. A webview fetch
 * would need the service to allow the wallet's origin (CORS).
 */
export async function postProvisioningRequest(
  requestHex: string,
  webhookUrl: string,
  bodyJson: string
): Promise<string> {
  return invokeWalletCommand<string>('post_provisioning_request', {
    request_hex: requestHex,
    webhook_url: webhookUrl,
    body_json: bodyJson,
  });
}

/**
 * Runs the approved stages of the open request with auto-lock paused (ADR
 * 0001). Rust caps the pause at five minutes; it ends when the stages stop.
 */
export async function withAutoLockPaused<T>(
  requestHex: string,
  stages: () => Promise<T>
): Promise<T> {
  const paused = await invokeWalletCommand<void>('pause_auto_lock_for_request', {
    request_hex: requestHex,
  }).then(
    () => true,
    () => false
  );
  try {
    return await stages();
  } finally {
    if (paused) await invokeWalletCommand<void>('resume_auto_lock').catch(() => {});
  }
}

export async function openGenericRequestCallback(
  requestHex: string,
  callbackUri: string,
  responseHex: string,
  sessionId: string
): Promise<void> {
  return invokeWalletCommand<void>('open_generic_request_callback', {
    request_hex: requestHex,
    callback_uri: callbackUri,
    response_hex: responseHex,
    session_id: sessionId,
  });
}

export async function prepareProvisioningSignature(
  requestHex: string,
  challengeHex: string,
  signingAddress: string,
  systemId: string
): Promise<string> {
  return invokeWalletCommand<string>('prepare_provisioning_signature', {
    request_hex: requestHex,
    challenge_hex: challengeHex,
    signing_address: signingAddress,
    system_id: systemId,
  });
}

export async function confirmProvisioningSignature(signingChallengeId: string): Promise<string> {
  return invokeWalletCommand<string>('confirm_provisioning_signature', {
    signing_challenge_id: signingChallengeId,
  });
}

export async function verifyIdentitySignatureHash(
  hashHex: string,
  signatureBase64: string,
  signerIdentityId: string,
  signerSystemId: string
): Promise<boolean> {
  return invokeWalletCommand<boolean>('verify_identity_signature_hash', {
    hash_hex: hashHex,
    signature_base64: signatureBase64,
    signer_identity_id: signerIdentityId,
    signer_system_id: signerSystemId,
  });
}

/**
 * Rust takes the target, expiry, and every control field from the open
 * request. The identity JSON supplies only content values under the keys the
 * request names.
 */
export async function preflightGenericIdentityUpdate(
  requestHex: string,
  requestedIdentityJson: Record<string, unknown>,
  sourceChannelId: string
): Promise<GenericIdentityUpdatePreflightResult> {
  return invokeWalletCommand<GenericIdentityUpdatePreflightResult>(
    'preflight_generic_identity_update',
    {
      request_hex: requestHex,
      requested_identity_json: requestedIdentityJson,
      source_channel_id: sourceChannelId,
    }
  );
}

export async function reviewGenericIdentityUpdate(
  requestHex: string,
  requestedIdentityJson: Record<string, unknown>
): Promise<GenericIdentityUpdateReviewResult> {
  return invokeWalletCommand<GenericIdentityUpdateReviewResult>('review_generic_identity_update', {
    request_hex: requestHex,
    requested_identity_json: requestedIdentityJson,
  });
}

export async function listIdentityProvisioningJobs(): Promise<ProvisioningJobRecord[]> {
  return invokeWalletCommand<ProvisioningJobRecord[]>('list_identity_provisioning_jobs');
}

export async function storeGenericProvisioningJob(
  request: StoreGenericProvisioningJobRequest
): Promise<ProvisioningJobRecord> {
  return invokeWalletCommand<ProvisioningJobRecord>('store_generic_provisioning_job', {
    request,
  });
}

export async function refreshIdentityProvisioningJobs(): Promise<ProvisioningJobRecord[]> {
  return invokeWalletCommand<ProvisioningJobRecord[]>('refresh_identity_provisioning_jobs');
}

export async function linkReadyIdentityProvisioning(
  jobId: string
): Promise<LinkReadyProvisioningJobResult> {
  const result = await invokeWalletCommand<LinkReadyProvisioningJobResult>(
    'link_ready_identity_provisioning',
    {
      job_id: jobId,
    }
  );
  invalidateWalletDisplayScopes();
  invalidateWalletDisplayHistory();
  return result;
}
