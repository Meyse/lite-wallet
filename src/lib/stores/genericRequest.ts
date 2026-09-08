import { get, writable } from 'svelte/store';

export type QueuedGenericRequestSource = 'manual' | 'deep-link' | 'provisioning';

export interface QueuedGenericRequestPayload {
  input: string;
  passthroughAutoLinkFqn: string | null;
  source: QueuedGenericRequestSource;
  enqueuedAt: number;
}

const STORAGE_KEY = 'lite_wallet_generic_request_queue_v1';

function readPersistedQueue(): QueuedGenericRequestPayload | null {
  if (typeof globalThis.localStorage === 'undefined') return null;

  try {
    const raw = globalThis.localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;

    const parsed = JSON.parse(raw) as Partial<QueuedGenericRequestPayload> | null;
    if (!parsed || typeof parsed.input !== 'string') {
      return null;
    }

    return {
      input: parsed.input,
      passthroughAutoLinkFqn:
        typeof parsed.passthroughAutoLinkFqn === 'string' && parsed.passthroughAutoLinkFqn.trim()
          ? parsed.passthroughAutoLinkFqn.trim()
          : null,
      source:
        parsed.source === 'deep-link' || parsed.source === 'provisioning' || parsed.source === 'manual'
          ? parsed.source
          : 'manual',
      enqueuedAt:
        typeof parsed.enqueuedAt === 'number' && Number.isFinite(parsed.enqueuedAt)
          ? parsed.enqueuedAt
          : Date.now()
    };
  } catch {
    return null;
  }
}

function persistQueuedRequest(payload: QueuedGenericRequestPayload | null): void {
  if (typeof globalThis.localStorage === 'undefined') return;

  try {
    if (!payload) {
      globalThis.localStorage.removeItem(STORAGE_KEY);
      return;
    }

    globalThis.localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
  } catch {
    // Ignore storage failures.
  }
}

const initialValue = readPersistedQueue();

export const genericRequestQueueStore = writable<QueuedGenericRequestPayload | null>(initialValue);

genericRequestQueueStore.subscribe((payload) => {
  persistQueuedRequest(payload);
});

export function queueGenericRequest(
  payload: Omit<QueuedGenericRequestPayload, 'enqueuedAt'> & { enqueuedAt?: number }
): void {
  genericRequestQueueStore.set({
    input: payload.input,
    passthroughAutoLinkFqn: payload.passthroughAutoLinkFqn?.trim() || null,
    source: payload.source,
    enqueuedAt: payload.enqueuedAt ?? Date.now()
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

export function hydrateQueuedGenericRequest(): void {
  genericRequestQueueStore.set(readPersistedQueue());
}
