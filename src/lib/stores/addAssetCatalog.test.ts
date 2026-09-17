import { describe, expect, it } from 'vitest';

import { buildAddAssetCatalogView } from './addAssetCatalog';

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
