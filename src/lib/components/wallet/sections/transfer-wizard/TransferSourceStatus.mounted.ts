// @vitest-environment jsdom

import { mount, unmount } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import TransferSourceStatus from './TransferSourceStatus.svelte';

describe('mounted transfer source status', () => {
  it.each(['light', 'dark'] as const)(
    'renders loading, all-failed, partial, empty, and retry states in %s mode',
    async (theme) => {
      document.documentElement.classList.toggle('dark', theme === 'dark');
      const cases = [
        { loading: true, failed: false, hasAssets: false, status: 'loading' },
        { loading: false, failed: true, hasAssets: false, status: 'failed' },
        { loading: false, failed: true, hasAssets: true, status: 'partial' },
        { loading: false, failed: false, hasAssets: false, status: 'empty' },
      ] as const;

      for (const state of cases) {
        const target = document.createElement('div');
        document.body.append(target);
        const onRetry = vi.fn();
        const component = mount(TransferSourceStatus, {
          target,
          props: { ...state, onRetry },
        });
        expect(
          target.querySelector(`[data-transfer-source-status="${state.status}"]`)
        ).not.toBeNull();
        const retry = target.querySelector('button') as HTMLButtonElement | null;
        if (state.failed) {
          expect(retry).not.toBeNull();
          retry?.click();
          expect(onRetry).toHaveBeenCalledOnce();
        } else {
          expect(retry).toBeNull();
        }
        await unmount(component);
        target.remove();
      }
    }
  );
});
