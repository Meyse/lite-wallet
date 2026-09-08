/**
 * Thin invoke wrappers for bridge transfer commands.
 * Security: No tx hex or signing payloads leave backend trust boundary.
 */

import type {
  BridgeCapabilitiesRequest,
  BridgeCapabilitiesResult,
  BridgeConversionEstimateRequest,
  BridgeConversionEstimateResult,
  BridgeConversionPathRequest,
  BridgeConversionPathsResult,
  BridgeExportFeeEstimateRequest,
  BridgeExportFeeEstimateResult,
  BridgeTransferPreflightParams,
  BridgeTransferPreflightResult
} from '$lib/types/wallet.js';
import { invokeWalletCommand } from './invokeWalletCommand.js';

export async function getBridgeCapabilities(
  request: BridgeCapabilitiesRequest
): Promise<BridgeCapabilitiesResult> {
  return invokeWalletCommand<BridgeCapabilitiesResult>('get_bridge_capabilities', {
    request: {
      coinId: request.coinId,
      channelId: request.channelId
    }
  });
}

export async function getBridgeConversionPaths(
  request: BridgeConversionPathRequest
): Promise<BridgeConversionPathsResult> {
  return invokeWalletCommand<BridgeConversionPathsResult>('get_bridge_conversion_paths', {
    request: {
      coinId: request.coinId,
      channelId: request.channelId,
      sourceCurrency: request.sourceCurrency,
      destinationCurrency: request.destinationCurrency ?? null
    }
  });
}

export async function estimateBridgeConversion(
  request: BridgeConversionEstimateRequest
): Promise<BridgeConversionEstimateResult> {
  return invokeWalletCommand<BridgeConversionEstimateResult>('estimate_bridge_conversion', {
    request: {
      coinId: request.coinId,
      channelId: request.channelId,
      sourceCurrency: request.sourceCurrency,
      convertTo: request.convertTo,
      amount: request.amount,
      via: request.via ?? null,
      preconvert: request.preconvert ?? null
    }
  });
}

export async function estimateBridgeExportFee(
  request: BridgeExportFeeEstimateRequest
): Promise<BridgeExportFeeEstimateResult> {
  return invokeWalletCommand<BridgeExportFeeEstimateResult>('estimate_bridge_export_fee', {
    request: {
      coinId: request.coinId,
      channelId: request.channelId
    }
  });
}

export async function preflightBridgeTransfer(
  params: BridgeTransferPreflightParams
): Promise<BridgeTransferPreflightResult> {
  return invokeWalletCommand<BridgeTransferPreflightResult>('preflight_bridge_transfer', {
    params: {
      coinId: params.coinId,
      channelId: params.channelId,
      sourceAddress: params.sourceAddress ?? null,
      destination: params.destination,
      amount: params.amount,
      convertTo: params.convertTo ?? null,
      exportTo: params.exportTo ?? null,
      via: params.via ?? null,
      feeCurrency: params.feeCurrency ?? null,
      feeSatoshis: params.feeSatoshis ?? null,
      preconvert: params.preconvert ?? null,
      mapTo: params.mapTo ?? null,
      vdxfTag: params.vdxfTag ?? null,
      memo: params.memo ?? null
    }
  });
}
