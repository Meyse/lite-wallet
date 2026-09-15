import { describe, expect, it } from 'vitest';
import type { CoinDefinition, CoinScope } from '$lib/types/wallet.js';
import { buildSpendableTransferSources, transferSourceSupportsConversion } from './transferSources';

const vrsc: CoinDefinition = {
  id: 'VRSC',
  currencyId: 'iVRSC',
  systemId: 'iVerus',
  displayTicker: 'VRSC',
  displayName: 'Verus',
  proto: 'vrsc',
  compatibleChannels: ['vrpc', 'dlight_private'],
  decimals: 8,
  vrpcEndpoints: [],
  secondsPerBlock: 60,
  isTestnet: false,
};

function scope(overrides: Partial<CoinScope>): CoinScope {
  return {
    channelId: 'vrpc.Rprimary.iVerus',
    coinId: 'VRSC',
    address: 'Rprimary',
    addressLabel: 'Primary address',
    systemId: 'iVerus',
    systemTicker: 'VRSC',
    systemDisplayName: 'Verus',
    isPrimaryAddress: true,
    isReadOnly: false,
    scopeKind: 'transparent',
    ...overrides,
  };
}

describe('buildSpendableTransferSources', () => {
  it('uses supported scope identity and excludes linked, watched, and unknown balances', () => {
    const primary = scope({});
    const linkedIdentity = scope({
      channelId: 'vrpc.iLinkedIdentity.iVerus',
      address: 'iLinkedIdentity',
      addressLabel: 'Linked identity',
      isPrimaryAddress: false,
    });
    const privateScope = scope({
      channelId: 'dlight_private.zs-owned.iVerus',
      address: 'zs-owned',
      addressLabel: 'Private address',
      isPrimaryAddress: false,
      scopeKind: 'shielded',
    });
    const watched = scope({
      channelId: 'vrpc.Rwatched.iVerus',
      address: 'Rwatched',
      isPrimaryAddress: false,
      isReadOnly: true,
    });

    const options = buildSpendableTransferSources(
      [vrsc],
      { VRSC: [primary, linkedIdentity, privateScope, watched] },
      {
        [primary.channelId]: { VRSC: { confirmed: '1', pending: '0', total: '1' } },
        [linkedIdentity.channelId]: { VRSC: { confirmed: '2', pending: '0', total: '2' } },
        [privateScope.channelId]: { VRSC: { confirmed: '3', pending: '0', total: '3' } },
        [watched.channelId]: { VRSC: { confirmed: '4', pending: '0', total: '4' } },
        'vrpc.Runknown.iVerus': { VRSC: { confirmed: '5', pending: '0', total: '5' } },
      }
    );

    expect(options.map((option) => option.scope.address)).toEqual(['Rprimary', 'zs-owned']);
    expect(options.map((option) => option.scope.systemId)).toEqual(['iVerus', 'iVerus']);
    expect(options.map((option) => option.sourceKind)).toEqual(['public', 'private']);
    expect(transferSourceSupportsConversion(options[0])).toBe(true);
    expect(transferSourceSupportsConversion(options[1])).toBe(false);
  });

  it('preserves the primary VRPC address across supported systems', () => {
    const verus = scope({});
    const chips = scope({
      channelId: 'vrpc.Rprimary.iChips',
      systemId: 'iChips',
      systemTicker: 'CHIPS',
      systemDisplayName: 'CHIPS',
    });

    const options = buildSpendableTransferSources(
      [vrsc],
      { VRSC: [verus, chips] },
      {
        [verus.channelId]: { VRSC: { confirmed: '1', pending: '0', total: '1' } },
        [chips.channelId]: { VRSC: { confirmed: '2', pending: '0', total: '2' } },
      }
    );

    expect(options.map((option) => option.scope.address)).toEqual(['Rprimary', 'Rprimary']);
    expect(options.map((option) => option.scope.systemId)).toEqual(['iVerus', 'iChips']);
  });

  it('rejects malformed or incompatible scopes even when a cached balance exists', () => {
    const malformed = scope({ channelId: 'vrpc.Rbad.iVerus', address: '' });
    const incompatible = scope({ channelId: 'eth.VRSC', address: '0xabc' });
    const options = buildSpendableTransferSources(
      [vrsc],
      { VRSC: [malformed, incompatible] },
      {
        [malformed.channelId]: { VRSC: { confirmed: '1', pending: '0', total: '1' } },
        [incompatible.channelId]: { VRSC: { confirmed: '1', pending: '0', total: '1' } },
      }
    );

    expect(options).toEqual([]);
  });

  it('rejects a primary VRPC scope whose channel identity does not match its metadata', () => {
    const mismatched = scope({ channelId: 'vrpc.Rother.iVerus' });
    const options = buildSpendableTransferSources(
      [vrsc],
      { VRSC: [mismatched] },
      {
        [mismatched.channelId]: { VRSC: { confirmed: '1', pending: '0', total: '1' } },
      }
    );

    expect(options).toEqual([]);
  });
});
