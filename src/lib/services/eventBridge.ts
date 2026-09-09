/**
 * Tauri event bridge: listen for wallet://* events and update Svelte stores.
 * Module 7 Update Engine emits balances-updated and transactions-updated; bridge updates stores.
 * Call setupWalletEventBridge() when wallet layout mounts; call returned cleanup on destroy.
 */

import { listen } from '@tauri-apps/api/event';
import { get } from 'svelte/store';
import { balanceStore } from '$lib/stores/balances.js';
import { transactionStore } from '$lib/stores/transactions.js';
import { networkStore } from '$lib/stores/network.js';
import { ratesStore } from '$lib/stores/rates.js';
import { walletBootstrapStore } from '$lib/stores/walletBootstrap.js';
import { walletChannelsStore } from '$lib/stores/walletChannels.js';
import {
  clearMatchingWalletBackgroundError,
  pushWalletBackgroundError,
} from '$lib/stores/walletErrors.js';
import type { BalanceResult, ChainInfo, Transaction } from '$lib/types/wallet.js';
import { canonicalizeVrpcChannelId } from '$lib/utils/channelId.js';
import { invalidateWalletDisplayHistory, primeDisplayBalance } from './walletDisplayService.js';

const BALANCES_UPDATED = 'wallet://balances-updated';
const TRANSACTIONS_UPDATED = 'wallet://transactions-updated';
const INFO_UPDATED = 'wallet://info-updated';
const RATES_UPDATED = 'wallet://rates-updated';
const BOOTSTRAP_UPDATED = 'wallet://bootstrap-updated';
const SESSION_EXPIRED = 'wallet://session-expired';
const ERROR = 'wallet://error';
const DEFAULT_COIN_KEY = '__default__';

interface BalancesUpdatedPayload {
  coinId?: string;
  channel?: string;
  channelId?: string;
  confirmed?: string;
  pending?: string;
  total?: string;
}

interface TransactionsUpdatedPayload {
  coinId?: string;
  channel?: string;
  channelId?: string;
  transactions?: Transaction[];
}

interface InfoUpdatedPayload {
  coinId?: string;
  channel?: string;
  channelId?: string;
  percent?: number;
  blocks?: number;
  longestChain?: number;
  syncing?: boolean;
  statusKind?: string;
  lastUpdated?: number;
  lastProgressAt?: number;
  stalled?: boolean;
  scanRateBlocksPerSec?: number;
}

interface RatesUpdatedPayload {
  coinId?: string;
  rates?: Record<string, number>;
  usdChange24hPct?: number | null;
}

interface BootstrapUpdatedPayload {
  inProgress?: boolean;
}

interface UpdateErrorPayload {
  dataType?: string;
  coinId?: string;
  channel?: string;
  message?: string;
}

interface SetupWalletEventBridgeOptions {
  onSessionExpired?: () => void | Promise<void>;
}

function shouldSuppressWalletError(payload: UpdateErrorPayload): boolean {
  const channel = (payload.channel ?? '').toLowerCase();
  if (!channel.startsWith('dlight_private.')) return false;

  const message = (payload.message ?? '').toLowerCase();
  return (
    message.includes('dlight synchronizer not ready') ||
    message.includes('network error') ||
    message.includes('temporarily unavailable')
  );
}

function normalizeChannelKey(key: string): string {
  if (!key.startsWith('vrpc.')) return key;
  const { vrpcAddress } = get(walletChannelsStore);
  return canonicalizeVrpcChannelId(key, vrpcAddress ?? undefined);
}

/**
 * Builds store key from payload. Backend sends channel = channel_id (e.g. vrpc.VRSC.i5w5... or btc.BTC).
 */
function balanceKey(p: BalancesUpdatedPayload): string {
  if (p.channel) return p.channel;
  if (p.channelId) return p.channelId;
  return p.coinId ?? 'unknown';
}

function txKey(p: TransactionsUpdatedPayload): string {
  if (p.channel) return p.channel;
  if (p.channelId) return p.channelId;
  return p.coinId ?? 'unknown';
}

function infoKey(p: InfoUpdatedPayload): string {
  if (p.channel) return p.channel;
  if (p.channelId) return p.channelId;
  return p.coinId ?? p.channel ?? 'unknown';
}

/**
 * Registers all wallet event listeners and returns a cleanup function.
 */
