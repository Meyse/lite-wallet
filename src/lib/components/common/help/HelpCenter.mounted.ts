import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '$lib/i18n';
import HelpCenter from './HelpCenter.svelte';
import HelpNavigationHarness from './__tests__/HelpNavigationHarness.svelte';
import HelpCenterLink from '../HelpCenterLink.svelte';

vi.mock('$lib/utils/externalLinks', () => ({
  openCommunityHangout: vi.fn(),
  openTrustedExternalUrl: vi.fn(),
}));
vi.stubGlobal(
  'ResizeObserver',
  class {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
);
HTMLElement.prototype.scrollTo = () => {};
HTMLElement.prototype.hasPointerCapture = () => false;
HTMLElement.prototype.setPointerCapture = () => {};
HTMLElement.prototype.releasePointerCapture = () => {};
let target: HTMLDivElement;
let component: ReturnType<typeof mount>;
async function settle() {
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 30));
  await tick();
}
function button(text: string) {
  const result = [...document.querySelectorAll<HTMLButtonElement>('button')].find(
    (item) => item.textContent?.trim() === text
  );
  if (!result) throw new Error(`Missing button: ${text}`);
  return result;
}
beforeEach(() => {
  setLocale('en');
  target = document.createElement('div');
  document.body.append(target);
});
afterEach(async () => {
  if (component) await unmount(component);
  target.remove();
  setLocale('en');
});

