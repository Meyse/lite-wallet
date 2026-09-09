import { get } from 'svelte/store';
import { isForcedWalletLockError, walletUnlockRedirectingStore } from './walletLockCoordinator.js';
import * as walletService from './walletService.js';

const ACTIVITY_TOUCH_THROTTLE_MS = 15_000;
const SCROLL_INTENT_WINDOW_MS = 1_000;
const PASSIVE_ACTIVITY_EVENTS = [
  'pointerdown',
  'pointermove',
  'wheel',
  'touchstart',
  'touchmove',
] as const;
const ACTIVE_ACTIVITY_EVENTS = ['keydown'] as const;
const SCROLL_INTENT_EVENTS = new Set<string>(['pointerdown', 'wheel', 'touchstart', 'keydown']);

const PASSIVE_CAPTURE_OPTIONS = { capture: true, passive: true } as const;
const CAPTURE_OPTIONS = { capture: true } as const;

export function startWalletActivityMonitor(): () => void {
  let disposed = false;
  let lastTouchedAt = Date.now();
  let inFlightTouch: Promise<void> | null = null;
  let pendingActivityAt: number | null = null;
  let trailingTouchTimer: ReturnType<typeof setTimeout> | null = null;
  let scrollIntentExpiresAt: number | null = null;

  function shouldSkipActivityTouch(): boolean {
    return (
      disposed ||
      get(walletUnlockRedirectingStore) ||
      typeof document === 'undefined' ||
      document.visibilityState === 'hidden'
    );
  }

  function clearTrailingTouch(): void {
    if (trailingTouchTimer === null) {
      return;
    }

    clearTimeout(trailingTouchTimer);
    trailingTouchTimer = null;
  }

  function scheduleActivityTouch(): void {
    if (pendingActivityAt === null || inFlightTouch || trailingTouchTimer !== null) {
      return;
    }

    const delay = Math.max(0, lastTouchedAt + ACTIVITY_TOUCH_THROTTLE_MS - Date.now());
    if (delay > 0) {
      trailingTouchTimer = setTimeout(() => {
        trailingTouchTimer = null;
        touchPendingActivity();
      }, delay);
      return;
    }

    touchPendingActivity();
  }

  function touchPendingActivity(): void {
    if (pendingActivityAt === null || inFlightTouch) {
      return;
    }

    if (Date.now() - pendingActivityAt > ACTIVITY_TOUCH_THROTTLE_MS) {
      pendingActivityAt = null;
      return;
    }

    if (shouldSkipActivityTouch()) {
      pendingActivityAt = null;
      return;
    }

    pendingActivityAt = null;
    lastTouchedAt = Date.now();
    inFlightTouch = walletService
      .touchSessionActivity()
      .catch((error) => {
        if (isForcedWalletLockError(error)) {
          pendingActivityAt = null;
          return;
        }

        console.error('[WALLET_ACTIVITY] Failed to refresh session activity', error);
      })
      .finally(() => {
        inFlightTouch = null;

        if (shouldSkipActivityTouch()) {
          pendingActivityAt = null;
          clearTrailingTouch();
          return;
        }

        scheduleActivityTouch();
      });
  }

  function recordActivity(): void {
    if (shouldSkipActivityTouch()) {
      return;
    }

    const now = Date.now();
    pendingActivityAt = now;
    scheduleActivityTouch();
  }

  function recordTrustedActivity(event: Event): void {
    if (!event.isTrusted) {
      return;
    }

    if (SCROLL_INTENT_EVENTS.has(event.type)) {
      scrollIntentExpiresAt = Date.now() + SCROLL_INTENT_WINDOW_MS;
    }

    recordActivity();
  }

  function recordWindowFocus(event: Event): void {
    if (!event.isTrusted || event.target !== window) {
      return;
    }

    recordActivity();
  }

  function recordUserScroll(): void {
    const now = Date.now();
    if (scrollIntentExpiresAt === null || now > scrollIntentExpiresAt) {
      return;
    }

    // A single direct input may authorize one resulting scroll event. Scroll
    // events alone are insufficient because script-driven scrolling can also
    // produce trusted events and must not keep the wallet unlocked indefinitely.
    scrollIntentExpiresAt = null;
    recordActivity();
  }

  for (const eventName of PASSIVE_ACTIVITY_EVENTS) {
    window.addEventListener(eventName, recordTrustedActivity, PASSIVE_CAPTURE_OPTIONS);
  }

  for (const eventName of ACTIVE_ACTIVITY_EVENTS) {
    window.addEventListener(eventName, recordTrustedActivity, CAPTURE_OPTIONS);
  }

  window.addEventListener('scroll', recordUserScroll, PASSIVE_CAPTURE_OPTIONS);
  window.addEventListener('focus', recordWindowFocus);

  return () => {
    disposed = true;
    pendingActivityAt = null;
    scrollIntentExpiresAt = null;
    clearTrailingTouch();

    for (const eventName of PASSIVE_ACTIVITY_EVENTS) {
      window.removeEventListener(eventName, recordTrustedActivity, PASSIVE_CAPTURE_OPTIONS);
    }

    for (const eventName of ACTIVE_ACTIVITY_EVENTS) {
      window.removeEventListener(eventName, recordTrustedActivity, CAPTURE_OPTIONS);
    }

    window.removeEventListener('scroll', recordUserScroll, PASSIVE_CAPTURE_OPTIONS);
    window.removeEventListener('focus', recordWindowFocus);
  };
}
