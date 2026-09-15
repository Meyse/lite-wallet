import { describe, expect, it } from 'vitest';
import { PreflightRequestGuard } from './preflightRequestGuard';

describe('PreflightRequestGuard', () => {
  it('accepts only the current request with the current signature', () => {
    const guard = new PreflightRequestGuard();
    const token = guard.begin('btc|standard');

    expect(guard.isCurrent(token, 'btc|standard')).toBe(true);
    expect(guard.isCurrent(token, 'btc|economy')).toBe(false);
  });

  it('rejects invalidated and superseded requests', () => {
    const guard = new PreflightRequestGuard();
    const invalidated = guard.begin('btc|standard');
    guard.invalidate();
    expect(guard.isCurrent(invalidated, 'btc|standard')).toBe(false);

    const stale = guard.begin('btc|economy');
    const current = guard.begin('btc|standard');
    expect(guard.isCurrent(stale, 'btc|economy')).toBe(false);
    expect(guard.isCurrent(current, 'btc|standard')).toBe(true);
  });

  it('rejects a request after its component session is disposed', () => {
    const guard = new PreflightRequestGuard();
    const token = guard.begin('eth|standard');
    guard.dispose();

    expect(guard.isCurrent(token, 'eth|standard')).toBe(false);
  });
});
