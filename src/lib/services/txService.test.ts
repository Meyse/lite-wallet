import { beforeEach, describe, expect, it, vi } from 'vitest';

const invokeWalletCommandMock = vi.hoisted(() => vi.fn());
const invokeSessionBoundWalletCommandMock = vi.hoisted(() => vi.fn());
const invalidateWalletDisplayHistoryMock = vi.hoisted(() => vi.fn());

vi.mock('./invokeWalletCommand.js', () => ({
  invokeWalletCommand: invokeWalletCommandMock,
  invokeSessionBoundWalletCommand: invokeSessionBoundWalletCommandMock,
}));

vi.mock('./walletDisplayService.js', () => ({
  invalidateWalletDisplayHistory: invalidateWalletDisplayHistoryMock,
}));

import {
  acknowledgePendingEthSubmission,
  getPendingEthSubmission,
  resumePendingEthSubmission,
  sendTransaction,
} from './txService.js';

const recoveredResult = {
  txid: `0x${'11'.repeat(32)}`,
  fee: '0.001',
  value: '1',
  toAddress: `0x${'22'.repeat(20)}`,
  fromAddress: `0x${'33'.repeat(20)}`,
  recoveryId: 'recovery-1',
};

describe('txService ETH recovery boundary', () => {
  beforeEach(() => {
    invokeWalletCommandMock.mockReset();
    invokeSessionBoundWalletCommandMock.mockReset();
    invalidateWalletDisplayHistoryMock.mockReset();
  });

  it('returns a durable send result without acknowledging it', async () => {
    invokeWalletCommandMock.mockResolvedValueOnce(recoveredResult);

    await expect(sendTransaction({ preflightId: 'preflight-1' })).resolves.toEqual(recoveredResult);
    expect(invokeWalletCommandMock.mock.calls).toEqual([
      ['send_transaction', { request: { preflightId: 'preflight-1' } }],
    ]);
    expect(invalidateWalletDisplayHistoryMock).toHaveBeenCalledOnce();
  });

  it('acknowledges only through the explicit command', async () => {
    invokeSessionBoundWalletCommandMock.mockResolvedValueOnce(undefined);

    await expect(acknowledgePendingEthSubmission('recovery-1')).resolves.toBeUndefined();
    expect(invokeSessionBoundWalletCommandMock.mock.calls).toEqual([
      ['acknowledge_pending_eth_submission', { recovery_id: 'recovery-1' }],
    ]);
    expect(invalidateWalletDisplayHistoryMock).not.toHaveBeenCalled();
  });

  it('uses the explicit recovery id without acknowledging before its caller renders', async () => {
    invokeSessionBoundWalletCommandMock.mockResolvedValueOnce(recoveredResult);

    await expect(resumePendingEthSubmission('recovery-1')).resolves.toEqual(recoveredResult);
    expect(invokeSessionBoundWalletCommandMock.mock.calls).toEqual([
      ['resume_pending_eth_submission', { recovery_id: 'recovery-1' }],
    ]);
  });

  it('loads the durable recovery review without acknowledging or mutating it', async () => {
    const review = {
      recoveryId: 'recovery-1',
      stage: 'bridge_approval',
      status: 'confirmed',
      txid: recoveredResult.txid,
      fee: recoveredResult.fee,
      value: recoveredResult.value,
      toAddress: recoveredResult.toAddress,
      fromAddress: recoveredResult.fromAddress,
      requiresResume: true,
      canAcknowledge: false,
      context: {
        walletNetwork: 'mainnet',
        chainId: 1,
        coinId: 'TOKEN_A',
        channelId: 'erc20.TOKEN_A',
        assetKind: 'bridge',
        contractAddress: `0x${'44'.repeat(20)}`,
        feeCurrency: 'ETH',
        destinationKind: 'vrpc',
        value: recoveredResult.value,
        fee: recoveredResult.fee,
        toAddress: recoveredResult.toAddress,
        fromAddress: recoveredResult.fromAddress,
        bridgeContractAddress: `0x${'55'.repeat(20)}`,
        mappedCurrencyId: 'iMappedCurrency',
        destinationSystemId: 'iDestinationSystem',
      },
    };
    invokeSessionBoundWalletCommandMock.mockResolvedValueOnce(review);

    await expect(getPendingEthSubmission()).resolves.toEqual(review);
    expect(invokeSessionBoundWalletCommandMock.mock.calls).toEqual([
      ['get_pending_eth_submission'],
    ]);
    expect(invalidateWalletDisplayHistoryMock).not.toHaveBeenCalled();
  });
});
