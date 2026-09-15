import type { BalancesByChannel } from '$lib/stores/balances.js';
import type { CoinDefinition, CoinScope } from '$lib/types/wallet.js';

export type TransferSourceOption = {
  id: string;
  coin: CoinDefinition;
  scope: CoinScope;
  channelId: string;
  sourceKind: 'public' | 'private';
  balanceTotal: string;
  balanceValue: number;
  displayName: string;
  displayTicker: string;
};

function finiteBalance(value: string | undefined): number {
  if (!value) return 0;
  const numeric = Number(value.trim());
  return Number.isFinite(numeric) ? numeric : 0;
}

function isCompatibleScope(coin: CoinDefinition, scope: CoinScope): boolean {
  const prefix = scope.channelId.trim().split('.')[0]?.toLowerCase() ?? '';
  if (prefix === 'dlight_private') {
    return (
      coin.compatibleChannels.includes('dlight_private') || coin.compatibleChannels.includes('vrpc')
    );
  }
  return coin.compatibleChannels.includes(prefix as CoinDefinition['compatibleChannels'][number]);
}

function hasSupportedSpendCapability(scope: CoinScope): boolean {
  const channelId = scope.channelId.trim();
  if (!channelId.startsWith('vrpc.')) return true;

  // Current VRPC preflight paths can only sign with the session's primary R-address.
  return (
    scope.scopeKind === 'transparent' &&
    scope.isPrimaryAddress &&
    channelId === `vrpc.${scope.address.trim()}.${scope.systemId.trim()}`
  );
}

export function buildSpendableTransferSources(
  coins: CoinDefinition[],
  scopesByCoinId: Record<string, CoinScope[]>,
  balances: BalancesByChannel
): TransferSourceOption[] {
  const seen = new Set<string>();
  const options: TransferSourceOption[] = [];

  for (const coin of coins) {
    for (const scope of scopesByCoinId[coin.id] ?? []) {
      const channelId = scope.channelId.trim();
      const address = scope.address.trim();
      const systemId = scope.systemId.trim();
      const key = `${coin.id}|${channelId}`;
      if (
        scope.isReadOnly ||
        !channelId ||
        !address ||
        !systemId ||
        seen.has(key) ||
        !isCompatibleScope(coin, scope) ||
        !hasSupportedSpendCapability(scope)
      ) {
        continue;
      }

      const balanceTotal = balances[channelId]?.[coin.id]?.total ?? '0';
      const balanceValue = finiteBalance(balanceTotal);
      if (balanceValue <= 0) continue;

      seen.add(key);
      options.push({
        id: key,
        coin,
        scope,
        channelId,
        sourceKind:
          scope.scopeKind === 'shielded' || channelId.startsWith('dlight_private.')
            ? 'private'
            : 'public',
        balanceTotal,
        balanceValue,
        displayName: coin.displayName,
        displayTicker: coin.displayTicker,
      });
    }
  }

  return options;
}

export function transferSourceSupportsConversion(source: TransferSourceOption | null): boolean {
  return !!source && source.sourceKind === 'public';
}
