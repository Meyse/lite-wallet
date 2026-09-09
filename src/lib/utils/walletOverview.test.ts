import { describe, expect, it } from 'vitest';
import { sortWalletOverviewRows, type WalletOverviewRowViewModel } from './walletOverview.js';

function overviewRow(
  key: string,
  name: string,
  hasBalance: boolean,
  fiatSortValue: number
): WalletOverviewRowViewModel {
  return {
    key,
    coinId: key,
    proto: 'vrsc',
    ticker: key,
    name,
    hasBalance,
    hasSnapshot: true,
    cryptoAmountDisplay: '',
    fiatValueDisplay: '',
    marketPriceDisplay: '',
    change24hDisplay: '',
    change24hDirection: 'none',
    unitRateDisplay: null,
    fiatSortValue,
  };
}

describe('sortWalletOverviewRows', () => {
  it('re-ranks funded rows as asynchronous balances and rates arrive', () => {
    const alphabetical = [
      overviewRow('VRSC', 'Verus', false, Number.NEGATIVE_INFINITY),
      overviewRow('PURE', 'Pure', false, Number.NEGATIVE_INFINITY),
    ];
    expect(sortWalletOverviewRows(alphabetical).map((row) => row.key)).toEqual(['PURE', 'VRSC']);

    const withTestnetBalance = [
      overviewRow('VRSCTEST', 'Verus Testnet', true, Number.NEGATIVE_INFINITY),
      overviewRow('PURE', 'Pure', false, Number.NEGATIVE_INFINITY),
    ];
    expect(sortWalletOverviewRows(withTestnetBalance).map((row) => row.key)).toEqual([
      'VRSCTEST',
      'PURE',
    ]);

    const withMainnetRates = [
      overviewRow('VRSC', 'Verus', true, 25),
      overviewRow('PURE', 'Pure', true, 40),
      overviewRow('ZERO', 'Zero asset', false, Number.NEGATIVE_INFINITY),
    ];
    expect(sortWalletOverviewRows(withMainnetRates).map((row) => row.key)).toEqual([
      'PURE',
      'VRSC',
      'ZERO',
    ]);
  });

  it('ranks a funded private row with other holdings and keeps zero rows last', () => {
    const rows: Array<WalletOverviewRowViewModel & { walletEntryKind?: 'coin' | 'private_verus' }> =
      [
        overviewRow('VRSC', 'Verus', true, 50),
        {
          ...overviewRow('private-VRSC', 'Private Verus', true, 20),
          walletEntryKind: 'private_verus' as const,
        },
        overviewRow('PURE', 'Pure', false, Number.NEGATIVE_INFINITY),
      ];

    const ranked = sortWalletOverviewRows(rows);

    expect(ranked.map((row) => row.key)).toEqual(['VRSC', 'private-VRSC', 'PURE']);
    expect(ranked[1]?.walletEntryKind).toBe('private_verus');
  });
});
