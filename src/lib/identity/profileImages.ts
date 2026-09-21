import { invoke, isTauri } from '@tauri-apps/api/core';
export type ProfileImageKind = 'avatar' | 'header';
export const MAX_PROFILE_SOURCE_BYTES = 10 * 1024 * 1024;
export const MAX_PROFILE_SOURCE_PIXELS = 20_000_000;
export const MAX_PROFILE_IMAGE_BYTES = 32 * 1024;
// Export and crop preview use the same opaque white matte,
// independent of the wallet's appearance preference.
export const PROFILE_IMAGE_BACKGROUND = 'white';
export const profileImageSize = (kind: ProfileImageKind): [number, number] =>
  kind === 'avatar' ? [256, 256] : [960, 160];

export function cropRectangle(
  width: number,
  height: number,
  kind: ProfileImageKind,
  zoom: number,
  x: number,
  y: number
) {
  const [outWidth, outHeight] = profileImageSize(kind);
  const scale = Math.min(width / outWidth, height / outHeight) / Math.max(1, Math.min(3, zoom));
  const cropWidth = outWidth * scale;
  const cropHeight = outHeight * scale;
  return {
    x: (width - cropWidth) * Math.max(0, Math.min(1, x)),
    y: (height - cropHeight) * Math.max(0, Math.min(1, y)),
    width: cropWidth,
    height: cropHeight,
  };
}

export async function loadProfileImage(file: File): Promise<ImageBitmap> {
  if (!['image/jpeg', 'image/png', 'image/webp'].includes(file.type))
    throw new Error('source_type');
  if (!file.size || file.size > MAX_PROFILE_SOURCE_BYTES) throw new Error('source_bytes');
  let bitmap: ImageBitmap;
  try {
    bitmap = await createImageBitmap(file);
  } catch {
    throw new Error('source_decode');
  }
  if (!bitmap.width || !bitmap.height || bitmap.width * bitmap.height > MAX_PROFILE_SOURCE_PIXELS) {
    bitmap.close();
    throw new Error('source_pixels');
  }
  return bitmap;
}

export async function encodeProfileImage(
  bitmap: ImageBitmap,
  kind: ProfileImageKind,
  zoom: number,
  x: number,
  y: number
): Promise<ProfileImageCandidates> {
  const [width, height] = profileImageSize(kind);
  const crop = cropRectangle(bitmap.width, bitmap.height, kind, zoom, x, y);
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('invalid_source');
  context.fillStyle = PROFILE_IMAGE_BACKGROUND;
  context.fillRect(0, 0, width, height);
  context.drawImage(bitmap, crop.x, crop.y, crop.width, crop.height, 0, 0, width, height);
  if (isTauri()) {
    // WKWebView can silently return PNG for WebP. Use the same native lossy
    // encoder on all desktop platforms; PNG here only transports cropped pixels.
    const png = canvas.toDataURL('image/png').split(',')[1];
    return invoke<ProfileImageCandidates>('encode_identity_profile_image', {
      kind,
      png_base64: png,
    });
  }
  return encodeProfileCandidates(canvas);
}

export async function encodeProfileCandidates(
  canvas: HTMLCanvasElement
): Promise<ProfileImageCandidates> {
  let balanced: EncodedProfileImage | null = null;
  let smaller: EncodedProfileImage | null = null;
  // Each candidate starts from the original cropped pixels. 5 KiB is only an
  // optimization target; the backend inspects the actual evidence layout.
  for (const quality of [0.86, 0.78, 0.7, 0.62, 0.55, 0.45]) {
    const blob = await new Promise<Blob | null>((resolve) =>
      canvas.toBlob(resolve, 'image/webp', quality)
    );
    if (!blob || blob.type !== 'image/webp') throw new Error('webp_unsupported');
    const bytes = new Uint8Array(await blob.arrayBuffer());
    if (!isWebP(bytes)) throw new Error('webp_unsupported');
    if (bytes.length > MAX_PROFILE_IMAGE_BYTES) continue;
    const candidate: EncodedProfileImage = {
      base64: btoa(Array.from(bytes, (byte) => String.fromCharCode(byte)).join('')),
      mimeType: 'image/webp',
      byteLength: bytes.length,
    };
    if (!balanced || balanced.byteLength > 16 * 1024) balanced = candidate;
    if (bytes.length <= 5 * 1024) {
      smaller = candidate;
      break;
    }
    if (!smaller || smaller.byteLength > bytes.length) smaller = candidate;
  }
  if (!balanced) throw new Error('too_large');
  return {
    balanced,
    smaller: smaller && smaller.byteLength < balanced.byteLength ? smaller : null,
  };
}

export interface EncodedProfileImage {
  base64: string;
  mimeType: 'image/webp';
  byteLength: number;
}
export interface ProfileImageCandidates {
  balanced: EncodedProfileImage;
  smaller: EncodedProfileImage | null;
}
export function isWebP(bytes: Uint8Array): boolean {
  return (
    bytes.length >= 12 &&
    String.fromCharCode(...bytes.slice(0, 4)) === 'RIFF' &&
    String.fromCharCode(...bytes.slice(8, 12)) === 'WEBP' &&
    new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(4, true) ===
      bytes.length - 8
  );
}
export function profileMediaUrl(
  base64: string | null | undefined,
  mime: string | null | undefined
): string | null {
  // Missing MIME is only the legacy persisted JPEG snapshot format.
  return base64 && (!mime || mime === 'image/jpeg' || mime === 'image/webp')
    ? `data:${mime || 'image/jpeg'};base64,${base64}`
    : null;
}
export function encodedByteLength(base64: string): number {
  return Math.max(
    0,
    Math.floor((base64.length * 3) / 4) - (base64.endsWith('==') ? 2 : base64.endsWith('=') ? 1 : 0)
  );
}
export function profileFeeDisplay(sats: string): string {
  const value = BigInt(sats);
  return `${value / 100000000n}.${(value % 100000000n).toString().padStart(8, '0')}`
    .replace(/0+$/, '')
    .replace(/\.$/, '');
}
