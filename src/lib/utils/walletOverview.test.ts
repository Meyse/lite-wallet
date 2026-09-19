import { describe, expect, it } from 'vitest';
import {
  aggregateOverviewBalances,
  buildWalletOverviewViewModel,
  filterWalletOverviewRows,
  overviewNetworkMetadata,
  sortWalletOverviewRows,
  type WalletOverviewRowViewModel,
} from './walletOverview.js';
import { buildWalletChannels } from '$lib/stores/walletChannels.js';
import { resolveCoinPresentationById } from '$lib/coins/presentation.js';
import type { CoinDefinition } from '$lib/types/wallet.js';

function coin(id: string, proto: CoinDefinition['proto'] = 'vrsc'): CoinDefinition {
  const presentation = resolveCoinPresentationById(id);
  return {
    id,
    currencyId: id,
    systemId: 'verus-system',
    displayName: id,
    displayTicker: id,
    isTestnet: false,
    decimals: 8,
    secondsPerBlock: 60,
    vrpcEndpoints: [],
    ...presentation,
    proto,
    compatibleChannels: [proto === 'vrsc' ? 'vrpc' : proto],
  };
}
function row(
  id: string,
  name = id,
  overrides: Partial<WalletOverviewRowViewModel> = {}
): WalletOverviewRowViewModel {
  return {
    key: id,
    coinId: id,
    proto: 'vrsc',
    ticker: id,
    name,
    hasBalance: true,
    hasSnapshot: true,
    cryptoAmountDisplay: '',
    fiatValueDisplay: '',
    marketPriceDisplay: '',
    change24hDisplay: '',
    change24hDirection: 'none',
    unitRateDisplay: null,
    fiatSortValue: 0,
    amountSortValue: 0,
    isConfirmedZero: false,
    ...overviewNetworkMetadata(coin(id)),
    ...overrides,
  };
}
const keys = (rows: WalletOverviewRowViewModel[]) => rows.map((row) => row.key);
const zero = { confirmed: '0', pending: '0', total: '0' };

describe('wallet overview ordering', () => {
  it('keeps Verus, private, hosted currencies, Ethereum and its currencies, then Bitcoin independent of rates', () => {
    const rows = [
      row('BTC', 'Bitcoin', overviewNetworkMetadata(coin('BTC', 'btc'))),
      row('ETH', 'Ethereum', overviewNetworkMetadata(coin('ETH', 'eth'))),
      row('USDC', 'USDC on Verus'),
      row('VRSC', 'Verus'),
      row('private-VRSC', 'Verus PRIVATE', { defaultSortGroup: 1 }),
      row('AAVE', 'Aave', overviewNetworkMetadata(coin('AAVE', 'erc20'))),
      row('other', 'Other network', { defaultSortGroup: 6, networkKey: 'other' }),
    ];
    const expected = ['VRSC', 'private-VRSC', 'USDC', 'ETH', 'AAVE', 'BTC', 'other'];
    expect(keys(sortWalletOverviewRows(rows))).toEqual(expected);
    expect(
      keys(
        sortWalletOverviewRows(
          rows.map((row, i) => ({ ...row, hasBalance: !!i, fiatSortValue: i * 300 }))
        )
      )
    ).toEqual(expected);
    expect(rows[0].key).toBe('BTC'); // Sorting must not mutate the balance-authority rows.
  });

  it('uses canonical metadata, including testnet, even with misleading display names', () => {
    const root = coin('VRSCTEST');
    expect(overviewNetworkMetadata({ ...root, isTestnet: true }).defaultSortGroup).toBe(0);
    expect(
      overviewNetworkMetadata({ ...coin('GETH', 'eth'), isTestnet: true }).networkName
    ).toContain('Sepolia');
    expect(
      overviewNetworkMetadata({ ...coin('token', 'erc20'), displayName: 'Verus' }).defaultSortGroup
    ).toBe(4);
    expect(
      overviewNetworkMetadata({ ...coin('token'), displayName: 'Bitcoin on Ethereum' })
        .defaultSortGroup
    ).toBe(2);
  });

  it('sorts the full list numerically, with known zero before unavailable values', () => {
    const rows = [
      row('VRSC', 'Verus', { fiatSortValue: 2, amountSortValue: 1000 }),
      row('BTC', 'Bitcoin', { fiatSortValue: 100, amountSortValue: 0.1 }),
      row('ZERO', 'Zero', { fiatSortValue: 0, amountSortValue: 0 }),
      row('UNKNOWN', 'A missing asset', { fiatSortValue: -Infinity, amountSortValue: null }),
    ];
    expect(keys(sortWalletOverviewRows(rows, 'value'))).toEqual(['BTC', 'VRSC', 'ZERO', 'UNKNOWN']);
    expect(keys(sortWalletOverviewRows(rows, 'amount'))).toEqual([
      'VRSC',
      'BTC',
      'ZERO',
      'UNKNOWN',
    ]);
    expect(keys(sortWalletOverviewRows(rows, 'name', 'en-US'))).toEqual([
      'UNKNOWN',
      'BTC',
      'VRSC',
      'ZERO',
    ]);
  });

  it('reverses explicit sorts while unavailable values stay last and Verus first stays fixed', () => {
    const rows = [
      row('VRSC', 'Verus', { fiatSortValue: 2, amountSortValue: 1000 }),
      row('BTC', 'Bitcoin', { fiatSortValue: 100, amountSortValue: 0.1 }),
      row('ZERO', 'Zero', { fiatSortValue: 0, amountSortValue: 0 }),
      row('UNKNOWN', 'A missing asset', { fiatSortValue: -Infinity, amountSortValue: null }),
    ];
    expect(keys(sortWalletOverviewRows(rows, 'value', 'en-US', true))).toEqual([
      'ZERO',
      'VRSC',
      'BTC',
      'UNKNOWN',
    ]);
    expect(keys(sortWalletOverviewRows(rows, 'amount', 'en-US', true))).toEqual([
      'ZERO',
      'BTC',
      'VRSC',
      'UNKNOWN',
    ]);
    expect(keys(sortWalletOverviewRows(rows, 'name', 'en-US', true))).toEqual([
      'ZERO',
      'VRSC',
      'BTC',
      'UNKNOWN',
    ]);
    expect(keys(sortWalletOverviewRows(rows, 'verus-first', 'en-US', true))).toEqual(
      keys(sortWalletOverviewRows(rows))
    );
  });

  it('breaks ties by localized name and then stable row identifier', () => {
    const rows = [row('z', 'Éther'), row('b', 'Alpha'), row('a', 'alpha'), row('x', 'Zulu')];
    for (const sort of ['name', 'value', 'amount'] as const) {
      expect(keys(sortWalletOverviewRows(rows, sort, 'nl-NL'))).toEqual(['a', 'b', 'z', 'x']);
    }
  });
});

