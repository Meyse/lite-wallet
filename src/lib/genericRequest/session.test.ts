import { Buffer } from 'buffer';
import { describe, expect, it, vi } from 'vitest';
import { GenericRequest } from 'verus-typescript-primitives';
import fixtures from './fixtures/legacy-requests.json';
import vectors from './fixtures/request-vectors.json';
import { parseGenericRequestSession } from './session';

// External CommonJS dependencies use Node's Buffer under Vitest. Production
// Vite bundles both sides with the buffer alias; verify that separately.
vi.mock('buffer', () => import('node:buffer'));

// Captured with primitives 30c9518 before the upgrade. Group fixtures carry a
// synthetic signature marker, not a valid signature: they test parsing only.
// The separate Rust vector has independently fixed bytes and signature hashes.
describe('persisted request compatibility', () => {
  it('agrees with the Rust generic-request vector', () => {
    const { request } = parseGenericRequestSession(fixtures.rustSignedRequest);
    if (!request) throw new Error('The Rust vector is a GenericRequest');
    expect(request.toBuffer().toString('hex')).toBe(fixtures.rustSignedRequest);
    expect(request.getRawDataSha256().toString('hex')).toBe(
      'eebf8a0602c116bd35d14319883dcb547a493df882b734b509fe5f86b851edd7'
    );
    expect(request.getDetailsIdentitySignatureHash(965771).toString('hex')).toBe(
      'a07b699e8c927fbc1dc680e9d1ed61ed139c70efa48b2839f0522e7b21fb46ef'
    );
  });

  for (const { types, hex } of fixtures.groups) {
    it(`imports persisted ${types} through hex and a deep link repeatedly`, () => {
      const uri = `verus://1/${Buffer.from(hex, 'hex').toString('base64url')}`;
      for (const input of [hex, uri, hex, uri]) {
        const session = parseGenericRequestSession(input);
        expect(session.details.map((detail) => detail.type).join('+')).toBe(types);
        expect(session.requestHex).toBe(hex);
        expect(session.testnet).toBe(true);
      }
    });
  }

  it('accepts the raw QR text that websites offer to copy', () => {
    const hex = fixtures.rustSignedRequest;
    const qrText = Buffer.from(hex, 'hex').toString('base64url');
    expect(parseGenericRequestSession(qrText).requestHex).toBe(hex);
  });

  it('refuses links of 2046 characters or more, as Verus Mobile does', () => {
    const link = `verus://1/${Buffer.from(fixtures.rustSignedRequest, 'hex').toString('base64url')}`;
    const padded = `${link}${'A'.repeat(2046 - link.length)}`;
    expect(padded).toHaveLength(2046);
    expect(() => parseGenericRequestSession(padded)).toThrow('genericRequest.rejected.tooLong');
  });

  it('uses fresh parsers after failed imports', () => {
    for (const input of ['', 'xx', fixtures.rustSignedRequest.slice(0, -20)]) {
      expect(() => parseGenericRequestSession(input)).toThrow();
      expect(parseGenericRequestSession(fixtures.rustSignedRequest).details[0].type).toBe(
        'authentication'
      );
    }
  });

  it('keeps a parsed request separate from later attempts', () => {
    const first = parseGenericRequestSession(fixtures.rustSignedRequest);
    const second = parseGenericRequestSession(fixtures.rustSignedRequest);
    expect(first.request).not.toBe(second.request);
    expect(first.request).toBeInstanceOf(GenericRequest);
  });

  it('rejects trailing, truncated, noncanonical and oversized declared payloads', () => {
    const hex = fixtures.rustSignedRequest;
    const malformed = [
      `${hex}00`,
      hex.slice(0, -2),
      `fd0100${hex.slice(2)}`, // CompactSize version 1 encoded noncanonically.
      `ffffffffffffffffff${hex.slice(2)}`, // Unsafe integer / oversized length.
    ];
    for (const input of malformed) {
      expect(() => parseGenericRequestSession(input)).toThrow();
      const uri = `verus://1/${Buffer.from(input, 'hex').toString('base64url')}`;
      expect(() => parseGenericRequestSession(uri)).toThrow();
    }
  });
});

describe('share data requests', () => {
  const vector = (name: string) => {
    const found = vectors.cases.find((entry) => entry.name === name);
    if (!found) throw new Error(`Missing vector ${name}`);
    return found;
  };

  it('reads the requested credential keys and the identity that must answer', () => {
    const { details } = parseGenericRequestSession(vector('auth-credential').hex);
    const expected = vector('auth-credential').expect.userData;
    if (!expected) throw new Error('The vector names its credential keys');
    expect(details[1]).toEqual({
      index: 1,
      type: 'userData',
      keys: expected.keys,
      requiredIdentity: expected.signer,
    });
  });

  it('refuses a second credential request in one request', () => {
    const request = new GenericRequest();
    request.fromBuffer(Buffer.from(vector('auth-credential').hex, 'hex'), 0);
    request.details = [...request.details, request.details[1]];
    request.setFlags();
    expect(() => parseGenericRequestSession(request.toBuffer().toString('hex'))).toThrow(
      'genericRequest.rejected.unsupportedDetail'
    );
  });
});

describe('legacy links', () => {
  const legacy = vectors.legacy;

  it('opens a legacy login as a sign in that answers to its redirect address', () => {
    const session = parseGenericRequestSession(legacy.login.link);
    expect(session.legacy).toBe('login');
    expect(session.requestHex).toBe(legacy.login.link);
    expect(session.signer).toEqual({
      isSigned: true,
      systemId: 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq',
      identityId: legacy.login.signingId,
    });
    expect(session.details).toEqual([
      expect.objectContaining({ type: 'authentication', requestId: legacy.login.challengeId }),
    ]);
    // A redirect address wins over a webhook, as on mobile.
    expect(session.responseUris).toEqual([{ mode: 'redirect', uri: legacy.login.redirect }]);
  });

  it('offers the new VerusID a legacy login names, from the first entry of each key', () => {
    const offer = legacy.loginProvisioning;
    const session = parseGenericRequestSession(offer.link);
    expect(session.details).toEqual([
      expect.objectContaining({ type: 'authentication', requestId: legacy.login.challengeId }),
      {
        index: 1,
        type: 'provisionIdentity',
        requestedFqn: offer.fullyQualifiedName,
        systemId: offer.systemId,
        parent: offer.parent,
        webhook: offer.webhook,
        requestId: legacy.login.challengeId,
        raw: null,
      },
    ]);
  });

  it('opens legacy invoices at versions 3 and 4 as an invoice', () => {
    for (const invoice of legacy.invoices) {
      const session = parseGenericRequestSession(invoice.link);
      expect(session.legacy).toBe('invoice');
      expect(session.details).toEqual([{ index: 0, type: 'invoice', acceptsAnyAmount: false }]);
      expect(session.signer.isSigned).toBe(invoice.signed);
      expect(session.responseUris).toEqual([]);
    }
  });
});
