import { describe, expect, it } from 'vitest';

import { contactChainId } from '$lib/contacts/identity';
import { isWalletSupportedAsset } from './supportedAssets';

describe('Ethereum asset network support', () => {
  it('accepts an ERC20 contract only on its recorded wallet network', () => {
    const sepoliaToken = {
      id: 'erc20_0x1111111111111111111111111111111111111111',
      currencyId: '0x1111111111111111111111111111111111111111',
      proto: 'erc20' as const,
      isTestnet: true,
    };

    expect(isWalletSupportedAsset(sepoliaToken, 'testnet')).toBe(true);
    expect(isWalletSupportedAsset(sepoliaToken, 'mainnet')).toBe(false);
  });
});

describe('Private testnet asset support', () => {
  it('uses VRSCTEST coins and never mainnet coins', () => {
    const vrsctest = {
      id: 'VRSCTEST',
      currencyId: 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq',
      proto: 'vrsc' as const,
      isTestnet: true,
    };
    const vrsc = {
      id: 'VRSC',
      currencyId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
      proto: 'vrsc' as const,
      isTestnet: false,
    };

    expect(isWalletSupportedAsset(vrsctest, 'privatetestnet')).toBe(true);
    expect(isWalletSupportedAsset(vrsc, 'privatetestnet')).toBe(false);
    expect(contactChainId('privatetestnet')).toBe('iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq');
  });
});
