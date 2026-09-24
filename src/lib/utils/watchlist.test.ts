import { describe, expect, it } from 'vitest';
import type {
  WatchlistEntrySnapshot,
  WatchlistHolding,
  WatchlistSource,
} from '$lib/types/watchlist.js';
import { buildWatchlistEntryViewModel, mergeWatchlistSnapshot } from './watchlist.js';

function holding(systemId: string, assetKey: string, balance: string): WatchlistHolding {
  return {
    assetKey,
    currencyId: `${assetKey}-currency`,
    systemId,
    systemTicker: systemId,
    systemDisplayName: systemId,
    balance,
  };
}

function source(systemId: string, status: WatchlistSource['status']): WatchlistSource {
  return {
    systemId,
    systemTicker: systemId,
    systemDisplayName: systemId,
    status,
  };
}

function multiSystemSnapshot(
  availability: WatchlistEntrySnapshot['availability'],
  sources: WatchlistSource[],
  holdings: WatchlistHolding[]
): WatchlistEntrySnapshot {
  return {
    entry: {
      id: 'entry-1',
      targetKind: 'identity',
      displayName: 'Alice@',
      address: `R${'a'.repeat(33)}`,
      systemId: null,
      createdAt: 1,
      updatedAt: 1,
    },
    holdings,
    sources,
    availability,
    refreshedAt: 2,
  };
}

function snapshot(
  availability: WatchlistEntrySnapshot['availability'],
  balance = '2.5'
): WatchlistEntrySnapshot {
  return {
    entry: {
      id: 'entry-1',
      targetKind: 'identity',
      displayName: 'Alice@',
      address: `R${'a'.repeat(33)}`,
      systemId: null,
      createdAt: 1,
      updatedAt: 1,
    },
    holdings:
      balance === ''
        ? []
        : [
            {
              assetKey: 'vrsc',
              currencyId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
              systemId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
              systemTicker: 'VRSC',
              systemDisplayName: 'Verus',
              balance,
              coin: {
                id: 'VRSC',
                currencyId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
                systemId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
                displayTicker: 'VRSC',
                displayName: 'Verus',
                proto: 'vrsc',
                compatibleChannels: ['vrpc'],
                decimals: 8,
                vrpcEndpoints: [],
                secondsPerBlock: 60,
                isTestnet: false,
              },
            },
          ],
    sources: [],
    availability,
    refreshedAt: 2,
  };
}

describe('watchlist view models', () => {
  it('formats public balances with current fiat rates', () => {
    const view = buildWatchlistEntryViewModel(
      { snapshot: snapshot('available'), stale: false },
      { VRSC: { rates: { EUR: 4 }, usdChange24hPct: null } },
      'en-US',
      'EUR'
    );

    expect(view.publicValueDisplay).toBe('€10.00');
    expect(view.holdings[0]).toMatchObject({
      balanceDisplay: '2.5',
      ticker: 'VRSC',
      fiatDisplay: '€10.00',
    });
  });

  it('keeps last-known holdings when a later provider response is unavailable', () => {
    const merged = mergeWatchlistSnapshot(snapshot('available'), snapshot('unavailable', ''));

    expect(merged.holdings).toHaveLength(1);
    expect(merged.availability).toBe('unavailable');

    const view = buildWatchlistEntryViewModel(
      { snapshot: merged, stale: merged.availability !== 'available' },
      { VRSC: { rates: { USD: 4 }, usdChange24hPct: null } },
      'en-US',
      'USD'
    );
    expect(view.publicValueDisplay).toBe('$10.00');
  });

  it('shows an unavailable value when balances exist but rates do not', () => {
    const view = buildWatchlistEntryViewModel(
      { snapshot: snapshot('partial'), stale: true },
      {},
      'nl-NL',
      'EUR'
    );

    expect(view.publicValueDisplay).toBe('—');
    expect(view.currencyCount).toBe(1);
  });
});

describe('partial refresh merging', () => {
  it('keeps failed-system holdings while accepting successful-system results', () => {
    const previous = multiSystemSnapshot(
      'available',
      [source('VRSC', 'available'), source('VRSCTEST', 'available')],
      [holding('VRSC', 'vrsc:vrsc', '1.5'), holding('VRSCTEST', 'vrsctest:vrsc', '2.5')]
    );
    const next = multiSystemSnapshot(
      'partial',
      [source('VRSC', 'unavailable'), source('VRSCTEST', 'available')],
      [holding('VRSCTEST', 'vrsctest:vrsc', '9.5')]
    );

    const merged = mergeWatchlistSnapshot(previous, next);

    expect(merged.holdings).toHaveLength(2);
    expect(merged.holdings.find((item) => item.systemId === 'VRSCTEST')?.balance).toBe('9.5');
    expect(merged.holdings.find((item) => item.systemId === 'VRSC')?.balance).toBe('1.5');
  });

  it('drops already-known holdings for systems reported successfully', () => {
    const previous = multiSystemSnapshot(
      'available',
      [source('VRSC', 'available'), source('VRSCTEST', 'available')],
      [holding('VRSC', 'vrsc:vrsc', '1.5'), holding('VRSCTEST', 'vrsctest:vrsc', '2.5')]
    );
    const next = multiSystemSnapshot(
      'partial',
      [source('VRSC', 'available'), source('VRSCTEST', 'unavailable')],
      []
    );

    const merged = mergeWatchlistSnapshot(previous, next);

    expect(merged.holdings).toHaveLength(1);
    expect(merged.holdings[0]).toMatchObject({ systemId: 'VRSCTEST', balance: '2.5' });
  });

  it('keeps known holdings on a wholly unavailable response, with or without source detail', () => {
    const previous = multiSystemSnapshot(
      'available',
      [source('VRSC', 'available'), source('VRSCTEST', 'available')],
      [holding('VRSC', 'vrsc:vrsc', '1.5'), holding('VRSCTEST', 'vrsctest:vrsc', '2.5')]
    );
    const withoutSources = multiSystemSnapshot('unavailable', [], []);
    const withSources = multiSystemSnapshot(
      'unavailable',
      [source('VRSC', 'unavailable'), source('VRSCTEST', 'unavailable')],
      []
    );

    expect(mergeWatchlistSnapshot(previous, withoutSources).holdings).toHaveLength(2);
    expect(mergeWatchlistSnapshot(previous, withSources).holdings).toHaveLength(2);
  });

  it('accepts a confirmed empty response and drops all known holdings', () => {
    const previous = multiSystemSnapshot(
      'available',
      [source('VRSC', 'available'), source('VRSCTEST', 'available')],
      [holding('VRSC', 'vrsc:vrsc', '1.5'), holding('VRSCTEST', 'vrsctest:vrsc', '2.5')]
    );
    const next = multiSystemSnapshot(
      'available',
      [source('VRSC', 'available'), source('VRSCTEST', 'available')],
      []
    );

    expect(mergeWatchlistSnapshot(previous, next).holdings).toEqual([]);
  });
});
