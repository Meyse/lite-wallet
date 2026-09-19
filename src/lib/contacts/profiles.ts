import { get, writable } from 'svelte/store';
import { invokeSessionBoundWalletCommand } from '$lib/services/invokeWalletCommand';
import type { ContactIdentity } from '$lib/types/addressBook';
import type { IdentityProfileLoadResult } from '$lib/types/wallet';
import { contactSession, isContactSessionCurrent, requireContactSession } from './session';
import { contactChainId, identityKey } from './identity';

export type ProfileEntry = {
  profile: IdentityProfileLoadResult | null;
  loading: boolean;
  unavailable: boolean;
  checkedAt: number;
};
export const identityProfiles = writable<Record<string, ProfileEntry>>({});
const FRESH_MS = 60_000;
const RETRY_MS = 15_000;
const MAX_ENTRIES = 100;
const MAX_CONCURRENT = 3;
let epoch = 0;
let running = 0;
const queue: Array<{
  key: string;
  priority: boolean;
  start: () => void;
  reject: (error: Error) => void;
}> = [];
const MAX_QUEUED = 100;
let inflight = new Map<string, Promise<IdentityProfileLoadResult>>();

contactSession.subscribe(() => {
  epoch++;
  inflight = new Map();
  identityProfiles.set({});
  for (const job of queue.splice(0)) job.reject(new Error('Obsolete profile request'));
});

async function slot<T>(key: string, priority: boolean, work: () => Promise<T>): Promise<T> {
  if (running >= MAX_CONCURRENT) {
    if (queue.length >= MAX_QUEUED) throw new Error('Profile queue full');
    await new Promise<void>((start, reject) => {
      const job = { key, priority, start, reject };
      if (priority) queue.unshift(job);
      else queue.push(job);
    });
  } else running++;
  try {
    return await work();
  } finally {
    // Transfer this slot directly so a new request cannot overtake a queued job.
    const next = queue.shift();
    if (next) next.start();
    else running--;
  }
}

function publish(key: string, entry: ProfileEntry): void {
  identityProfiles.update((entries) => {
    const next = { ...entries, [key]: entry };
    const oldest = Object.keys(next)
      .filter((key) => !next[key].loading)
      .sort((a, b) => next[a].checkedAt - next[b].checkedAt);
    while (Object.keys(next).length > MAX_ENTRIES && oldest.length) {
      const expired = oldest.shift();
      if (expired) delete next[expired];
    }
    return next;
  });
}

export function loadIdentityProfile(
  identity: ContactIdentity,
  refresh = false,
  priority = false
): Promise<IdentityProfileLoadResult> {
  const session = requireContactSession();
  const key = identityKey(identity);
  if (
    identity.network !== session.network ||
    identity.chainId !== contactChainId(session.network)
  ) {
    return Promise.resolve({ state: 'unavailable', issues: [], revisionTxid: null });
  }
  const current = get(identityProfiles)[key];
  const pending = inflight.get(key);
  if (pending) {
    if (priority) {
      const index = queue.findIndex((job) => job.key === key);
      if (index > 0) {
        const [job] = queue.splice(index, 1);
        job.priority = true;
        queue.unshift(job);
      }
    }
    return pending;
  }
  if (
    !refresh &&
    current &&
    Date.now() - current.checkedAt < (current.unavailable ? RETRY_MS : FRESH_MS)
  ) {
    return Promise.resolve(
      current.profile ?? { state: 'unavailable', issues: [], revisionTxid: null }
    );
  }
  const generation = epoch;
  const previous = current?.profile ?? null;
  const request = slot(key, priority, async () => {
    if (generation !== epoch) throw new Error('Obsolete profile request');
    publish(key, { profile: previous, loading: true, unavailable: false, checkedAt: 0 });
    return invokeSessionBoundWalletCommand<IdentityProfileLoadResult>('get_identity_profile', {
      identity_address: identity.identityAddress,
      expected_session_id: session.sessionId,
      chain_id: identity.chainId,
    });
  })
    .then((profile) => {
      if (!isContactSessionCurrent(session)) throw new Error('Obsolete profile request');
      // Unavailable refreshes retain only already-confirmed session content. Empty is a removal.
      const displayed = profile.state === 'unavailable' && previous ? previous : profile;
      publish(key, {
        profile: displayed,
        loading: false,
        unavailable: profile.state === 'unavailable',
        checkedAt: Date.now(),
      });
      return displayed;
    })
    .catch((error: unknown) => {
      if (isContactSessionCurrent(session))
        publish(key, {
          profile: previous,
          loading: false,
          unavailable: true,
          checkedAt: Date.now(),
        });
      throw error;
    })
    .finally(() => {
      if (generation === epoch) inflight.delete(key);
    });
  inflight.set(key, request);
  return request;
}

export function profileImage(profile: IdentityProfileLoadResult | null | undefined): string | null {
  const avatar = profile?.avatar?.value;
  return avatar && ['image/jpeg', 'image/png', 'image/webp'].includes(avatar.mimeType)
    ? `data:${avatar.mimeType};base64,${avatar.base64}`
    : null;
}
