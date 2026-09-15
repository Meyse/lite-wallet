import { describe, expect, it } from 'vitest';
import type { ResolvedPreflightRequest } from './preflightRequest';
import {
  preflightRequestSignature,
  runGuardedPreflight,
  transferWalletSessionKey,
} from './preflightRequest';
import { PreflightRequestGuard } from './preflightRequestGuard';

const bridgeRequest: ResolvedPreflightRequest = {
  kind: 'bridge',
  params: {
    coinId: 'VRSC',
    channelId: 'vrpc.Rsource.iVerus',
    sourceAddress: 'Rsource',
    destination: 'Rdestination',
    amount: '1',
    convertTo: 'iConverted',
    exportTo: 'iDestinationSystem',
    via: 'iBasket',
    mapTo: '0xToken',
    preconvert: null,
  },
};

describe('preflightRequestSignature', () => {
  it.each([
    ['sourceAddress', { sourceAddress: 'Rother' }],
    ['destination', { destination: 'Rother' }],
    ['amount', { amount: '2' }],
    ['convertTo', { convertTo: 'iOther' }],
    ['exportTo', { exportTo: 'iOtherSystem' }],
    ['via', { via: 'iOtherBasket' }],
    ['mapTo', { mapTo: '0xOtherToken' }],
  ] as const)('changes when resolved bridge field %s changes', (_name, change) => {
    const changed: ResolvedPreflightRequest = {
      kind: 'bridge',
      params: { ...bridgeRequest.params, ...change },
    };
    expect(preflightRequestSignature(changed)).not.toBe(preflightRequestSignature(bridgeRequest));
  });

  it('changes with direct fee mode and channel source', () => {
    const standard: ResolvedPreflightRequest = {
      kind: 'direct',
      params: {
        coinId: 'BTC',
        channelId: 'btc.BTC',
        toAddress: 'bc1qrecipient',
        amount: '0.1',
        memo: null,
        feeMode: 'standard',
      },
    };
    const economy: ResolvedPreflightRequest = {
      kind: 'direct',
      params: { ...standard.params, feeMode: 'economy' },
    };
    const otherSource: ResolvedPreflightRequest = {
      kind: 'direct',
      params: { ...standard.params, channelId: 'btc.BTC-other' },
    };

    expect(preflightRequestSignature(economy)).not.toBe(preflightRequestSignature(standard));
    expect(preflightRequestSignature(otherSource)).not.toBe(preflightRequestSignature(standard));
  });

  it('changes across wallets and networks even when the visible request is identical', () => {
    expect(
      preflightRequestSignature(bridgeRequest, {
        walletKey: 'wallet-a::mainnet',
        walletNetwork: 'mainnet',
      })
    ).not.toBe(
      preflightRequestSignature(bridgeRequest, {
        walletKey: 'wallet-b::mainnet',
        walletNetwork: 'mainnet',
      })
    );
    expect(
      preflightRequestSignature(bridgeRequest, {
        walletKey: 'wallet-a::mainnet',
        walletNetwork: 'mainnet',
      })
    ).not.toBe(
      preflightRequestSignature(bridgeRequest, {
        walletKey: 'wallet-a::testnet',
        walletNetwork: 'testnet',
      })
    );
  });

  it('changes for a replacement session of the same wallet and network', () => {
    expect(transferWalletSessionKey('Wallet A', 'mainnet', 'session-1')).not.toBe(
      transferWalletSessionKey('Wallet A', 'mainnet', 'session-2')
    );
  });
});

describe('runGuardedPreflight', () => {
  it('discards an old quote after a fee change and accepts the fresh quote', async () => {
    const guard = new PreflightRequestGuard();
    let currentSignature = 'btc|standard';
    let resolveOld: (value: string) => void = () => {};
    const oldQuote = runGuardedPreflight({
      guard,
      signature: currentSignature,
      currentSignature: () => currentSignature,
      execute: () =>
        new Promise<string>((resolve) => {
          resolveOld = resolve;
        }),
    });

    currentSignature = 'btc|economy';
    guard.invalidate();
    const freshQuote = runGuardedPreflight({
      guard,
      signature: currentSignature,
      currentSignature: () => currentSignature,
      execute: async () => 'economy quote',
    });
    resolveOld('standard quote');

    await expect(freshQuote).resolves.toEqual({ status: 'applied', value: 'economy quote' });
    await expect(oldQuote).resolves.toEqual({ status: 'stale' });
  });

  it('returns a current quote failure without an applied value', async () => {
    const guard = new PreflightRequestGuard();
    const outcome = await runGuardedPreflight({
      guard,
      signature: 'eth|economy',
      currentSignature: () => 'eth|economy',
      execute: async () => {
        throw new Error('quote unavailable');
      },
    });

    expect(outcome.status).toBe('failed');
    expect('value' in outcome).toBe(false);
  });

  it('discards an in-flight quote when its component session is disposed', async () => {
    const guard = new PreflightRequestGuard();
    let resolveQuote: (value: string) => void = () => {};
    const quote = runGuardedPreflight({
      guard,
      signature: 'erc20|standard',
      currentSignature: () => 'erc20|standard',
      execute: () =>
        new Promise<string>((resolve) => {
          resolveQuote = resolve;
        }),
    });
    guard.dispose();
    resolveQuote('late quote');

    await expect(quote).resolves.toEqual({ status: 'stale' });
  });
});
