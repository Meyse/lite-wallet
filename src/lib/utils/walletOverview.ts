import type { WalletChannelsState } from '$lib/stores/walletChannels.js';
import type { CoinRatesSnapshot } from '$lib/stores/rates.js';
import type { BalanceResult, CoinDefinition, WalletNetwork } from '$lib/types/wallet.js';
import { resolveCoinPresentation, resolveCoinPresentationById } from '$lib/coins/presentation.js';
import { parseVrpcChannelId } from '$lib/utils/channelId.js';
import {
  formatFiatAmount,
  formatFiatAmountParts,
  getRateForCurrency,
  normalizeDisplayCurrency,
} from '$lib/utils/fiatDisplay.js';

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
  amountSortValue: number | null;
  isConfirmedZero: boolean;
  networkName: string;
  networkKey: string;
  defaultSortGroup: number;
}

export const WALLET_OVERVIEW_SORTS = ['verus-first', 'value', 'name', 'amount'] as const;
export type WalletOverviewSort = (typeof WALLET_OVERVIEW_SORTS)[number];

export function normalizeWalletOverviewSort(value: unknown): WalletOverviewSort {
  return WALLET_OVERVIEW_SORTS.includes(value as WalletOverviewSort)
    ? (value as WalletOverviewSort)
    : 'verus-first';
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
    maximumFractionDigits,
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

function getChangeDirection(
  changePct: number | null
): WalletOverviewRowViewModel['change24hDirection'] {
  if (changePct === null) return 'none';
  if (Math.abs(changePct) < 0.01) return 'flat';
  if (changePct > 0) return 'up';
  return 'down';
}

function equalsIgnoreCase(left: string, right: string): boolean {
  return left.trim().toLowerCase() === right.trim().toLowerCase();
}

/** A zero is filterable only when every expected scope has settled at zero. */
export function aggregateOverviewBalances(snapshots: (BalanceResult | undefined)[]): {
  hasSnapshot: boolean;
  amountValue: number;
  isConfirmedZero: boolean;
} {
  let hasSnapshot = false;
  let amountValue = 0;
  let isConfirmedZero = snapshots.length > 0;
  for (const snapshot of snapshots) {
    const amount = toFiniteNumber(snapshot?.total);
    if (amount !== null) {
      hasSnapshot = true;
      amountValue += amount;
    }
    isConfirmedZero &&=
      amount === 0 &&
      toFiniteNumber(snapshot?.confirmed) === 0 &&
      toFiniteNumber(snapshot?.pending) === 0;
  }
  return { hasSnapshot, amountValue, isConfirmedZero };
}

function resolveCoinBalanceSnapshot(
  coin: CoinDefinition,
  walletChannels: WalletChannelsState,
  balances: Record<string, Record<string, BalanceResult>>,
  scopeChannelIdsByCoinId?: Record<string, string[]>
): ReturnType<typeof aggregateOverviewBalances> {
  if (coin.compatibleChannels.includes('vrpc')) {
    const scopedChannelIds = scopeChannelIdsByCoinId?.[coin.id] ?? [];
    if (scopedChannelIds.length > 0) {
      return aggregateOverviewBalances(scopedChannelIds.map((id) => balances[id]?.[coin.id]));
    }
    const snapshots = Object.entries(balances)
      .filter(([id]) => {
        const channel = parseVrpcChannelId(id);
        return channel && equalsIgnoreCase(channel.systemId, coin.systemId);
      })
      .map(([, channelBalances]) => channelBalances[coin.id]);
    if (snapshots.length > 0) return aggregateOverviewBalances(snapshots);
  }
  const channelId = walletChannels.byCoinId[coin.id];
  return aggregateOverviewBalances([channelId ? balances[channelId]?.[coin.id] : undefined]);
}

/** Network families come from protocol/system metadata, never a display-name heuristic. */
export function overviewNetworkMetadata(
  coin: CoinDefinition
): Pick<WalletOverviewRowViewModel, 'defaultSortGroup' | 'networkName' | 'networkKey'> {
  const system = resolveCoinPresentationById(coin.systemId);
  const rootId = coin.isTestnet ? 'VRSCTEST' : 'VRSC';
  const root = resolveCoinPresentationById(rootId);
  if (coin.proto === 'vrsc') {
    const native =
      coin.id === rootId ||
      (!!root &&
        equalsIgnoreCase(coin.currencyId, root.currencyId) &&
        equalsIgnoreCase(coin.systemId, root.systemId));
    return {
      defaultSortGroup: native ? 0 : 2,
      networkName: [root?.displayName ?? rootId, system?.displayName, system?.displayTicker]
        .filter(Boolean)
        .join(' '),
      networkKey: 'verus',
    };
  }
  if (coin.proto === 'eth' || coin.proto === 'erc20') {
    return {
      defaultSortGroup: coin.proto === 'eth' ? 3 : 4,
      networkName: coin.isTestnet ? 'Ethereum Sepolia ETH' : 'Ethereum ETH',
      networkKey: 'ethereum',
    };
  }
  return {
    defaultSortGroup: coin.id === 'BTC' ? 5 : 6,
    networkName: system?.displayName ?? coin.displayName,
    networkKey: coin.systemId,
  };
}

function formatPercentChange(changePct: number, intlLocale: string): string {
  const formatter = new Intl.NumberFormat(intlLocale, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
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
    (primaryCoinIdFromChannel
      ? coins.find((coin) => coin.id === primaryCoinIdFromChannel)
      : null) ??
    coins.find((coin) => coin.id === defaultPrimaryId) ??
    coins.find((coin) => coin.compatibleChannels.includes('vrpc')) ??
    coins[0] ??
    null
  );
}

export function sortWalletOverviewRows<Row extends WalletOverviewRowViewModel>(
  rows: readonly Row[],
  sort: WalletOverviewSort = 'verus-first',
  intlLocale?: string,
  reversed = false
): Row[] {
  const collator = new Intl.Collator(intlLocale, { sensitivity: 'base' });
  return [...rows].sort((a, b) => {
    if (sort === 'verus-first') {
      const group = a.defaultSortGroup - b.defaultSortGroup;
      if (group) return group;
      const network = collator.compare(a.networkKey, b.networkKey);
      if (network) return network;
    } else if (sort === 'value' || sort === 'amount') {
      const left = sort === 'value' ? a.fiatSortValue : a.amountSortValue;
      const right = sort === 'value' ? b.fiatSortValue : b.amountSortValue;
      const leftKnown = left !== null && Number.isFinite(left);
      const rightKnown = right !== null && Number.isFinite(right);
      if (leftKnown !== rightKnown) return leftKnown ? -1 : 1;
      if (leftKnown && rightKnown && left !== null && right !== null && left !== right)
        return reversed ? left - right : right - left;
    }
    const nameOrder = collator.compare(a.name, b.name) * (sort === 'name' && reversed ? -1 : 1);
    return nameOrder || (a.key < b.key ? -1 : a.key > b.key ? 1 : 0);
  });
}

export function filterWalletOverviewRows<Row extends WalletOverviewRowViewModel>(
  rows: readonly Row[],
  query: string,
  withBalance: boolean
): Row[] {
  const search = query.trim().toLowerCase();
  return rows.filter(
    (row) =>
      (!withBalance || !row.isConfirmedZero) &&
      (!search ||
        [row.name, row.ticker, row.networkName].some((value) =>
          value.toLowerCase().includes(search)
        ))
  );
}

export function buildWalletOverviewViewModel({
  coins,
  walletChannels,
  balances,
  scopeChannelIdsByCoinId,
  rates,
  intlLocale,
  displayCurrency,
  network,
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
    const { hasSnapshot, amountValue, isConfirmedZero } = resolveCoinBalanceSnapshot(
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
        ? formatCryptoAmount(
            amountValue,
            displayTicker,
            intlLocale,
            rowFractionDigits,
            rowFractionDigits
          )
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
      fiatSortValue: fiatValue ?? Number.NEGATIVE_INFINITY,
      amountSortValue: hasSnapshot ? amountValue : null,
      isConfirmedZero,
      ...overviewNetworkMetadata(coin),
    };
  });

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
    rows: sortWalletOverviewRows(rows, 'verus-first', intlLocale),
    hasUsableLiveData: hasNonZeroRows || hasPrimarySnapshot,
    primaryTicker,
  };
}
