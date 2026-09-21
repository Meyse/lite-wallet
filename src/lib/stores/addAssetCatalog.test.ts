import { describe, expect, it } from 'vitest';

import { buildAddAssetCatalogView } from './addAssetCatalog';
import type { CoinDefinition } from '$lib/types/wallet';

describe('custom asset catalog identity', () => {
  it.each([true, false])(
    'merges registered UNI with the catalog while preserving portfolio state %s',
    (active) => {
      const contract = '0x1f9840a85d5af5bf1d1762f925bdaddc4201f984';
      const coin: CoinDefinition = {
        id: `erc20_${contract}`,
        currencyId: `0x${contract.slice(2).toUpperCase()}`,
        systemId: contract,
        displayTicker: 'UNI',
        displayName: 'Uniswap',
        proto: 'erc20',
        compatibleChannels: ['erc20'],
        decimals: 18,
        vrpcEndpoints: [],
        secondsPerBlock: 12,
        isTestnet: false,
      };
      const view = buildAddAssetCatalogView({
        coins: [coin],
        network: 'mainnet',
        query: 'UNI',
        activeCoinIds: active ? [coin.id] : [],
      });
      const entries = [...view.addedEntries, ...view.availableEntries].filter(
        (entry) => entry.currencyId.toLowerCase() === contract
      );
      expect(entries).toHaveLength(1);
      expect(entries[0]).toMatchObject({
        id: coin.id,
        source: 'both',
        addStrategy: 'activate',
        status: active ? 'added' : 'available',
      });
    }
  );
});

describe('Sepolia asset catalog presentation', () => {
  it('keeps the compatibility id while hiding imported gETH labels', () => {
    const view = buildAddAssetCatalogView({
      coins: [],
      network: 'testnet',
      query: '',
      activeCoinIds: [],
    });
    const sepolia = view.availableEntries.find((entry) => entry.id === 'GETH');

    expect(sepolia).toMatchObject({
      id: 'GETH',
      displayTicker: 'ETH',
      displayName: 'Sepolia ETH',
      source: 'catalog',
    });
  });
});
