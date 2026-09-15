import { openUrl } from '@tauri-apps/plugin-opener';

export const COMMUNITY_HANGOUT_URL = 'https://verus.io/discord';

export async function openTrustedExternalUrl(url: string): Promise<void> {
  try {
    await openUrl(url);
    return;
  } catch {
    // Fall through to browser API fallback.
  }

  if (typeof globalThis.open === 'function') {
    globalThis.open(url, '_blank', 'noopener,noreferrer');
  }
}

export async function openCommunityHangout(): Promise<void> {
  await openTrustedExternalUrl(COMMUNITY_HANGOUT_URL);
}
