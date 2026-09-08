import { onDestroy } from 'svelte';

export async function writeClipboardText(value: string): Promise<boolean> {
  const trimmed = value.trim();
  if (!trimmed) return false;

  const clipboard = globalThis.navigator?.clipboard;
  if (!clipboard) return false;

  try {
    await clipboard.writeText(trimmed);
    return true;
  } catch {
    return false;
  }
}

export class TimedValueState<T> {
  current = $state<T | null>(null);
  #timer: ReturnType<typeof setTimeout> | null = null;

  constructor() {
    onDestroy(() => {
      this.destroy();
    });
  }

  set(value: T, duration = 2000): void {
    this.clearTimer();
    this.current = value;
    this.#timer = setTimeout(() => {
      this.current = null;
      this.#timer = null;
    }, duration);
  }

  clear(): void {
    this.current = null;
    this.clearTimer();
  }

  destroy(): void {
    this.clear();
  }

  private clearTimer(): void {
    if (!this.#timer) return;
    clearTimeout(this.#timer);
    this.#timer = null;
  }
}

export class TimedRecordState<T> {
  values = $state<Record<string, T>>({});
  #timers = new Map<string, ReturnType<typeof setTimeout>>();

  constructor() {
    onDestroy(() => {
      this.destroy();
    });
  }

  set(key: string, value: T, duration = 2000): void {
    this.values = {
      ...this.values,
      [key]: value,
    };

    const existingTimer = this.#timers.get(key);
    if (existingTimer) {
      clearTimeout(existingTimer);
    }

    const timer = setTimeout(() => {
      this.clear(key);
    }, duration);

    this.#timers.set(key, timer);
  }

  clear(key: string): void {
    const existingTimer = this.#timers.get(key);
    if (existingTimer) {
      clearTimeout(existingTimer);
      this.#timers.delete(key);
    }

    if (!(key in this.values)) return;

    const nextValues = { ...this.values };
    delete nextValues[key];
    this.values = nextValues;
  }

  clearAll(): void {
    for (const timer of this.#timers.values()) {
      clearTimeout(timer);
    }
    this.#timers.clear();
    this.values = {};
  }

  destroy(): void {
    this.clearAll();
  }
}