describe('wallet overview filtering and balances', () => {
  it('combines trimmed case-insensitive name, ticker and network search with confirmed-zero filtering', () => {
    const rows = [
      row('one', 'Dollar', { ticker: 'USDC', networkName: 'Verus' }),
      row('two', 'Ether', { ticker: 'ETH', networkName: 'Ethereum', isConfirmedZero: true }),
      row('three', 'Unknown', {
        networkName: 'Ethereum',
        hasSnapshot: false,
        amountSortValue: null,
      }),
    ];
    expect(keys(filterWalletOverviewRows(rows, ' uSdC ', false))).toEqual(['one']);
    expect(keys(filterWalletOverviewRows(rows, 'dollar', false))).toEqual(['one']);
    expect(keys(filterWalletOverviewRows(rows, 'ethereum', true))).toEqual(['three']);
    expect(keys(filterWalletOverviewRows(rows, 'verus', true))).toEqual(['one']);
    expect(filterWalletOverviewRows(rows, 'Solana', false)).toEqual([]);
  });

  it('retains unknown, invalid, pending, and partially loaded scope balances', () => {
    expect(aggregateOverviewBalances([zero, zero]).isConfirmedZero).toBe(true);
    for (const snapshots of [
      [],
      [undefined],
      [zero, undefined],
      [{ ...zero, total: '' }],
      [{ ...zero, confirmed: '2', pending: '-2' }],
      [{ ...zero, total: 'NaN' }],
      [{ ...zero, pending: '1' }],
    ]) {
      expect(aggregateOverviewBalances(snapshots).isConfirmedZero).toBe(false);
    }
    expect(aggregateOverviewBalances([{ ...zero, total: 'bad' }]).hasSnapshot).toBe(false);
    expect(
      aggregateOverviewBalances([
        { ...zero, total: '2' },
        { ...zero, total: '3' },
      ]).amountValue
    ).toBe(5);
  });

  it('builds numeric values and the full-wallet total without coercing unknown balances or rates to zero', () => {
    const coins = [coin('VRSC'), coin('ETH', 'eth'), coin('BTC', 'btc'), coin('unknown')];
    const channels = buildWalletChannels(coins, 'Rtest');
    const balances = Object.fromEntries(
      coins
        .slice(0, 3)
        .map((c) => [
          channels.byCoinId[c.id],
          { [c.id]: { ...zero, total: c.id === 'BTC' ? '0' : '2' } },
        ])
    );
    const model = buildWalletOverviewViewModel({
      coins,
      walletChannels: channels,
      balances,
      rates: {
        VRSC: { rates: { EUR: 10 }, usdChange24hPct: null },
        BTC: { rates: { EUR: 40000 }, usdChange24hPct: null },
      },
      intlLocale: 'en-US',
      displayCurrency: 'EUR',
      network: 'mainnet',
    });
    expect(model.heroFiatValueDisplay).toBe('20.00');
    expect(model.heroHasPartialRates).toBe(true);
    expect(model.rows.find((row) => row.coinId === 'unknown')?.amountSortValue).toBeNull();
    expect(model.rows.find((row) => row.coinId === 'ETH')?.fiatSortValue).toBe(-Infinity);
    expect(model.rows.find((row) => row.coinId === 'BTC')?.fiatSortValue).toBe(0);
    expect(keys(filterWalletOverviewRows(model.rows, 'Ethereum', true))).toEqual(['ETH']);
    expect(model.heroFiatValueDisplay).toBe('20.00');
  });
});
