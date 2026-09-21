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

describe('ERC-20 presentation', () => {
  const uniContract = '0x1f9840a85d5af5bf1d1762f925bdaddc4201f984';
  const uniRuntimeId = `erc20_${uniContract}`;

  it('uses the catalog token logo for a resolved contract runtime ID', () => {
    const presentation = resolveCoinPresentationById(uniRuntimeId, 'erc20');

    expect(presentation?.icon).toMatchObject({
      kind: 'asset',
      dark: '/images/coin-logos/web3/uni_dark.svg',
    });
  });

  it('uses catalog metadata for a mainnet runtime coin while preserving its ID', () => {
    const coin = {
      ...fallbackSepoliaCoin(),
      id: uniRuntimeId,
      currencyId: uniContract,
      systemId: uniContract,
      displayTicker: 'UNI',
      displayName: 'Uniswap',
      proto: 'erc20' as const,
      compatibleChannels: ['erc20'] as CoinDefinition['compatibleChannels'],
      isTestnet: false,
    };
    const presentation = resolveCoinPresentation(coin);

    expect(presentation.id).toBe(uniRuntimeId);
    expect(presentation.icon).toMatchObject({
      kind: 'asset',
      dark: '/images/coin-logos/web3/uni_dark.svg',
    });

    expect(resolveCoinPresentation({ ...coin, isTestnet: true }).icon.kind).toBe('generated');
  });

  it('does not present an unknown ERC-20 as ETH', () => {
    const presentation = resolveCoinPresentationById(
      'erc20_0x1111111111111111111111111111111111111111',
      'erc20'
    );

    expect(presentation?.icon.kind).toBe('generated');
  });
});
