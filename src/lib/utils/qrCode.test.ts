import jsQR from 'jsqr';
import { describe, expect, it } from 'vitest';
import { encodeQrCode, QR_QUIET_ZONE_MODULES, type QrCodeMatrix } from './qrCode';

function rasterize(matrix: QrCodeMatrix, scale = 8): ImageData {
  const width = (matrix.size + QR_QUIET_ZONE_MODULES * 2) * scale;
  const pixels = new Uint8ClampedArray(width * width * 4);
  pixels.fill(255);

  for (let row = 0; row < matrix.size; row += 1) {
    for (let column = 0; column < matrix.size; column += 1) {
      if (!matrix.modules[row * matrix.size + column]) continue;

      const startX = (column + QR_QUIET_ZONE_MODULES) * scale;
      const startY = (row + QR_QUIET_ZONE_MODULES) * scale;
      for (let y = startY; y < startY + scale; y += 1) {
        for (let x = startX; x < startX + scale; x += 1) {
          const offset = (y * width + x) * 4;
          pixels[offset] = 0;
          pixels[offset + 1] = 0;
          pixels[offset + 2] = 0;
          pixels[offset + 3] = 255;
        }
      }
    }
  }

  return { data: pixels, width, height: width, colorSpace: 'srgb' };
}

describe('QR code encoding', () => {
  it('round-trips the exact synthetic private-key payload with a four-module quiet zone', async () => {
    const payload = 'synthetic-private-key-for-tests-only-7f1a9c2e4b6d8f0a';
    const encoded = await encodeQrCode(payload);
    const image = rasterize(encoded);
    const decoded = jsQR(image.data, image.width, image.height, {
      inversionAttempts: 'dontInvert',
    });

    expect(decoded?.data).toBe(payload);
    expect(encoded.quietZone).toBe(4);
    expect(encoded.viewBoxSize).toBe(encoded.size + 8);
    expect(encoded.path).toContain('M4 4');
  });
});
