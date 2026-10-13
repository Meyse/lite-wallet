import type { Protocol, WalletNetwork } from '$lib/types/wallet.js';
import { isTestChain } from '$lib/utils/walletNetwork';

type SupportedAssetProto = Protocol | 'fiat';

export interface SupportedAssetInput {
  id: string;
  currencyId: string;
  proto: SupportedAssetProto;
  isTestnet: boolean;
}

/**
 * Central wallet capability gate.
 * Supported wallet assets stay bound to the active wallet network.
 */
export function isWalletSupportedAsset(
  asset: SupportedAssetInput,
  network: WalletNetwork
): boolean {
  const isTestnet = isTestChain(network);
  if (asset.isTestnet !== isTestnet) {
    return false;
  }

  if (asset.proto === 'vrsc' || asset.proto === 'eth') {
    return true;
  }

  if (asset.proto === 'erc20') {
    return asset.currencyId.startsWith('0x');
  }

  if (asset.proto === 'btc') {
    return asset.id === 'BTC' || asset.id === 'BTCTEST';
  }

  return false;
}
