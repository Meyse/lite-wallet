// @vitest-environment jsdom

import { get } from 'svelte/store';
import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '$lib/i18n';
import { settingsStore } from '$lib/stores/settings.js';
import DisplayLanguageSettings from './DisplayLanguageSettings.svelte';

class ResizeObserverStub {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}

Object.defineProperty(globalThis, 'ResizeObserver', {
  configurable: true,
  value: ResizeObserverStub,
});

async function settle(): Promise<void> {
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 0));
}

function findButton(label: string): HTMLButtonElement | undefined {
  return [...document.body.querySelectorAll<HTMLButtonElement>('button')].find((button) =>
    button.textContent?.includes(label)
  );
}

describe('mounted display and language settings', () => {
  beforeEach(() => {
    localStorage.clear();
    setLocale('en');
    settingsStore.set({ theme: 'system', displayCurrency: 'USD', autoLockMinutes: 15 });
  });

  afterEach(() => {
    document.body.replaceChildren();
  });

  it('filters, clears, and dismisses the currency sheet after a selection', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(DisplayLanguageSettings, {
      target,
      props: { onBack: vi.fn() },
    });

    await settle();
    findButton('Display currency')?.click();
    await settle();

    const search = document.body.querySelector<HTMLInputElement>(
      'input[placeholder="Search currencies"]'
    );
    expect(search).not.toBeNull();
    expect(findButton('EUR')).toBeDefined();

    if (search) {
      search.value = 'dollar';
      search.dispatchEvent(new InputEvent('input', { bubbles: true }));
    }
    await settle();

    expect(document.body.textContent).toContain('6 currencies');
    expect(document.body.textContent).toContain('Australian Dollar');
    expect(document.body.textContent).not.toContain('Euro');

    document.body.querySelector<HTMLButtonElement>('button[aria-label="Clear search"]')?.click();
    await settle();
    expect(document.body.textContent).toContain('All currencies');
    expect(findButton('EUR')).toBeDefined();

    findButton('EUR')?.click();
    await settle();
    expect(get(settingsStore).displayCurrency).toBe('EUR');
    expect(document.body.querySelector('[data-slot="sheet-content"]')).toBeNull();

    await unmount(component);
  });
});
