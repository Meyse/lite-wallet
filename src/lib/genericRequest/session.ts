import type { Buffer } from 'buffer';
import {
  APP_ENCRYPTION_REQUEST_VDXF_KEY,
  AUTHENTICATION_REQUEST_VDXF_KEY,
  AuthenticationRequestOrdinalVDXFObject,
  DATA_PACKET_REQUEST_VDXF_KEY,
  GenericRequest,
  IDENTITY_UPDATE_REQUEST_VDXF_KEY,
  IdentityUpdateRequestOrdinalVDXFObject,
  PROVISION_IDENTITY_DETAILS_VDXF_KEY,
  ProvisionIdentityDetailsOrdinalVDXFObject,
  ResponseURI,
  USER_DATA_REQUEST_VDXF_KEY,
  VERUSPAY_INVOICE_DETAILS_VDXF_KEY,
} from 'verus-typescript-primitives';
import { bytesToHex, hexToBytes } from '$lib/utils/bytes.js';

export type GenericRequestDetailType = 'authentication' | 'identityUpdate' | 'provisionIdentity';

export type GenericRequestDeliveryMode = 'post' | 'redirect' | 'unknown';

export type GenericRequestResponseUri = {
  mode: GenericRequestDeliveryMode;
  uri: string;
};

export type GenericRequestSigner = {
  systemId: string | null;
  identityId: string | null;
  isSigned: boolean;
};

export type AuthenticationConstraint = {
  type: number;
  identity: string;
};

export type AuthenticationDetailSession = {
  index: number;
  type: 'authentication';
  requestId: string | null;
  expiryTime: number | null;
  constraints: AuthenticationConstraint[];
  raw: InstanceType<typeof AuthenticationRequestOrdinalVDXFObject>;
};

export type ProvisionIdentityDetailSession = {
  index: number;
  type: 'provisionIdentity';
  requestedFqn: string | null;
  systemId: string | null;
  parent: string | null;
  webhook: string | null;
  requestId: string | null;
  raw: InstanceType<typeof ProvisionIdentityDetailsOrdinalVDXFObject>;
};

export type IdentityUpdateDetailSession = {
  index: number;
  type: 'identityUpdate';
  requestId: string | null;
  targetIdentityAddress: string;
  requestedIdentityJson: Record<string, unknown>;
  sourceSystemId: string | null;
  expiryHeight: number | null;
  raw: InstanceType<typeof IdentityUpdateRequestOrdinalVDXFObject>;
};

export type SupportedGenericRequestDetail =
  | AuthenticationDetailSession
  | ProvisionIdentityDetailSession
  | IdentityUpdateDetailSession;

export type GenericRequestFlowSession = {
  request: GenericRequest;
  requestHex: string;
  requestId: string | null;
  testnet: boolean;
  responseUris: GenericRequestResponseUri[];
  signer: GenericRequestSigner;
  details: SupportedGenericRequestDetail[];
  passthroughAutoLinkFqn: string | null;
};

const SUPPORTED_DETAIL_KEYS = new Set([
  AUTHENTICATION_REQUEST_VDXF_KEY.vdxfid,
  IDENTITY_UPDATE_REQUEST_VDXF_KEY.vdxfid,
  PROVISION_IDENTITY_DETAILS_VDXF_KEY.vdxfid,
]);

type ParsedRequestEnvelope = {
  request: GenericRequest;
  requestHex: string;
};

function asBuffer(bytes: Uint8Array): Buffer {
  return bytes as unknown as Buffer;
}

function normalizeHex(input: string): string | null {
  const trimmed = input.trim();
  if (!trimmed) return null;

  const candidate = trimmed.startsWith('0x') ? trimmed.slice(2) : trimmed;
  if (!/^[0-9a-fA-F]+$/.test(candidate) || candidate.length % 2 !== 0) {
    return null;
  }

  return candidate.toLowerCase();
}

function parseRequestEnvelope(input: string): ParsedRequestEnvelope {
  const trimmed = input.trim();
  if (!trimmed) {
    throw new Error('genericRequest.import.error.empty');
  }

  if (trimmed.includes('://') || trimmed.startsWith('verus:')) {
    const request = GenericRequest.fromWalletDeeplinkUri(trimmed);
    return {
      request,
      requestHex: bytesToHex(request.toBuffer()),
    };
  }

  const normalizedHex = normalizeHex(trimmed);
  if (!normalizedHex) {
    throw new Error('genericRequest.import.error.invalid');
  }

  const request = new GenericRequest();
  request.fromBuffer(asBuffer(hexToBytes(normalizedHex)), 0);
  return { request, requestHex: normalizedHex };
}

function responseUriMode(uri: ResponseURI): GenericRequestDeliveryMode {
  const stringType = uri.type?.toString?.(10);
  if (stringType === ResponseURI.TYPE_POST.toString(10)) return 'post';
  if (stringType === ResponseURI.TYPE_REDIRECT.toString(10)) return 'redirect';
  return 'unknown';
}

function parseAuthenticationDetail(
  detail: InstanceType<typeof AuthenticationRequestOrdinalVDXFObject>,
  index: number
): AuthenticationDetailSession {
  const requestId = detail.data.hasRequestID()
    ? (detail.data.requestID?.toAddress() ?? null)
    : null;
  const expiryTime = detail.data.hasExpiryTime()
    ? (detail.data.expiryTime?.toNumber() ?? null)
    : null;
  const constraints =
    detail.data.recipientConstraints?.map((constraint) => ({
      type: constraint.type,
      identity: constraint.identity.toAddress(),
    })) ?? [];

  return {
    index,
    type: 'authentication',
    requestId,
    expiryTime,
    constraints,
    raw: detail,
  };
}

