import type { BridgeTransferPreflightParams, PreflightParams } from '$lib/types/wallet.js';
import type { PreflightRequestGuard } from './preflightRequestGuard.js';

export type ResolvedPreflightRequest =
  | { kind: 'direct'; params: PreflightParams }
  | { kind: 'bridge'; params: BridgeTransferPreflightParams };

export type PreflightWalletContext = {
  walletKey: string;
  walletNetwork: string;
};

export function transferWalletSessionKey(
  walletName: string,
  walletNetwork: string,
  walletSessionId: string
): string {
  return `${walletName.trim().toLowerCase()}::${walletNetwork}::${walletSessionId}`;
}

export function preflightRequestSignature(
  request: ResolvedPreflightRequest | null,
  context?: PreflightWalletContext
): string {
  return request ? JSON.stringify({ context: context ?? null, request }) : '';
}

export type GuardedPreflightOutcome<T> =
  { status: 'applied'; value: T } | { status: 'failed'; error: unknown } | { status: 'stale' };

export async function runGuardedPreflight<T>(options: {
  guard: PreflightRequestGuard;
  signature: string;
  currentSignature: () => string;
  execute: () => Promise<T>;
}): Promise<GuardedPreflightOutcome<T>> {
  const token = options.guard.begin(options.signature);
  try {
    const value = await options.execute();
    return options.guard.isCurrent(token, options.currentSignature())
      ? { status: 'applied', value }
      : { status: 'stale' };
  } catch (error) {
    return options.guard.isCurrent(token, options.currentSignature())
      ? { status: 'failed', error }
      : { status: 'stale' };
  }
}
