import type {
  AssetDiscoveryHolding,
  BalanceResult,
  CoinDefinition,
  CoinScopesResult,
} from '$lib/types/wallet.js';

export interface KnownAssetBalance {
  assetKey: string;
  coin: CoinDefinition;
  systemId: string;
  systemTicker: string;
  systemDisplayName: string;
  balance: string | null;
  status: 'available' | 'unavailable';
}

export interface AggregatedDiscoveryHolding {
  assetKey: string;
  coin: CoinDefinition | null;
  currencyId: string;
  total: number;
  totalDisplay: string;
  status: 'available' | 'partial';
  includesReadOnly: boolean;
  networks: AssetDiscoveryHolding[];
}

export function assetKeyForCoin(coin: CoinDefinition): string {
  const identity = coin.currencyId.trim() || coin.id.trim();
  return `${coin.proto}:${identity.toLowerCase()}`;
}

export function finiteAssetBalance(value: string | null): number {
  if (value === null) return 0;
  const amount = Number(value);
  return Number.isFinite(amount) ? amount : 0;
}

export function formatAssetBalance(value: number): string {
  if (value === 0) return '0';
  if (Math.abs(value) < 0.0001) {
    return value.toLocaleString('en-US', {
      minimumFractionDigits: 0,
      maximumFractionDigits: 8,
      useGrouping: false,
    });
  }
  return value.toLocaleString('en-US', {
    minimumFractionDigits: 0,
    maximumFractionDigits: 8,
    useGrouping: false,
  });
}

export function aggregateDiscoveryHoldings(
  holdings: AssetDiscoveryHolding[]
): AggregatedDiscoveryHolding[] {
  const grouped = new Map<string, AggregatedDiscoveryHolding>();

  for (const holding of holdings) {
    const existing = grouped.get(holding.assetKey);
    const amount = finiteAssetBalance(holding.balance);
    if (existing) {
      existing.total += amount;
      existing.totalDisplay = formatAssetBalance(existing.total);
      existing.includesReadOnly ||= holding.includesReadOnly;
      if (holding.balanceStatus === 'partial') existing.status = 'partial';
      if (!existing.coin && holding.coin) existing.coin = holding.coin;
      existing.networks.push(holding);
      continue;
    }

    grouped.set(holding.assetKey, {
      assetKey: holding.assetKey,
      coin: holding.coin ?? null,
      currencyId: holding.currencyId,
      total: amount,
      totalDisplay: formatAssetBalance(amount),
      status: holding.balanceStatus,
      includesReadOnly: holding.includesReadOnly,
      networks: [holding],
    });
  }

  return Array.from(grouped.values()).sort((left, right) => {
    const leftName = left.coin?.displayName ?? left.currencyId;
    const rightName = right.coin?.displayName ?? right.currencyId;
    return leftName.localeCompare(rightName, undefined, { sensitivity: 'base' });
  });
}

export async function scanKnownNonVrpcAssets(
  coins: CoinDefinition[],
  getScopes: (coinId: string) => Promise<CoinScopesResult>,
  getBalance: (channelId: string, coinId: string) => Promise<BalanceResult>
): Promise<KnownAssetBalance[]> {
  return Promise.all(
    coins
      .filter((coin) => !coin.compatibleChannels.includes('vrpc'))
      .map(async (coin): Promise<KnownAssetBalance> => {
        try {
          const scopes = await getScopes(coin.id);
          const scope = scopes.scopes[0];
          if (!scope) throw new Error('No supported scope');
          const balance = await getBalance(scope.channelId, coin.id);
          return {
            assetKey: assetKeyForCoin(coin),
            coin,
            systemId: scope.systemId,
            systemTicker: scope.systemTicker,
            systemDisplayName: scope.systemDisplayName,
            balance: balance.total,
            status: 'available',
          };
        } catch {
          return {
            assetKey: assetKeyForCoin(coin),
            coin,
            systemId: coin.systemId,
            systemTicker: coin.systemId,
            systemDisplayName: coin.systemId,
            balance: null,
            status: 'unavailable',
          };
        }
      })
  );
}
