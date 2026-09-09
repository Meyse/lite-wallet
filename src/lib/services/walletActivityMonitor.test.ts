import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const touchSessionActivityMock = vi.hoisted(() => vi.fn<() => Promise<void>>());

vi.mock('./walletService.js', () => ({
  touchSessionActivity: touchSessionActivityMock,
}));

import { ForcedWalletLockError, walletUnlockRedirectingStore } from './walletLockCoordinator.js';
import { startWalletActivityMonitor } from './walletActivityMonitor.js';

interface RegisteredListener {
  listener: TestEventListener;
  capture: boolean;
}

type TestEventListener = ((event: Event) => void) | { handleEvent(event: Event): void };
type TestListenerOptions = boolean | { capture?: boolean };

class TestWindow {
  private readonly listeners = new Map<string, RegisteredListener[]>();

  addEventListener(
    type: string,
    listener: TestEventListener | null,
    options?: TestListenerOptions
  ): void {
    if (!listener) return;
    const capture = typeof options === 'boolean' ? options : (options?.capture ?? false);
    this.listeners.set(type, [...(this.listeners.get(type) ?? []), { listener, capture }]);
  }

  removeEventListener(
    type: string,
    listener: TestEventListener | null,
    options?: TestListenerOptions
  ): void {
    if (!listener) return;
    const capture = typeof options === 'boolean' ? options : (options?.capture ?? false);
    this.listeners.set(
      type,
      (this.listeners.get(type) ?? []).filter(
        (registered) => registered.listener !== listener || registered.capture !== capture
      )
    );
  }

  dispatchFromNestedTarget(type: string, { trusted = true, stoppedAtTarget = false } = {}): void {
    const event = new Event(type);
    Object.defineProperty(event, 'isTrusted', { value: trusted });
    Object.defineProperty(event, 'target', { value: {} });
    const listeners = this.listeners.get(type) ?? [];

    for (const { listener } of listeners.filter(({ capture }) => capture)) {
      this.callListener(listener, event);
    }

    if (!stoppedAtTarget) {
      for (const { listener } of listeners.filter(({ capture }) => !capture)) {
        this.callListener(listener, event);
      }
    }
  }

  dispatchAtWindow(type: string, { trusted = true } = {}): void {
    const event = new Event(type);
    Object.defineProperty(event, 'isTrusted', { value: trusted });
    Object.defineProperty(event, 'target', { value: this });

    for (const { listener } of this.listeners.get(type) ?? []) {
      this.callListener(listener, event);
    }
  }

  listenerCount(): number {
    return [...this.listeners.values()].reduce((total, listeners) => total + listeners.length, 0);
  }

  private callListener(listener: TestEventListener, event: Event): void {
    if (typeof listener === 'function') {
      listener(event);
    } else {
      listener.handleEvent(event);
    }
  }
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

describe('startWalletActivityMonitor', () => {
  let testWindow: TestWindow;
  let visibilityState: 'visible' | 'hidden';
  let stopMonitor: (() => void) | null;

  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(0);
    testWindow = new TestWindow();
    visibilityState = 'visible';
    stopMonitor = null;
    touchSessionActivityMock.mockReset();
    touchSessionActivityMock.mockResolvedValue(undefined);
    walletUnlockRedirectingStore.set(false);
    vi.stubGlobal('window', testWindow);
    vi.stubGlobal('document', {
      get visibilityState() {
        return visibilityState;
      },
    });
  });

