import { describe, expect, it, vi } from 'vitest';
import {
  EthRecoveryLifetime,
  resolveTransferResultContext,
  runEthRecovery,
} from './ethRecovery.js';
import type { EthPendingSubmissionReview } from '$lib/types/wallet.js';

const result = {
  txid: `0x${'11'.repeat(32)}`,
  fee: '0.01',
  value: '5',
  toAddress: `0x${'aa'.repeat(20)}`,
  fromAddress: `0x${'bb'.repeat(20)}`,
  recoveryId: 'recovery-a',
};

const review: EthPendingSubmissionReview = {
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

const advancedReview: EthPendingSubmissionReview = {
  ...review,
  recoveryId: 'recovery-b',
  stage: 'bridge_transfer',
  status: 'prepared',
  requiresResume: true,
  canAcknowledge: false,
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
      reload: vi.fn(),
      complete,
      refresh: vi.fn(),
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
    const reload = vi.fn();
    await runEthRecovery({
      lifetime: new EthRecoveryLifetime(),
      resume: () => Promise.reject({ type: 'WalletSessionChanged' }),
      reload,
      complete,
      refresh: vi.fn(),
      fail,
      settle,
    });
    expect(complete).not.toHaveBeenCalled();
    expect(fail).not.toHaveBeenCalled();
    expect(settle).not.toHaveBeenCalled();
    expect(reload).not.toHaveBeenCalled();
  });

  it('shows advanced review B and submits it only after the next explicit click', async () => {
    let displayedReview: EthPendingSubmissionReview = {
      ...review,
      stage: 'bridge_approval',
      requiresResume: true,
    };
    let visibleError = '';
    const resume = vi
      .fn<(recoveryId: string) => Promise<typeof result>>()
      .mockRejectedValueOnce({ type: 'EthBroadcastUncertain' })
      .mockResolvedValueOnce({ ...result, recoveryId: advancedReview.recoveryId });
    const complete = vi.fn();
    const lifetime = new EthRecoveryLifetime();

    async function clickContinue() {
      const reviewedSubmission = displayedReview;
      await runEthRecovery({
        lifetime,
        resume: () => resume(reviewedSubmission.recoveryId),
        reload: () => Promise.resolve(advancedReview),
        complete,
        refresh: (nextReview) => {
          if (nextReview) displayedReview = nextReview;
        },
        fail: (_error, recovery) => {
          visibleError = recovery.review?.recoveryId === advancedReview.recoveryId ? 'visible' : '';
        },
        settle: vi.fn(),
      });
    }

    await clickContinue();
    expect(displayedReview.recoveryId).toBe('recovery-b');
    expect(displayedReview.stage).toBe('bridge_transfer');
    expect(visibleError).toBe('visible');
    expect(resume).toHaveBeenCalledTimes(1);
    expect(resume).toHaveBeenLastCalledWith('recovery-a');
    expect(complete).not.toHaveBeenCalled();

    await clickContinue();
    expect(resume.mock.calls.map(([recoveryId]) => recoveryId)).toEqual([
      'recovery-a',
      'recovery-b',
    ]);
    expect(complete).toHaveBeenCalledTimes(1);
  });

  it('clears the pending review when no recovery journal remains', async () => {
    let displayedReview: typeof review | null = review;
    const fail = vi.fn();
    await runEthRecovery({
      lifetime: new EthRecoveryLifetime(),
      resume: () => Promise.reject({ type: 'NetworkError' }),
      reload: () => Promise.resolve(null),
      complete: vi.fn(),
      refresh: (nextReview) => (displayedReview = nextReview),
      fail,
      settle: vi.fn(),
    });
    expect(displayedReview).toBeNull();
    expect(fail).toHaveBeenCalledWith(
      { type: 'NetworkError' },
      { review: null, reloadFailed: false }
    );
  });

  it('preserves the last review with a visible failure when reload fails', async () => {
    const displayedReview = review;
    const refresh = vi.fn();
    const fail = vi.fn();
    await runEthRecovery({
      lifetime: new EthRecoveryLifetime(),
      resume: () => Promise.reject({ type: 'NetworkError' }),
      reload: () => Promise.reject({ type: 'SecureStorageUnavailable' }),
      complete: vi.fn(),
      refresh,
      fail,
      settle: vi.fn(),
    });
    expect(displayedReview).toBe(review);
    expect(refresh).not.toHaveBeenCalled();
    expect(fail).toHaveBeenCalledWith({ type: 'NetworkError' }, { reloadFailed: true });
  });

  it('does not update stale UI when disposed during the recovery reload', async () => {
    let resolveReload!: (value: typeof advancedReview) => void;
    const lifetime = new EthRecoveryLifetime();
    const refresh = vi.fn();
    const fail = vi.fn();
    const settle = vi.fn();
    const reload = vi.fn(
      () => new Promise<typeof advancedReview>((resolve) => (resolveReload = resolve))
    );
    const request = runEthRecovery({
      lifetime,
      resume: () => Promise.reject({ type: 'NetworkError' }),
      reload,
      complete: vi.fn(),
      refresh,
      fail,
      settle,
    });
    await vi.waitFor(() => expect(reload).toHaveBeenCalledTimes(1));
    lifetime.dispose();
    resolveReload(advancedReview);
    await request;
    expect(refresh).not.toHaveBeenCalled();
    expect(fail).not.toHaveBeenCalled();
    expect(settle).not.toHaveBeenCalled();
  });
});
