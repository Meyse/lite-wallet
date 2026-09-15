import { describe, expect, it } from 'vitest';
import {
  isValidTransferAmount,
  resolveTransferNetworkName,
  sumFiatComponents,
  transferAmountExceedsBalance,
} from './transferDisplay';

describe('transfer amount validation', () => {
  it('honors the selected asset precision exactly', () => {
    expect(isValidTransferAmount('1.234567', 6)).toBe(true);
    expect(isValidTransferAmount('1.2345678', 6)).toBe(false);
    for (const invalid of ['0', '-1', '+1', '1e2', '1.', '.1']) {
      expect(isValidTransferAmount(invalid, 6)).toBe(false);
    }
  });

  it('compares balances without floating-point rounding', () => {
    expect(transferAmountExceedsBalance('9007199254740993', '9007199254740992')).toBe(true);
    expect(transferAmountExceedsBalance('1.00000001', '1.00000000')).toBe(true);
    expect(transferAmountExceedsBalance('1', '1.00000000')).toBe(false);
  });
});

describe('fiat totals', () => {
  it('uses each component currency rate', () => {
    expect(
      sumFiatComponents([
        { amount: '10', rate: 1 },
        { amount: '0.001', rate: 2_000 },
      ])
    ).toBe(12);
  });

  it('fails closed when any component rate is missing or stale-cleared', () => {
    expect(
      sumFiatComponents([
        { amount: '10', rate: 1 },
        { amount: '0.001', rate: null },
      ])
    ).toBeNull();
  });
});

describe('canonical execution network labels', () => {
  it.each([
    ['eth', 'mainnet', 'Ethereum'],
    ['erc20', 'mainnet', 'Ethereum'],
    ['eth', 'testnet', 'Sepolia'],
    ['erc20', 'testnet', 'Sepolia'],
    ['btc', 'mainnet', 'Bitcoin'],
    ['btc', 'testnet', 'Bitcoin Testnet'],
  ] as const)('%s on %s is %s', (channelPrefix, walletNetwork, expected) => {
    expect(resolveTransferNetworkName({ channelPrefix, walletNetwork })).toBe(expected);
  });

  it('keeps Verus and PBaaS scope identities', () => {
    expect(
      resolveTransferNetworkName({
        channelPrefix: 'vrpc',
        walletNetwork: 'mainnet',
        systemId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
      })
    ).toBe('Verus');
    expect(
      resolveTransferNetworkName({
        channelPrefix: 'vrpc',
        walletNetwork: 'mainnet',
        systemId: 'iPbaas',
        scopeDisplayName: 'CHIPS',
      })
    ).toBe('CHIPS');
  });
});
