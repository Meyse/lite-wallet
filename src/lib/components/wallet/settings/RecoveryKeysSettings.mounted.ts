// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '$lib/i18n';
import type { WalletRecoverySecretsResult } from '$lib/types/wallet';
import RecoveryKeysSettings from './RecoveryKeysSettings.svelte';

const walletService = vi.hoisted(() => ({
  getWalletRecoverySecrets: vi.fn(),
}));
const qrEncoder = vi.hoisted(() => ({
  encodeQrCode: vi.fn(),
}));

vi.mock('$lib/services/walletService', () => walletService);
vi.mock('$lib/utils/qrCode', () => qrEncoder);

class ResizeObserverStub {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}

Object.defineProperty(globalThis, 'ResizeObserver', {
  configurable: true,
  value: ResizeObserverStub,
});

const syntheticPrimaryPhrase =
  'alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo sierra tango uniform victor whiskey xray';
const syntheticVerusWif = 'synthetic-verus-private-key-material'.padEnd(52, 'x');
const syntheticBitcoinWif = 'synthetic-bitcoin-private-key-material'.padEnd(52, 'x');
const syntheticEthereumKey = 'synthetic-ethereum-private-key-material'.padEnd(64, '0');
const syntheticDlightSpendingKey = 'synthetic-private-verus-spending-key-material'.padEnd(64, 'z');

