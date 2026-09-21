import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  cropRectangle,
  encodeProfileCandidates,
  encodeProfileImage,
  isWebP,
  loadProfileImage,
  profileMediaUrl,
} from './profileImages';

describe('profile image geometry and source budgets', () => {
  it('crops avatar squares and 6:1 banners without stretching', () => {
    expect(cropRectangle(1200, 800, 'avatar', 1, 0.5, 0.5)).toEqual({
      x: 200,
      y: 0,
      width: 800,
      height: 800,
    });
    expect(cropRectangle(1200, 800, 'header', 1, 0.5, 0.5)).toEqual({
      x: 0,
      y: 300,
      width: 1200,
      height: 200,
    });
    expect(cropRectangle(1200, 800, 'header', 2, 1, 1)).toEqual({
      x: 600,
      y: 700,
      width: 600,
      height: 100,
    });
  });
  it('clamps crop positions and rejects unsupported/oversized uploads before decoding', async () => {
    expect(cropRectangle(1200, 800, 'avatar', 0, -5, 2).x).toBe(0);
    await expect(loadProfileImage({ type: 'image/svg+xml', size: 10 } as File)).rejects.toThrow(
      'source_type'
    );
    await expect(
      loadProfileImage({ type: 'image/jpeg', size: 10 * 1024 * 1024 + 1 } as File)
    ).rejects.toThrow('source_bytes');
  });
});

function webpBlob(size: number, mime = 'image/webp'): Blob {
  const bytes = new Uint8Array(size);
  bytes.set([82, 73, 70, 70], 0);
  new DataView(bytes.buffer).setUint32(4, size - 8, true);
  bytes.set([87, 69, 66, 80], 8);
  return new Blob([bytes], { type: mime });
}
afterEach(() => vi.unstubAllGlobals());
describe('verified WebP candidates', () => {
  it('keeps balanced quality and a smaller candidate from the same crop canvas', async () => {
    const sizes = [20_000, 12_000, 8_000, 5_000];
    const encode = vi.fn((done: (blob: Blob | null) => void, mime: string) =>
      done(webpBlob(sizes.shift() ?? 5000, mime))
    );
    const result = await encodeProfileCandidates({
      toBlob: encode,
    } as unknown as HTMLCanvasElement);
    expect(result.balanced.byteLength).toBe(12_000);
    expect(result.smaller?.byteLength).toBe(5_000);
    expect(encode.mock.calls.map((call) => call[1])).toEqual(Array(4).fill('image/webp'));
    expect(isWebP(new Uint8Array(await webpBlob(100).arrayBuffer()))).toBe(true);
  });
  it.each(['image/png', 'image/jpeg'])('rejects a silent %s fallback', async (mime) => {
    const canvas = { toBlob: (done: (blob: Blob | null) => void) => done(webpBlob(100, mime)) };
    await expect(encodeProfileCandidates(canvas as HTMLCanvasElement)).rejects.toThrow(
      'webp_unsupported'
    );
  });
  it('rejects null, spoofed and oversized results without inventing a fallback', async () => {
    for (const blob of [null, new Blob(['RIFF-spoof'], { type: 'image/webp' })]) {
      await expect(
        encodeProfileCandidates({
          toBlob: (done: (blob: Blob | null) => void) => done(blob),
        } as HTMLCanvasElement)
      ).rejects.toThrow('webp_unsupported');
    }
    await expect(
      encodeProfileCandidates({
        toBlob: (done: (blob: Blob | null) => void) => done(webpBlob(33000)),
      } as HTMLCanvasElement)
    ).rejects.toThrow('too_large');
  });
  it('flattens onto white once and encodes each candidate from those pixels', async () => {
    const context = { fillStyle: '', fillRect: vi.fn(), drawImage: vi.fn() };
    const canvas = {
      width: 0,
      height: 0,
      getContext: () => context,
      toBlob: (done: (blob: Blob | null) => void) => done(webpBlob(100)),
    };
    vi.stubGlobal('document', { createElement: () => canvas });
    const result = await encodeProfileImage(
      { width: 1200, height: 800 } as ImageBitmap,
      'header',
      1,
      0.5,
      0.5
    );
    expect(context.fillStyle).toBe('white');
    expect(context.fillRect).toHaveBeenCalledWith(0, 0, 960, 160);
    expect(context.drawImage).toHaveBeenCalledTimes(1);
    expect(result.smaller).toBeNull();
    expect(profileMediaUrl(result.balanced.base64, result.balanced.mimeType)).toMatch(
      /^data:image\/webp;/
    );
    expect(profileMediaUrl('legacy', null)).toBe('data:image/jpeg;base64,legacy');
    expect(profileMediaUrl('unsafe', 'image/svg+xml')).toBeNull();
  });
});
