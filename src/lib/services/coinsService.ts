/**
 * Thin invoke wrappers for coin registry Tauri commands.
 */

import { invoke } from '@tauri-apps/api/core';
import type { CoinDefinition, Erc20ResolveResult, PbaasResolveResult } from '$lib/types/wallet.js';
import { invokeWalletCommand } from './invokeWalletCommand.js';

export async function getCoinRegistry(): Promise<CoinDefinition[]> {
  return invoke<CoinDefinition[]>('get_coin_registry');
}

export async function addPbaasCurrency(definition: CoinDefinition): Promise<void> {
  await invokeWalletCommand('add_pbaas_currency', { definition });
}

export async function addCoinDefinition(definition: CoinDefinition): Promise<CoinDefinition> {
  return invokeWalletCommand<CoinDefinition>('add_coin_definition', { definition });
}

export async function resolvePbaasCurrency(query: string): Promise<PbaasResolveResult> {
  return invokeWalletCommand<PbaasResolveResult>('resolve_pbaas_currency', { query });
}

export async function resolveErc20Contract(contract: string): Promise<Erc20ResolveResult> {
  return invokeWalletCommand<Erc20ResolveResult>('resolve_erc20_contract', { contract });
}
