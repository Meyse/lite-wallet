import { invoke } from '@tauri-apps/api/core';
import type {
  BuildAndSignGenericResponseRequest,
  BuildAndSignGenericResponseResult,
  GenericIdentityUpdatePreflightResult,
  GenericIdentityUpdateRequestMeta,
  GenericRequestVerificationResult,
  LinkReadyProvisioningJobResult,
  ProvisioningJobRecord,
  StoreGenericProvisioningJobRequest,
} from '$lib/types/wallet.js';

export async function verifyGenericRequestSignature(
  requestHex: string
): Promise<GenericRequestVerificationResult> {
  return invoke<GenericRequestVerificationResult>('verify_generic_request_signature', {
    request_hex: requestHex,
  });
}

export async function buildAndSignGenericResponse(
  request: BuildAndSignGenericResponseRequest
): Promise<BuildAndSignGenericResponseResult> {
  return invoke<BuildAndSignGenericResponseResult>('build_and_sign_generic_response', {
    request_hex: request.requestHex,
    signer: request.signer,
    authentication: request.authentication ?? null,
    identity_update: request.identityUpdate ?? null,
  });
}

export async function postGenericResponseCallback(
  callbackUri: string,
  responseHex: string
): Promise<void> {
  return invoke('post_generic_response_callback', {
    callback_uri: callbackUri,
    response_hex: responseHex,
  });
}

export async function openGenericRequestCallback(
  callbackUri: string,
  responseHex: string
): Promise<void> {
  return invoke('open_generic_request_callback', {
    callback_uri: callbackUri,
    response_hex: responseHex,
  });
}

export async function signIdentitySignatureHash(
  hashHex: string,
  systemId: string
): Promise<string> {
  return invoke<string>('sign_identity_signature_hash', {
    hash_hex: hashHex,
    system_id: systemId,
  });
}

export async function verifyIdentitySignatureHash(
  hashHex: string,
  signatureBase64: string,
  signerIdentityId: string,
  signerSystemId: string
): Promise<boolean> {
  return invoke<boolean>('verify_identity_signature_hash', {
    hash_hex: hashHex,
    signature_base64: signatureBase64,
    signer_identity_id: signerIdentityId,
    signer_system_id: signerSystemId,
  });
}

export async function preflightGenericIdentityUpdate(
  requestedIdentityJson: Record<string, unknown>,
  targetIdentityAddress: string,
  sourceChannelId: string,
  requestMeta?: GenericIdentityUpdateRequestMeta | null
): Promise<GenericIdentityUpdatePreflightResult> {
  return invoke<GenericIdentityUpdatePreflightResult>('preflight_generic_identity_update', {
    requested_identity_json: requestedIdentityJson,
    target_identity_address: targetIdentityAddress,
    source_channel_id: sourceChannelId,
    request_meta: requestMeta ?? null,
  });
}

export async function listIdentityProvisioningJobs(): Promise<ProvisioningJobRecord[]> {
  return invoke<ProvisioningJobRecord[]>('list_identity_provisioning_jobs');
}

export async function storeGenericProvisioningJob(
  request: StoreGenericProvisioningJobRequest
): Promise<ProvisioningJobRecord> {
  return invoke<ProvisioningJobRecord>('store_generic_provisioning_job', {
    request,
  });
}

export async function refreshIdentityProvisioningJobs(): Promise<ProvisioningJobRecord[]> {
  return invoke<ProvisioningJobRecord[]>('refresh_identity_provisioning_jobs');
}

export async function linkReadyIdentityProvisioning(
  jobId: string
): Promise<LinkReadyProvisioningJobResult> {
  return invoke<LinkReadyProvisioningJobResult>('link_ready_identity_provisioning', {
    job_id: jobId,
  });
}
