// @vitest-environment jsdom

import { get } from 'svelte/store';
import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { localeStore, setLocale } from '$lib/i18n';
import LocaleSelector from './LocaleSelector.svelte';

async function chooseLocale(size: 'compact' | 'regular', label: string): Promise<void> {
  const target = document.createElement('div');
  target.style.width = size === 'compact' ? '152px' : '320px';
  document.body.append(target);
  const component = mount(LocaleSelector, {
    target,
    props: {
      size,
      triggerAriaLabel: `Choose ${size} locale`,
    },
  });

  await tick();
  (target.querySelector('button') as HTMLButtonElement).dispatchEvent(
    new PointerEvent('pointerdown', { bubbles: true, button: 0 })
  );
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 0));
  const item = [...document.body.querySelectorAll<HTMLElement>('[role="menuitemradio"]')].find(
    (candidate) => candidate.textContent?.trim() === label
  );
  expect(item).toBeDefined();
  item?.click();
  await tick();

  await unmount(component);
  target.remove();
}

describe('mounted locale selector', () => {
  beforeEach(() => {
    localStorage.clear();
    setLocale('en');
  });

  afterEach(() => {
    document.body.replaceChildren();
  });

  it('changes and persists locale from the regular onboarding selector', async () => {
    await chooseLocale('regular', 'Nederlands');
    expect(get(localeStore)).toBe('nl');
    expect(localStorage.getItem('lite_wallet_locale_v1')).toBe('nl');
  });

  it('changes and persists locale from the compact settings selector', async () => {
    await chooseLocale('compact', 'Deutsch');
    expect(get(localeStore)).toBe('de');
    expect(localStorage.getItem('lite_wallet_locale_v1')).toBe('de');
  });
});
