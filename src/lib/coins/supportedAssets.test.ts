import { describe, expect, it } from 'vitest';

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
