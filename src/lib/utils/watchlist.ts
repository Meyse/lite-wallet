import type { CoinRatesSnapshot } from '$lib/stores/rates.js';
import type {
  WatchlistAvailability,
  WatchlistEntrySnapshot,
  WatchlistHolding,
  WatchlistTargetKind,
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
  targetKind: WatchlistTargetKind;
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
    targetKind: record.snapshot.entry.targetKind,
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

/**
 * Merges a refreshed snapshot with the last known one without inventing an
 * empty result. Systems that responded successfully are authoritative, so
 * their fresh holdings (including a confirmed empty list) replace the previous
 * ones. Holdings from systems that failed or were not reported are kept as
 * last known values instead of disappearing.
 */
export function mergeWatchlistSnapshot(
  previous: WatchlistEntrySnapshot | undefined,
  next: WatchlistEntrySnapshot
): WatchlistEntrySnapshot {
  if (next.availability === 'available' || !previous?.holdings.length) return next;

  const statusBySystem = new Map(
    next.sources.map((source) => [source.systemId, source.status] as const)
  );
  const freshKeys = new Set(next.holdings.map((holding) => holding.assetKey));
  const retained = previous.holdings.filter((holding) => {
    if (freshKeys.has(holding.assetKey)) return false;
    const status = statusBySystem.get(holding.systemId);
    // A system that is missing from the report is not proof that its holdings
    // disappeared, and a failed system keeps its last known value.
    return status === undefined || status !== 'available';
  });

  if (!retained.length) return next;
  return { ...next, holdings: [...next.holdings, ...retained] };
}
