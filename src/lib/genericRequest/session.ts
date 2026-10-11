import { Buffer } from 'buffer';
import {
  APP_ENCRYPTION_REQUEST_VDXF_KEY,
  AUTHENTICATION_REQUEST_VDXF_KEY,
  AuthenticationRequestOrdinalVDXFObject,
  CREATE_WALLET_BACKUP_DETAILS_VDXF_KEY,
  DATA_PACKET_REQUEST_VDXF_KEY,
  DataDescriptor,
  DataPacketRequestOrdinalVDXFObject,
  GenericRequest,
  IDENTITY_UPDATE_REQUEST_VDXF_KEY,
  IdentityUpdateRequestOrdinalVDXFObject,
  PROVISION_IDENTITY_DETAILS_VDXF_KEY,
  ProvisionIdentityDetailsOrdinalVDXFObject,
  ResponseURI,
  SPENDABLE_KEY_DETAILS_VDXF_KEY,
  USER_DATA_REQUEST_VDXF_KEY,
  VERUSPAY_INVOICE_DETAILS_VDXF_KEY,
  VerusPayInvoiceDetailsOrdinalVDXFObject,
} from 'verus-typescript-primitives';
import { sha256 } from '@noble/hashes/sha2.js';
import { bytesToHex, hexToBytes } from '$lib/utils/bytes.js';
import { rejectionError } from './rejections';

export type GenericRequestDetailType =
  'authentication' | 'identityUpdate' | 'provisionIdentity' | 'invoice';

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

/**
 * Pay invoice. Rust reads the amount, currency, and destination from the
 * verified request (`review_generic_invoice`); the session only marks the step.
 */
export type InvoiceDetailSession = {
  index: number;
  type: 'invoice';
  acceptsAnyAmount: boolean;
};

/** One thing a data packet asks the user to sign, as the review shows it. */
export type SignableItemView = {
  kind: 'text' | 'data';
  label: string | null;
  mimeType: string | null;
  /** The text of a message, or of data that is valid UTF-8. */
  text: string | null;
  hex: string;
  size: number;
  /** SHA-256 of the signed bytes; Rust signs the same hash. */
  sha256: string;
};

/** Sign data: Rust signs each item and statement from the verified request. */
export type DataPacketDetailSession = {
  index: number;
  type: 'dataPacket';
  items: SignableItemView[];
  statements: string[];
};

export type SupportedGenericRequestDetail =
  | AuthenticationDetailSession
  | ProvisionIdentityDetailSession
  | IdentityUpdateDetailSession
  | InvoiceDetailSession
  | DataPacketDetailSession;

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

/** Every request detail type that Verus Mobile knows. */
const REQUEST_DETAIL_KEYS = new Set([
  AUTHENTICATION_REQUEST_VDXF_KEY.vdxfid,
  IDENTITY_UPDATE_REQUEST_VDXF_KEY.vdxfid,
  PROVISION_IDENTITY_DETAILS_VDXF_KEY.vdxfid,
  VERUSPAY_INVOICE_DETAILS_VDXF_KEY.vdxfid,
  APP_ENCRYPTION_REQUEST_VDXF_KEY.vdxfid,
  DATA_PACKET_REQUEST_VDXF_KEY.vdxfid,
  USER_DATA_REQUEST_VDXF_KEY.vdxfid,
  SPENDABLE_KEY_DETAILS_VDXF_KEY.vdxfid,
  CREATE_WALLET_BACKUP_DETAILS_VDXF_KEY.vdxfid,
]);

/** The detail types the desktop flow can show today. */
const SUPPORTED_DETAIL_KEYS = new Set([
  DATA_PACKET_REQUEST_VDXF_KEY.vdxfid,
  VERUSPAY_INVOICE_DETAILS_VDXF_KEY.vdxfid,
  AUTHENTICATION_REQUEST_VDXF_KEY.vdxfid,
  IDENTITY_UPDATE_REQUEST_VDXF_KEY.vdxfid,
  PROVISION_IDENTITY_DETAILS_VDXF_KEY.vdxfid,
]);

type ParsedRequestEnvelope = {
  request: GenericRequest;
  requestHex: string;
};

