import { describe, expect, it, vi } from 'vitest';
import { DisposableScope } from './disposableScope';

describe('DisposableScope', () => {
  it('disposes registered work once in reverse setup order', () => {
    const calls: string[] = [];
    const scope = new DisposableScope();

    scope.add(() => calls.push('first'));
    scope.add(() => calls.push('second'));
    scope.dispose();
    scope.dispose();

    expect(calls).toEqual(['second', 'first']);
  });

  it('immediately cleans up asynchronous setup that resolves after disposal', async () => {
    const cleanup = vi.fn();
    let resolveSetup: ((dispose: () => void) => void) | undefined;
    const setup = new Promise<() => void>((resolve) => {
      resolveSetup = resolve;
    });
    const scope = new DisposableScope();

    const registration = setup.then((dispose) => scope.add(dispose));
    scope.dispose();
    resolveSetup?.(cleanup);

    await expect(registration).resolves.toBe(false);
    expect(cleanup).toHaveBeenCalledOnce();
    expect(scope.active).toBe(false);
  });
});
