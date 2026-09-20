// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '$lib/i18n';
import {
  clearDlightSetupSession,
  getDlightSetupOperation,
} from '$lib/utils/dlightSetupCoordinator';
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

function findIdentifier(value: string): HTMLElement | undefined {
  return [...document.body.querySelectorAll<HTMLElement>('.identifier-text')].find(
    (identifier) => identifier.getAttribute('title') === value || identifier.textContent === value
  );
}

function mountPrivate(
  onOpenRecovery = vi.fn(),
  walletSessionKey = 'mounted-private-verus-session',
  onBack = vi.fn()
) {
  const target = document.createElement('div');
  document.body.append(target);
  const component = mount(PrivateVerusSettings, {
    target,
    props: {
      walletNetwork: 'testnet',
      walletSessionKey,
      onBack,
      onOpenRecovery,
    },
  });
  return { component, onBack, onOpenRecovery };
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
    clearDlightSetupSession('mounted-private-verus-session');
    clearDlightSetupSession('dismissed-private-verus-session');
    clearDlightSetupSession('escaped-private-verus-session');
    clearDlightSetupSession('abandoned-create-session');
    clearDlightSetupSession('pending-create-session');
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
    const shieldedAddress = document.body.querySelector('[title="zs1syntheticprivateaddress"]');
    expect(shieldedAddress?.textContent).toBe('zs1synthetic…ivateaddress');
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

  it('refreshes the shielded address after initial setup', async () => {
    walletService.getDlightSeedStatus
      .mockResolvedValueOnce({ configured: false })
      .mockResolvedValueOnce({
        configured: true,
        shieldedAddress: 'zs1freshinitialaddress',
      });
    walletService.setupDlightSeed.mockResolvedValue({
      configured: true,
      requiresRelogin: false,
    });
    const { component } = mountPrivate();
    await settle();

    findButton('Reuse primary Secret Recovery Phrase')?.click();
    await settle();

    expect(walletService.getDlightSeedStatus).toHaveBeenCalledTimes(2);
    expect(findIdentifier('zs1freshinitialaddress')).toBeDefined();
    expect(document.body.textContent).toContain('Private Verus is ready to use.');

    await unmount(component);
  });

  it('invalidates the previous shielded address while replacing setup and renders the refreshed one', async () => {
    walletService.getDlightSeedStatus
      .mockResolvedValueOnce({
        configured: true,
        shieldedAddress: 'zs1oldreplacementaddress',
      })
      .mockResolvedValueOnce({
        configured: true,
        shieldedAddress: 'zs1newreplacementaddress',
      });
    let resolveSetup:
      ((value: { configured: boolean; requiresRelogin: boolean }) => void) | undefined;
    walletService.setupDlightSeed.mockReturnValue(
      new Promise((resolve) => {
        resolveSetup = resolve;
      })
    );
    const { component } = mountPrivate();
    await settle();

    expect(findIdentifier('zs1oldreplacementaddress')).toBeDefined();
    findButton('Advanced')?.click();
    await settle();
    findButton('Create new privacy recovery secret')?.click();
    await settle();
    resolveSetup?.({ configured: true, requiresRelogin: true });
    await settle();

    expect(findIdentifier('zs1oldreplacementaddress')).toBeUndefined();
    expect(findIdentifier('zs1newreplacementaddress')).toBeDefined();
    expect(document.body.textContent).toContain('Lock and unlock your wallet');

    await unmount(component);
  });

  it('returns from Advanced with the header back action without leaving Private Verus', async () => {
    walletService.getDlightSeedStatus.mockResolvedValue({
      configured: true,
      shieldedAddress: 'zs1advancednavigationaddress',
    });
    const { component, onBack } = mountPrivate();
    await settle();

    findButton('Advanced')?.click();
    await settle();
    expect(document.body.textContent).toContain('Replace Private Verus setup?');
    expect(findButton('Cancel')).toBeUndefined();

    findButton('Back to settings')?.click();
    await settle();
    expect(document.body.textContent).toContain('Private Verus is set up');
    expect(document.body.textContent).not.toContain('Replace Private Verus setup?');
    expect(onBack).not.toHaveBeenCalled();

    findButton('Back to settings')?.click();
    expect(onBack).toHaveBeenCalledOnce();

    await unmount(component);
  });

  it('reports backup copy success and failure truthfully', async () => {
    walletService.getDlightSeedStatus
      .mockResolvedValueOnce({ configured: false })
      .mockResolvedValueOnce({ configured: true, shieldedAddress: 'zs1copyfeedback' });
    walletService.setupDlightSeed.mockResolvedValue({
      configured: true,
      generatedSeedPhrase: 'alpha beta gamma synthetic backup only',
      requiresRelogin: false,
    });
    const writeText = vi
      .fn()
      .mockResolvedValueOnce(undefined)
      .mockRejectedValueOnce(new Error('clipboard denied'));
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    const { component } = mountPrivate();
    await settle();

    findButton('Create new privacy recovery secret')?.click();
    await settle();
    findButton('Copy phrase')?.click();
    await settle();
    expect(findButton('Copied')).toBeDefined();

    findButton('Copied')?.click();
    await settle();
    expect(document.body.textContent).toContain('Copy failed');
    expect(findButton('Copy phrase')).toBeDefined();

    await unmount(component);
  });

  it('ignores a late backup copy result after the backup sheet is dismissed', async () => {
    walletService.getDlightSeedStatus
      .mockResolvedValueOnce({ configured: false })
      .mockResolvedValueOnce({ configured: true, shieldedAddress: 'zs1latebackupcopy' });
    walletService.setupDlightSeed.mockResolvedValue({
      configured: true,
      generatedSeedPhrase: 'alpha beta gamma synthetic abandoned backup',
      requiresRelogin: false,
    });
    let resolveCopy: (() => void) | undefined;
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: {
        writeText: vi.fn(
          () =>
            new Promise<void>((resolve) => {
              resolveCopy = resolve;
            })
        ),
      },
    });
    const { component } = mountPrivate();
    await settle();

    findButton('Create new privacy recovery secret')?.click();
    await settle();
    findButton('Copy phrase')?.click();
    document.body.querySelector<HTMLButtonElement>('button[data-slot="sheet-close"]')?.click();
    await settle();
    expect(document.body.textContent).not.toContain('synthetic abandoned backup');

    resolveCopy?.();
    await settle();
    expect(findButton('Copied')).toBeUndefined();
    expect(document.body.textContent).not.toContain('Copy failed');

    await unmount(component);
  });

  it('does not replay an abandoned generated phrase after setup settles without a mounted view', async () => {
    walletService.getDlightSeedStatus
      .mockResolvedValueOnce({ configured: false })
      .mockResolvedValueOnce({
        configured: true,
        shieldedAddress: 'zs1abandonedcreateaddress',
      });
    let resolveSetup:
      | ((value: {
          configured: boolean;
          generatedSeedPhrase: string;
          requiresRelogin: boolean;
        }) => void)
      | undefined;
    walletService.setupDlightSeed.mockReturnValue(
      new Promise((resolve) => {
        resolveSetup = resolve;
      })
    );
    const walletSessionKey = 'abandoned-create-session';
    const firstMount = mountPrivate(vi.fn(), walletSessionKey);
    await settle();

    findButton('Create new privacy recovery secret')?.click();
    await settle();
    await unmount(firstMount.component);
    resolveSetup?.({
      configured: true,
      generatedSeedPhrase: 'synthetic phrase that must never replay',
      requiresRelogin: true,
    });
    await settle();

    expect(getDlightSetupOperation(walletSessionKey)).toBeNull();
    const secondMount = mountPrivate(vi.fn(), walletSessionKey);
    await settle();
    expect(document.body.textContent).not.toContain('synthetic phrase that must never replay');
    expect(document.body.textContent).not.toContain('Back up your Secret Recovery Phrase');
    expect(findIdentifier('zs1abandonedcreateaddress')).toBeDefined();
    expect(walletService.setupDlightSeed).toHaveBeenCalledOnce();

    await unmount(secondMount.component);
  });

  it('keeps create-new locked while pending across remount without redisplaying its phrase', async () => {
    walletService.getDlightSeedStatus
      .mockResolvedValueOnce({ configured: false })
      .mockResolvedValueOnce({
        configured: true,
        shieldedAddress: 'zs1pendingcreateaddress',
      });
    let resolveSetup:
      | ((value: {
          configured: boolean;
          generatedSeedPhrase: string;
          requiresRelogin: boolean;
        }) => void)
      | undefined;
    walletService.setupDlightSeed.mockReturnValue(
      new Promise((resolve) => {
        resolveSetup = resolve;
      })
    );
    const walletSessionKey = 'pending-create-session';
    const firstMount = mountPrivate(vi.fn(), walletSessionKey);
    await settle();
    findButton('Create new privacy recovery secret')?.click();
    await settle();
    await unmount(firstMount.component);

    const secondMount = mountPrivate(vi.fn(), walletSessionKey);
    await settle();
    expect(findButton('Create new privacy recovery secret')).toBeUndefined();
    expect(walletService.setupDlightSeed).toHaveBeenCalledOnce();

    resolveSetup?.({
      configured: true,
      generatedSeedPhrase: 'synthetic pending phrase that must not replay',
      requiresRelogin: false,
    });
    await settle();
    expect(document.body.textContent).not.toContain(
      'synthetic pending phrase that must not replay'
    );
    expect(document.body.textContent).not.toContain('Back up your Secret Recovery Phrase');
    expect(findIdentifier('zs1pendingcreateaddress')).toBeDefined();
    expect(walletService.setupDlightSeed).toHaveBeenCalledOnce();

    await unmount(secondMount.component);
  });

  it.each([
    ['close button', 'dismissed-private-verus-session'],
    ['Escape', 'escaped-private-verus-session'],
  ])(
    'keeps an import write locked across %s dismissal and a same-session remount',
    async (dismissMethod, walletSessionKey) => {
      walletService.getDlightSeedStatus
        .mockResolvedValueOnce({ configured: false })
        .mockResolvedValueOnce({
          configured: true,
          shieldedAddress: 'zs1settledafterdismissal',
        });
      let resolveSetup:
        ((value: { configured: boolean; requiresRelogin: boolean }) => void) | undefined;
      walletService.setupDlightSeed.mockReturnValue(
        new Promise((resolve) => {
          resolveSetup = resolve;
        })
      );
      const firstMount = mountPrivate(vi.fn(), walletSessionKey);
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
      [...document.body.querySelectorAll<HTMLButtonElement>('button')]
        .find((button) => button.textContent?.trim() === 'Import privacy recovery secret')
        ?.click();
      await settle();
      expect(walletService.setupDlightSeed).toHaveBeenCalledOnce();

      if (dismissMethod === 'Escape') {
        document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
      } else {
        document.body.querySelector<HTMLButtonElement>('button[data-slot="sheet-close"]')?.click();
      }
      await settle();
      expect(document.body.querySelector('textarea')).toBeNull();
      findButton('Create new privacy recovery secret')?.click();
      expect(walletService.setupDlightSeed).toHaveBeenCalledOnce();

      await unmount(firstMount.component);
      const secondMount = mountPrivate(vi.fn(), walletSessionKey);
      await settle();
      expect(walletService.setupDlightSeed).toHaveBeenCalledOnce();
      expect(findButton('Create new privacy recovery secret')).toBeUndefined();

      resolveSetup?.({ configured: true, requiresRelogin: false });
      await settle();
      expect(findIdentifier('zs1settledafterdismissal')).toBeDefined();
      expect(walletService.setupDlightSeed).toHaveBeenCalledOnce();

      await unmount(secondMount.component);
    }
  );

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
