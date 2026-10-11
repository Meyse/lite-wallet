import { get, writable } from 'svelte/store';

export type QueuedGenericRequestSource = 'manual' | 'deep-link' | 'provisioning' | 'lock';

export interface QueuedGenericRequestPayload {
  input: string;
  passthroughAutoLinkFqn: string | null;
  source: QueuedGenericRequestSource;
  enqueuedAt: number;
}

// Earlier versions kept the waiting request in plain-text localStorage. A
// request now waits in memory only (spec "Intake"); remove the old copy once.
const LEGACY_STORAGE_KEY = 'lite_wallet_generic_request_queue_v1';
try {
  globalThis.localStorage?.removeItem(LEGACY_STORAGE_KEY);
} catch {
  // Storage can be unavailable; there is then nothing to remove.
}

/** The one request that waits to open: memory only, never written to disk. */
export const genericRequestQueueStore = writable<QueuedGenericRequestPayload | null>(null);

export function queueGenericRequest(
  payload: Omit<QueuedGenericRequestPayload, 'enqueuedAt'> & { enqueuedAt?: number }
): void {
  genericRequestQueueStore.set({
    input: payload.input,
    passthroughAutoLinkFqn: payload.passthroughAutoLinkFqn?.trim() || null,
    source: payload.source,
    enqueuedAt: payload.enqueuedAt ?? Date.now(),
  });
}

export function clearQueuedGenericRequest(): void {
  genericRequestQueueStore.set(null);
}

export function consumeQueuedGenericRequest(): QueuedGenericRequestPayload | null {
  const current = get(genericRequestQueueStore);
  if (!current) return null;
  clearQueuedGenericRequest();
  return current;
}
