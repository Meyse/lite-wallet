import { describe, expect, it } from 'vitest';
import {
  classifySubmittedTransferRoute,
  finalizeSubmittedTransferSnapshot,
  getSubmittedReceiptCopy,
  transactionExplorerUrl,
} from './transferReceipt.js';
import type {
  SubmittedTransferRouteSnapshot,
  SubmittedTransferSnapshot,
} from './transferReceipt.js';

const route = (
  overrides: Partial<SubmittedTransferRouteSnapshot> = {}
): SubmittedTransferRouteSnapshot => ({
  sourceChannelId: 'vrpc.address.system',
  sourceSystemId: 'system',
  conversionEnabled: false,
  exportSystemId: null,
  ethereumDestination: false,
  ...overrides,
});

const snapshot = (
  overrides: Partial<SubmittedTransferSnapshot> = {}
): SubmittedTransferSnapshot => ({
  ...route(),
  walletNetwork: 'mainnet',
  sourceAddress: 'Rsource',
  sourceLabel: 'Public address',
  sourceNetworkLabel: 'Verus',
  destinationAddress: 'Rdestination',
  destinationNetworkLabel: 'Verus',
  submittedAmount: '1',
  submittedTicker: 'VRSC',
  estimatedReceive: null,
  ...overrides,
});

describe('submitted transfer route classification', () => {
  it('gives both bridge directions precedence over generic conversion', () => {
    expect(
      classifySubmittedTransferRoute(
        route({ conversionEnabled: true, exportSystemId: 'bridge.vETH', ethereumDestination: true })
      )
    ).toBe('verus_to_ethereum');
    expect(
      classifySubmittedTransferRoute(
        route({
          sourceChannelId: 'erc20.USDC',
          conversionEnabled: true,
          exportSystemId: 'verus',
        })
      )
    ).toBe('ethereum_to_verus');
  });

  it('separates same-chain conversion, PBaaS export, BTC, EVM, and Verus', () => {
    expect(classifySubmittedTransferRoute(route({ conversionEnabled: true }))).toBe('conversion');
    expect(classifySubmittedTransferRoute(route({ exportSystemId: 'chips' }))).toBe(
      'pbaas_cross_chain'
    );
    expect(classifySubmittedTransferRoute(route({ sourceChannelId: 'btc.BTC' }))).toBe('btc');
    expect(classifySubmittedTransferRoute(route({ sourceChannelId: 'eth.ETH' }))).toBe('evm');
    expect(classifySubmittedTransferRoute(route())).toBe('verus');
  });

  it('routes explorers from the immutable source identity', () => {
    expect(
      transactionExplorerUrl(
        snapshot({ sourceChannelId: 'btc.BTC', walletNetwork: 'mainnet' }),
        'abc'
      )
    ).toBe('https://mempool.space/tx/abc');
    expect(
      transactionExplorerUrl(
        snapshot({ sourceChannelId: 'eth.ETH', walletNetwork: 'testnet' }),
        '0xabc'
      )
    ).toBe('https://sepolia.etherscan.io/tx/0xabc');
    expect(
      transactionExplorerUrl(
        snapshot({
          sourceSystemId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
          walletNetwork: 'mainnet',
        }),
        'abc'
      )
    ).toBe('https://scan.verus.cx/vrsc/tx/abc');
    expect(
      transactionExplorerUrl(
        snapshot({
          sourceSystemId: 'iJ3WZocnjG9ufv7GKUA4LijQno5gTMb7tP',
          walletNetwork: 'mainnet',
        }),
        'abc'
      )
    ).toBe('https://scan.verus.cx/chips/tx/abc');
    expect(
      transactionExplorerUrl(
        snapshot({ sourceSystemId: 'iUnsupported', walletNetwork: 'mainnet' }),
        'abc'
      )
    ).toBeNull();
    expect(
      transactionExplorerUrl(
        snapshot({
          sourceSystemId: 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq',
          walletNetwork: 'testnet',
        }),
        'abc'
      )
    ).toBeNull();
  });

  it('keeps recovery explorer routing on its submitted source snapshot', () => {
    const recovery = snapshot({
      sourceChannelId: 'eth.ETH',
      sourceSystemId: 'ethereum',
      walletNetwork: 'mainnet',
    });
    const liveEditedWizard = snapshot({
      sourceChannelId: 'btc.BTC',
      sourceSystemId: 'bitcoin',
      walletNetwork: 'testnet',
    });

    expect(transactionExplorerUrl(recovery, '0xabc')).toBe('https://etherscan.io/tx/0xabc');
    expect(transactionExplorerUrl(recovery, '0xabc')).not.toBe(
      transactionExplorerUrl(liveEditedWizard, '0xabc')
    );
  });

  it('replaces mutable draft addresses and amount with the submitted result', () => {
    const finalized = finalizeSubmittedTransferSnapshot(
      snapshot({
        sourceAddress: 'RdraftFrom',
        sourceLabel: 'Public address',
        sourceNetworkLabel: 'Verus',
        destinationAddress: 'RdraftTo',
        destinationNetworkLabel: 'Verus',
        submittedAmount: '10',
        submittedTicker: 'VRSC',
        estimatedReceive: null,
      }),
      {
        txid: 'abc',
        fee: '0.0001',
        value: '9.9999',
        fromAddress: 'RauthoritativeFrom',
        toAddress: 'RauthoritativeTo',
      }
    );

    expect(finalized.sourceAddress).toBe('RauthoritativeFrom');
    expect(finalized.destinationAddress).toBe('RauthoritativeTo');
    expect(finalized.submittedAmount).toBe('9.9999');
  });

  it('uses the operation snapshot for the submitted title', () => {
    const t = (key: string) => key;
    expect(getSubmittedReceiptCopy('pbaas_cross_chain', true, t).title).toBe(
      'wallet.transfer.receipt.conversionSubmitted'
    );
    expect(getSubmittedReceiptCopy('pbaas_cross_chain', false, t).title).toBe(
      'wallet.transfer.receipt.sendSubmitted'
    );
  });
});
