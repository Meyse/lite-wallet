import { describe, expect, it } from 'vitest';
import { loadTransferScopes } from './scopeLoader';

describe('transfer scope loading', () => {
  it('keeps failures distinct from authoritative empty results and supports retry', async () => {
    let attempts = 0;
    const load = async (coinId: string) => {
      attempts += 1;
      if (attempts === 1) throw new Error('temporary failure');
      return { coinId, scopes: [] };
    };

    const failed = await loadTransferScopes(['USDC'], load, () => false);
    expect(failed.failures).toEqual({ USDC: true });
    expect(failed.loaded).toEqual({});

    const retried = await loadTransferScopes(['USDC'], load, () => false);
    expect(retried.failures).toEqual({});
    expect(retried.loaded).toEqual({ USDC: true });
    expect(retried.scopes).toEqual({ USDC: [] });
  });
});
