import type { IdentityProfileAvatar } from '$lib/types/wallet';

/** Presentation contract only. Future readers must validate claims before setting verified. */
export type ProfileVerification = 'verified' | 'unverified' | 'unavailable';
export type ProfileWebsite = { url: string; verification: ProfileVerification };
export type ProfileSocial = {
  platform: 'linkedin' | 'x';
  profileUrl: string;
  proofUrl?: string;
  verification: ProfileVerification;
};
export type PublicProfileContent = {
  header?: IdentityProfileAvatar | null;
  websites?: ProfileWebsite[];
  socials?: ProfileSocial[];
  addresses?: { network: string; address: string; verification: ProfileVerification }[];
};

export function profileExternalUrl(value: string | undefined): string | null {
  if (!value) return null;
  try {
    const url = new URL(value);
    return url.protocol === 'https:' && !url.username && !url.password ? url.href : null;
  } catch {
    return null;
  }
}

/** No remote image requests: the reader supplies a decoded, bounded on-chain image. */
export function profileHeaderImage(value: IdentityProfileAvatar | null | undefined): string | null {
  if (!value || !['image/jpeg', 'image/png', 'image/webp'].includes(value.mimeType)) return null;
  if (value.byteLength > 32768 || value.base64.length > 43692) return null;
  return `data:${value.mimeType};base64,${value.base64}`;
}
