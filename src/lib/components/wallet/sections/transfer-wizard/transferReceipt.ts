import type { SendResult, WalletNetwork } from '$lib/types/wallet.js';

const VERUS_MAINNET_SYSTEM_ID = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
const CHIPS_MAINNET_SYSTEM_ID = 'iJ3WZocnjG9ufv7GKUA4LijQno5gTMb7tP';

const VERUS_EXPLORER_SLUG_BY_SYSTEM_ID: Readonly<Record<string, string>> = {
  [VERUS_MAINNET_SYSTEM_ID]: 'vrsc',
  [CHIPS_MAINNET_SYSTEM_ID]: 'chips',
};

export type SubmittedTransferRoute =
  | 'conversion'
  | 'verus'
  | 'pbaas_cross_chain'
  | 'btc'
  | 'evm'
  | 'verus_to_ethereum'
  | 'ethereum_to_verus';

export type SubmittedTransferRouteSnapshot = {
  sourceChannelId: string;
  sourceSystemId: string;
  conversionEnabled: boolean;
  exportSystemId: string | null;
  ethereumDestination: boolean;
};

export type SubmittedTransferSnapshot = SubmittedTransferRouteSnapshot & {
  walletNetwork: WalletNetwork;
  sourceAddress: string;
  sourceLabel: string;
  sourceNetworkLabel: string;
  destinationAddress: string;
  destinationNetworkLabel: string;
  submittedAmount: string;
  submittedTicker: string;
  estimatedReceive: string | null;
};

export function classifySubmittedTransferRoute(
  snapshot: SubmittedTransferRouteSnapshot
): SubmittedTransferRoute {
  const sourcePrefix = snapshot.sourceChannelId.trim().split('.')[0]?.toLowerCase() ?? '';

  // Bridge direction wins over the generic conversion/cross-chain categories.
  if (sourcePrefix === 'eth' || sourcePrefix === 'erc20') {
    if (snapshot.conversionEnabled || snapshot.exportSystemId) return 'ethereum_to_verus';
  }
  if (snapshot.ethereumDestination && snapshot.exportSystemId) return 'verus_to_ethereum';
  if (snapshot.conversionEnabled && !snapshot.exportSystemId) return 'conversion';
  if (snapshot.exportSystemId) return 'pbaas_cross_chain';
  if (sourcePrefix === 'btc') return 'btc';
  if (sourcePrefix === 'eth' || sourcePrefix === 'erc20') return 'evm';
  return 'verus';
}

export function getSubmittedReceiptCopy(
  route: SubmittedTransferRoute,
  isConversion: boolean,
  t: (key: string) => string
): { title: string; timing: string } {
  return {
    title: t(
      isConversion
        ? 'wallet.transfer.receipt.conversionSubmitted'
        : 'wallet.transfer.receipt.sendSubmitted'
    ),
    timing: t(`wallet.transfer.receipt.timing.${route}`),
  };
}

export function transactionExplorerUrl(
  snapshot: SubmittedTransferSnapshot,
  txid: string
): string | null {
  const encodedTxid = encodeURIComponent(txid.trim());
  if (!encodedTxid) return null;
  const sourcePrefix = snapshot.sourceChannelId.trim().split('.')[0]?.toLowerCase() ?? '';

  if (sourcePrefix === 'btc') {
    return snapshot.walletNetwork === 'testnet'
      ? `https://mempool.space/testnet/tx/${encodedTxid}`
      : `https://mempool.space/tx/${encodedTxid}`;
  }
  if (sourcePrefix === 'eth' || sourcePrefix === 'erc20') {
    return snapshot.walletNetwork === 'testnet'
      ? `https://sepolia.etherscan.io/tx/${encodedTxid}`
      : `https://etherscan.io/tx/${encodedTxid}`;
  }
  if (snapshot.walletNetwork !== 'mainnet') return null;

  const explorerSlug = VERUS_EXPLORER_SLUG_BY_SYSTEM_ID[snapshot.sourceSystemId.trim()];
  return explorerSlug ? `https://scan.verus.cx/${explorerSlug}/tx/${encodedTxid}` : null;
}

export function finalizeSubmittedTransferSnapshot(
  snapshot: SubmittedTransferSnapshot,
  result: SendResult
): SubmittedTransferSnapshot {
  return {
    ...snapshot,
    sourceAddress: result.fromAddress.trim() || snapshot.sourceAddress,
    destinationAddress: result.toAddress.trim() || snapshot.destinationAddress,
    submittedAmount: result.value.trim() || snapshot.submittedAmount,
  };
}
