/**
 * Non-blocking wallet error state used for update/polling visibility.
 */

import { writable } from 'svelte/store';

export interface WalletErrorsState {
  latest: WalletErrorEntry | null;
  history: string[];
}

export type WalletErrorPresentation =
  | { kind: 'message'; message: string }
  | {
      kind: 'background-update';
      dataType: string;
      coinId?: string;
      channel?: string;
    };
type WalletBackgroundErrorPresentation = Extract<
  WalletErrorPresentation,
  { kind: 'background-update' }
>;

export interface WalletBackgroundErrorContext {
  dataType: string;
  coinId?: string;
  channel?: string;
}

export interface WalletErrorEntry {
  diagnostic: string;
  presentation: WalletErrorPresentation;
}

const initialState: WalletErrorsState = {
  latest: null,
  history: [],
};
const DEDUPE_WINDOW_MS = 10_000;
const SEEN_RETENTION_MS = DEDUPE_WINDOW_MS * 6;
const recentErrors = new Map<string, { seenAt: number; presentation: WalletErrorPresentation }>();
const unresolvedBackgroundErrors = new Map<string, { entry: WalletErrorEntry; sequence: number }>();
let backgroundErrorSequence = 0;

export const walletErrorsStore = writable<WalletErrorsState>(initialState);

function normalizeMessage(message: string): string {
  return message.trim().replace(/\s+/g, ' ');
}

function errorDedupeKey(diagnostic: string, presentation: WalletErrorPresentation): string {
  if (presentation.kind === 'message') return diagnostic;
  return JSON.stringify([
    presentation.kind,
    presentation.dataType.toLowerCase(),
    presentation.coinId ?? '',
    presentation.channel ?? '',
    diagnostic,
  ]);
}

function backgroundContextKey(presentation: WalletBackgroundErrorPresentation): string {
  return JSON.stringify([
    presentation.dataType.toLowerCase(),
    presentation.coinId ?? '',
    presentation.channel ?? '',
  ]);
}

function mostRecentUnresolvedBackgroundError(): WalletErrorEntry | null {
  let mostRecent: { entry: WalletErrorEntry; sequence: number } | null = null;
  for (const unresolved of unresolvedBackgroundErrors.values()) {
    if (!mostRecent || unresolved.sequence > mostRecent.sequence) {
      mostRecent = unresolved;
    }
  }
  return mostRecent?.entry ?? null;
}

function seenRecently(key: string, now: number): boolean {
  const recent = recentErrors.get(key);
  return recent !== undefined && now - recent.seenAt < DEDUPE_WINDOW_MS;
}

function rememberError(key: string, presentation: WalletErrorPresentation, now: number): void {
  recentErrors.set(key, { seenAt: now, presentation });
  for (const [recentKey, recent] of recentErrors.entries()) {
    if (now - recent.seenAt > SEEN_RETENTION_MS) {
      recentErrors.delete(recentKey);
    }
  }
}

function pushWalletErrorEntry(diagnostic: string, presentation: WalletErrorPresentation): void {
  const text = normalizeMessage(diagnostic);
  if (!text) return;
  const now = Date.now();
  const dedupeKey = errorDedupeKey(text, presentation);
  if (seenRecently(dedupeKey, now)) return;
  rememberError(dedupeKey, presentation, now);
  const entry = { diagnostic: text, presentation };
  if (presentation.kind === 'background-update') {
    backgroundErrorSequence += 1;
    unresolvedBackgroundErrors.set(backgroundContextKey(presentation), {
      entry,
      sequence: backgroundErrorSequence,
    });
  }

  walletErrorsStore.update((s) => {
    if (s.latest && errorDedupeKey(s.latest.diagnostic, s.latest.presentation) === dedupeKey) {
      return s;
    }
    return {
      latest: entry,
      history: [text, ...s.history].slice(0, 20),
    };
  });
}

export function pushWalletError(message: string): void {
  const text = normalizeMessage(message);
  if (!text) return;
  pushWalletErrorEntry(text, { kind: 'message', message: text });
}

export function pushWalletBackgroundError(
  diagnostic: string,
  context: WalletBackgroundErrorContext = { dataType: 'wallet' }
): void {
  pushWalletErrorEntry(diagnostic, {
    kind: 'background-update',
    dataType: normalizeMessage(context.dataType) || 'wallet',
    coinId: context.coinId,
    channel: context.channel,
  });
}

export function clearMatchingWalletBackgroundError({
  dataTypes,
  coinId,
  channel,
}: {
  dataTypes: string[];
  coinId?: string;
  channel?: string;
}): void {
  const types = new Set(dataTypes.map((value) => normalizeMessage(value).toLowerCase()));
  const matches = (presentation: WalletErrorPresentation): boolean => {
    if (presentation.kind !== 'background-update') return false;
    if (!types.has(presentation.dataType.toLowerCase())) return false;
    if (presentation.coinId && presentation.coinId !== coinId) return false;
    if (presentation.channel && presentation.channel !== channel) return false;
    return true;
  };

  for (const [key, recent] of recentErrors.entries()) {
    if (matches(recent.presentation)) {
      recentErrors.delete(key);
    }
  }
  for (const [key, unresolved] of unresolvedBackgroundErrors.entries()) {
    if (matches(unresolved.entry.presentation)) {
      unresolvedBackgroundErrors.delete(key);
    }
  }

  walletErrorsStore.update((state) => {
    const presentation = state.latest?.presentation;
    if (!presentation || !matches(presentation)) return state;
    return { ...state, latest: mostRecentUnresolvedBackgroundError() };
  });
}

export function clearWalletErrors(): void {
  recentErrors.clear();
  unresolvedBackgroundErrors.clear();
  backgroundErrorSequence = 0;
  walletErrorsStore.set(initialState);
}

export function dismissWalletError(): void {
  unresolvedBackgroundErrors.clear();
  walletErrorsStore.update((s) => ({ ...s, latest: null }));
}
