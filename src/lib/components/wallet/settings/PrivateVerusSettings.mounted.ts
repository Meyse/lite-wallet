// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '$lib/i18n';
import PrivateVerusSettings from './PrivateVerusSettings.svelte';

const walletService = vi.hoisted(() => ({
  getDlightSeedStatus: vi.fn(),
  setupDlightSeed: vi.fn(),
}));

vi.mock('$lib/services/walletService', () => walletService);

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

function mountPrivate(onOpenRecovery = vi.fn()) {
  const target = document.createElement('div');
  document.body.append(target);
  const component = mount(PrivateVerusSettings, {
    target,
    props: {
      walletNetwork: 'testnet',
      onBack: vi.fn(),
      onOpenRecovery,
    },
  });
  return { component, onOpenRecovery };
}

describe('mounted Private Verus settings', () => {
  beforeEach(() => {
    setLocale('en');
    walletService.getDlightSeedStatus.mockReset();
    walletService.setupDlightSeed.mockReset();
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
    });
  });

  afterEach(() => {
    document.body.replaceChildren();
  });

  it('does not enable replacement actions when status loading fails', async () => {
    walletService.getDlightSeedStatus.mockRejectedValue(new Error('offline'));
    const { component } = mountPrivate();
    await settle();

    expect(document.body.textContent).toContain('Status unavailable');
    expect(findButton('Reuse primary Secret Recovery Phrase')).toBeUndefined();
    expect(findButton('Create new privacy recovery secret')).toBeUndefined();

    await unmount(component);
  });

  it('shows configured network and address state and opens recovery in context', async () => {
    walletService.getDlightSeedStatus.mockResolvedValue({
      configured: true,
      shieldedAddress: 'zs1syntheticprivateaddress',
    });
    const { component, onOpenRecovery } = mountPrivate();
    await settle();

    expect(document.body.textContent).toContain('Private Verus is set up');
    expect(document.body.textContent).toContain('Testnet');
    expect(document.body.textContent).toContain('zs1syntheticprivateaddress');
    findButton('Recovery and keys')?.click();
    expect(onOpenRecovery).toHaveBeenCalledOnce();

    await unmount(component);
  });

  it('maps primary and import validation failures to actionable messages', async () => {
    walletService.getDlightSeedStatus.mockResolvedValue({ configured: false });
    walletService.setupDlightSeed
      .mockRejectedValueOnce({ type: 'InvalidSeedPhrase' })
      .mockRejectedValueOnce({ type: 'InvalidImportText' });
    const { component } = mountPrivate();
    await settle();

    findButton('Reuse primary Secret Recovery Phrase')?.click();
    await settle();
    expect(document.body.textContent).toContain('cannot be reused for Private Verus');

    findButton('Import privacy recovery secret')?.click();
    await settle();
    const textarea = document.body.querySelector<HTMLTextAreaElement>('textarea');
    if (textarea) {
      textarea.value = 'invalid synthetic input';
      textarea.dispatchEvent(new InputEvent('input', { bubbles: true }));
    }
    await settle();
    [...document.body.querySelectorAll<HTMLButtonElement>('button')]
      .find((button) => button.textContent?.trim() === 'Import privacy recovery secret')
      ?.click();
    await settle();
    expect(document.body.textContent).toContain(
      'Enter a valid Secret Recovery Phrase or Private Verus spending key.'
    );

    findButton('Cancel')?.click();
    await settle();
    await unmount(component);
  });

  it('requires backup acknowledgement and never reruns setup when the backup sheet closes', async () => {
    walletService.getDlightSeedStatus.mockResolvedValue({ configured: false });
    walletService.setupDlightSeed.mockResolvedValue({
      configured: true,
      generatedSeedPhrase: 'alpha beta gamma synthetic backup only',
      requiresRelogin: true,
    });
    const { component } = mountPrivate();
    await settle();

    findButton('Create new privacy recovery secret')?.click();
    await settle();
    expect(document.body.textContent).toContain('alpha beta gamma synthetic backup only');
    const done = findButton('Done');
    expect(done?.disabled).toBe(true);

    document.body.querySelector<HTMLButtonElement>('[data-slot="checkbox"]')?.click();
    await settle();
    expect(done?.disabled).toBe(false);
    done?.click();
    await settle();

    expect(document.body.textContent).not.toContain('alpha beta gamma synthetic backup only');
    expect(walletService.setupDlightSeed).toHaveBeenCalledOnce();

    await unmount(component);
  });

  it('clears imports on dismissal and suppresses duplicate submissions', async () => {
    walletService.getDlightSeedStatus.mockResolvedValue({ configured: false });
    let resolveSetup:
      ((value: { configured: boolean; requiresRelogin: boolean }) => void) | undefined;
    walletService.setupDlightSeed.mockReturnValue(
      new Promise((resolve) => {
        resolveSetup = resolve;
      })
    );
    const { component } = mountPrivate();
    await settle();

    findButton('Import privacy recovery secret')?.click();
    await settle();
    const textarea = document.body.querySelector<HTMLTextAreaElement>('textarea');
    expect(textarea).not.toBeNull();
    if (textarea) {
      textarea.value = 'synthetic imported secret';
      textarea.dispatchEvent(new InputEvent('input', { bubbles: true }));
    }
    await settle();
    const importButton = [...document.body.querySelectorAll<HTMLButtonElement>('button')].find(
      (button) => button.textContent?.trim() === 'Import privacy recovery secret'
    );
    importButton?.click();
    importButton?.click();
    expect(walletService.setupDlightSeed).toHaveBeenCalledOnce();

    findButton('Cancel')?.click();
    await settle();
    expect(document.body.querySelector('textarea')).toBeNull();

    resolveSetup?.({ configured: true, requiresRelogin: false });
    await settle();
    expect(document.body.textContent).not.toContain('synthetic imported secret');

    await unmount(component);
  });
});
