/**
 * Settings store with local persistence for app-wide preferences.
 */

import { writable } from 'svelte/store';
import { setMode } from 'mode-watcher';
import { DEFAULT_DISPLAY_CURRENCY, normalizeDisplayCurrency } from '$lib/utils/fiatDisplay.js';
import {
  type AutoLockMinutes,
  DEFAULT_AUTO_LOCK_MINUTES,
  normalizeAutoLockMinutes,
} from '$lib/security/sessionTimeout.js';
import {
  type AppearancePreference,
  DEFAULT_APPEARANCE_PREFERENCE,
  normalizeAppearancePreference,
} from '$lib/utils/appearance.js';

export interface Settings {
  theme: AppearancePreference;
  displayCurrency: string;
  autoLockMinutes: AutoLockMinutes;
}

const SETTINGS_STORAGE_KEY = 'lite_wallet_settings_v1';
const MODE_WATCHER_STORAGE_KEY = 'mode-watcher-mode';

const initialState: Settings = {
  theme: DEFAULT_APPEARANCE_PREFERENCE,
  displayCurrency: DEFAULT_DISPLAY_CURRENCY,
  autoLockMinutes: DEFAULT_AUTO_LOCK_MINUTES,
};

function canUseStorage(): boolean {
  return typeof globalThis.localStorage !== 'undefined';
}

function readStoredSettings(): Settings {
  if (!canUseStorage()) return initialState;

  try {
    const raw = globalThis.localStorage.getItem(SETTINGS_STORAGE_KEY);
    if (!raw) {
      return {
        ...initialState,
        theme: normalizeAppearancePreference(
          globalThis.localStorage.getItem(MODE_WATCHER_STORAGE_KEY)
        ),
      };
    }

    const parsed = JSON.parse(raw) as Partial<Settings> | null;
    return {
      theme: normalizeAppearancePreference(
        globalThis.localStorage.getItem(MODE_WATCHER_STORAGE_KEY) ?? parsed?.theme
      ),
      displayCurrency: normalizeDisplayCurrency(parsed?.displayCurrency),
      autoLockMinutes: normalizeAutoLockMinutes(parsed?.autoLockMinutes),
    };
  } catch {
    return initialState;
  }
}

function persistSettings(settings: Settings): void {
  if (!canUseStorage()) return;

  try {
    globalThis.localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // Ignore persistence failures (private mode / restricted storage).
  }
}

export const settingsStore = writable<Settings>(readStoredSettings());

settingsStore.subscribe((settings) => {
  persistSettings({
    theme: settings.theme,
    displayCurrency: normalizeDisplayCurrency(settings.displayCurrency),
    autoLockMinutes: normalizeAutoLockMinutes(settings.autoLockMinutes),
  });
});

export function setDisplayCurrency(code: string): void {
  settingsStore.update((settings) => ({
    ...settings,
    displayCurrency: normalizeDisplayCurrency(code),
  }));
}

export function setAppearancePreference(theme: unknown): void {
  const normalized = normalizeAppearancePreference(theme);
  setMode(normalized);
  settingsStore.update((settings) => ({
    ...settings,
    theme: normalized,
  }));
}

export function setAutoLockMinutes(minutes: unknown): void {
  settingsStore.update((settings) => ({
    ...settings,
    autoLockMinutes: normalizeAutoLockMinutes(minutes),
  }));
}

export function resetSettings(): void {
  setMode(DEFAULT_APPEARANCE_PREFERENCE);
  settingsStore.set(initialState);
}
