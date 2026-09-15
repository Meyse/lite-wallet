import type { CoinRatesSnapshot } from '$lib/stores/rates.js';
import type { CoinDefinition } from '$lib/types/wallet.js';
import { getRateForCurrency } from '$lib/utils/fiatDisplay.js';

export function resolveCanonicalTransferCurrency(
  coins: CoinDefinition[],
  currencyReference: string | null | undefined
): CoinDefinition | null {
  const normalized = currencyReference?.trim().toLowerCase();
  if (!normalized) return null;

  return (
    coins.find(
      (coin) =>
        coin.id.trim().toLowerCase() === normalized ||
        coin.currencyId.trim().toLowerCase() === normalized
    ) ?? null
  );
}

export function resolveCanonicalTransferCurrencyRate(
  coins: CoinDefinition[],
  rates: Record<string, CoinRatesSnapshot>,
  currencyReference: string | null | undefined,
  displayCurrency: string
): number | null {
  const coin = resolveCanonicalTransferCurrency(coins, currencyReference);
  if (!coin) return null;

  for (const coinId of [coin.id, coin.currencyId, coin.mappedTo]) {
    if (!coinId) continue;
    const rate = getRateForCurrency(rates[coinId]?.rates, displayCurrency);
    if (rate !== null && Number.isFinite(rate) && rate > 0) return rate;
  }

  return null;
}
