import type { CoinRatesSnapshot } from '$lib/stores/rates.js';
import type {
  WatchedCurrencyRate,
  WatchlistAvailability,
  WatchlistEntrySnapshot,
  WatchlistHolding,
  WatchlistRateRequest,
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
  /** The chain's name, set only when the entry holds this currency on more than one chain. */
  systemLabel: string | null;
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
  hasPartialFiatValue: boolean;
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

/**
 * Priced holdings first, highest value first, so the holding that matters most
 * is at the top; then unpriced holdings by name; then by chain.
 */
function compareHoldings(
  left: WatchlistHoldingViewModel,
  right: WatchlistHoldingViewModel,
  intlLocale: string
): number {
  if (left.fiatValue !== null || right.fiatValue !== null) {
    if (left.fiatValue === null) return 1;
    if (right.fiatValue === null) return -1;
    if (left.fiatValue !== right.fiatValue) return right.fiatValue - left.fiatValue;
  }
  return (
    left.name.localeCompare(right.name, intlLocale) ||
    left.systemName.localeCompare(right.systemName, intlLocale)
  );
}

export function buildWatchlistEntryViewModel(
  record: WatchlistRecord,
  rates: Record<string, CoinRatesSnapshot>,
  intlLocale: string,
  displayCurrency: string,
  unavailableDisplay = '—'
): WatchlistEntryViewModel {
  const chainCounts = new Map<string, number>();
  for (const holding of record.snapshot.holdings) {
    const key = holding.currencyId.toLowerCase();
    chainCounts.set(key, (chainCounts.get(key) ?? 0) + 1);
  }
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

    const systemName = holding.systemDisplayName || holding.systemTicker;
    return {
      key: holding.assetKey,
      coinId: holding.coin?.id ?? holding.currencyId,
      currencyId: holding.currencyId,
      name: holding.coin?.displayName?.trim() || ticker,
      ticker,
      systemName,
      systemLabel: (chainCounts.get(holding.currencyId.toLowerCase()) ?? 0) > 1 ? systemName : null,
      balanceDisplay,
      fiatDisplay:
        fiatValue === null
          ? unavailableDisplay
          : formatFiatAmount(fiatValue, intlLocale, displayCurrency),
      fiatValue,
    };
  });
  holdings.sort((left, right) => compareHoldings(left, right, intlLocale));

  const knownFiatValues = holdings
    .map((holding) => holding.fiatValue)
    .filter((value): value is number => value !== null);
  const hasPartialFiatValue =
    knownFiatValues.length > 0 && knownFiatValues.length < holdings.length;
  const publicValueDisplay =
    (record.stale && holdings.length === 0) || (holdings.length > 0 && knownFiatValues.length === 0)
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
    hasPartialFiatValue,
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

/** Matches the backend's display limit for public rates (`MARKET_RATE_DISPLAY_MAX_AGE_SECS`). */
export const WATCHED_RATE_MAX_AGE_MS = 15 * 60 * 1000;

/**
 * Adds watched-currency rates to the engine's rates for Watchlist display only.
 * Engine rates win, and a watched rate older than the display limit is dropped,
 * so an expired value is never shown as current.
 */
export function mergeWatchedRates(
  engineRates: Record<string, CoinRatesSnapshot>,
  watchedRates: Record<string, WatchedCurrencyRate>,
  nowMs: number
): Record<string, CoinRatesSnapshot> {
  const merged: Record<string, CoinRatesSnapshot> = {};
  for (const [coinId, entry] of Object.entries(watchedRates)) {
    // The visit's clock ticks once a minute, so a rate fetched after the last
    // tick has a negative age; it is fresh, not invalid.
    if (nowMs - entry.fetchedAt * 1000 > WATCHED_RATE_MAX_AGE_MS) continue;
    merged[coinId] = { rates: entry.rates, usdChange24hPct: entry.usdChange24hPct };
  }
  return { ...merged, ...engineRates };
}

/**
 * Lists watched currencies that have no engine rate in the display currency and
 * were not requested yet in this Watchlist visit, once per holding key.
 */
export function watchedRateRequests(
  records: WatchlistRecord[],
  engineRates: Record<string, CoinRatesSnapshot>,
  displayCurrency: string,
  requestedKeys: ReadonlySet<string>
): (WatchlistRateRequest & { key: string })[] {
  const requests = new Map<string, WatchlistRateRequest & { key: string }>();
  for (const record of records) {
    for (const holding of record.snapshot.holdings) {
      const key = holding.assetKey;
      if (requestedKeys.has(key) || requests.has(key)) continue;
      if (resolveRate(holding, engineRates, displayCurrency) !== null) continue;
      requests.set(key, { key, systemId: holding.systemId, currencyId: holding.currencyId });
    }
  }
  return [...requests.values()];
}
