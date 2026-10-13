// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '$lib/i18n';
import StepperLayout from './StepperLayout.svelte';

function networkButtons(): HTMLButtonElement[] {
  return [...document.body.querySelectorAll<HTMLButtonElement>('button[aria-pressed]')];
}

describe('mounted wallet network choice', () => {
  beforeEach(() => {
    setLocale('en');
  });

  afterEach(() => {
    document.body.replaceChildren();
  });

  it('offers private testnet next to mainnet and testnet', async () => {
    const onNetworkChange = vi.fn();
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(StepperLayout, {
      target,
      props: {
        currentStep: 1,
        totalSteps: 5,
        showNetworkToggle: true,
        network: 'mainnet',
        onNetworkChange,
      },
    });
    await tick();

    expect(networkButtons().map((button) => button.textContent?.trim())).toEqual([
      'Mainnet',
      'Testnet',
      'Private testnet',
    ]);
    expect(networkButtons().map((button) => button.getAttribute('aria-pressed'))).toEqual([
      'true',
      'false',
      'false',
    ]);

    networkButtons()[2].click();
    expect(onNetworkChange).toHaveBeenCalledWith('privatetestnet');

    unmount(component);
  });
});
