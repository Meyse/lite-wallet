import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { Buffer } from 'node:buffer';
import { describe, expect, it } from 'vitest';
// @ts-expect-error The pinned Verus fork does not publish TypeScript declarations.
import utxo from '@bitgo/utxo-lib';
import {
  AuthenticationRequestOrdinalVDXFObject,
  CompactIAddressObject,
  GenericRequest,
  OrdinalVDXFObject,
  PartialIdentity,
  SignedSessionObject,
  VerifiableSignatureData,
} from 'verus-typescript-primitives';
import parity from './fixtures/signature-parity.json';

const require = createRequire(import.meta.url);
const mainnet = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
const testnet = 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq';

describe('intended upstream protocol corrections', () => {
  it('matches the shared Rust fixtures with independently encoded hash preimages', () => {
    for (const fixture of parity.cases) {
      const request = new GenericRequest();
      request.fromBuffer(Buffer.from(fixture.hex, 'hex'));
      expect(request.getRawDataSha256().toString('hex')).toBe(fixture.rawHash);
      expect(request.getDetailsIdentitySignatureHash(965771).toString('hex')).toBe(
        fixture.identityHash
      );
    }
  });
  it('rejects parser reuse including after a failed attempt', () => {
    for (const first of ['02010100', '02']) {
      const parser = new AuthenticationRequestOrdinalVDXFObject();
      try {
        parser.fromBuffer(Buffer.from(first, 'hex'));
      } catch {
        /* malformed first attempt */
      }
      expect(() => parser.fromBuffer(Buffer.from('02010100', 'hex'))).toThrow(
        'Deserialization already attempted'
      );
    }
    expect(() =>
      OrdinalVDXFObject.createFromBuffer(Buffer.from('02010500deadbeef', 'hex'))
    ).toThrow();
    expect(() => new SignedSessionObject()).toThrow();
  });

  it('distinguishes an omitted primary-address update from an explicit empty update', () => {
    expect(new PartialIdentity().containsPrimaryAddresses()).toBe(false);
    expect(new PartialIdentity({ primaryAddresses: [] }).containsPrimaryAddresses()).toBe(true);
    const update = new PartialIdentity();
    update.clearContentMultiMap();
    expect(update.containsContentMultiMap()).toBe(true);
  });

  for (const compressed of [true, false]) {
    it(`recovers the original ${compressed ? 'compressed' : 'uncompressed'} public key`, () => {
      // Public synthetic scalar 1; never use wallet material in these vectors.
      const scalar = Buffer.alloc(32);
      scalar[31] = 1;
      const pair = utxo.ECPair.fromPrivateKeyBuffer(scalar, utxo.networks.verus);
      pair.compressed = compressed;
      const hash = Buffer.alloc(32, 42);
      const sig = pair.sign(hash);
      const recovered = [];
      for (let recovery = 0; recovery < 4; recovery++) {
        try {
          recovered.push(
            utxo.ECPair.recoverFromSignature(
              hash,
              sig.toCompact(recovery, compressed),
              utxo.networks.verus
            )
              .getPublicKeyBuffer()
              .toString('hex')
          );
        } catch {
          /* not every recovery point exists */
        }
      }
      expect(recovered).toContain(pair.getPublicKeyBuffer().toString('hex'));
    });
  }

  for (const { isTestnet, system, root } of [
    { isTestnet: false, system: mainnet, root: 'VRSC' },
    { isTestnet: true, system: testnet, root: 'VRSCTEST' },
  ]) {
    for (const named of [false, true]) {
      it(`preserves ${root} context for ${named ? 'compact names' : 'addresses'} and an implicit system`, () => {
        const identityID = named
          ? CompactIAddressObject.fromFQN('alice@', root)
          : CompactIAddressObject.fromAddress(mainnet, root);
        const signature = new VerifiableSignatureData({ identityID, isTestnet });
        // Explicitly exercise the on-wire optional-system form; constructor now
        // includes the default system automatically, but older requests may omit it.
        signature.flags = signature.flags.and(VerifiableSignatureData.FLAG_HAS_SYSTEM.notn(32));
        const parsed = new VerifiableSignatureData({ isTestnet });
        parsed.fromBuffer(signature.toBuffer());
        expect(parsed.systemID.toIAddress()).toBe(system);
        expect(parsed.identityID.toIAddress()).toBe(identityID.toIAddress());
      });
    }
  }

  it('orders metadata by UTF-8 bytes as the daemon does', () => {
    const signature = new VerifiableSignatureData({
      identityID: CompactIAddressObject.fromAddress(mainnet),
      vdxfKeyNames: ['\u{10000}', '\ue000'],
    });
    // Independently encoded CompactSize vector: count 2, then UTF-8 byte lengths.
    // UTF-8 EExx sorts before F0xx; JavaScript UTF-16 sort reverses these.
    const metadata = Buffer.from('0203ee808004f0908080', 'hex');
    const systemHash = utxo.address.fromBase58Check(mainnet).hash;
    const height = Buffer.alloc(4);
    height.writeUInt32LE(965771);
    const expected = createHash('sha256')
      .update(metadata)
      .update(systemHash)
      .update(height)
      .update(systemHash)
      .update(Buffer.from([19]))
      .update(Buffer.from('Verus signed data:\n'))
      .update(Buffer.alloc(32, 42))
      .digest('hex');
    expect(signature.getIdentityHash(965771, Buffer.alloc(32, 42)).toString('hex')).toBe(expected);
  });

  it('resolves every client and BitGo peer to one primitives class identity', () => {
    const root = require.resolve('verus-typescript-primitives');
    for (const name of ['@bitgo/utxo-lib', 'verusd-rpc-ts-client', 'verusid-ts-client']) {
      expect(
        require.resolve('verus-typescript-primitives', { paths: [require.resolve(name)] })
      ).toBe(root);
    }
    expect(require('bitcoin-ops/evals.json')).toBeTruthy();
  });
});