function parseProvisionIdentityDetail(
  detail: InstanceType<typeof ProvisionIdentityDetailsOrdinalVDXFObject>,
  index: number
): ProvisionIdentityDetailSession {
  const possibleUri = (detail.data as { uri?: { getUriString?: () => string } }).uri;

  return {
    index,
    type: 'provisionIdentity',
    requestedFqn: detail.data.identityID?.toAddress?.() ?? null,
    systemId: detail.data.hasSystemId() ? (detail.data.systemID?.toAddress?.() ?? null) : null,
    parent: detail.data.hasParentId() ? (detail.data.parentID?.toAddress?.() ?? null) : null,
    webhook: typeof possibleUri?.getUriString === 'function' ? possibleUri.getUriString() : null,
    requestId: null,
    raw: detail,
  };
}

function parseIdentityUpdateDetail(
  detail: InstanceType<typeof IdentityUpdateRequestOrdinalVDXFObject>,
  index: number,
  isTestnet: boolean
): IdentityUpdateDetailSession {
  return {
    index,
    type: 'identityUpdate',
    requestId: detail.data.containsRequestID()
      ? (detail.data.requestID?.toAddress() ?? null)
      : null,
    targetIdentityAddress: detail.data.getIdentityAddress(isTestnet),
    requestedIdentityJson: detail.data.toCLIJson() as Record<string, unknown>,
    sourceSystemId: detail.data.containsSystem()
      ? (detail.data.systemID?.toAddress?.() ?? null)
      : null,
    expiryHeight: detail.data.expires() ? (detail.data.expiryHeight?.toNumber() ?? null) : null,
    raw: detail,
  };
}

function validateSupportedGrouping(details: SupportedGenericRequestDetail[]): void {
  const authCount = details.filter((detail) => detail.type === 'authentication').length;
  const updateCount = details.filter((detail) => detail.type === 'identityUpdate').length;
  const provisioningCount = details.filter((detail) => detail.type === 'provisionIdentity').length;

  if (authCount > 1 || updateCount > 1 || provisioningCount > 1) {
    throw new Error('genericRequest.unsupported.invalidGrouping');
  }

  if (details.length === 0) {
    throw new Error('genericRequest.unsupported.empty');
  }

  if (details.some((detail) => detail.type === 'provisionIdentity') && authCount === 0) {
    throw new Error('genericRequest.unsupported.provisioningNeedsAuth');
  }

  const supportedCombinations = new Set([
    'authentication',
    'authentication+identityUpdate',
    'authentication+provisionIdentity',
    'authentication+identityUpdate+provisionIdentity',
    'identityUpdate',
  ]);

  const combination = details
    .map((detail) => detail.type)
    .sort()
    .join('+');

  if (!supportedCombinations.has(combination)) {
    throw new Error('genericRequest.unsupported.invalidGrouping');
  }
}

export function parseGenericRequestSession(
  input: string,
  passthroughAutoLinkFqn: string | null = null
): GenericRequestFlowSession {
  const { request, requestHex } = parseRequestEnvelope(input);
  if (!request.isSigned()) {
    throw new Error('genericRequest.unsupported.unsigned');
  }

  const unsupportedDetail = request.details.find(
    (detail) => !SUPPORTED_DETAIL_KEYS.has(detail.getIAddressKey())
  );
  if (unsupportedDetail) {
    const key = unsupportedDetail.getIAddressKey();
    if (
      key === VERUSPAY_INVOICE_DETAILS_VDXF_KEY.vdxfid ||
      key === APP_ENCRYPTION_REQUEST_VDXF_KEY.vdxfid ||
      key === DATA_PACKET_REQUEST_VDXF_KEY.vdxfid ||
      key === USER_DATA_REQUEST_VDXF_KEY.vdxfid
    ) {
      throw new Error('genericRequest.unsupported.detail');
    }

    throw new Error('genericRequest.unsupported.detail');
  }

  const details = request.details.map((detail, index) => {
    if (detail instanceof AuthenticationRequestOrdinalVDXFObject) {
      return parseAuthenticationDetail(detail, index);
    }
    if (detail instanceof IdentityUpdateRequestOrdinalVDXFObject) {
      return parseIdentityUpdateDetail(detail, index, request.isTestnet());
    }
    if (detail instanceof ProvisionIdentityDetailsOrdinalVDXFObject) {
      return parseProvisionIdentityDetail(detail, index);
    }

    throw new Error('genericRequest.unsupported.detail');
  });

  validateSupportedGrouping(details);

  return {
    request,
    requestHex,
    requestId: request.requestID?.toAddress?.() ?? null,
    testnet: request.isTestnet(),
    responseUris:
      request.responseURIs?.map((uri) => ({
        mode: responseUriMode(uri),
        uri: uri.getUriString(),
      })) ?? [],
    signer: {
      isSigned: request.isSigned(),
      systemId: request.signature?.systemID?.toIAddress?.() ?? null,
      identityId: request.signature?.identityID?.toIAddress?.() ?? null,
    },
    details,
    passthroughAutoLinkFqn: passthroughAutoLinkFqn?.trim() || null,
  };
}
