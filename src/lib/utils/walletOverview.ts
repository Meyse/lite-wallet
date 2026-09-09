import type { WalletChannelsState } from '$lib/stores/walletChannels.js';
import type { CoinRatesSnapshot } from '$lib/stores/rates.js';
import type { BalanceResult, CoinDefinition, WalletNetwork } from '$lib/types/wallet.js';
import { resolveCoinPresentation } from '$lib/coins/presentation.js';
import { parseVrpcChannelId } from '$lib/utils/channelId.js';
import { formatFiatAmount, formatFiatAmountParts, getRateForCurrency, normalizeDisplayCurrency } from '$lib/utils/fiatDisplay.js';

export const OVERVIEW_UNAVAILABLE_DISPLAY = '—';

export interface WalletOverviewRowViewModel {
  key: string;
  coinId: string;
  proto: CoinDefinition['proto'];
  ticker: string;
  name: string;
  hasBalance: boolean;
  hasSnapshot: boolean;
  cryptoAmountDisplay: string;
  fiatValueDisplay: string;
  marketPriceDisplay: string;
  change24hDisplay: string;
  change24hDirection: 'up' | 'down' | 'flat' | 'none';
  unitRateDisplay: string | null;
  fiatSortValue: number;
}

export interface WalletOverviewViewModel {
  heroFiatDisplay: string;
  heroFiatSymbolDisplay: string;
  heroFiatValueDisplay: string;
  heroHasPartialRates: boolean;
  heroPrimaryCryptoDisplay: string;
  assetCount: number;
  identityCount: number;
  rows: WalletOverviewRowViewModel[];
  hasUsableLiveData: boolean;
  primaryTicker: string;
}

export interface BuildWalletOverviewParams {
  coins: CoinDefinition[];
  walletChannels: WalletChannelsState;
  balances: Record<string, Record<string, BalanceResult>>;
  scopeChannelIdsByCoinId?: Record<string, string[]>;
  rates: Record<string, CoinRatesSnapshot>;
  intlLocale: string;
  displayCurrency: string;
  network?: WalletNetwork;
}

export function formatCryptoAmount(
  value: number,
  ticker: string,
  intlLocale: string,
  minimumFractionDigits: number,
  maximumFractionDigits: number
): string {
  return `${new Intl.NumberFormat(intlLocale, {
    minimumFractionDigits,
    maximumFractionDigits
  }).format(value)} ${ticker}`;
}

function toFiniteNumber(value: unknown): number | null {
  if (typeof value === 'number') {
    return Number.isFinite(value) ? value : null;
  }

  if (typeof value === 'string') {
    const trimmed = value.trim();
    if (!trimmed) return null;
    const parsed = Number(trimmed);
    return Number.isFinite(parsed) ? parsed : null;
  }

  return null;
}

function getChangeDirection(changePct: number | null): WalletOverviewRowViewModel['change24hDirection'] {
  if (changePct === null) return 'none';
  if (Math.abs(changePct) < 0.01) return 'flat';
  if (changePct > 0) return 'up';
  return 'down';
}

function equalsIgnoreCase(left: string, right: string): boolean {
  return left.trim().toLowerCase() === right.trim().toLowerCase();
}

function resolveCoinBalanceSnapshot(
  coin: CoinDefinition,
  walletChannels: WalletChannelsState,
  balances: Record<string, Record<string, BalanceResult>>,
  scopeChannelIdsByCoinId?: Record<string, string[]>
): { hasSnapshot: boolean; amountValue: number } {
  if (coin.compatibleChannels.includes('vrpc')) {
    const scopedChannelIds = scopeChannelIdsByCoinId?.[coin.id] ?? [];
    if (scopedChannelIds.length > 0) {
      let hasSnapshot = false;
      let amountValue = 0;

      for (const channelId of scopedChannelIds) {
        const snapshot = balances[channelId]?.[coin.id];
        if (snapshot === undefined) continue;

        hasSnapshot = true;
        const amount = toFiniteNumber(snapshot.total);
        if (amount !== null) amountValue += amount;
      }

      if (hasSnapshot) {
        return { hasSnapshot, amountValue };
      }
    }

    let hasSnapshot = false;
    let amountValue = 0;

    for (const [channelId, channelBalances] of Object.entries(balances)) {
      if (!channelId.startsWith('vrpc.')) continue;
      const parsed = parseVrpcChannelId(channelId);
      if (!parsed || !equalsIgnoreCase(parsed.systemId, coin.systemId)) continue;

      const snapshot = channelBalances?.[coin.id];
      if (snapshot === undefined) continue;

      hasSnapshot = true;
      const amount = toFiniteNumber(snapshot.total);
      if (amount !== null) amountValue += amount;
    }

    if (hasSnapshot) {
      return { hasSnapshot, amountValue };
    }
  }

  const channelId = walletChannels.byCoinId[coin.id];
  const snapshot = channelId ? balances[channelId]?.[coin.id] : undefined;
  if (snapshot === undefined) {
    return { hasSnapshot: false, amountValue: 0 };
  }

  return {
    hasSnapshot: true,
    amountValue: toFiniteNumber(snapshot.total) ?? 0
  };
}