describe('Help center navigation', () => {
  it('starts with categories, preserves a category on reopen, and returns through the category to home', async () => {
    component = mount(HelpNavigationHarness, { target });
    await settle();
    button('Open help').click();
    await settle();
    expect(document.querySelectorAll('[data-help-categories] button')).toHaveLength(6);
    expect(document.querySelector('[data-help-results]')).toBeNull();
    button('Sending and receiving').click();
    await settle();
    expect(document.querySelector('[data-help-search]')).toBeNull();
    expect(document.querySelector('[data-help-results]')?.textContent).toContain(
      'Sending a payment'
    );
    button('Back to wallet').click();
    await settle();
    button('Open help').click();
    await settle();
    expect(document.activeElement?.textContent?.trim()).toBe('Sending and receiving');
    button('Sending a payment').click();
    await settle();
    expect(document.querySelector('[data-help-search]')).toBeNull();
    button('Back to wallet').click();
    await settle();
    button('Open help').click();
    await settle();
    expect(document.activeElement?.textContent?.trim()).toBe('Sending a payment');
    button('Back to sending and receiving').click();
    await settle();
    expect(document.querySelector('[data-help-results]')?.textContent).toContain(
      'Sending a payment'
    );
    button('Back to Help').click();
    await settle();
    expect(document.querySelectorAll('[data-help-categories] button')).toHaveLength(6);
    expect(document.querySelector('[data-help-search]')).not.toBeNull();
    expect(document.activeElement?.textContent?.trim()).toBe('What do you need help with?');
  });

  it('opens a search result and related article, then returns to the same results', async () => {
    component = mount(HelpCenter, {
      target,
      props: { onClose: vi.fn(), backLabel: 'Back to wallet' },
    });
    await settle();
    const input = target.querySelector<HTMLInputElement>('input');
    if (!input) throw new Error('Missing search field');
    input.value = 'stuck';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await settle();
    const result = target.querySelector<HTMLButtonElement>('[data-help-results] button');
    if (!result) throw new Error('Missing search result');
    expect(result.textContent).toContain('My transfer has not arrived');
    result.click();
    await settle();
    expect(target.querySelector('[data-help-article="pending"]')).not.toBeNull();
    expect(target.querySelector('[data-help-search]')).toBeNull();
    expect(document.activeElement?.tagName).toBe('H2');
    button('How cross-chain transfers work').click();
    await settle();
    button('Back to article').click();
    await settle();
    expect(target.querySelector('[data-help-article="pending"]')).not.toBeNull();
    button('Back to results').click();
    await settle();
    expect(required(target.querySelector<HTMLInputElement>('[data-help-search]')).value).toBe(
      'stuck'
    );
    expect(target.querySelector('[data-help-results]')?.textContent).toContain(
      'My transfer has not arrived'
    );
  });

  it('browses localized topics and recovers from an empty search', async () => {
    setLocale('nl');
    component = mount(HelpCenter, {
      target,
      props: { onClose: vi.fn(), backLabel: 'Terug naar wallet' },
    });
    await settle();
    button('VerusID en profielen').click();
    await settle();
    expect(target.textContent).toContain('Wat wordt openbaar in mijn profiel?');
    expect(target.querySelector('[data-help-search]')).toBeNull();
    button('Terug naar Help').click();
    await settle();
    const input = target.querySelector<HTMLInputElement>('input');
    if (!input) throw new Error('Missing search field');
    input.value = 'zz-no-results';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await settle();
    expect(target.textContent).toContain('Geen artikelen gevonden');
    const clear = target.querySelector<HTMLButtonElement>(
      'button[aria-label="Zoekopdracht wissen"]'
    );
    if (!clear) throw new Error('Missing clear search button');
    clear.click();
    await settle();
    expect(input.value).toBe('');
    expect(target.querySelectorAll('[data-help-categories] button')).toHaveLength(6);
    expect(target.querySelector('[data-help-results]')).toBeNull();
  });

  it('links only the screen name and explains the unlock requirement without navigating', async () => {
    const close = vi.fn();
    component = mount(HelpCenter, {
      target,
      props: { onClose: close, backLabel: 'Back to sign in', initialArticleId: 'assets' },
    });
    await settle();
    const link = required(
      target.querySelector<HTMLButtonElement>('[data-help-destination="manage-assets"]')
    );
    expect(link.textContent).toBe('Manage assets');
    expect(link.parentElement?.textContent).toContain('controls which currencies appear');
    link.click();
    await settle();
    expect(target.querySelector('[role="status"]')?.textContent).toContain('Unlock your wallet');
    expect(target.querySelector('[data-help-article="assets"]')).not.toBeNull();
    expect(close).not.toHaveBeenCalled();
    button('Back to getting started').click();
    await settle();
    expect(target.querySelector('[data-help-results]')?.textContent).toContain(
      'Adding or hiding an asset'
    );
    button('Back to Help').click();
    await settle();
    expect(target.querySelector('[data-help-categories]')).not.toBeNull();
  });

  it('retains article, scroll, search and related-article history after leaving for a screen', async () => {
    component = mount(HelpNavigationHarness, { target });
    await settle();
    button('Open help').click();
    await settle();
    const input = required(document.querySelector<HTMLInputElement>('[data-help-search]'));
    input.value = 'assets';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await settle();
    button('Adding or hiding an asset').click();
    await settle();
    button('Choosing the right network').click();
    await settle();
    button('Back to article').click();
    await settle();
    const viewport = required(
      required(document.querySelector('[data-help-article]')).closest<HTMLDivElement>(
        '[data-slot="scroll-area-viewport"]'
      )
    );
    viewport.scrollTop = 143;
    viewport.dispatchEvent(new Event('scroll'));
    required(
      document.querySelector<HTMLButtonElement>('[data-help-destination="manage-assets"]')
    ).click();
    await settle();
    expect(document.querySelector('[data-help-center]')).toBeNull();
    expect(target.querySelector('[data-destination]')?.textContent).toBe('manage-assets');
    button('Open help').click();
    await settle();
    expect(document.querySelector('[data-help-article="assets"]')).not.toBeNull();
    expect(
      required(
        required(document.querySelector('[data-help-article]')).closest(
          '[data-slot="scroll-area-viewport"]'
        )
      ).scrollTop
    ).toBe(143);
    expect(document.activeElement?.getAttribute('data-help-destination')).toBe('manage-assets');
    button('Back to results').click();
    await settle();
    expect(required(document.querySelector<HTMLInputElement>('[data-help-search]')).value).toBe(
      'assets'
    );
  });

  it('opens pre-unlock help and closes with Escape without losing its host', async () => {
    component = mount(HelpCenterLink, {
      target,
      props: { linkText: 'Get help', backLabel: 'Back to welcome' },
    });
    await settle();
    const trigger = button('Get help');
    trigger.focus();
    trigger.click();
    await settle();
    expect(document.querySelector('[role="dialog"] [data-help-center]')).not.toBeNull();
    expect(document.activeElement?.matches('[data-help-search]')).toBe(true);
    document.activeElement?.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })
    );
    await settle();
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });
});

function required<T>(value: T | null | undefined): T {
  if (value === null || value === undefined) throw new Error('Expected test element or value');
  return value;
}
