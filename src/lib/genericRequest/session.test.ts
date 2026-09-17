import { Buffer } from 'buffer';
import { describe, expect, it, vi } from 'vitest';
import { GenericRequest } from 'verus-typescript-primitives';
import fixtures from './fixtures/legacy-requests.json';
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
