import { get, writable } from 'svelte/store';
import type { WalletNetwork } from '$lib/types/wallet';
import { addressBookStore } from '$lib/stores/addressBook';

export type ContactSession = { sessionId: string; network: WalletNetwork };
export const contactSession = writable<ContactSession | null>(null);
export const contactsLoadState = writable<'idle' | 'loading' | 'ready' | 'error'>('idle');
export const activeProfilePreview = writable<string | null>(null);

export function setContactSession(session: ContactSession | null): void {
  const current = get(contactSession);
  if (current?.sessionId === session?.sessionId && current?.network === session?.network) return;
  addressBookStore.set([]);
  contactsLoadState.set('idle');
  activeProfilePreview.set(null);
  contactSession.set(session);
}

export function requireContactSession(): ContactSession {
  const session = get(contactSession);
  if (!session) throw new Error('Contact session unavailable');
  return session;
}

export function isContactSessionCurrent(session: ContactSession): boolean {
  return get(contactSession) === session;
}
