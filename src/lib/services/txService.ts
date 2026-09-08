/**
 * Thin invoke wrappers for transaction Tauri commands (preflight, send).
 * Security: No tx hex or signing data; send by preflight_id only.
 */

import type { PreflightParams, PreflightResult, SendRequest, SendResult } from '$lib/types/wallet.js';
import { invokeWalletCommand } from './invokeWalletCommand.js';

export async function preflightSend(params: PreflightParams): Promise<PreflightResult> {
  return invokeWalletCommand<PreflightResult>('preflight_send', {
    params: {
      coinId: params.coinId,
      channelId: params.channelId,
      toAddress: params.toAddress,
      amount: params.amount,
      memo: params.memo ?? null
    }
  });
}

export async function sendTransaction(request: SendRequest): Promise<SendResult> {
  return invokeWalletCommand<SendResult>('send_transaction', {
    request: { preflightId: request.preflightId }
  });
}
