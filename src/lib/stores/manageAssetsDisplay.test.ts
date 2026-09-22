import { beforeEach, describe, expect, it } from 'vitest';
import {
  invalidateWalletDisplayScopes,
  resetWalletDisplaySession,
} from '$lib/services/walletDisplayService.js';
import { readManageAssetsDisplay, writeManageAssetsDisplay } from './manageAssetsDisplay.js';
import type { KnownAssetBalance } from './manageAssets.js';

const value = { discovery: null, knownBalances: [], discoveryStale: true };

describe('Manage assets display reuse', () => {
  beforeEach(resetWalletDisplaySession);

  it('reuses only the current session and network', () => {
    const current = readManageAssetsDisplay('first', 'mainnet');
    writeManageAssetsDisplay('first', 'mainnet', current.generation, value);
    expect(readManageAssetsDisplay('first', 'mainnet').snapshot).toEqual(value);
    expect(readManageAssetsDisplay('first', 'testnet').snapshot).toBeNull();
    expect(readManageAssetsDisplay('second', 'mainnet').snapshot).toBeNull();
    writeManageAssetsDisplay('first', 'mainnet', current.generation, value);
    expect(readManageAssetsDisplay('second', 'mainnet').snapshot).toBeNull();
  });

  it('isolates the retained snapshot from both the writer and the reader', () => {
    const current = readManageAssetsDisplay('first', 'mainnet');
    const input = {
      discovery: null,
      knownBalances: [] as KnownAssetBalance[],
      discoveryStale: false,
    };
    writeManageAssetsDisplay('first', 'mainnet', current.generation, input);

    input.knownBalances.push({ assetKey: 'later' } as KnownAssetBalance);
    const firstRead = readManageAssetsDisplay('first', 'mainnet').snapshot;
    expect(firstRead?.knownBalances).toEqual([]);
    firstRead?.knownBalances.push({ assetKey: 'reader' } as KnownAssetBalance);
    expect(readManageAssetsDisplay('first', 'mainnet').snapshot?.knownBalances).toEqual([]);
  });

  it.each([resetWalletDisplaySession, invalidateWalletDisplayScopes])(
    'clears retained holdings and rejects old results after %s',
    (invalidate) => {
      const old = readManageAssetsDisplay('same-session', 'mainnet');
      writeManageAssetsDisplay('same-session', 'mainnet', old.generation, value);
      invalidate();
      const next = readManageAssetsDisplay('same-session', 'mainnet');
      expect(next.snapshot).toBeNull();
      writeManageAssetsDisplay('same-session', 'mainnet', old.generation, value);
      expect(readManageAssetsDisplay('same-session', 'mainnet').snapshot).toBeNull();
      writeManageAssetsDisplay('same-session', 'mainnet', next.generation, value);
      expect(readManageAssetsDisplay('same-session', 'mainnet').snapshot).toEqual(value);
    }
  );
});
