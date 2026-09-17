import { createHash as nodeCreateHash } from 'node:crypto';
import { Buffer } from 'node:buffer';
import { describe, expect, it } from 'vitest';
import createHash from './create-hash.cjs';

describe('browser hash adapter', () => {
  for (const algorithm of ['sha256', 'ripemd160', 'rmd160']) {
    const nodeAlgorithm = algorithm === 'rmd160' ? 'ripemd160' : algorithm;

    it(`matches independent ${algorithm} digests across encodings and chunk boundaries`, () => {
      const data = Buffer.from('Verus identity · café 🔑');
      const expected = nodeCreateHash(nodeAlgorithm).update(data).digest();
      const digest = createHash(algorithm)
        .update(data.subarray(0, 7))
        .update(data.subarray(7))
        .digest();
      expect(Buffer.isBuffer(digest)).toBe(true);
      if (!Buffer.isBuffer(digest)) throw new Error('Hash adapter must return a Buffer');
      expect(digest.equals(expected)).toBe(true);
      /** @type {BufferEncoding[]} */
      const encodings = ['hex', 'base64'];
      for (const encoding of encodings) {
        expect(createHash(algorithm).update(data.toString('hex'), 'hex').digest(encoding)).toBe(
          expected.toString(encoding)
        );
      }
      expect(createHash(algorithm).update(data.toString('utf8')).digest('hex')).toBe(
        expected.toString('hex')
      );
    });

    it(`hashes empty input and only the selected view bytes with ${algorithm}`, () => {
      expect(createHash(algorithm).digest('hex')).toBe(nodeCreateHash(nodeAlgorithm).digest('hex'));
      const bytes = new Uint8Array([99, 1, 2, 3, 88]);
      const expected = nodeCreateHash(nodeAlgorithm).update(bytes.subarray(1, 4)).digest('hex');
      expect(createHash(algorithm).update(bytes.subarray(1, 4)).digest('hex')).toBe(expected);
      expect(
        createHash(algorithm)
          .update(new DataView(bytes.buffer, 1, 3))
          .digest('hex')
      ).toBe(expected);
      expect(createHash(algorithm).update(bytes.buffer.slice(1, 4)).digest('hex')).toBe(expected);
    });
  }

  it('keeps the named CommonJS export and rejects unsupported algorithms', () => {
    expect(createHash.createHash).toBe(createHash);
    expect(() => createHash('sha1')).toThrow('Unsupported hash algorithm');
  });
});
