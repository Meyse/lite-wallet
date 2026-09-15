import type { WalletNetwork } from '$lib/types/wallet.js';

const VRSC_SYSTEM_ID = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
const VRSCTEST_SYSTEM_ID = 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq';

export function isValidTransferAmount(input: string, decimals: number): boolean {
  const trimmed = input.trim();
  const normalizedDecimals = Number.isInteger(decimals) && decimals >= 0 ? decimals : 0;
  const match = /^(\d+)(?:\.(\d+))?$/.exec(trimmed);
  if (!match) return false;
  const fraction = match[2] ?? '';
  if (fraction.length > normalizedDecimals) return false;

  const exact = `${match[1]}${fraction.padEnd(normalizedDecimals, '0')}`.replace(/^0+/, '');
  return exact.length > 0;
}

export function transferAmountExceedsBalance(input: string, balance: string): boolean {
  const requested = parseUnsignedDecimalParts(input);
  const available = parseUnsignedDecimalParts(balance);
  if (!requested || !available) return false;

  const fractionDigits = Math.max(requested.fraction.length, available.fraction.length);
  const requestedUnits = BigInt(
    `${requested.whole}${requested.fraction.padEnd(fractionDigits, '0')}`
  );
  const availableUnits = BigInt(
    `${available.whole}${available.fraction.padEnd(fractionDigits, '0')}`
  );
  return requestedUnits > availableUnits;
}

function parseUnsignedDecimalParts(input: string): { whole: string; fraction: string } | null {
  const match = /^(\d+)(?:\.(\d+))?$/.exec(input.trim());
  if (!match) return null;
  return {
    whole: match[1].replace(/^0+(?=\d)/, ''),
    fraction: (match[2] ?? '').replace(/0+$/, ''),
  };
}

export type FiatComponent = {
  amount: string | null | undefined;
  rate: number | null;
};

export function sumFiatComponents(components: FiatComponent[]): number | null {
  let total = 0;
  for (const component of components) {
    const amount = Number(component.amount?.trim());
    if (
      !component.amount?.trim() ||
      !Number.isFinite(amount) ||
      amount < 0 ||
      component.rate === null ||
      !Number.isFinite(component.rate) ||
      component.rate <= 0
    ) {
      return null;
    }
    total += amount * component.rate;
  }
  return Number.isFinite(total) ? total : null;
}

export function resolveTransferNetworkName(options: {
  channelPrefix: string;
  walletNetwork: WalletNetwork;
  systemId?: string | null;
  scopeDisplayName?: string | null;
  fallbackName?: string | null;
}): string {
  const prefix = options.channelPrefix.trim().toLowerCase();
  if (prefix === 'eth' || prefix === 'erc20') {
    return options.walletNetwork === 'testnet' ? 'Sepolia' : 'Ethereum';
  }
  if (prefix === 'btc') {
    return options.walletNetwork === 'testnet' ? 'Bitcoin Testnet' : 'Bitcoin';
  }

  const systemId = options.systemId?.trim() ?? '';
  if (systemId.toLowerCase() === VRSC_SYSTEM_ID.toLowerCase()) return 'Verus';
  if (systemId.toLowerCase() === VRSCTEST_SYSTEM_ID.toLowerCase()) return 'Verus Testnet';
  return options.scopeDisplayName?.trim() || options.fallbackName?.trim() || 'Verus';
}
