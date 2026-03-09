export function hexToBytes(hex: string): Uint8Array {
  const normalized = hex.trim().replace(/^0x/i, '');
  if (normalized.length === 0 || normalized.length % 2 !== 0 || !/^[\da-f]+$/i.test(normalized)) {
    throw new Error('Invalid hex string');
  }

  const bytes = new Uint8Array(normalized.length / 2);
  for (let index = 0; index < normalized.length; index += 2) {
    bytes[index / 2] = Number.parseInt(normalized.slice(index, index + 2), 16);
  }

  return bytes;
}

export function bytesToHex(bytes: ArrayLike<number>): string {
  let output = '';
  for (let index = 0; index < bytes.length; index += 1) {
    output += bytes[index].toString(16).padStart(2, '0');
  }
  return output;
}

export function reverseBytes(bytes: ArrayLike<number>): Uint8Array {
  const reversed = Uint8Array.from(bytes);
  reversed.reverse();
  return reversed;
}

export function base64ToBytes(value: string): Uint8Array {
  const normalized = value.trim().replace(/-/g, '+').replace(/_/g, '/');
  const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, '=');
  const binary = globalThis.atob(padded);
  const bytes = new Uint8Array(binary.length);

  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }

  return bytes;
}

export function bytesToBase64Url(bytes: ArrayLike<number>): string {
  let binary = '';
  const chunkSize = 0x8000;

  for (let index = 0; index < bytes.length; index += chunkSize) {
    const slice = Array.from(bytes).slice(index, index + chunkSize);
    binary += String.fromCharCode(...slice);
  }

  return globalThis
    .btoa(binary)
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/g, '');
}

export function readUint32LE(bytes: ArrayLike<number>, offset: number): number {
  if (offset < 0 || offset + 4 > bytes.length) {
    throw new Error('Offset is out of range');
  }

  return (
    bytes[offset] |
    (bytes[offset + 1] << 8) |
    (bytes[offset + 2] << 16) |
    (bytes[offset + 3] << 24)
  ) >>> 0;
}
