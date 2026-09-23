import { describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { ProfilePublicationController } from './profilePublication';
import type {
  IdentityProfileLoadResult,
  IdentityProfilePreflightResult,
  ProfilePublicationState,
} from '$lib/types/wallet';

const receipt = {
  identityAddress: 'identity',
  txid: 'a'.repeat(64),
  submittedAt: 1,
  previousProfile: {},
  proposedProfile: { description: 'Confirmed' },
};
const canonical: IdentityProfileLoadResult = {
  state: 'ready',
  issues: [],
  revisionTxid: receipt.txid,
  description: {
    value: 'Confirmed',
    source: {
      systemId: 'test',
      txid: receipt.txid,
      height: 1,
      blockhash: 'block',
      vout: 0,
      digest: 'digest',
    },
  },
};
const request = {
  identityAddress: 'identity',
  coinId: 'VRSCTEST',
  channelId: 'test',
  avatar: { action: 'keep' as const },
  description: { action: 'set' as const, value: 'Confirmed' },
};
function plan(step: 1 | 2 = 1, totalSteps: 1 | 2 = 1): ProfilePublicationState {
  return {
    planId: 'plan',
    identityAddress: 'identity',
    status: 'ready',
    step,
    totalSteps,
    request,
    pending: null,
    settledTxids: [],
  };
}
function prepared(step = 1, totalSteps = 1) {
  return { publication: { planId: 'plan', step, totalSteps } } as IdentityProfilePreflightResult;
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}
function harness() {
  const io = {
    current: vi.fn(() => true),
    read: vi.fn<() => Promise<ProfilePublicationState | null>>().mockResolvedValue(null),
    confirm: vi.fn<() => Promise<IdentityProfileLoadResult | null>>().mockResolvedValue(canonical),
    submitted: vi.fn(),
    settled: vi.fn(),
    confirmed: vi.fn(),
  };
  return { io, owner: new ProfilePublicationController('identity', io) };
}
describe('receipt-bound publication owner', () => {
  it('keeps an ambiguous submission closed when the old ready plan is still returned', async () => {
    const { io, owner } = harness();
    owner.reviewed(prepared(), request);
    expect(owner.beginSubmission()).toBe(true);
    io.read.mockResolvedValue(plan());
    owner.submissionFailed();
    await owner.refresh();
    expect(get(owner).uncertain).toBe(true);
    expect(get(owner).plan?.status).toBe('ready');
    expect(owner.beginSubmission()).toBe(false);
    expect(io.submitted).not.toHaveBeenCalled();
  });
  it('recovers an uncertain send that completed before the first confirmation check', async () => {
    const { io, owner } = harness();
    owner.reviewed(prepared(), request);
    expect(owner.beginSubmission()).toBe(true);
    io.read.mockResolvedValue({
      ...plan(),
      status: 'complete',
      completedReceipt: receipt,
      settledTxids: [receipt.txid],
    });
    owner.submissionFailed();
    await owner.refresh();
    expect(get(owner).completed).toBe(true);
    expect(io.confirmed).toHaveBeenCalledExactlyOnceWith(receipt, canonical, true);
    expect(io.submitted).not.toHaveBeenCalled();
  });
  it.each([
    [1, 1],
    [1, 2],
    [2, 2],
  ] as const)('ignores a delayed ready observation after step %i/%i sends', async (step, total) => {
    const { io, owner } = harness();
    const old = deferred<ProfilePublicationState>();
    owner.reviewed(prepared(step, total), request);
    io.read
      .mockReturnValueOnce(old.promise)
      .mockResolvedValue({ ...plan(step, total), status: 'waiting', pending: receipt });
    const refresh = owner.refresh();
    expect(owner.beginSubmission()).toBe(true);
    owner.submitted(receipt);
    expect(get(owner).plan?.status).toBe('waiting');
    old.resolve(plan(step, total));
    await refresh;
    expect(io.read).toHaveBeenCalledTimes(2);
    expect(get(owner).receipt).toEqual(receipt);
    expect(get(owner).plan?.status).toBe('waiting');
    expect(io.submitted).toHaveBeenCalledExactlyOnceWith(receipt);
  });
  it('queues manual refresh while checking, and rejects unmounted/session-changed results', async () => {
    const { io, owner } = harness();
    const old = deferred<ProfilePublicationState>();
    io.read.mockReturnValueOnce(old.promise).mockResolvedValue(plan());
    const running = owner.refresh();
    void owner.refresh();
    old.resolve(plan());
    await running;
    expect(io.read).toHaveBeenCalledTimes(2);
    const late = deferred<ProfilePublicationState>();
    io.read.mockReturnValueOnce(late.promise);
    const done = owner.refresh();
    io.current.mockReturnValue(false);
    late.resolve({ ...plan(), status: 'stale' });
    await done;
    expect(get(owner).plan?.status).toBe('ready');
    owner.dispose();
    await owner.refresh();
    expect(io.read).toHaveBeenCalledTimes(3);
  });
  it.each(['complete', 'null'])(
    'consumes %s with confirmed matching evidence exactly once',
    async (kind) => {
      const { io, owner } = harness();
      owner.reviewed(prepared(), request);
      owner.remember(receipt);
      io.read.mockResolvedValue(
        kind === 'complete' ? { ...plan(), status: 'complete', settledTxids: [receipt.txid] } : null
      );
      await owner.refresh();
      await owner.refresh();
      expect(get(owner).completed).toBe(true);
      expect(io.confirmed).toHaveBeenCalledExactlyOnceWith(receipt, canonical, true);
      expect(io.settled).toHaveBeenCalledExactlyOnceWith([receipt.txid]);
    }
  );
  it.each([
    null,
    { ...canonical, revisionTxid: 'b'.repeat(64) },
    { ...canonical, state: 'unavailable' as const },
    { ...canonical, description: null },
  ])('does not mistake null or mismatched evidence for success', async (profile) => {
    const { io, owner } = harness();
    owner.reviewed(prepared(), request);
    owner.remember(receipt);
    io.confirm.mockResolvedValue(profile);
    await owner.refresh();
    expect(get(owner).completed).toBe(false);
    expect(get(owner).receipt).toEqual(receipt);
    expect(io.confirmed).not.toHaveBeenCalled();
  });
  it('allows fresh conflict/expiry and reorg recovery without treating settled IDs as success', async () => {
    const { io, owner } = harness();
    owner.reviewed(prepared(2, 2), request);
    owner.remember(receipt);
    io.read.mockResolvedValue({ ...plan(1, 2), status: 'stale', settledTxids: [receipt.txid] });
    await owner.refresh();
    expect(get(owner).plan?.status).toBe('stale');
    expect(get(owner).plan?.step).toBe(1);
    expect(io.confirmed).not.toHaveBeenCalled();
    expect(io.confirm).not.toHaveBeenCalled();
  });
  it('blocks actions during ambiguous transport until backend observation resolves it', async () => {
    const { io, owner } = harness();
    owner.reviewed(prepared(), request);
    const pending = deferred<ProfilePublicationState>();
    io.read.mockReturnValue(pending.promise);
    owner.beginSubmission();
    owner.submissionFailed();
    expect(get(owner).uncertain).toBe(true);
    expect(owner.beginSubmission()).toBe(false);
    pending.resolve({ ...plan(), status: 'waiting', pending: receipt });
    await owner.refresh();
    expect(get(owner).receipt).toEqual(receipt);
    expect(get(owner).plan?.status).toBe('waiting');
    expect(io.submitted).toHaveBeenCalledExactlyOnceWith(receipt);
  });
  it('ignores observations from another plan and confirms an all-field removal', async () => {
    const { io, owner } = harness();
    owner.reviewed(prepared(), request);
    owner.remember({ ...receipt, proposedProfile: {} });
    io.read.mockResolvedValue({ ...plan(), planId: 'other' });
    await owner.refresh();
    expect(get(owner).plan?.planId).toBe('plan');
    io.read.mockResolvedValue(null);
    io.confirm.mockResolvedValue({ state: 'empty', issues: [], revisionTxid: receipt.txid });
    await owner.refresh();
    expect(get(owner).completed).toBe(true);
  });
});
