import { get } from 'svelte/store';
import { setContactSession } from '$lib/contacts/session';
import { setWatchlistSession, watchlistSessionEpoch } from '$lib/watchlist/session';
import type { WalletNetwork } from '$lib/types/wallet.js';
import { walletLockEpoch } from './walletLockCoordinator.js';
import * as walletService from './walletService.js';

export interface RecoveredWalletSession {
  name: string;
  emoji: string;
  color: string;
  network: WalletNetwork;
  sessionId: string;
}

/**
 * Re-reads the active wallet after the route had to fall back to placeholder
 * metadata. Returns null when the backend has no verified active wallet, and
 * discards a result if a forced lock or session reset happened while the read
 * was pending. It never fabricates a placeholder binding or reuses a previous
 * wallet session.
 */
export async function recoverActiveWalletSession(): Promise<RecoveredWalletSession | null> {
  const lockEpoch = get(walletLockEpoch);
  const watchlistEpoch = get(watchlistSessionEpoch);
  const active = await walletService.getActiveWallet();
  if (
    !active ||
    get(walletLockEpoch) !== lockEpoch ||
    get(watchlistSessionEpoch) !== watchlistEpoch
  ) {
    return null;
  }
  return {
    name: active.wallet_name,
    emoji: active.emoji || '💰',
    color: active.color || 'blue',
    network: active.network ?? 'mainnet',
    sessionId: active.session_id,
  };
}

/**
 * Binds the route-owned Contact and Watchlist sessions to a verified wallet.
 * The caller decides when recovery is still relevant for its route lifetime.
 */
export function bindRecoveredWalletSession(session: RecoveredWalletSession): void {
  const binding = { sessionId: session.sessionId, network: session.network };
  setContactSession(binding);
  setWatchlistSession(binding);
}