function formatPercentChange(changePct: number, intlLocale: string): string {
  const formatter = new Intl.NumberFormat(intlLocale, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  });
  const absDisplay = formatter.format(Math.abs(changePct));

  if (changePct > 0) return `+${absDisplay}%`;
  if (changePct < 0) return `-${absDisplay}%`;
  return `${absDisplay}%`;
}

function resolvePrimaryCoin(
  coins: CoinDefinition[],
  walletChannels: WalletChannelsState,
  network?: WalletNetwork
): CoinDefinition | null {
  const primaryChannelId = walletChannels.primaryChannelId;
  const matchingPrimaryCoinIds = primaryChannelId
    ? Object.entries(walletChannels.byCoinId)
        .filter(([, channel]) => channel === primaryChannelId)
        .map(([coinId]) => coinId)
    : [];
  const primaryCoinIdFromChannel =
    matchingPrimaryCoinIds.length === 1 ? matchingPrimaryCoinIds[0] : null;

  const defaultPrimaryId = network === 'testnet' ? 'VRSCTEST' : 'VRSC';

  return (
    (primaryCoinIdFromChannel ? coins.find((coin) => coin.id === primaryCoinIdFromChannel) : null) ??
    coins.find((coin) => coin.id === defaultPrimaryId) ??
    coins.find((coin) => coin.compatibleChannels.includes('vrpc')) ??
    coins[0] ??
    null
  );
}

export function sortWalletOverviewRows<Row extends WalletOverviewRowViewModel>(rows: Row[]): Row[] {
  return rows.sort((a, b) => {
    if (a.hasBalance !== b.hasBalance) {
      return a.hasBalance ? -1 : 1;
    }

    if (a.hasBalance && b.hasBalance && a.fiatSortValue !== b.fiatSortValue) {
      return b.fiatSortValue - a.fiatSortValue;
    }

    const nameCompare = a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
    if (nameCompare !== 0) return nameCompare;
    return a.ticker.localeCompare(b.ticker, undefined, { sensitivity: 'base' });
  });
}

