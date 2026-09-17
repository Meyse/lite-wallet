// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '$lib/i18n';
import type { WalletRecoverySecretsResult } from '$lib/types/wallet';
import RecoveryKeysSettings from './RecoveryKeysSettings.svelte';

const walletService = vi.hoisted(() => ({
  getWalletRecoverySecrets: vi.fn(),
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

const syntheticSecrets: WalletRecoverySecretsResult = {
  primarySecretKind: 'seed_text',
  primarySecret: 'alpha beta gamma delta synthetic only',
  verusWif: 'synthetic-verus-wif',
  btcWif: 'synthetic-bitcoin-wif',
  ethPrivateKey: 'synthetic-ethereum-private-key',
  verusAddress: 'RsyntheticVerusAddress',
  btcAddress: 'bc1syntheticbitcoinaddress',
  ethAddress: '0x0000000000000000000000000000000000000001',
  dlightSecret: 'synthetic-private-verus-spending-key',
  dlightSecretKind: 'spending_key',
  dlightShieldedAddress: 'zs1syntheticprivateaddress',
  dlightDerivedSpendingKey: null,
};

async function settle(): Promise<void> {
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 0));
}

function findButton(label: string): HTMLButtonElement | undefined {
  return [...document.body.querySelectorAll<HTMLButtonElement>('button')].find((button) =>
    button.textContent?.includes(label)
  );
}

function mountRecovery(onBack = vi.fn()) {
  const target = document.createElement('div');
  document.body.append(target);
  const component = mount(RecoveryKeysSettings, {
    target,
    props: {
      walletNetwork: 'testnet',
      walletName: 'Synthetic test wallet',
      backLabel: 'Profile and security',
      onBack,
    },
  });
  return { component, onBack };
}

async function authenticate(): Promise<void> {
  const input = document.body.querySelector<HTMLInputElement>('input[type="password"]');
  expect(input).not.toBeNull();
  if (input) {
    input.value = 'synthetic-password';
    input.dispatchEvent(new InputEvent('input', { bubbles: true }));
  }
  await settle();
  findButton('Reveal')?.click();
  await settle();
}

describe('mounted recovery settings', () => {
  beforeEach(() => {
    setLocale('en');
    walletService.getWalletRecoverySecrets.mockReset();
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
    });
  });

  afterEach(() => {
    document.body.replaceChildren();
  });

  it('prompts immediately and cancels a pending request without accepting its late result', async () => {
    let resolveRequest: ((value: WalletRecoverySecretsResult) => void) | undefined;
    walletService.getWalletRecoverySecrets.mockReturnValue(
      new Promise((resolve) => {
        resolveRequest = resolve;
      })
    );
    const { component, onBack } = mountRecovery();

    expect(document.body.querySelector('input[type="password"]')).not.toBeNull();
    await authenticate();
    findButton('Cancel')?.click();
    await settle();
    expect(onBack).toHaveBeenCalledOnce();

    resolveRequest?.(syntheticSecrets);
    await settle();
    expect(document.body.textContent).not.toContain('Primary secret');

    await unmount(component);
  });

  it('keeps secrets masked, maps the Private Verus spending key, and creates an exact local QR', async () => {
    walletService.getWalletRecoverySecrets.mockResolvedValue(syntheticSecrets);
    const { component } = mountRecovery();
    await authenticate();

    expect(document.body.textContent).toContain('Primary secret');
    expect(document.body.textContent).toContain('Privacy secret');
    expect(document.body.textContent).not.toContain(syntheticSecrets.primarySecret);

    findButton('Derived keys')?.click();
    await settle();
    expect(document.body.textContent).toContain('Derived private spending key');
    expect(document.body.textContent).not.toContain('synthetic-private-verus-spending-key');

    document.body
      .querySelector<HTMLButtonElement>('button[aria-label="Show QR code for Verus WIF"]')
      ?.click();
    await vi.waitFor(() => {
      expect(document.body.querySelector('svg[role="img"]')).not.toBeNull();
    });
    expect(document.body.textContent).toContain('This QR code contains your private key.');
    expect(document.body.textContent).not.toContain('synthetic-verus-wif');

    findButton('Back to keys')?.click();
    await settle();
    expect(document.body.querySelector('svg[role="img"]')).toBeNull();
    document.body.querySelector<HTMLButtonElement>('button[aria-label="Close"]')?.click();
    await settle();

    await unmount(component);
  });

  it('omits Private Verus sections and keys when the backend has no private data', async () => {
    walletService.getWalletRecoverySecrets.mockResolvedValue({
      ...syntheticSecrets,
      dlightSecret: null,
      dlightSecretKind: null,
      dlightShieldedAddress: null,
      dlightDerivedSpendingKey: null,
    });
    const { component } = mountRecovery();
    await authenticate();

    expect(document.body.textContent).not.toContain('Privacy secret');
    findButton('Derived keys')?.click();
    await settle();
    expect(document.body.textContent).not.toContain('Derived private spending key');
    expect(document.body.querySelectorAll('button[aria-label^="Show QR code for"]').length).toBe(3);

    document.body.querySelector<HTMLButtonElement>('button[aria-label="Close"]')?.click();
    await settle();
    await unmount(component);
  });

  it('distinguishes password and secure-storage errors without exposing recovery data', async () => {
    walletService.getWalletRecoverySecrets
      .mockRejectedValueOnce({ type: 'InvalidPassword' })
      .mockRejectedValueOnce({ type: 'SecureStorageUnavailable' });
    const { component } = mountRecovery();

    await authenticate();
    expect(document.body.textContent).toContain('Password is incorrect.');
    await authenticate();
    expect(document.body.textContent).toContain('Secure wallet storage is unavailable');
    expect(document.body.textContent).not.toContain('Primary secret');

    await unmount(component);
  });
});
