import { describe, expect, it } from 'vitest';
import {
  DEFAULT_DIRECT_SEND_FEE_MODE,
  directSendFeeDraftForOpen,
  directSendFeeSelectionNeedsRefresh,
  type DirectSendRouteState,
  feeModeForPreflight,
  isDirectSendFeeEligible,
  withDirectSendFeeMode,
} from './directSendFee.js';

const directRoute = (channelId: string): DirectSendRouteState => ({
  operation: 'preflight_send',
  channelId,
  conversionEnabled: false,
  exportSystemId: null,
});

describe('direct send fee eligibility', () => {
  it('defaults every new wizard session to standard', () => {
    expect(DEFAULT_DIRECT_SEND_FEE_MODE).toBe('standard');
  });

  it.each(['btc.BTC', 'eth.ETH', 'erc20.USDC'])('allows eligible direct route %s', (channelId) => {
    expect(isDirectSendFeeEligible(directRoute(channelId))).toBe(true);
    expect(feeModeForPreflight(directRoute(channelId), 'economy')).toBe('economy');
  });

  it('excludes bridge, conversion, VRPC, and dLight routes', () => {
    const excluded: DirectSendRouteState[] = [
      { ...directRoute('erc20.USDC'), operation: 'bridge_transfer' },
      { ...directRoute('btc.BTC'), conversionEnabled: true },
      { ...directRoute('eth.ETH'), exportSystemId: 'bridge.vETH' },
      directRoute('vrpc.RAddress.system'),
      directRoute('dlight_private.zsAddress.system'),
    ];
    expect(excluded.every((route) => !isDirectSendFeeEligible(route))).toBe(true);
    expect(excluded.every((route) => feeModeForPreflight(route, 'economy') === undefined)).toBe(
      true
    );
  });

  it('maps a mode only into eligible ordinary preflight params', () => {
    const params = {
      coinId: 'BTC',
      channelId: 'btc.BTC',
      toAddress: 'destination',
      amount: '0.1',
      memo: null,
    };
    expect(withDirectSendFeeMode(params, directRoute('btc.BTC'), 'economy')).toEqual({
      ...params,
      feeMode: 'economy',
    });
    expect(withDirectSendFeeMode(params, directRoute('vrpc.address.system'), 'economy')).toEqual(
      params
    );
  });

  it('opens from the backend-authoritative mode and keeps cancellation draft-only', () => {
    const appliedMode = directSendFeeDraftForOpen('standard', 'economy');
    let pendingMode: 'economy' | 'standard' = appliedMode;

    pendingMode = 'economy';
    expect(directSendFeeSelectionNeedsRefresh(appliedMode, pendingMode)).toBe(true);

    // Closing the sheet discards only the draft. Opening again starts from the
    // mode attached to the active backend quote, not the last local selection.
    pendingMode = directSendFeeDraftForOpen(appliedMode, pendingMode);
    expect(pendingMode).toBe('standard');
    expect(directSendFeeSelectionNeedsRefresh(appliedMode, pendingMode)).toBe(false);
  });

  it('uses the selected mode only before a backend quote exists', () => {
    expect(directSendFeeDraftForOpen(null, 'economy')).toBe('economy');
    expect(directSendFeeSelectionNeedsRefresh(null, 'economy')).toBe(false);
  });
});
