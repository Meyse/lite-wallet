import { describe, expect, it, vi } from 'vitest';
import { ApprovalRun, approvalStages, planRequestFlow, timeLeft } from './flowPlan';
import type { SupportedGenericRequestDetail } from './session';

const detail = (type: SupportedGenericRequestDetail['type']) =>
  ({ type }) as SupportedGenericRequestDetail;
const auth = detail('authentication');
const update = detail('identityUpdate');
const provision = detail('provisionIdentity');
const userData = detail('userData');

describe('planRequestFlow', () => {
  // Expected layouts and steps follow the requests spec ("Layout", "Steps per detail").
  it('keeps a sign in alone on a single screen', () => {
    expect(planRequestFlow([auth], { newIdentity: false })).toEqual({
      layout: 'single',
      steps: ['signIn'],
    });
  });

  it('puts an identity update on the rail with a changes step, a fee step, and Confirm', () => {
    expect(planRequestFlow([auth, update], { newIdentity: false })).toEqual({
      layout: 'rail',
      steps: ['signIn', 'updateChanges', 'networkFee', 'confirm'],
    });
    expect(planRequestFlow([update], { newIdentity: false }).steps).toEqual([
      'updateChanges',
      'networkFee',
      'confirm',
    ]);
  });

  it('adds a new VerusID step only when the user asks for one', () => {
    expect(planRequestFlow([auth, provision], { newIdentity: false })).toEqual({
      layout: 'single',
      steps: ['signIn'],
    });
    expect(planRequestFlow([auth, provision], { newIdentity: true })).toEqual({
      layout: 'rail',
      steps: ['signIn', 'newIdentity', 'confirm'],
    });
  });
});

describe('planRequestFlow share data', () => {
  it('adds a share data step after the sign in, on the rail', () => {
    // Share data shares data, so it never takes the single screen.
    expect(planRequestFlow([auth, userData], { newIdentity: false })).toEqual({
      layout: 'rail',
      steps: ['signIn', 'shareData', 'confirm'],
    });
    expect(planRequestFlow([userData], { newIdentity: false })).toEqual({
      layout: 'rail',
      steps: ['shareData', 'confirm'],
    });
  });
});

describe('approvalStages', () => {
  it('orders the transaction, signing, and delivery', () => {
    expect(approvalStages({ hasTransaction: true, signsResponse: true, delivers: true })).toEqual([
      'transaction',
      'signing',
      'delivery',
    ]);
    expect(approvalStages({ hasTransaction: false, signsResponse: true, delivers: false })).toEqual(
      ['signing']
    );
  });
});

describe('ApprovalRun', () => {
  it('retries only the failed stage and never repeats a finished broadcast', async () => {
    const broadcast = vi.fn(async () => {});
    const sign = vi
      .fn()
      .mockRejectedValueOnce(new Error('signing failed'))
      .mockResolvedValue(undefined);
    const deliver = vi.fn(async () => {});
    const run = new ApprovalRun(['transaction', 'signing', 'delivery'], {
      transaction: broadcast,
      signing: sign,
      delivery: deliver,
    });

    expect(await run.run()).toBe(false);
    expect(run.status).toEqual({ transaction: 'done', signing: 'failed', delivery: 'pending' });
    expect(run.failedStage).toBe('signing');
    expect(deliver).not.toHaveBeenCalled();

    expect(await run.run()).toBe(true);
    expect(run.finished).toBe(true);
    expect(broadcast).toHaveBeenCalledTimes(1);
    expect(sign).toHaveBeenCalledTimes(2);
    expect(deliver).toHaveBeenCalledTimes(1);
  });

  it('does not start a second run while one is running', async () => {
    let release = () => {};
    const sign = vi.fn(() => new Promise<void>((resolve) => (release = resolve)));
    const run = new ApprovalRun(['signing'], {
      transaction: vi.fn(),
      signing: sign,
      delivery: vi.fn(),
    });
    const first = run.run();
    expect(await run.run()).toBe(false);
    release();
    expect(await first).toBe(true);
    expect(sign).toHaveBeenCalledTimes(1);
  });
});

describe('timeLeft', () => {
  it('shows minutes under 90 minutes, hours above, and expiry at zero', () => {
    expect(timeLeft(1_000 + 9 * 60, 1_000)).toEqual({ kind: 'minutes', minutes: 9 });
    expect(timeLeft(1_000 + 61, 1_000)).toEqual({ kind: 'minutes', minutes: 2 });
    expect(timeLeft(1_000 + 2 * 3600, 1_000)).toEqual({ kind: 'hours', hours: 2 });
    expect(timeLeft(1_000, 1_000)).toEqual({ kind: 'expired' });
  });
});
