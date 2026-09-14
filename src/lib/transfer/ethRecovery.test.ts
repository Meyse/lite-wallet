import { describe, expect, it, vi } from 'vitest';
import {
  EthRecoveryLifetime,
  resolveTransferResultContext,
  runEthRecovery,
} from './ethRecovery.js';

const result = {
  txid: `0x${'11'.repeat(32)}`,
  fee: '0.01',
  value: '5',
  toAddress: `0x${'aa'.repeat(20)}`,
  fromAddress: `0x${'bb'.repeat(20)}`,
  recoveryId: 'recovery-a',
};

const review = {
  recoveryId: 'recovery-a',
  stage: 'erc20',
  status: 'broadcast_known',
  txid: result.txid,
  fee: result.fee,
  value: result.value,
  toAddress: result.toAddress,
  fromAddress: result.fromAddress,
  requiresResume: false,
  canAcknowledge: true,
  context: {
    walletNetwork: 'mainnet' as const,
    chainId: 1,
    coinId: 'TOKEN_A',
    channelId: 'erc20.TOKEN_A',
    assetKind: 'erc20' as const,
    contractAddress: `0x${'cc'.repeat(20)}`,
    feeCurrency: 'ETH',
    destinationKind: 'eth' as const,
    value: result.value,
    fee: result.fee,
    toAddress: result.toAddress,
    fromAddress: result.fromAddress,
    bridgeContractAddress: null,
    mappedCurrencyId: null,
    destinationSystemId: null,
  },
};

describe('ETH recovery caller boundary', () => {
  it('uses recovered transfer A rather than the currently edited transfer B', () => {
    expect(
      resolveTransferResultContext(result, review, {
        coinId: 'TOKEN_B',
        channelId: 'erc20.TOKEN_B',
        destinationKind: 'eth',
        toAddress: `0x${'dd'.repeat(20)}`,
      })
    ).toEqual({
      coinId: 'TOKEN_A',
      channelId: 'erc20.TOKEN_A',
      destinationKind: 'eth',
      toAddress: result.toAddress,
    });
  });

  it('does not update a disposed component after recovery resolves', async () => {
    let resolve!: (value: typeof result) => void;
    const lifetime = new EthRecoveryLifetime();
    const complete = vi.fn();
    const request = runEthRecovery({
      lifetime,
      resume: () => new Promise((resolvePromise) => (resolve = resolvePromise)),
      complete,
      fail: vi.fn(),
      settle: vi.fn(),
    });
    lifetime.dispose();
    resolve(result);
    await request;
    expect(complete).not.toHaveBeenCalled();
  });

  it('does not update a replacement wallet after an old-session cancellation', async () => {
    const complete = vi.fn();
    const fail = vi.fn();
    const settle = vi.fn();
    await runEthRecovery({
      lifetime: new EthRecoveryLifetime(),
      resume: () => Promise.reject({ type: 'WalletSessionChanged' }),
      complete,
      fail,
      settle,
    });
    expect(complete).not.toHaveBeenCalled();
    expect(fail).not.toHaveBeenCalled();
    expect(settle).not.toHaveBeenCalled();
  });
});
