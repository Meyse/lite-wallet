import { describe, expect, it } from 'vitest';
import type { CoinDefinition } from '$lib/types/wallet.js';
import {
  resolveCanonicalTransferCurrency,
  resolveCanonicalTransferCurrencyRate,
} from './transferCurrency';

function coin(
  id: string,
  currencyId: string,
  systemId: string,
  displayTicker: string
): CoinDefinition {
  return {
    id,
    currencyId,
    systemId,
    displayTicker,
    displayName: displayTicker,
    proto: 'vrsc',
    compatibleChannels: ['vrpc'],
    decimals: 8,
    vrpcEndpoints: [],
    secondsPerBlock: 60,
    isTestnet: false,
  };
}

describe('canonical transfer currency display resolution', () => {
  const nativeId = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
  const tokenId = 'i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd';
  const token = coin('vUSDC.vETH', tokenId, nativeId, 'vUSDC');
  const native = coin('VRSC', nativeId, nativeId, 'VRSC');

  it('selects the canonical native asset when a token on that system appears first', () => {
    expect(resolveCanonicalTransferCurrency([token, native], nativeId)).toBe(native);
  });

  it('does not substitute a token when the native asset is unavailable', () => {
    expect(resolveCanonicalTransferCurrency([token], nativeId)).toBeNull();
  });

  it('uses the native rate for a native fee when native and token rates differ', () => {
    expect(
      resolveCanonicalTransferCurrencyRate(
        [token, native],
        {
          [token.id]: { rates: { USD: 1 }, usdChange24hPct: null },
          [native.id]: { rates: { USD: 4.25 }, usdChange24hPct: null },
        },
        nativeId,
        'USD'
      )
    ).toBe(4.25);
  });

  it('keeps an unknown native rate unavailable', () => {
    expect(
      resolveCanonicalTransferCurrencyRate(
        [token],
        { [token.id]: { rates: { USD: 1 }, usdChange24hPct: null } },
        nativeId,
        'USD'
      )
    ).toBeNull();
  });
});
