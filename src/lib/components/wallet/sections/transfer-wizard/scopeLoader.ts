import type { CoinScope, CoinScopesResult } from '$lib/types/wallet.js';

export type TransferScopeLoadResult = {
  scopes: Record<string, CoinScope[]>;
  loaded: Record<string, true>;
  failures: Record<string, true>;
};

export async function loadTransferScopes(
  coinIds: string[],
  load: (coinId: string) => Promise<CoinScopesResult>,
  isInvalidated: (error: unknown) => boolean
): Promise<TransferScopeLoadResult> {
  const results = await Promise.all(
    coinIds.map(async (coinId) => {
      try {
        return { coinId, result: await load(coinId), failed: false as const };
      } catch (error) {
        if (isInvalidated(error)) return null;
        return { coinId, result: null, failed: true as const };
      }
    })
  );

  const scopes: Record<string, CoinScope[]> = {};
  const loaded: Record<string, true> = {};
  const failures: Record<string, true> = {};
  for (const result of results) {
    if (!result) continue;
    if (result.failed || !result.result) {
      failures[result.coinId] = true;
      continue;
    }
    scopes[result.coinId] = result.result.scopes;
    loaded[result.coinId] = true;
  }
  return { scopes, loaded, failures };
}