const syntheticSecrets: WalletRecoverySecretsResult = {
  primarySecretKind: 'seed_text',
  primarySecret: syntheticPrimaryPhrase,
  verusWif: syntheticVerusWif,
  btcWif: syntheticBitcoinWif,
  ethPrivateKey: syntheticEthereumKey,
  verusAddress: 'RsyntheticVerusAddress',
  btcAddress: 'bc1syntheticbitcoinaddress',
  ethAddress: '0x0000000000000000000000000000000000000001',
  dlightSecret: syntheticDlightSpendingKey,
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
      backLabel: 'Back to profile and security',
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
    qrEncoder.encodeQrCode.mockReset();
    qrEncoder.encodeQrCode.mockResolvedValue({
      size: 1,
      modules: Uint8Array.of(1),
      path: 'M4 4h1v1h-1z',
      quietZone: 4,
      viewBoxSize: 9,
    });
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
    expect(document.body.textContent).not.toContain(syntheticDlightSpendingKey);

    document.body
      .querySelector<HTMLButtonElement>('button[aria-label="Show QR code for Verus WIF"]')
      ?.click();
    await vi.waitFor(() => {
      expect(document.body.querySelector('svg[role="img"]')).not.toBeNull();
    });
    expect(document.body.textContent).toContain('This QR code contains your private key.');
    expect(document.body.textContent).not.toContain(syntheticVerusWif);

    findButton('Back to keys')?.click();
    await settle();
    expect(document.body.querySelector('svg[role="img"]')).toBeNull();
    document.body.querySelector<HTMLButtonElement>('button[data-slot="sheet-close"]')?.click();
    await settle();

    await unmount(component);
  });

  it('renders revealed recovery phrases and full-length keys without clipping', async () => {
    expect(syntheticPrimaryPhrase.split(' ')).toHaveLength(24);
    expect(syntheticVerusWif).toHaveLength(52);
    expect(syntheticEthereumKey).toHaveLength(64);
    walletService.getWalletRecoverySecrets.mockResolvedValue(syntheticSecrets);
    const { component } = mountRecovery();
    await authenticate();

    findButton('Primary secret')?.click();
    await settle();
    document.body
      .querySelector<HTMLButtonElement>('button[aria-label="Reveal Primary secret material"]')
      ?.click();
    await settle();
    const primaryEntry = document.body.querySelector<HTMLElement>(
      '[data-recovery-entry="primarySecret"]'
    );
    const primaryValue = primaryEntry?.querySelector<HTMLElement>('.identifier-text');
    expect(primaryEntry?.dataset.revealed).toBe('true');
    expect(primaryEntry?.classList.contains('h-[74px]')).toBe(false);
    expect(primaryValue?.textContent).toBe(syntheticPrimaryPhrase);
    expect(primaryValue?.className).not.toContain('truncate');
    expect(primaryValue?.className).not.toContain('whitespace-nowrap');

    document.body.querySelector<HTMLButtonElement>('button[data-slot="sheet-close"]')?.click();
    await settle();
    findButton('Derived keys')?.click();
    await settle();
    document.body
      .querySelector<HTMLButtonElement>('button[aria-label="Reveal Verus WIF"]')
      ?.click();
    await settle();
    const keyEntry = document.body.querySelector<HTMLElement>('[data-recovery-entry="verusWif"]');
    const keyValue = keyEntry?.querySelector<HTMLElement>('.identifier-text');
    expect(keyEntry?.dataset.revealed).toBe('true');
    expect(keyEntry?.classList.contains('h-[74px]')).toBe(false);
    expect(keyValue?.textContent).toBe(syntheticVerusWif);
    expect(keyValue?.className).not.toContain('truncate');
    expect(keyValue?.className).not.toContain('whitespace-nowrap');

    await unmount(component);
  });

  it('shows a visible copy failure and allows its card to grow', async () => {
    walletService.getWalletRecoverySecrets.mockResolvedValue(syntheticSecrets);
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: vi.fn().mockRejectedValue(new Error('clipboard denied')) },
    });
    const { component } = mountRecovery();
    await authenticate();
    findButton('Derived keys')?.click();
    await settle();
    document.body.querySelector<HTMLButtonElement>('button[aria-label="Copy Verus WIF"]')?.click();
    await settle();

    const feedback = document.body.querySelector<HTMLElement>('[role="status"]');
    const keyEntry = document.body.querySelector<HTMLElement>('[data-recovery-entry="verusWif"]');
    expect(feedback?.textContent).toContain('Could not copy this value. Try again.');
    expect(feedback?.className).not.toContain('sr-only');
    expect(keyEntry?.classList.contains('h-[74px]')).toBe(false);

    await unmount(component);
  });

  it.each(['detail close', 'unmount'])(
    'ignores a late clipboard completion after %s',
    async (exitMethod) => {
      walletService.getWalletRecoverySecrets.mockResolvedValue(syntheticSecrets);
      let rejectCopy: ((reason: Error) => void) | undefined;
      Object.defineProperty(navigator, 'clipboard', {
        configurable: true,
        value: {
          writeText: vi.fn(
            () =>
              new Promise<void>((_resolve, reject) => {
                rejectCopy = reject;
              })
          ),
        },
      });
      const { component } = mountRecovery();
      await authenticate();
      findButton('Derived keys')?.click();
      await settle();
      document.body
        .querySelector<HTMLButtonElement>('button[aria-label="Copy Verus WIF"]')
        ?.click();

      let didUnmount = false;
      if (exitMethod === 'unmount') {
        await unmount(component);
        didUnmount = true;
      } else {
        document.body.querySelector<HTMLButtonElement>('button[data-slot="sheet-close"]')?.click();
        await settle();
      }

      rejectCopy?.(new Error('late clipboard failure'));
      await settle();
      expect(document.body.textContent).not.toContain('Could not copy this value. Try again.');

      if (!didUnmount) await unmount(component);
    }
  );

  it('offers QR only for private keys and spending keys, never recovery phrases', async () => {
    walletService.getWalletRecoverySecrets.mockResolvedValue({
      ...syntheticSecrets,
      dlightSecret: 'alpha beta gamma synthetic privacy mnemonic',
      dlightSecretKind: 'mnemonic',
    });
    const { component } = mountRecovery();
    await authenticate();

    findButton('Primary secret')?.click();
    await settle();
    expect(document.body.querySelector('button[aria-label^="Show QR code for"]')).toBeNull();
    document.body.querySelector<HTMLButtonElement>('button[data-slot="sheet-close"]')?.click();
    await settle();

    findButton('Privacy secret')?.click();
    await settle();
    expect(document.body.querySelector('button[aria-label^="Show QR code for"]')).toBeNull();
    document.body.querySelector<HTMLButtonElement>('button[data-slot="sheet-close"]')?.click();
    await settle();

    findButton('Derived keys')?.click();
    await settle();
    expect(document.body.querySelectorAll('button[aria-label^="Show QR code for"]')).toHaveLength(
      3
    );

    await unmount(component);
  });

  it('offers QR for primary private-key material and a Private Verus spending key', async () => {
    walletService.getWalletRecoverySecrets.mockResolvedValue({
      ...syntheticSecrets,
      primarySecretKind: 'wif',
      primarySecret: 'synthetic-primary-wif',
      dlightSecretKind: 'spending_key',
    });
    const { component } = mountRecovery();
    await authenticate();

    findButton('Primary secret')?.click();
    await settle();
    expect(
      document.body.querySelector('button[aria-label="Show QR code for Primary secret material"]')
    ).not.toBeNull();
    document.body.querySelector<HTMLButtonElement>('button[data-slot="sheet-close"]')?.click();
    await settle();

    findButton('Privacy secret')?.click();
    await settle();
    expect(
      document.body.querySelector(
        'button[aria-label="Show QR code for Privacy recovery secret or spending key"]'
      )
    ).not.toBeNull();

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

    document.body.querySelector<HTMLButtonElement>('button[data-slot="sheet-close"]')?.click();
    await settle();
    await unmount(component);
  });

  it.each(['back', 'close', 'unmount'])(
    'ignores a deferred QR result after %s',
    async (exitMethod) => {
      walletService.getWalletRecoverySecrets.mockResolvedValue(syntheticSecrets);
      let resolveQr:
        | ((value: {
            size: number;
            modules: Uint8Array;
            path: string;
            quietZone: number;
            viewBoxSize: number;
          }) => void)
        | undefined;
      qrEncoder.encodeQrCode.mockReturnValue(
        new Promise((resolve) => {
          resolveQr = resolve;
        })
      );
      const { component } = mountRecovery();
      await authenticate();
      findButton('Derived keys')?.click();
      await settle();
      document.body
        .querySelector<HTMLButtonElement>('button[aria-label="Show QR code for Verus WIF"]')
        ?.click();
      await settle();

      let didUnmount = false;
      if (exitMethod === 'back') {
        findButton('Back to keys')?.click();
      } else if (exitMethod === 'close') {
        document.body.querySelector<HTMLButtonElement>('button[data-slot="sheet-close"]')?.click();
      } else {
        await unmount(component);
        didUnmount = true;
      }
      await settle();

      resolveQr?.({
        size: 1,
        modules: Uint8Array.of(1),
        path: 'M4 4h1v1h-1z',
        quietZone: 4,
        viewBoxSize: 9,
      });
      await settle();
      expect(document.body.querySelector('svg[role="img"]')).toBeNull();

      if (!didUnmount) await unmount(component);
    }
  );

  it('shows QR failure, retries locally, and labels the selected key context', async () => {
    walletService.getWalletRecoverySecrets.mockResolvedValue(syntheticSecrets);
    qrEncoder.encodeQrCode.mockRejectedValueOnce(new Error('synthetic encoder failure'));
    const { component } = mountRecovery();
    await authenticate();
    findButton('Derived keys')?.click();
    await settle();
    document.body
      .querySelector<HTMLButtonElement>('button[aria-label="Show QR code for Verus WIF"]')
      ?.click();
    await settle();

    expect(document.body.textContent).toContain('The QR code could not be created.');
    expect(document.body.textContent).toContain('Verus WIF');
    expect(document.body.textContent).toContain('Testnet · WIF');
    expect(document.body.textContent).toContain('Synthetic test wallet');

    findButton('Try again')?.click();
    await settle();
    expect(qrEncoder.encodeQrCode).toHaveBeenCalledTimes(2);
    expect(document.body.querySelector('svg[role="img"]')).not.toBeNull();

    await unmount(component);
  });

  it('ignores a recovery request that resolves after unmount', async () => {
    let resolveRequest: ((value: WalletRecoverySecretsResult) => void) | undefined;
    walletService.getWalletRecoverySecrets.mockReturnValue(
      new Promise((resolve) => {
        resolveRequest = resolve;
      })
    );
    const { component } = mountRecovery();
    await authenticate();
    await unmount(component);

    resolveRequest?.(syntheticSecrets);
    await settle();
    expect(document.body.textContent).not.toContain(syntheticSecrets.primarySecret);
  });

  it('clears authenticated secrets when the recovery view remounts', async () => {
    walletService.getWalletRecoverySecrets.mockResolvedValue(syntheticSecrets);
    const firstMount = mountRecovery();
    await authenticate();
    expect(document.body.textContent).toContain('Primary secret');
    await unmount(firstMount.component);

    const secondMount = mountRecovery();
    await settle();
    expect(document.body.querySelector('input[type="password"]')).not.toBeNull();
    expect(document.body.textContent).not.toContain('Primary secret');
    expect(document.body.textContent).not.toContain(syntheticSecrets.primarySecret);

    await unmount(secondMount.component);
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
