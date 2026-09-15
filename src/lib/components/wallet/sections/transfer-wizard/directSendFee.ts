import type { DirectSendFeeMode, PreflightParams } from '$lib/types/wallet.js';

export { type DirectSendFeeMode } from '$lib/types/wallet.js';

export const DEFAULT_DIRECT_SEND_FEE_MODE: DirectSendFeeMode = 'standard';

export function directSendFeeDraftForOpen(
  authoritativeMode: DirectSendFeeMode | null,
  selectedMode: DirectSendFeeMode
): DirectSendFeeMode {
  return authoritativeMode ?? selectedMode;
}

export function directSendFeeSelectionNeedsRefresh(
  authoritativeMode: DirectSendFeeMode | null,
  pendingMode: DirectSendFeeMode
): boolean {
  return authoritativeMode !== null && pendingMode !== authoritativeMode;
}

export type DirectSendRouteState = {
  operation: 'preflight_send' | 'bridge_transfer' | 'vrpc_transfer';
  channelId: string | null;
  conversionEnabled: boolean;
  exportSystemId: string | null;
};

export function isDirectSendFeeEligible(route: DirectSendRouteState): boolean {
  if (route.operation !== 'preflight_send' || route.conversionEnabled || route.exportSystemId) {
    return false;
  }
  const prefix = route.channelId?.trim().split('.')[0]?.toLowerCase() ?? '';
  return prefix === 'btc' || prefix === 'eth' || prefix === 'erc20';
}

export function feeModeForPreflight(
  route: DirectSendRouteState,
  selectedMode: DirectSendFeeMode
): DirectSendFeeMode | undefined {
  return isDirectSendFeeEligible(route) ? selectedMode : undefined;
}

export function withDirectSendFeeMode(
  params: Omit<PreflightParams, 'feeMode'>,
  route: DirectSendRouteState,
  selectedMode: DirectSendFeeMode
): PreflightParams {
  const feeMode = feeModeForPreflight(route, selectedMode);
  return feeMode ? { ...params, feeMode } : params;
}

export type DirectSendFeeOption = {
  value: DirectSendFeeMode;
  label: string;
  description: string;
};

export function getDirectSendFeeOptions(t: (key: string) => string): DirectSendFeeOption[] {
  return [
    {
      value: 'economy',
      label: t('wallet.transfer.fee.economy'),
      description: t('wallet.transfer.fee.lowerFee'),
    },
    {
      value: 'standard',
      label: t('wallet.transfer.fee.standard'),
      description: t('wallet.transfer.fee.recommended'),
    },
  ];
}
