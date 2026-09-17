// Noble v2 is ESM-only. The pinned Node runtime supports require(ESM), and
// Vite bundles these imports for the webview's CommonJS crypto consumers.
const { ripemd160 } = require('@noble/hashes/legacy.js');
const { sha256 } = require('@noble/hashes/sha2.js');
const { Buffer } = require('buffer');

/**
 * @param {string | ArrayBuffer | ArrayBufferView} value
 * @param {BufferEncoding} [encoding]
 */
function toBuffer(value, encoding) {
  if (typeof value === 'string') {
    return Buffer.from(value, encoding);
  }

  if (value instanceof ArrayBuffer) {
    return Buffer.from(value);
  }

  return Buffer.from(value.buffer, value.byteOffset, value.byteLength);
}

class BrowserHash {
  /** @param {string} algorithm */
  constructor(algorithm) {
    const normalized = String(algorithm).toLowerCase();
    if (normalized !== 'sha256' && normalized !== 'ripemd160' && normalized !== 'rmd160') {
      throw new Error(`Unsupported hash algorithm: ${algorithm}`);
    }

    this.algorithm = normalized === 'rmd160' ? 'ripemd160' : normalized;
    /** @type {Buffer[]} */
    this.chunks = [];
  }

  /**
   * @param {string | ArrayBuffer | ArrayBufferView} value
   * @param {BufferEncoding} [encoding]
   */
  update(value, encoding) {
    this.chunks.push(toBuffer(value, encoding));
    return this;
  }

  /** @param {BufferEncoding} [encoding] */
  digest(encoding) {
    const input =
      this.chunks.length === 0
        ? Buffer.alloc(0)
        : this.chunks.length === 1
          ? this.chunks[0]
          : Buffer.concat(this.chunks);

    this.chunks = [];

    const output =
      this.algorithm === 'sha256' ? Buffer.from(sha256(input)) : Buffer.from(ripemd160(input));

    return encoding ? output.toString(encoding) : output;
  }
}

/** @param {string} algorithm */
function createHash(algorithm) {
  return new BrowserHash(algorithm);
}

module.exports = createHash;
module.exports.createHash = createHash;
