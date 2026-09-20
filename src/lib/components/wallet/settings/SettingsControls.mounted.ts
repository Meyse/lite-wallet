// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '$lib/i18n';
import AppearanceSelector from './AppearanceSelector.svelte';
import ProfileSecuritySettings from './ProfileSecuritySettings.svelte';

describe('mounted settings controls', () => {
  beforeEach(() => {
    localStorage.clear();
    setLocale('en');
  });

  afterEach(() => {
    document.body.replaceChildren();
  });

  it('supports click and arrow-key appearance selection', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    const onChange = vi.fn();
    const component = mount(AppearanceSelector, {
      target,
      props: {
        value: 'light',
        label: 'Appearance',
        systemDescription: 'Use your device appearance.',
        options: [
          { value: 'light', label: 'Light' },
          { value: 'dark', label: 'Dark' },
          { value: 'system', label: 'System' },
        ],
        onChange,
      },
    });

    const controls = target.querySelectorAll<HTMLButtonElement>('[role="radio"]');
    expect(
      [...controls].every((control) =>
        control.querySelector('span[aria-hidden="true"]')?.classList.contains('size-[13px]')
      )
    ).toBe(true);
    controls[1]?.click();
    expect(onChange).toHaveBeenCalledWith('dark');

    controls[0]?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }));
    expect(onChange).toHaveBeenCalledWith('system');

    await unmount(component);
  });

  it('keeps the supported auto-lock durations in the shared compact menu', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    const onSetAutoLockMinutes = vi.fn();
    const component = mount(ProfileSecuritySettings, {
      target,
      props: {
        autoLockMinutes: 15,
        autoLockOptions: [5, 15, 30, 60],
        onSetAutoLockMinutes,
        onBack: vi.fn(),
        onOpenRecovery: vi.fn(),
      },
    });

    const trigger = target.querySelector<HTMLButtonElement>(
      'button[aria-label="Auto-lock after inactivity"]'
    );
    expect(trigger).not.toBeNull();
    await tick();
    trigger?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0 }));
    await tick();
    await new Promise((resolve) => setTimeout(resolve, 0));

    const items = [...document.body.querySelectorAll<HTMLElement>('[role="menuitemradio"]')];
    expect(items.map((item) => item.textContent?.trim())).toEqual([
      '5 min',
      '15 min',
      '30 min',
      '60 min',
    ]);
    items[2]?.click();
    expect(onSetAutoLockMinutes).toHaveBeenCalledWith(30);

    await unmount(component);
  });

  it('keeps the contextual back action sized to its label', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(ProfileSecuritySettings, {
      target,
      props: {
        autoLockMinutes: 15,
        autoLockOptions: [5, 15, 30, 60],
        onSetAutoLockMinutes: vi.fn(),
        onBack: vi.fn(),
        onOpenRecovery: vi.fn(),
      },
    });

    const backButton = [...target.querySelectorAll<HTMLButtonElement>('button')].find(
      (button) => button.textContent?.trim() === 'Back to settings'
    );
    expect(backButton?.classList.contains('w-fit')).toBe(true);

    await unmount(component);
  });
});
