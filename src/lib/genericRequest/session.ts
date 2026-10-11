import { Buffer } from 'buffer';
import {
  APP_ENCRYPTION_REQUEST_VDXF_KEY,
  AppEncryptionRequestOrdinalVDXFObject,
  AUTHENTICATION_REQUEST_VDXF_KEY,
  AuthenticationRequestOrdinalVDXFObject,
  CREATE_WALLET_BACKUP_DETAILS_VDXF_KEY,
  DATA_PACKET_REQUEST_VDXF_KEY,
  DataDescriptor,
  DataPacketRequestOrdinalVDXFObject,
  GenericRequest,
  IDENTITY_UPDATE_REQUEST_VDXF_KEY,
  LOGIN_CONSENT_REDIRECT_VDXF_KEY,
  LOGIN_CONSENT_REQUEST_VDXF_KEY,
  LOGIN_CONSENT_WEBHOOK_VDXF_KEY,
  LoginConsentRequest,
  IdentityUpdateRequestOrdinalVDXFObject,
  PROVISION_IDENTITY_DETAILS_VDXF_KEY,
  ProvisionIdentityDetailsOrdinalVDXFObject,
  ResponseURI,
  SPENDABLE_KEY_DETAILS_VDXF_KEY,
  USER_DATA_REQUEST_VDXF_KEY,
  UserDataRequestOrdinalVDXFObject,
  VERUSPAY_INVOICE_DETAILS_VDXF_KEY,
  VERUSPAY_INVOICE_VDXF_KEY,
  VerusPayInvoice,
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
  /** Null for a legacy login, which has no sign-in detail. */
  raw: InstanceType<typeof AuthenticationRequestOrdinalVDXFObject> | null;
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

/** Encryption key: Rust derives the keys from the wallet's private account. */
export type AppEncryptionDetailSession = {
  index: number;
  type: 'appEncryption';
  derivationNumber: number;
  derivationId: string | null;
  returnsSpendingKey: boolean;
};

/** Share data: Rust reads the matching credentials from the responder. */
export type UserDataDetailSession = {
  index: number;
  type: 'userData';
  /** The credential keys the app asks for. */
  keys: string[];
  /** The VerusID that must answer, when the app names one. */
  requiredIdentity: string | null;
};

export type SupportedGenericRequestDetail =
  | AuthenticationDetailSession
  | ProvisionIdentityDetailSession
  | IdentityUpdateDetailSession
  | InvoiceDetailSession
  | DataPacketDetailSession
  | AppEncryptionDetailSession
  | UserDataDetailSession;

/** A legacy link that Verus Mobile still answers (`src/sagas/deeplink.js`). */
export type LegacyRequestKind = 'login' | 'invoice';

export type GenericRequestFlowSession = {
  /** Null for a legacy link. */
  request: GenericRequest | null;
  requestHex: string;
  requestId: string | null;
  testnet: boolean;
  responseUris: GenericRequestResponseUri[];
  signer: GenericRequestSigner;
  details: SupportedGenericRequestDetail[];
  passthroughAutoLinkFqn: string | null;
  /** A legacy link: a login consent request or a VerusPay invoice. */
  legacy?: LegacyRequestKind | null;
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
  APP_ENCRYPTION_REQUEST_VDXF_KEY.vdxfid,
  USER_DATA_REQUEST_VDXF_KEY.vdxfid,
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

const TESTNET_SYSTEM_ID = 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq';

/**
 * A link in the legacy form, `<scheme>://x-callback-url/<key>/...`. A
 * `verus://` link is always a request, as on mobile.
 */
export function isLegacyLink(input: string): boolean {
  const trimmed = input.trim();
  const scheme = trimmed.split('://')[0];
  return trimmed.includes('://x-callback-url/') && scheme !== 'verus' && scheme !== 'verus0';
}

/**
 * A legacy login or invoice, for review. Rust verifies the link and holds it
 * as the open request under the link itself. A legacy login answers to the
 * one address mobile uses: a redirect when the request has one, else a
 * webhook (`LoginRequestComplete.js`).
 */
function parseLegacySession(
  link: string,
  passthroughAutoLinkFqn: string | null
): GenericRequestFlowSession {
  if (link.length >= MAX_REQUEST_LINK_LENGTH) throw rejectionError('tooLong');
  const key = link.split('://')[1]?.split('/')[1]?.split('?')[0];
  const base = {
    request: null,
    requestHex: link,
    requestId: null,
    passthroughAutoLinkFqn: passthroughAutoLinkFqn?.trim() || null,
  };
  if (key === LOGIN_CONSENT_REQUEST_VDXF_KEY.vdxfid) {
    const request = LoginConsentRequest.fromWalletDeeplinkUri(link);
    const lastOf = (kind: string) =>
      [...(request.challenge.redirect_uris ?? [])].reverse().find((uri) => uri.vdxfkey === kind);
    const redirect = lastOf(LOGIN_CONSENT_REDIRECT_VDXF_KEY.vdxfid);
    const webhook = lastOf(LOGIN_CONSENT_WEBHOOK_VDXF_KEY.vdxfid);
    return {
      ...base,
      testnet: request.system_id === TESTNET_SYSTEM_ID,
      responseUris: redirect
        ? [{ mode: 'redirect', uri: redirect.uri }]
        : webhook
          ? [{ mode: 'post', uri: webhook.uri }]
          : [],
      signer: { isSigned: true, systemId: request.system_id, identityId: request.signing_id },
      details: [
        {
          index: 0,
          type: 'authentication',
          requestId: request.challenge.challenge_id ?? null,
          expiryTime: null,
          constraints: [],
          raw: null,
        },
      ],
      legacy: 'login',
    };
  }
  if (key === VERUSPAY_INVOICE_VDXF_KEY.vdxfid) {
    const invoice = VerusPayInvoice.fromWalletDeeplinkUri(link);
    const signed = invoice.isSigned();
    return {
      ...base,
      testnet: invoice.details.isTestnet(),
      responseUris: [],
      signer: {
        isSigned: signed,
        systemId: signed ? (invoice.system_id ?? null) : null,
        identityId: signed ? (invoice.signing_id ?? null) : null,
      },
      details: [
        { index: 0, type: 'invoice', acceptsAnyAmount: invoice.details.acceptsAnyAmount() },
      ],
      legacy: 'invoice',
    };
  }
  throw rejectionError('unknownDetail');
}

export function parseGenericRequestSession(
  input: string,
  passthroughAutoLinkFqn: string | null = null
): GenericRequestFlowSession {
  if (isLegacyLink(input)) return parseLegacySession(input.trim(), passthroughAutoLinkFqn);

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
    if (detail instanceof AppEncryptionRequestOrdinalVDXFObject) {
      return {
        index,
        type: 'appEncryption' as const,
        derivationNumber: detail.data.derivationNumber.toNumber(),
        derivationId: detail.data.derivationID?.toAddress?.() ?? null,
        returnsSpendingKey: detail.data.returnESK(),
      };
    }
    if (detail instanceof DataPacketRequestOrdinalVDXFObject) {
      return parseDataPacketDetail(detail, index);
    }
    if (detail instanceof UserDataRequestOrdinalVDXFObject) {
      return {
        index,
        type: 'userData' as const,
        keys: detail.data.searchDataKey.flatMap((item) => Object.keys(item)),
        requiredIdentity: detail.data.hasSigner()
          ? (detail.data.signer?.toIAddress?.() ?? null)
          : null,
      };
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
  // One Sign data and one Share data step per request: a second data packet
  // or credential request is not shown yet.
  if (
    details.filter((detail) => detail.type === 'dataPacket').length > 1 ||
    details.filter((detail) => detail.type === 'userData').length > 1
  ) {
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
