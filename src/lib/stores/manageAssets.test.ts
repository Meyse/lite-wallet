import { afterEach, describe, expect, it, vi } from 'vitest';

import type {
  AssetDiscoveryHolding,
  BalanceResult,
  CoinDefinition,
  CoinScopesResult,
} from '$lib/types/wallet.js';
import {
  aggregateDiscoveryHoldings,
  ASSET_LOOKUP_TIMEOUT_MS,
  assetKeyForCoin,
  finiteAssetBalance,
  formatAssetBalance,
  scanKnownNonVrpcAssets,
} from './manageAssets.js';

const ethCoin: CoinDefinition = {
  id: 'ETH',
  currencyId: '0x0000000000000000000000000000000000000000',
  systemId: 'ETH',
  displayTicker: 'ETH',
  displayName: 'Ethereum',
  proto: 'eth',
  compatibleChannels: ['eth'],
  decimals: 18,
  vrpcEndpoints: [],
  secondsPerBlock: 12,
  isTestnet: false,
};

describe('manage assets state', () => {
  afterEach(() => vi.useRealTimers());

  it.each(['scope', 'balance'])(
    'publishes healthy balances before a stalled %s lookup times out and ignores its late result',
    async (stalledStep) => {
      vi.useFakeTimers();
      const stalled = {
        ...ethCoin,
        id: 'UNI',
        currencyId: '0x1111111111111111111111111111111111111111',
        proto: 'erc20' as const,
      };
      const scopes = (coinId: string): CoinScopesResult => ({
        coinId,
        scopes: [
          {
            channelId: `eth.${coinId}`,
            coinId,
            address: '0xexample',
            addressLabel: 'example',
            systemId: 'ETH',
            systemTicker: 'ETH',
            systemDisplayName: 'Ethereum',
            isPrimaryAddress: true,
            isReadOnly: false,
            scopeKind: 'transparent',
          },
        ],
      });
      let finishScopes!: (value: CoinScopesResult) => void;
      let finishBalance!: (value: BalanceResult) => void;
      const scopeRequest = new Promise<CoinScopesResult>((resolve) => (finishScopes = resolve));
      const balanceRequest = new Promise<BalanceResult>((resolve) => (finishBalance = resolve));
      const getBalance = vi.fn(async (_channelId: string, coinId: string) =>
        coinId === 'UNI' && stalledStep === 'balance'
          ? balanceRequest
          : { total: '3', confirmed: '3', pending: '0' }
      );
      const onBalance = vi.fn();
      const request = scanKnownNonVrpcAssets(
        [ethCoin, stalled],
        async (coinId) =>
          coinId === 'UNI' && stalledStep === 'scope' ? scopeRequest : scopes(coinId),
        getBalance,
        onBalance
      );
      await vi.advanceTimersByTimeAsync(0);
      expect(onBalance).toHaveBeenCalledTimes(1);
      expect(onBalance.mock.calls[0][0]).toMatchObject({
        coin: { id: 'ETH' },
        balance: '3',
        status: 'available',
      });
      await vi.advanceTimersByTimeAsync(ASSET_LOOKUP_TIMEOUT_MS);
      expect((await request)[1]).toMatchObject({
        coin: { id: 'UNI' },
        balance: null,
        status: 'unavailable',
      });
      expect(onBalance).toHaveBeenCalledTimes(2);
      finishScopes(scopes('UNI'));
      finishBalance({ total: '999', confirmed: '999', pending: '0' });
      await vi.advanceTimersByTimeAsync(0);
      expect(onBalance).toHaveBeenCalledTimes(2);
      if (stalledStep === 'scope') expect(getBalance).toHaveBeenCalledTimes(1);
      expect(vi.getTimerCount()).toBe(0);
    }
  );
  it('aggregates one PBaaS currency across holding networks without aliasing ETH', () => {
    const holdings: AssetDiscoveryHolding[] = [
      {
        assetKey: 'vrsc:icurrency',
        currencyId: 'iCurrency',
        systemId: 'iVerus',
        systemTicker: 'VRSC',
        systemDisplayName: 'Verus',
        balance: '0.00000001',
        includesReadOnly: false,
        balanceStatus: 'available',
      },
      {
        assetKey: 'vrsc:icurrency',
        currencyId: 'iCurrency',
        systemId: 'ivDEX',
        systemTicker: 'vDEX',
        systemDisplayName: 'vDEX',
        balance: '2',
        includesReadOnly: true,
        balanceStatus: 'partial',
      },
    ];

    const [currency] = aggregateDiscoveryHoldings(holdings);
    expect(currency.totalDisplay).toBe('2.00000001');
    expect(currency.networks).toHaveLength(2);
    expect(currency.includesReadOnly).toBe(true);
    expect(currency.status).toBe('partial');
    expect(assetKeyForCoin(ethCoin)).toBe('eth:0x0000000000000000000000000000000000000000');
    expect(finiteAssetBalance('0.00000001')).toBeGreaterThan(0);
    expect(formatAssetBalance(26.5)).toBe('26.5');
  });

  it('retains successful known-chain balances when another provider is unavailable', async () => {
    const broken = {
      ...ethCoin,
      id: 'BROKEN',
      displayName: 'Broken token',
      proto: 'erc20' as const,
    };
    const results = await scanKnownNonVrpcAssets(
      [ethCoin, broken],
      async (coinId) => {
        if (coinId === 'BROKEN') throw new Error('provider unavailable');
        return {
          coinId,
          scopes: [
            {
              channelId: 'eth.ETH',
              coinId,
              address: '0xexample',
              addressLabel: '0xexample',
              systemId: 'ETH',
              systemTicker: 'ETH',
              systemDisplayName: 'Ethereum',
              isPrimaryAddress: true,
              isReadOnly: false,
              scopeKind: 'transparent',
            },
          ],
        };
      },
      async () => ({ confirmed: '0', pending: '0', total: '0' })
    );

    expect(results.map((result) => result.status)).toEqual(['available', 'unavailable']);
    expect(results[0].balance).toBe('0');
    expect(results[1].balance).toBeNull();
  });
});
