import { describe, expect, it, vi } from 'vitest';
import { ResponseSubmissionLifetime, runResponseSubmission } from './responseSubmission.js';

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

describe('response submission lifetime', () => {
  it('does not deliver or complete when the flow is disposed during signing', async () => {
    const signing = deferred<string>();
    const lifetime = new ResponseSubmissionLifetime();
    const deliver = vi.fn(async () => undefined);
    const complete = vi.fn();
    const fail = vi.fn();
    const settle = vi.fn();

    const submission = runResponseSubmission({
      lifetime,
      sign: () => signing.promise,
      deliver,
      complete,
      fail,
      settle,
    });

    lifetime.dispose();
    signing.resolve('signed-response');
    await submission;

    expect(deliver).not.toHaveBeenCalled();
    expect(complete).not.toHaveBeenCalled();
    expect(fail).not.toHaveBeenCalled();
    expect(settle).not.toHaveBeenCalled();
  });

  it('does not complete when the flow is cancelled while callback delivery is pending', async () => {
    const callback = deferred<void>();
    const lifetime = new ResponseSubmissionLifetime();
    const complete = vi.fn();
    const fail = vi.fn();
    const settle = vi.fn();

    const submission = runResponseSubmission({
      lifetime,
      sign: async () => 'signed-response',
      deliver: async () => callback.promise,
      complete,
      fail,
      settle,
    });

    await Promise.resolve();
    lifetime.cancel();
    callback.resolve();
    await submission;

    expect(complete).not.toHaveBeenCalled();
    expect(fail).not.toHaveBeenCalled();
    expect(settle).not.toHaveBeenCalled();
  });
});
