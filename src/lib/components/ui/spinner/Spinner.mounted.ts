import { mount, unmount } from 'svelte';
import { get } from 'svelte/store';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { i18nStore, setLocale, SUPPORTED_LOCALES } from '$lib/i18n';
import DelayedStatus from '$lib/components/common/DelayedStatus.svelte';
import Spinner from './spinner.svelte';

let component: ReturnType<typeof mount>;

afterEach(async () => {
  await unmount(component);
  document.body.replaceChildren();
  setLocale('en');
  vi.useRealTimers();
});

describe('spinner accessibility', () => {
  it('leaves the status announcement to its parent by default', () => {
    component = mount(Spinner, { target: document.body });
    const icon = document.querySelector('svg');
    expect(icon?.getAttribute('aria-hidden')).toBe('true');
    expect(icon?.hasAttribute('aria-label')).toBe(false);
    expect(document.querySelector('[role="status"]')).toBeNull();
  });

  it.each(SUPPORTED_LOCALES)('supports a localized standalone status in %s', (locale) => {
    setLocale(locale);
    const label = get(i18nStore).t('common.loading');
    component = mount(Spinner, {
      target: document.body,
      props: { 'aria-label': label, class: 'size-10', absoluteStrokeWidth: true },
    });
    const status = document.querySelector('[role="status"]');
    expect(status?.getAttribute('aria-label')).toBe(label);
    expect(status?.getAttribute('aria-hidden')).not.toBe('true');
    expect(status?.classList.contains('size-10')).toBe(true);
    expect(status?.classList.contains('size-4')).toBe(false);
  });

  it('supports a status named by existing localized text', () => {
    const label = document.createElement('span');
    label.id = 'loading-label';
    label.textContent = get(i18nStore).t('common.loading');
    document.body.append(label);
    component = mount(Spinner, {
      target: document.body,
      props: { 'aria-labelledby': 'loading-label' },
    });
    expect(document.querySelector('[role="status"]')?.getAttribute('aria-labelledby')).toBe(
      'loading-label'
    );
    expect(document.querySelector('svg')?.getAttribute('aria-hidden')).not.toBe('true');
  });

  it('keeps a delayed localized status as a single announcement', async () => {
    vi.useFakeTimers();
    setLocale('nl');
    const label = get(i18nStore).t('wallet.loading.fetchingAddresses');
    component = mount(DelayedStatus, {
      target: document.body,
      props: { active: true, label },
    });
    await vi.advanceTimersByTimeAsync(1_100);
    expect(document.querySelectorAll('[role="status"]')).toHaveLength(1);
    expect(document.querySelector('[role="status"]')?.textContent?.trim()).toBe(label);
    expect(document.querySelector('svg')?.getAttribute('aria-hidden')).toBe('true');
  });
});
