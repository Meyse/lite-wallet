import type { WalletNetwork } from '$lib/types/wallet.js';
import { normalizeWalletOverviewSort, type WalletOverviewSort } from '$lib/utils/walletOverview.js';

export interface WalletOverviewPreferences {
  sort: WalletOverviewSort;
  withBalance: boolean;
  reversed: boolean;
}

// Wallet names are the persistent identity used by the wallet route; session IDs change on unlock.
function storageKey(walletName: string, network: WalletNetwork): string {
  return `lite-wallet.overview.v1:${JSON.stringify([walletName.trim().toLowerCase(), network])}`;
}

export function readWalletOverviewPreferences(
  walletName: string,
  network: WalletNetwork
): WalletOverviewPreferences {
  try {
    const raw = globalThis.localStorage?.getItem(storageKey(walletName, network));
    const value = raw ? JSON.parse(raw) : null;
    const sort = normalizeWalletOverviewSort(value?.sort);
    return {
      sort,
      reversed: sort !== 'verus-first' && value?.reversed === true,
      withBalance: value?.withBalance === true,
    };
  } catch {
    return { sort: 'verus-first', withBalance: false, reversed: false };
  }
}

export function writeWalletOverviewPreferences(
  walletName: string,
  network: WalletNetwork,
  preferences: WalletOverviewPreferences
): void {
  try {
    const sort = normalizeWalletOverviewSort(preferences.sort);
    globalThis.localStorage?.setItem(
      storageKey(walletName, network),
      JSON.stringify({
        sort,
        reversed: sort !== 'verus-first' && preferences.reversed === true,
        withBalance: preferences.withBalance === true,
      })
    );
  } catch {
    // Like other display preferences, keep the current selection usable if storage is unavailable.
  }
}
