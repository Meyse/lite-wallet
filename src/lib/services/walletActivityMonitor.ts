import { get } from 'svelte/store';
import { isForcedWalletLockError, walletUnlockRedirectingStore } from './walletLockCoordinator.js';
import * as walletService from './walletService.js';

const ACTIVITY_TOUCH_THROTTLE_MS = 15_000;
const PASSIVE_ACTIVITY_EVENTS = ['pointerdown', 'wheel', 'touchstart'] as const;
const ACTIVE_ACTIVITY_EVENTS = ['keydown'] as const;

export function startWalletActivityMonitor(): () => void {
  let disposed = false;
  let lastTouchedAt = Date.now();
  let inFlightTouch: Promise<void> | null = null;

  function shouldSkipActivityTouch(): boolean {
    return (
      disposed ||
      get(walletUnlockRedirectingStore) ||
      typeof document === 'undefined' ||
      document.visibilityState === 'hidden'
    );
  }

  function recordActivity(): void {
    if (shouldSkipActivityTouch() || inFlightTouch) {
      return;
    }

    const now = Date.now();
    if (now - lastTouchedAt < ACTIVITY_TOUCH_THROTTLE_MS) {
      return;
    }

    lastTouchedAt = now;
    inFlightTouch = walletService
      .touchSessionActivity()
      .catch((error) => {
        if (isForcedWalletLockError(error)) {
          return;
        }

        console.error('[WALLET_ACTIVITY] Failed to refresh session activity', error);
      })
      .finally(() => {
        inFlightTouch = null;
      });
  }

  for (const eventName of PASSIVE_ACTIVITY_EVENTS) {
    window.addEventListener(eventName, recordActivity, { passive: true });
  }

  for (const eventName of ACTIVE_ACTIVITY_EVENTS) {
    window.addEventListener(eventName, recordActivity);
  }

  window.addEventListener('focus', recordActivity);

  return () => {
    disposed = true;

    for (const eventName of PASSIVE_ACTIVITY_EVENTS) {
      window.removeEventListener(eventName, recordActivity);
    }

    for (const eventName of ACTIVE_ACTIVITY_EVENTS) {
      window.removeEventListener(eventName, recordActivity);
    }

    window.removeEventListener('focus', recordActivity);
  };
}
