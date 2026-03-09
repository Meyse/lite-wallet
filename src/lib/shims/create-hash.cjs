const { ripemd160 } = require('@noble/hashes/ripemd160');
const { sha256 } = require('@noble/hashes/sha256');
const { Buffer } = require('buffer');

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
  constructor(algorithm) {
    const normalized = String(algorithm).toLowerCase();
    if (normalized !== 'sha256' && normalized !== 'ripemd160' && normalized !== 'rmd160') {
      throw new Error(`Unsupported hash algorithm: ${algorithm}`);
    }

    this.algorithm = normalized === 'rmd160' ? 'ripemd160' : normalized;
    this.chunks = [];
  }

  update(value, encoding) {
    this.chunks.push(toBuffer(value, encoding));
    return this;
  }

  digest(encoding) {
    const input =
      this.chunks.length === 0
        ? Buffer.alloc(0)
        : this.chunks.length === 1
          ? this.chunks[0]
          : Buffer.concat(this.chunks);

    this.chunks = [];

    const output =
      this.algorithm === 'sha256'
        ? Buffer.from(sha256(input))
        : Buffer.from(ripemd160(input));

    return encoding ? output.toString(encoding) : output;
  }
}

function createHash(algorithm) {
  return new BrowserHash(algorithm);
}

module.exports = createHash;
module.exports.createHash = createHash;
