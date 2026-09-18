import type { CoinRatesSnapshot } from '$lib/stores/rates.js';
import type {
  WatchlistAvailability,
  WatchlistEntrySnapshot,
  WatchlistHolding,
} from '$lib/types/watchlist.js';
import { formatFiatAmount, getRateForCurrency } from '$lib/utils/fiatDisplay.js';

export interface WatchlistRecord {
  snapshot: WatchlistEntrySnapshot;
  stale: boolean;
}

export interface WatchlistHoldingViewModel {
  key: string;
  coinId: string;
  currencyId: string;
  name: string;
  ticker: string;
  systemName: string;
  balanceDisplay: string;
  fiatDisplay: string;
  fiatValue: number | null;
}

export interface WatchlistEntryViewModel {
  id: string;
  displayName: string;
  address: string;
  availability: WatchlistAvailability;
  stale: boolean;
  holdings: WatchlistHoldingViewModel[];
  publicValueDisplay: string;
  currencyCount: number;
  refreshedAt: number;
}

function finiteNumber(value: unknown): number | null {
  if (typeof value === 'number') return Number.isFinite(value) ? value : null;
  if (typeof value !== 'string' || !value.trim()) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function fallbackTicker(currencyId: string): string {
  const trimmed = currencyId.trim();
  if (trimmed.length <= 12) return trimmed;
  return `${trimmed.slice(0, 6)}…${trimmed.slice(-4)}`;
}

function resolveRate(
  holding: WatchlistHolding,
  rates: Record<string, CoinRatesSnapshot>,
  displayCurrency: string
): number | null {
  const candidateIds = [
    holding.coin?.id,
    holding.coin?.currencyId,
    holding.coin?.mappedTo,
    holding.currencyId,
  ].filter((value): value is string => Boolean(value));

  for (const candidateId of candidateIds) {
    const snapshot = rates[candidateId];
    const rate = getRateForCurrency(snapshot?.rates, displayCurrency);
    if (rate !== null) return rate;
  }
  return null;
}

export function buildWatchlistEntryViewModel(
  record: WatchlistRecord,
  rates: Record<string, CoinRatesSnapshot>,
  intlLocale: string,
  displayCurrency: string,
  unavailableDisplay = '—'
): WatchlistEntryViewModel {
  const holdings = record.snapshot.holdings.map((holding): WatchlistHoldingViewModel => {
    const balance = finiteNumber(holding.balance);
    const ticker = holding.coin?.displayTicker?.trim() || fallbackTicker(holding.currencyId);
    const rate = resolveRate(holding, rates, displayCurrency);
    const fiatValue = balance !== null && rate !== null ? balance * rate : null;
    const balanceDisplay =
      balance === null
        ? unavailableDisplay
        : new Intl.NumberFormat(intlLocale, {
            minimumFractionDigits: 0,
            maximumFractionDigits: Math.min(8, holding.coin?.decimals ?? 8),
          }).format(balance);

    return {
      key: holding.assetKey,
      coinId: holding.coin?.id ?? holding.currencyId,
      currencyId: holding.currencyId,
      name: holding.coin?.displayName?.trim() || ticker,
      ticker,
      systemName: holding.systemDisplayName || holding.systemTicker,
      balanceDisplay,
      fiatDisplay:
        fiatValue === null
          ? unavailableDisplay
          : formatFiatAmount(fiatValue, intlLocale, displayCurrency),
      fiatValue,
    };
  });

  const knownFiatValues = holdings
    .map((holding) => holding.fiatValue)
    .filter((value): value is number => value !== null);
  const publicValueDisplay =
    (record.stale && holdings.length === 0) || knownFiatValues.length !== holdings.length
      ? unavailableDisplay
      : formatFiatAmount(
          knownFiatValues.reduce((sum, value) => sum + value, 0),
          intlLocale,
          displayCurrency
        );

  return {
    id: record.snapshot.entry.id,
    displayName: record.snapshot.entry.displayName,
    address: record.snapshot.entry.address,
    availability: record.snapshot.availability,
    stale: record.stale,
    holdings,
    publicValueDisplay,
    currencyCount: holdings.length,
    refreshedAt: record.snapshot.refreshedAt,
  };
}

export function mergeWatchlistSnapshots(
  previous: WatchlistRecord[],
  next: WatchlistEntrySnapshot[]
): WatchlistRecord[] {
  const previousById = new Map(previous.map((record) => [record.snapshot.entry.id, record]));

  return next.map((snapshot) => {
    const existing = previousById.get(snapshot.entry.id);
    if (snapshot.availability === 'unavailable' && existing?.snapshot.holdings.length) {
      return {
        snapshot: {
          ...snapshot,
          holdings: existing.snapshot.holdings,
        },
        stale: true,
      };
    }

    return { snapshot, stale: snapshot.availability !== 'available' };
  });
}
