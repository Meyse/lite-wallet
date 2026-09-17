import { describe, expect, it } from 'vitest';

import { resolveCoinPresentation, resolveCoinPresentationById } from './presentation';
import type { CoinDefinition } from '$lib/types/wallet.js';

function fallbackSepoliaCoin(): CoinDefinition {
  return {
    id: 'geth',
    currencyId: '0x0000000000000000000000000000000000000000',
    systemId: 'GETH',
    displayTicker: 'GETH',
    displayName: 'Ethereum Testnet',
    coinPaprikaId: null,
    proto: 'eth',
    compatibleChannels: ['eth'],
    decimals: 18,
    vrpcEndpoints: [],
    electrumEndpoints: null,
    secondsPerBlock: 12,
    mappedTo: null,
    isTestnet: true,
  };
}

describe('Sepolia compatibility presentation', () => {
  it('overrides imported gETH catalog labels without renaming the compatibility id', () => {
    const presentation = resolveCoinPresentationById('GETH', 'eth');

    expect(presentation).toMatchObject({
      id: 'GETH',
      displayTicker: 'ETH',
      displayName: 'Sepolia ETH',
      isTestnet: true,
      source: 'catalog',
    });
  });

  it('overrides backend fallback labels using the same policy', () => {
    const presentation = resolveCoinPresentation(fallbackSepoliaCoin());

    expect(presentation.source).toBe('fallback');
    expect(presentation.id).toBe('geth');
    expect(presentation.displayTicker).toBe('ETH');
    expect(presentation.displayName).toBe('Sepolia ETH');
  });
});
