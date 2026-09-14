import { beforeEach, describe, expect, it, vi } from 'vitest';

const invokeWalletCommandMock = vi.hoisted(() => vi.fn());
const invalidateWalletDisplayHistoryMock = vi.hoisted(() => vi.fn());

vi.mock('./invokeWalletCommand.js', () => ({
  invokeWalletCommand: invokeWalletCommandMock,
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
    invokeWalletCommandMock.mockResolvedValueOnce(undefined);

    await expect(acknowledgePendingEthSubmission('recovery-1')).resolves.toBeUndefined();
    expect(invokeWalletCommandMock.mock.calls).toEqual([
      ['acknowledge_pending_eth_submission', { recovery_id: 'recovery-1' }],
    ]);
    expect(invalidateWalletDisplayHistoryMock).not.toHaveBeenCalled();
  });

  it('uses the explicit recovery id without acknowledging before its caller renders', async () => {
    invokeWalletCommandMock.mockResolvedValueOnce(recoveredResult);

    await expect(resumePendingEthSubmission('recovery-1')).resolves.toEqual(recoveredResult);
    expect(invokeWalletCommandMock.mock.calls).toEqual([
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
    };
    invokeWalletCommandMock.mockResolvedValueOnce(review);

    await expect(getPendingEthSubmission()).resolves.toEqual(review);
    expect(invokeWalletCommandMock.mock.calls).toEqual([['get_pending_eth_submission']]);
    expect(invalidateWalletDisplayHistoryMock).not.toHaveBeenCalled();
  });
});
