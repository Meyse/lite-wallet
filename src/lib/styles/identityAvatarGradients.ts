export const IDENTITY_AVATAR_GRADIENTS = [
  ['#2563EB', '#0891B2'],
  ['#2563EB', '#4F46E5'],
  ['#0F766E', '#2563EB'],
  ['#1D4ED8', '#0E7490'],
  ['#0EA5E9', '#1D4ED8'],
  ['#1E3A8A', '#0284C7'],
] as const;

export type IdentityAvatarGradient = (typeof IDENTITY_AVATAR_GRADIENTS)[number];

export function pickIdentityAvatarGradient(seed: string): IdentityAvatarGradient {
  return IDENTITY_AVATAR_GRADIENTS[hashSeed(seed) % IDENTITY_AVATAR_GRADIENTS.length];
}

function hashSeed(value: string): number {
  let hash = 0;
  for (const char of value.trim().toLowerCase()) {
    hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  }
  return hash;
}
