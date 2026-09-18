import { describe, expect, it } from 'vitest';
import type { WatchlistEntrySnapshot } from '$lib/types/watchlist.js';
import { buildWatchlistEntryViewModel, mergeWatchlistSnapshots } from './watchlist.js';

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
    const merged = mergeWatchlistSnapshots(
      [{ snapshot: snapshot('available'), stale: false }],
      [snapshot('unavailable', '')]
    );

    expect(merged).toHaveLength(1);
    expect(merged[0].stale).toBe(true);
    expect(merged[0].snapshot.holdings).toHaveLength(1);
    expect(merged[0].snapshot.availability).toBe('unavailable');

    const view = buildWatchlistEntryViewModel(
      merged[0],
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