  afterEach(() => {
    stopMonitor?.();
    walletUnlockRedirectingStore.set(false);
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it('records trusted pointer movement in capture and sends only one trailing touch', async () => {
    stopMonitor = startWalletActivityMonitor();

    testWindow.dispatchFromNestedTarget('pointermove', {
      trusted: false,
      stoppedAtTarget: true,
    });
    await vi.advanceTimersByTimeAsync(1_000);
    expect(touchSessionActivityMock).not.toHaveBeenCalled();

    testWindow.dispatchFromNestedTarget('pointermove', { stoppedAtTarget: true });
    await vi.advanceTimersByTimeAsync(13_500);
    testWindow.dispatchFromNestedTarget('pointermove', { stoppedAtTarget: true });
    expect(touchSessionActivityMock).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(500);
    expect(touchSessionActivityMock).toHaveBeenCalledOnce();

    await vi.advanceTimersByTimeAsync(60_000);
    expect(touchSessionActivityMock).toHaveBeenCalledOnce();
  });

  it.each(['wheel', 'touchstart', 'touchmove', 'keydown'])(
    'counts trusted %s scrolling input',
    async (type) => {
      stopMonitor = startWalletActivityMonitor();
      await vi.advanceTimersByTimeAsync(15_000);

      testWindow.dispatchFromNestedTarget(type, { stoppedAtTarget: true });

      expect(touchSessionActivityMock).toHaveBeenCalledOnce();
    }
  );

  it('accepts trusted window focus without accepting descendant or synthetic focus', async () => {
    stopMonitor = startWalletActivityMonitor();
    await vi.advanceTimersByTimeAsync(15_000);

    testWindow.dispatchFromNestedTarget('focus');
    testWindow.dispatchAtWindow('focus', { trusted: false });
    expect(touchSessionActivityMock).not.toHaveBeenCalled();

    testWindow.dispatchAtWindow('focus');
    expect(touchSessionActivityMock).toHaveBeenCalledOnce();
  });

  it('observes one nested scroll after direct input without accepting a scroll-event loop', async () => {
    stopMonitor = startWalletActivityMonitor();
    await vi.advanceTimersByTimeAsync(15_000);

    // A scroll event may be trusted even when script-driven, so it is ignored
    // until direct input opens the one-shot correlation window.
    testWindow.dispatchFromNestedTarget('scroll', { stoppedAtTarget: true });
    expect(touchSessionActivityMock).not.toHaveBeenCalled();

    testWindow.dispatchFromNestedTarget('wheel', { stoppedAtTarget: true });
    expect(touchSessionActivityMock).toHaveBeenCalledOnce();
    await vi.advanceTimersByTimeAsync(100);

    testWindow.dispatchFromNestedTarget('scroll', { stoppedAtTarget: true });
    for (let index = 0; index < 20; index += 1) {
      await vi.advanceTimersByTimeAsync(500);
      testWindow.dispatchFromNestedTarget('scroll', { stoppedAtTarget: true });
    }

    await vi.advanceTimersByTimeAsync(4_900);
    expect(touchSessionActivityMock).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(45_000);
    expect(touchSessionActivityMock).toHaveBeenCalledTimes(2);
  });

  it('retains the latest activity while a backend touch is pending', async () => {
    const firstTouch = deferred<void>();
    touchSessionActivityMock.mockReturnValueOnce(firstTouch.promise).mockResolvedValue(undefined);
    stopMonitor = startWalletActivityMonitor();
    await vi.advanceTimersByTimeAsync(15_000);

    testWindow.dispatchFromNestedTarget('pointerdown');
    expect(touchSessionActivityMock).toHaveBeenCalledOnce();

    await vi.advanceTimersByTimeAsync(16_000);
    testWindow.dispatchFromNestedTarget('pointermove');
    expect(touchSessionActivityMock).toHaveBeenCalledOnce();

    firstTouch.resolve();
    await vi.advanceTimersByTimeAsync(0);
    expect(touchSessionActivityMock).toHaveBeenCalledTimes(2);
  });

  it('discards activity that became stale behind a stalled backend touch', async () => {
    const firstTouch = deferred<void>();
    touchSessionActivityMock.mockReturnValueOnce(firstTouch.promise).mockResolvedValue(undefined);
    stopMonitor = startWalletActivityMonitor();
    await vi.advanceTimersByTimeAsync(15_000);

    testWindow.dispatchFromNestedTarget('pointerdown');
    await vi.advanceTimersByTimeAsync(1_000);
    testWindow.dispatchFromNestedTarget('pointermove');
    await vi.advanceTimersByTimeAsync(15_001);

    firstTouch.resolve();
    await vi.advanceTimersByTimeAsync(0);
    expect(touchSessionActivityMock).toHaveBeenCalledOnce();
  });

  it('drops queued work while hidden or redirecting and removes it on disposal', async () => {
    stopMonitor = startWalletActivityMonitor();
    testWindow.dispatchFromNestedTarget('pointermove');
    visibilityState = 'hidden';
    await vi.advanceTimersByTimeAsync(15_000);
    expect(touchSessionActivityMock).not.toHaveBeenCalled();
    stopMonitor();

    visibilityState = 'visible';
    stopMonitor = startWalletActivityMonitor();
    testWindow.dispatchFromNestedTarget('pointermove');
    walletUnlockRedirectingStore.set(true);
    await vi.advanceTimersByTimeAsync(15_000);
    expect(touchSessionActivityMock).not.toHaveBeenCalled();
    walletUnlockRedirectingStore.set(false);
    stopMonitor();

    stopMonitor = startWalletActivityMonitor();
    testWindow.dispatchFromNestedTarget('pointermove');
    stopMonitor();
    await vi.advanceTimersByTimeAsync(15_000);
    expect(touchSessionActivityMock).not.toHaveBeenCalled();
    expect(testWindow.listenerCount()).toBe(0);
  });

  it('logs ordinary failures and accepts later activity', async () => {
    const error = new Error('touch failed');
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    touchSessionActivityMock.mockRejectedValueOnce(error).mockResolvedValue(undefined);
    stopMonitor = startWalletActivityMonitor();
    await vi.advanceTimersByTimeAsync(15_000);

    testWindow.dispatchFromNestedTarget('keydown');
    await vi.advanceTimersByTimeAsync(0);
    expect(consoleError).toHaveBeenCalledWith(
      '[WALLET_ACTIVITY] Failed to refresh session activity',
      error
    );

    await vi.advanceTimersByTimeAsync(1_000);
    testWindow.dispatchFromNestedTarget('pointermove');
    await vi.advanceTimersByTimeAsync(14_000);
    expect(touchSessionActivityMock).toHaveBeenCalledTimes(2);
  });

  it('silences a forced-lock failure and discards activity queued behind it', async () => {
    const touch = deferred<void>();
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    touchSessionActivityMock.mockReturnValueOnce(touch.promise).mockResolvedValue(undefined);
    stopMonitor = startWalletActivityMonitor();
    await vi.advanceTimersByTimeAsync(15_000);

    testWindow.dispatchFromNestedTarget('pointerdown');
    await vi.advanceTimersByTimeAsync(1_000);
    testWindow.dispatchFromNestedTarget('pointermove');
    touch.reject(new ForcedWalletLockError());
    await vi.advanceTimersByTimeAsync(0);

    await vi.advanceTimersByTimeAsync(30_000);
    expect(touchSessionActivityMock).toHaveBeenCalledOnce();
    expect(consoleError).not.toHaveBeenCalled();
  });
});
