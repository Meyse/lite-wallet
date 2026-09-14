import type { EthPendingSubmissionReview, SendResult } from '$lib/types/wallet.js';
import { extractWalletErrorType } from '$lib/utils/walletErrors.js';

type TransferDestinationKind = 'vrpc' | 'btc' | 'eth' | 'dlight';

export type EthRecoveryToken = Readonly<{ sequence: number }>;

export class EthRecoveryLifetime {
  private sequence = 0;
  private disposed = false;

  begin(): EthRecoveryToken {
    this.sequence += 1;
    return { sequence: this.sequence };
  }

  dispose(): void {
    this.disposed = true;
    this.sequence += 1;
  }

  isActive(token: EthRecoveryToken): boolean {
    return !this.disposed && token.sequence === this.sequence;
  }
}

export type TransferResultContext = {
  coinId: string;
  channelId: string;
  destinationKind: TransferDestinationKind;
  toAddress: string;
};

export function resolveTransferResultContext(
  result: SendResult,
  recovered: EthPendingSubmissionReview | null,
  current: TransferResultContext
): TransferResultContext {
  if (!recovered) return current;
  return {
    coinId: recovered.context.coinId,
    channelId: recovered.context.channelId,
    destinationKind: recovered.context.destinationKind,
    toAddress: recovered.context.toAddress || result.toAddress,
  };
}

type RunEthRecoveryOptions = {
  lifetime: EthRecoveryLifetime;
  resume: () => Promise<SendResult>;
  reload: () => Promise<EthPendingSubmissionReview | null>;
  complete: (result: SendResult) => void;
  refresh: (review: EthPendingSubmissionReview | null) => void;
  fail: (
    error: unknown,
    recovery: {
      review?: EthPendingSubmissionReview | null;
      reloadFailed: boolean;
    }
  ) => void;
  settle: () => void;
};

export async function runEthRecovery({
  lifetime,
  resume,
  reload,
  complete,
  refresh,
  fail,
  settle,
}: RunEthRecoveryOptions): Promise<void> {
  const token = lifetime.begin();
  try {
    const result = await resume();
    if (lifetime.isActive(token)) complete(result);
  } catch (error) {
    if (!lifetime.isActive(token)) return;
    const errorType = extractWalletErrorType(error);
    if (errorType === 'WalletSessionChanged' || errorType === 'WalletLocked') {
      lifetime.dispose();
      return;
    }
    try {
      const review = await reload();
      if (!lifetime.isActive(token)) return;
      refresh(review);
      fail(error, { review, reloadFailed: false });
    } catch (reloadError) {
      if (!lifetime.isActive(token)) return;
      const reloadErrorType = extractWalletErrorType(reloadError);
      if (reloadErrorType === 'WalletSessionChanged' || reloadErrorType === 'WalletLocked') {
        lifetime.dispose();
        return;
      }
      fail(error, { reloadFailed: true });
    }
  } finally {
    if (lifetime.isActive(token)) settle();
  }
}