function asBuffer(bytes: Uint8Array): Buffer {
  return Buffer.from(bytes);
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

/** Verus Mobile refuses deep links of this length or longer. */
export const MAX_REQUEST_LINK_LENGTH = 2046;

function exactRequest(bytes: Buffer): ParsedRequestEnvelope {
  const request = new GenericRequest();
  const consumed = request.fromBuffer(bytes, 0);
  // Do not let trailing or noncanonical bytes disappear before Rust verifies them.
  if (consumed !== bytes.length || !bytes.equals(request.toBuffer())) {
    throw new Error('genericRequest.import.error.invalid');
  }
  return { request, requestHex: bytesToHex(bytes) };
}

/**
 * Paste and OS links accept a `verus://` link, the raw QR text (the request
 * in base64url), or hex.
 */
function parseRequestEnvelope(input: string): ParsedRequestEnvelope {
  const trimmed = input.trim();
  if (!trimmed) {
    throw new Error('genericRequest.import.error.empty');
  }

  if (trimmed.includes('://') || trimmed.startsWith('verus:')) {
    if (trimmed.length >= MAX_REQUEST_LINK_LENGTH) throw rejectionError('tooLong');
    // Validates the scheme and the protocol version.
    GenericRequest.fromWalletDeeplinkUri(trimmed);
    const encoded = trimmed.split('/')[3];
    return exactRequest(Buffer.from(encoded.replace(/-/g, '+').replace(/_/g, '/'), 'base64'));
  }

  const normalizedHex = normalizeHex(trimmed);
  if (normalizedHex) {
    return exactRequest(asBuffer(hexToBytes(normalizedHex)));
  }

  if (/^[A-Za-z0-9_-]+$/.test(trimmed)) {
    return exactRequest(Buffer.from(trimmed.replace(/-/g, '+').replace(/_/g, '/'), 'base64'));
  }

  throw new Error('genericRequest.import.error.invalid');
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

function utf8Text(bytes: Buffer): string | null {
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    return null;
  }
}

function signableItem(object: string | DataDescriptor): SignableItemView {
  const bytes =
    typeof object === 'string' ? Buffer.from(object, 'utf8') : Buffer.from(object.toBuffer());
  const base = { hex: '', size: 0, sha256: bytesToHex(sha256(bytes)) };
  if (typeof object === 'string') {
    return {
      ...base,
      kind: 'text',
      label: null,
      mimeType: null,
      text: object,
      hex: bytesToHex(bytes),
      size: bytes.length,
    };
  }
  const data = Buffer.from(object.objectdata ?? []);
  return {
    ...base,
    kind: 'data',
    label: object.label?.trim() || null,
    mimeType: object.mimeType?.trim() || null,
    text: utf8Text(data),
    hex: bytesToHex(data),
    size: data.length,
  };
}

function parseDataPacketDetail(
  detail: InstanceType<typeof DataPacketRequestOrdinalVDXFObject>,
  index: number
): DataPacketDetailSession {
  return {
    index,
    type: 'dataPacket',
    items: detail.data.signableObjects.map(signableItem),
    statements: detail.data.hasStatements() ? [...(detail.data.statements ?? [])] : [],
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

export function parseGenericRequestSession(
  input: string,
  passthroughAutoLinkFqn: string | null = null
): GenericRequestFlowSession {
  // Rust decides whether the request is acceptable (order, signature, network,
  // expiry). This parse is for review, and it stops at detail types the
  // desktop flow cannot show yet.
  const { request, requestHex } = parseRequestEnvelope(input);
  for (const detail of request.details) {
    const key = detail.getIAddressKey();
    if (SUPPORTED_DETAIL_KEYS.has(key)) continue;
    if (key === CREATE_WALLET_BACKUP_DETAILS_VDXF_KEY.vdxfid) throw rejectionError('walletBackup');
    throw rejectionError(REQUEST_DETAIL_KEYS.has(key) ? 'unsupportedDetail' : 'unknownDetail');
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
    if (detail instanceof DataPacketRequestOrdinalVDXFObject) {
      return parseDataPacketDetail(detail, index);
    }
    if (detail instanceof VerusPayInvoiceDetailsOrdinalVDXFObject) {
      return {
        index,
        type: 'invoice' as const,
        acceptsAnyAmount: detail.data.acceptsAnyAmount(),
      };
    }

    throw rejectionError('unsupportedDetail');
  });
  // One Sign data step per request: a second data packet is not shown yet.
  if (details.filter((detail) => detail.type === 'dataPacket').length > 1) {
    throw rejectionError('unsupportedDetail');
  }

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
