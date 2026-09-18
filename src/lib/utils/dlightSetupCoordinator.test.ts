import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  clearDlightSetupSession,
  getDlightSetupOperation,
  startDlightSetupOperation,
  waitForDlightSetupOperation,
} from './dlightSetupCoordinator';

const SESSION_KEY = 'coordinator-test-session';

describe('dLight setup coordinator', () => {
  afterEach(() => {
    clearDlightSetupSession(SESSION_KEY);
  });

  it('removes settled operations and exposes only nonsecret settlement state', async () => {
    let resolveSetup:
      | ((value: {
          configured: boolean;
          generatedSeedPhrase: string;
          requiresRelogin: boolean;
        }) => void)
      | undefined;
    const run = vi.fn(
      () =>
        new Promise<{
          configured: boolean;
          generatedSeedPhrase: string;
          requiresRelogin: boolean;
        }>((resolve) => {
          resolveSetup = resolve;
        })
    );

    const started = startDlightSetupOperation(SESSION_KEY, 'create_new', run);
    expect(started.started).toBe(true);
    if (!started.started) return;

    const settlementPromise = waitForDlightSetupOperation(started.operation);
    resolveSetup?.({
      configured: true,
      generatedSeedPhrase: 'synthetic secret that must not enter coordinator state',
      requiresRelogin: true,
    });

    await expect(settlementPromise).resolves.toEqual({
      status: 'success',
      configured: true,
      requiresRelogin: true,
    });
    expect(getDlightSetupOperation(SESSION_KEY)).toBeNull();
    expect(JSON.stringify(started.operation)).not.toContain('synthetic secret');
    await expect(started.ownerResult).resolves.toMatchObject({
      generatedSeedPhrase: 'synthetic secret that must not enter coordinator state',
    });
  });

  it('cancels waiters on session clear and ignores stale settlement callbacks', async () => {
    let resolveSetup:
      ((value: { configured: boolean; requiresRelogin: boolean }) => void) | undefined;
    const started = startDlightSetupOperation(
      SESSION_KEY,
      'reuse_primary',
      () =>
        new Promise((resolve) => {
          resolveSetup = resolve;
        })
    );
    expect(started.started).toBe(true);
    if (!started.started) return;

    const settlementPromise = waitForDlightSetupOperation(started.operation);
    clearDlightSetupSession(SESSION_KEY);
    await expect(settlementPromise).resolves.toEqual({ status: 'cancelled' });
    expect(getDlightSetupOperation(SESSION_KEY)).toBeNull();

    resolveSetup?.({ configured: true, requiresRelogin: false });
    await started.ownerResult;
    await expect(waitForDlightSetupOperation(started.operation)).resolves.toEqual({
      status: 'cancelled',
    });
    expect(getDlightSetupOperation(SESSION_KEY)).toBeNull();
  });
});
