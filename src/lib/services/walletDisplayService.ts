import { invokeWalletCommand } from './invokeWalletCommand.js';
import { resetTransactionHistoryPages } from '$lib/stores/transactionHistoryPages.js';
import type {
  BalanceResult,
  CoinScopesResult,
  DlightSeedStatusResult,
  TransactionHistoryPage,
} from '$lib/types/wallet.js';

const DISPLAY_BALANCE_TTL_MS = 5_000;

type TimedBalance = {
  value: BalanceResult;
  expiresAt: number;
};

export class WalletDisplayRequestInvalidatedError extends Error {
  constructor() {
    super('Wallet display request invalidated');
    this.name = 'WalletDisplayRequestInvalidatedError';
  }
}

let sessionGeneration = 0;
let scopeGeneration = 0;
let historyGeneration = 0;

const scopeCache = new Map<string, CoinScopesResult>();
const scopeRequests = new Map<string, Promise<CoinScopesResult>>();
const balanceCache = new Map<string, TimedBalance>();
const balanceRequests = new Map<string, Promise<BalanceResult>>();
const historyRequests = new Map<string, Promise<TransactionHistoryPage>>();
let dlightStatusCache: DlightSeedStatusResult | null = null;
let dlightStatusRequest: Promise<DlightSeedStatusResult> | null = null;

function balanceKey(channelId: string, coinId?: string): string {
  return `${channelId}::${coinId ?? ''}`;
}

function historyKey(
  channelId: string,
  coinId: string | undefined,
  cursor: string | undefined,
  limit: number
): string {
  return `${channelId}::${coinId ?? ''}::${cursor ?? ''}::${limit}`;
}

export function isWalletDisplayRequestInvalidated(
  error: unknown
): error is WalletDisplayRequestInvalidatedError {
  return error instanceof WalletDisplayRequestInvalidatedError;
}

export function resetWalletDisplaySession(): void {
  sessionGeneration += 1;
  scopeGeneration += 1;
  historyGeneration += 1;
  scopeCache.clear();
  scopeRequests.clear();
  balanceCache.clear();
  balanceRequests.clear();
  historyRequests.clear();
  dlightStatusCache = null;
  dlightStatusRequest = null;
  resetTransactionHistoryPages();
}

export function invalidateWalletDisplayScopes(): void {
  scopeGeneration += 1;
  scopeCache.clear();
  scopeRequests.clear();
  dlightStatusCache = null;
  dlightStatusRequest = null;
}

export function invalidateWalletDisplayHistory(channelId?: string, coinId?: string): void {
  historyGeneration += 1;
  historyRequests.clear();
  resetTransactionHistoryPages(channelId, coinId);
}

export function primeDisplayBalance(channelId: string, coinId: string, value: BalanceResult): void {
  balanceCache.set(balanceKey(channelId, coinId), {
    value,
    expiresAt: Date.now() + DISPLAY_BALANCE_TTL_MS,
  });
}

export async function getDisplayCoinScopes(coinId: string): Promise<CoinScopesResult> {
  const cached = scopeCache.get(coinId);
  if (cached) return cached;

  const existing = scopeRequests.get(coinId);
  if (existing) return existing;

  const requestSessionGeneration = sessionGeneration;
  const requestScopeGeneration = scopeGeneration;
  const request = invokeWalletCommand<CoinScopesResult>('get_coin_scopes', {
    coin_id: coinId,
  }).then((result) => {
    if (
      requestSessionGeneration !== sessionGeneration ||
      requestScopeGeneration !== scopeGeneration
    ) {
      throw new WalletDisplayRequestInvalidatedError();
    }
    scopeCache.set(coinId, result);
    return result;
  });

  scopeRequests.set(coinId, request);
  const clearRequest = () => {
    if (scopeRequests.get(coinId) === request) {
      scopeRequests.delete(coinId);
    }
  };
  void request.then(clearRequest, clearRequest);
  return request;
}

export async function getDisplayDlightSeedStatus(): Promise<DlightSeedStatusResult> {
  if (dlightStatusCache) return dlightStatusCache;
  if (dlightStatusRequest) return dlightStatusRequest;

  const requestSessionGeneration = sessionGeneration;
  const requestScopeGeneration = scopeGeneration;
  const request = invokeWalletCommand<DlightSeedStatusResult>('get_dlight_seed_status').then(
    (result) => {
      if (
        requestSessionGeneration !== sessionGeneration ||
        requestScopeGeneration !== scopeGeneration
      ) {
        throw new WalletDisplayRequestInvalidatedError();
      }
      dlightStatusCache = result;
      return result;
    }
  );
  dlightStatusRequest = request;
  const clearRequest = () => {
    if (dlightStatusRequest === request) {
      dlightStatusRequest = null;
    }
  };
  void request.then(clearRequest, clearRequest);
  return request;
}

export async function getDisplayBalance(
  channelId: string,
  coinId?: string
): Promise<BalanceResult> {
  const key = balanceKey(channelId, coinId);
  const cached = balanceCache.get(key);
  if (cached && cached.expiresAt > Date.now()) return cached.value;

  const existing = balanceRequests.get(key);
  if (existing) return existing;

  const requestGeneration = sessionGeneration;
  const request = invokeWalletCommand<BalanceResult>('get_balances', {
    channel_id: channelId,
    ...(coinId ? { coin_id: coinId } : {}),
  }).then((result) => {
    if (requestGeneration !== sessionGeneration) {
      throw new WalletDisplayRequestInvalidatedError();
    }
    primeDisplayBalance(channelId, coinId ?? '', result);
    return result;
  });

  balanceRequests.set(key, request);
  const clearRequest = () => {
    if (balanceRequests.get(key) === request) {
      balanceRequests.delete(key);
    }
  };
  void request.then(clearRequest, clearRequest);
  return request;
}

export async function getDisplayTransactionHistoryPage(
  channelId: string,
  coinId: string | undefined,
  cursor: string | undefined,
  limit: number
): Promise<TransactionHistoryPage> {
  const key = historyKey(channelId, coinId, cursor, limit);
  const existing = historyRequests.get(key);
  if (existing) return existing;

  const requestSessionGeneration = sessionGeneration;
  const requestHistoryGeneration = historyGeneration;
  const request = invokeWalletCommand<TransactionHistoryPage>('get_transaction_history_page', {
    request: {
      channelId,
      ...(coinId ? { coinId } : {}),
      ...(cursor ? { cursor } : {}),
      limit,
    },
  }).then((result) => {
    if (
      requestSessionGeneration !== sessionGeneration ||
      requestHistoryGeneration !== historyGeneration
    ) {
      throw new WalletDisplayRequestInvalidatedError();
    }
    return result;
  });

  historyRequests.set(key, request);
  const clearRequest = () => {
    if (historyRequests.get(key) === request) {
      historyRequests.delete(key);
    }
  };
  void request.then(clearRequest, clearRequest);
  return request;
}