export async function setupWalletEventBridge(
  options: SetupWalletEventBridgeOptions = {}
): Promise<() => void> {
  const unsubs: (() => void)[] = [];
  let active = true;

  const cleanup = () => {
    if (!active) return;
    active = false;
    for (const unsubscribe of unsubs.splice(0).reverse()) {
      unsubscribe();
    }
  };

  const register = async <T>(
    eventName: string,
    handler: Parameters<typeof listen<T>>[1]
  ): Promise<void> => {
    const unsubscribe = await listen<T>(eventName, (event) => {
      if (active) handler(event);
    });
    unsubs.push(unsubscribe);
  };

  try {
    await register<BalancesUpdatedPayload>(BALANCES_UPDATED, (event) => {
      const p = event.payload;
      const key = normalizeChannelKey(balanceKey(p));
      const coinId = p.coinId ?? DEFAULT_COIN_KEY;
      if (
        typeof p.confirmed !== 'string' ||
        typeof p.pending !== 'string' ||
        typeof p.total !== 'string'
      ) {
        return;
      }
      const value: BalanceResult = {
        confirmed: p.confirmed,
        pending: p.pending,
        total: p.total,
      };
      clearMatchingWalletBackgroundError({
        dataTypes: ['balance'],
        coinId,
        channel: key,
      });
      primeDisplayBalance(key, coinId, value);
      balanceStore.update((m) => ({
        ...m,
        [key]: {
          ...(m[key] ?? {}),
          [coinId]: value,
        },
      }));
    });

    await register<TransactionsUpdatedPayload>(TRANSACTIONS_UPDATED, (event) => {
      const p = event.payload;
      const key = normalizeChannelKey(txKey(p));
      const coinId = p.coinId ?? DEFAULT_COIN_KEY;
      const list = p.transactions ?? [];
      clearMatchingWalletBackgroundError({
        dataTypes: ['transactions', 'transactions_warning'],
        coinId,
        channel: key,
      });
      transactionStore.update((m) => ({
        ...m,
        [key]: {
          ...(m[key] ?? {}),
          [coinId]: list,
        },
      }));
      invalidateWalletDisplayHistory(key, coinId);
    });

    await register<InfoUpdatedPayload>(INFO_UPDATED, (event) => {
      const p = event.payload;
      const key = normalizeChannelKey(infoKey(p));
      const value: ChainInfo = {
        channel: p.channel ?? p.channelId,
        percent: p.percent,
        blocks: p.blocks,
        longestChain: p.longestChain,
        syncing: p.syncing,
        statusKind: p.statusKind,
        lastUpdated: p.lastUpdated,
        lastProgressAt: p.lastProgressAt,
        stalled: p.stalled,
        scanRateBlocksPerSec: p.scanRateBlocksPerSec,
      };
      clearMatchingWalletBackgroundError({
        dataTypes: ['info'],
        coinId: p.coinId ?? DEFAULT_COIN_KEY,
        channel: key,
      });
      networkStore.update((m) => ({ ...m, [key]: value }));
    });

    await register<RatesUpdatedPayload>(RATES_UPDATED, (event) => {
      const p = event.payload;
      const coinId = p.coinId ?? 'default';
      const rates = p.rates ?? {};
      ratesStore.update((m) => ({
        ...m,
        [coinId]: {
          rates,
          usdChange24hPct: typeof p.usdChange24hPct === 'number' ? p.usdChange24hPct : null,
        },
      }));
    });

    await register<BootstrapUpdatedPayload>(BOOTSTRAP_UPDATED, (event) => {
      const inProgress = event.payload?.inProgress;
      walletBootstrapStore.set(typeof inProgress === 'boolean' ? inProgress : false);
    });

    await register<UpdateErrorPayload>(ERROR, (event) => {
      const p = event.payload;
      if (shouldSuppressWalletError(p)) return;

      const channel = p.channel ? normalizeChannelKey(p.channel) : '';
      const type = p.dataType ?? 'wallet';
      const message = p.message ?? 'Temporarily unavailable';
      const prefix = channel ? `${type} (${channel})` : type;
      pushWalletBackgroundError(`${prefix}: ${message}`, {
        dataType: type,
        coinId: p.coinId,
        channel: channel || undefined,
      });
    });

    await register(SESSION_EXPIRED, () => {
      void options.onSessionExpired?.();
    });
  } catch (error) {
    cleanup();
    throw error;
  }

  return cleanup;
}
