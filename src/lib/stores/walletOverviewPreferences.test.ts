// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  readWalletOverviewPreferences as read,
  writeWalletOverviewPreferences as write,
} from './walletOverviewPreferences.js';

describe('wallet overview preferences', () => {
  beforeEach(() => localStorage.clear());
  afterEach(() => vi.restoreAllMocks());

  it('persists only sort/filter and isolates wallets and networks across reads', () => {
    write('Main wallet', 'mainnet', { sort: 'amount', reversed: false, withBalance: true });
    write('Other', 'mainnet', { sort: 'name', reversed: false, withBalance: false });
    expect(read(' Main Wallet ', 'mainnet')).toEqual({
      sort: 'amount',
      reversed: false,
      withBalance: true,
    });
    expect(read('Main wallet', 'testnet')).toEqual({
      sort: 'verus-first',
      reversed: false,
      withBalance: false,
    });
    expect(read('Other', 'mainnet')).toEqual({ sort: 'name', reversed: false, withBalance: false });
    expect(JSON.parse(localStorage.getItem(localStorage.key(0) ?? '') ?? '{}')).toEqual({
      sort: 'amount',
      reversed: false,
      withBalance: true,
    });
  });

  it('persists reverse direction and defaults older preferences to the original direction', () => {
    write('wallet', 'mainnet', { sort: 'name', withBalance: true, reversed: true });
    expect(read('wallet', 'mainnet')).toEqual({ sort: 'name', withBalance: true, reversed: true });
    const key = localStorage.key(0) ?? '';
    localStorage.setItem(key, JSON.stringify({ sort: 'value', withBalance: true }));
    expect(read('wallet', 'mainnet').reversed).toBe(false);
    localStorage.setItem(key, JSON.stringify({ sort: 'verus-first', reversed: true }));
    expect(read('wallet', 'mainnet').reversed).toBe(false);
  });

  it('validates stored preferences and tolerates corrupt or blocked storage', () => {
    write('wallet', 'mainnet', { sort: 'value', reversed: false, withBalance: true });
    const key = localStorage.key(0) ?? '';
    localStorage.setItem(
      key,
      JSON.stringify({ sort: 'surprise', withBalance: 'true', query: 'private' })
    );
    expect(read('wallet', 'mainnet')).toEqual({
      sort: 'verus-first',
      reversed: false,
      withBalance: false,
    });
    localStorage.setItem(key, '{broken');
    expect(read('wallet', 'mainnet')).toEqual({
      sort: 'verus-first',
      reversed: false,
      withBalance: false,
    });
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked');
    });
    expect(() =>
      write('wallet', 'mainnet', { sort: 'value', reversed: false, withBalance: true })
    ).not.toThrow();
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked');
    });
    expect(read('wallet', 'mainnet')).toEqual({
      sort: 'verus-first',
      reversed: false,
      withBalance: false,
    });
  });
});
