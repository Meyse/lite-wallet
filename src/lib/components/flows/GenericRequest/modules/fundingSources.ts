import * as walletService from '$lib/services/walletService.js';
import type { BalanceResult, CoinDefinition, CoinScope } from '$lib/types/wallet.js';
import type { FundingSource } from '../types';

/**
 * Addresses in this wallet that hold `coin` on `systemId`. Watched addresses
 * are read-only scopes and never pay for a request (spec "Network fee").
 */
export async function loadFundingSources(input: {
  coin: CoinDefinition;
  systemId: string;
  /** Rust pays some details only from the wallet's own primary address. */
  primaryOnly?: boolean;
  formatBalance: (value: number, coin: CoinDefinition) => string;
}): Promise<FundingSource[]> {
  const { coin, systemId } = input;
  const result = await walletService.getCoinScopes(coin.id);
  const scopes = result.scopes.filter(
    (scope) =>
      scope.scopeKind === 'transparent' &&
      !scope.isReadOnly &&
      (!input.primaryOnly || scope.isPrimaryAddress) &&
      scope.systemId.toLowerCase() === systemId.toLowerCase()
  );
  const balances = await Promise.all(
    scopes.map(async (scope) => ({
      scope,
      balance: await walletService.getBalances(scope.channelId, coin.id),
    }))
  );
  return balances
    .map(({ scope, balance }) => toFundingSource(scope, balance, coin, input.formatBalance))
    .filter((source): source is FundingSource => source !== null)
    .sort((left, right) =>
      left.isPrimaryAddress !== right.isPrimaryAddress
        ? left.isPrimaryAddress
          ? -1
          : 1
        : right.balanceValue - left.balanceValue
    );
}

function toFundingSource(
  scope: CoinScope,
  balance: BalanceResult,
  coin: CoinDefinition,
  formatBalance: (value: number, coin: CoinDefinition) => string
): FundingSource | null {
  const value = Number(balance.total);
  if (!Number.isFinite(value) || value <= 0) return null;
  return {
    channelId: scope.channelId,
    address: scope.address,
    addressLabel: scope.addressLabel,
    systemId: scope.systemId,
    balanceValue: value,
    balanceDisplay: formatBalance(value, coin),
    isPrimaryAddress: scope.isPrimaryAddress,
  };
}
