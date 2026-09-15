import { describe, expect, it } from 'vitest';
import { extractWalletErrorMessage, extractWalletErrorType } from './walletErrors';

describe('wallet error extraction', () => {
  it('keeps structured wallet errors', () => {
    expect(extractWalletErrorType({ type: 'EthNotConfigured' })).toBe('EthNotConfigured');
  });

  it.each([
    'Command get_pending_eth_submission not allowed by ACL',
    'command not found: resume_pending_eth_submission',
  ])(
    'classifies native command contract failures without exposing them as backend copy',
    (error) => {
      expect(extractWalletErrorType(error)).toBe('NativeCommandUnavailable');
    }
  );

  it('retains plain safe messages for callers that deliberately use them', () => {
    expect(extractWalletErrorMessage('A safe message')).toBe('A safe message');
  });
});
