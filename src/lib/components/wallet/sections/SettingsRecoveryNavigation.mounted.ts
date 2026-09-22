// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '$lib/i18n';
import Settings from './Settings.svelte';
import { loadRuntimeAppInfo } from '$lib/utils/appInfo.js';

const walletService = vi.hoisted(() => ({
  getDlightSeedStatus: vi.fn(),
  getWalletRecoverySecrets: vi.fn(),
  setSessionTimeoutMinutes: vi.fn(),
  touchSessionActivity: vi.fn(),
}));

vi.mock('$lib/services/walletService', () => walletService);
vi.mock('$lib/utils/appInfo.js', () => ({
  loadRuntimeAppInfo: vi.fn().mockResolvedValue({ name: 'Verus Wallet', version: '0.1.0' }),
}));

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

describe('mounted recovery navigation', () => {
  beforeEach(() => {
    localStorage.clear();
    setLocale('en');
    walletService.getDlightSeedStatus.mockReset();
    walletService.getDlightSeedStatus.mockResolvedValue({
      configured: true,
      shieldedAddress: 'zs1syntheticprivateaddress',
    });
    walletService.getWalletRecoverySecrets.mockReset();
  });

  afterEach(() => {
    document.body.replaceChildren();
  });

  it('ends a failed version lookup with an unavailable summary', async () => {
    let rejectVersion!: (reason: Error) => void;
    vi.mocked(loadRuntimeAppInfo).mockReturnValueOnce(
      new Promise((_resolve, reject) => {
        rejectVersion = reject;
      })
    );
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(Settings, {
      target,
      props: { walletNetwork: 'mainnet', walletName: 'Fixture', walletSessionKey: 'fixture' },
    });
    await settle();
    expect(findButton('About and support')?.querySelector('[data-slot="skeleton"]')).not.toBeNull();
    rejectVersion(new Error('Version unavailable'));
    await settle();
    expect(findButton('About and support')?.textContent).toContain('Version unavailable');
    expect(target.textContent).not.toContain('Loading');
    await unmount(component);
  });

  it('prompts immediately and returns cancel to the initiating settings section', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(Settings, {
      target,
      props: {
        walletNetwork: 'testnet',
        walletName: 'Synthetic test wallet',
        walletSessionKey: 'mounted-settings-session',
      },
    });
    await settle();

    const displayLanguage = findButton('Display and language');
    expect(
      displayLanguage?.querySelector('svg')?.classList.contains('text-settings-muted-foreground')
    ).toBe(true);

    findButton('Profile and security')?.click();
    await settle();
    findButton('Recovery and keys')?.click();
    await settle();
    expect(document.body.querySelector('input[type="password"]')).not.toBeNull();
    findButton('Cancel')?.click();
    await settle();
    expect(target.textContent).toContain('Profile and security');
    expect(walletService.getWalletRecoverySecrets).not.toHaveBeenCalled();

    findButton('Back to settings')?.click();
    await settle();
    findButton('Private Verus')?.click();
    await settle();
    findButton('Recovery and keys')?.click();
    await settle();
    expect(document.body.querySelector('input[type="password"]')).not.toBeNull();
    findButton('Cancel')?.click();
    await settle();
    expect(target.textContent).toContain('Private Verus is set up');
    expect(walletService.getWalletRecoverySecrets).not.toHaveBeenCalled();

    await unmount(component);
  });
});