export function buildWalletOverviewViewModel({
  coins,
  walletChannels,
  balances,
  scopeChannelIdsByCoinId,
  rates,
  intlLocale,
  displayCurrency,
  network
}: BuildWalletOverviewParams): WalletOverviewViewModel {
  const activeDisplayCurrency = normalizeDisplayCurrency(displayCurrency);
  const primaryCoin = resolvePrimaryCoin(coins, walletChannels, network);
  const primaryPresentation = primaryCoin ? resolveCoinPresentation(primaryCoin) : null;
  const fallbackPrimaryTicker = network === 'testnet' ? 'VRSCTEST' : 'VRSC';
  const primaryTicker = primaryPresentation?.displayTicker ?? fallbackPrimaryTicker;
  const primaryBalanceSnapshot = primaryCoin
    ? resolveCoinBalanceSnapshot(primaryCoin, walletChannels, balances, scopeChannelIdsByCoinId)
    : null;
  const hasPrimarySnapshot = primaryBalanceSnapshot?.hasSnapshot ?? false;
  const primaryTotal = hasPrimarySnapshot ? (primaryBalanceSnapshot?.amountValue ?? 0) : null;

  const rows = coins.map<WalletOverviewRowViewModel>((coin) => {
    const coinPresentation = resolveCoinPresentation(coin);
    const displayTicker = coinPresentation.displayTicker;
    const displayName = coinPresentation.displayName;
    const { hasSnapshot, amountValue } = resolveCoinBalanceSnapshot(
      coin,
      walletChannels,
      balances,
      scopeChannelIdsByCoinId
    );
    const hasBalance = hasSnapshot && amountValue > 0;
    const rateSnapshot = rates[coin.id];
    const fiatRate = getRateForCurrency(rateSnapshot?.rates, activeDisplayCurrency);
    const rawChange = rateSnapshot?.usdChange24hPct;
    const change24hPct =
      typeof rawChange === 'number' && Number.isFinite(rawChange) ? rawChange : null;
    const change24hDirection = getChangeDirection(change24hPct);
    const fiatValue = hasSnapshot && fiatRate !== null ? amountValue * fiatRate : null;
    const rowFractionDigits = Math.max(0, Math.min(4, coin.decimals));
    const marketPriceDisplay =
      fiatRate === null
        ? OVERVIEW_UNAVAILABLE_DISPLAY
        : formatFiatAmount(fiatRate, intlLocale, activeDisplayCurrency);
    const change24hDisplay =
      change24hDirection === 'none' || change24hPct === null
        ? OVERVIEW_UNAVAILABLE_DISPLAY
        : formatPercentChange(change24hPct, intlLocale);

    return {
      key: coin.id,
      coinId: coin.id,
      proto: coin.proto,
      ticker: displayTicker,
      name: displayName,
      hasBalance,
      hasSnapshot,
      cryptoAmountDisplay: hasSnapshot
        ? formatCryptoAmount(amountValue, displayTicker, intlLocale, rowFractionDigits, rowFractionDigits)
        : `${OVERVIEW_UNAVAILABLE_DISPLAY} ${displayTicker}`,
      fiatValueDisplay:
        fiatValue === null
          ? OVERVIEW_UNAVAILABLE_DISPLAY
          : formatFiatAmount(fiatValue, intlLocale, activeDisplayCurrency),
      marketPriceDisplay,
      change24hDisplay,
      change24hDirection,
      unitRateDisplay:
        fiatRate === null ? null : formatFiatAmount(fiatRate, intlLocale, activeDisplayCurrency),
      fiatSortValue: hasBalance && fiatValue !== null ? fiatValue : Number.NEGATIVE_INFINITY
    };
  });

  sortWalletOverviewRows(rows);

  const hasNonZeroRows = rows.some((row) => row.hasBalance);
  const hasHoldings = rows.some((row) => row.hasBalance);
  const hasAnySnapshot = rows.some((row) => row.hasSnapshot) || hasPrimarySnapshot;
  const hasAnyFiatForHoldings = rows.some(
    (row) => row.hasBalance && row.fiatSortValue !== Number.NEGATIVE_INFINITY
  );
  const hasMissingFiatForHoldings = rows.some(
    (row) => row.hasBalance && row.fiatSortValue === Number.NEGATIVE_INFINITY
  );
  const heroHasPartialRates = hasHoldings && hasAnyFiatForHoldings && hasMissingFiatForHoldings;
  const totalFiat = rows
    .filter((row) => row.hasBalance && row.fiatSortValue !== Number.NEGATIVE_INFINITY)
    .reduce((sum, row) => sum + row.fiatSortValue, 0);
  const heroFiatIsUnavailable = !hasAnySnapshot || (hasHoldings && !hasAnyFiatForHoldings);
  const heroFiatDisplay = heroFiatIsUnavailable
    ? OVERVIEW_UNAVAILABLE_DISPLAY
    : formatFiatAmount(totalFiat, intlLocale, activeDisplayCurrency);
  const heroFiatParts = heroFiatIsUnavailable
    ? null
    : formatFiatAmountParts(totalFiat, intlLocale, activeDisplayCurrency);

  return {
    heroFiatDisplay,
    heroFiatSymbolDisplay: heroFiatParts?.symbol ?? '',
    heroFiatValueDisplay: heroFiatParts?.value ?? OVERVIEW_UNAVAILABLE_DISPLAY,
    heroHasPartialRates,
    heroPrimaryCryptoDisplay:
      primaryTotal === null
        ? `${OVERVIEW_UNAVAILABLE_DISPLAY} ${primaryTicker}`
        : formatCryptoAmount(primaryTotal, primaryTicker, intlLocale, 0, 4),
    assetCount: rows.length,
    identityCount: 0,
    rows,
    hasUsableLiveData: hasNonZeroRows || hasPrimarySnapshot,
    primaryTicker
  };
}
