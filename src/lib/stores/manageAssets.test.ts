import { describe, expect, it } from 'vitest';

import type { AssetDiscoveryHolding, CoinDefinition } from '$lib/types/wallet.js';
import {
  aggregateDiscoveryHoldings,
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
