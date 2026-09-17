import { get } from 'svelte/store';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const setModeMock = vi.hoisted(() => vi.fn());

vi.mock('mode-watcher', () => ({
  setMode: setModeMock,
}));

class MemoryStorage implements Storage {
  private readonly values = new Map<string, string>();

  get length(): number {
    return this.values.size;
  }

  clear(): void {
    this.values.clear();
  }

  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }

  key(index: number): string | null {
    return [...this.values.keys()][index] ?? null;
  }

  removeItem(key: string): void {
    this.values.delete(key);
  }

  setItem(key: string, value: string): void {
    this.values.set(key, value);
  }
}

describe('settings persistence', () => {
  let storage: MemoryStorage;

  beforeEach(() => {
    vi.resetModules();
    setModeMock.mockReset();
    storage = new MemoryStorage();
    vi.stubGlobal('localStorage', storage);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('hydrates the existing mode-watcher preference and persists explicit changes', async () => {
    storage.setItem('mode-watcher-mode', 'dark');
    const { settingsStore, setAppearancePreference } = await import('./settings.js');

    expect(get(settingsStore).theme).toBe('dark');

    setAppearancePreference('light');

    expect(setModeMock).toHaveBeenCalledWith('light');
    expect(get(settingsStore).theme).toBe('light');
    expect(JSON.parse(storage.getItem('lite_wallet_settings_v1') ?? '{}')).toMatchObject({
      theme: 'light',
    });
  });

  it('keeps currency and auto-lock values normalized when persisted', async () => {
    const { settingsStore, setAutoLockMinutes, setDisplayCurrency } = await import('./settings.js');

    setDisplayCurrency(' eur ');
    setAutoLockMinutes(30);

    expect(get(settingsStore)).toMatchObject({
      theme: 'system',
      displayCurrency: 'EUR',
      autoLockMinutes: 30,
    });
    expect(JSON.parse(storage.getItem('lite_wallet_settings_v1') ?? '{}')).toMatchObject({
      displayCurrency: 'EUR',
      autoLockMinutes: 30,
    });
  });
});
