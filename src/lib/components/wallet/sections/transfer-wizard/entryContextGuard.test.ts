import { describe, expect, it } from 'vitest';
import { EntryContextGuard } from './entryContextGuard';

describe('EntryContextGuard', () => {
  it('waits for authoritative source data and applies an entry context only once', () => {
    const guard = new EntryContextGuard();
    expect(guard.claim('VRSC|vrpc.Rprimary.iVerus|transparent|owned', false)).toBe(false);
    expect(guard.claim('VRSC|vrpc.Rprimary.iVerus|transparent|owned', true)).toBe(true);

    // Balance/scope recomputation must not reclaim the source after a manual choice.
    expect(guard.claim('VRSC|vrpc.Rprimary.iVerus|transparent|owned', true)).toBe(false);
  });

  it('allows a genuinely new entry context in the same mounted session', () => {
    const guard = new EntryContextGuard();
    expect(guard.claim('VRSC|vrpc.Rprimary.iVerus|transparent|owned', true)).toBe(true);
    expect(guard.claim('BTC|btc.BTC|transparent|owned', true)).toBe(true);
  });
});
