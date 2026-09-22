// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '$lib/i18n';
import AboutSupportSettings from './AboutSupportSettings.svelte';

const appInfo = vi.hoisted(() => ({ loadRuntimeAppInfo: vi.fn() }));
const externalLinks = vi.hoisted(() => ({ openCommunityHangout: vi.fn() }));

vi.mock('$lib/utils/appInfo.js', () => appInfo);
vi.mock('$lib/utils/externalLinks.js', () => externalLinks);

async function settle(): Promise<void> {
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 0));
}

describe('mounted about and support settings', () => {
  beforeEach(() => {
    setLocale('en');
    appInfo.loadRuntimeAppInfo.mockReset();
    externalLinks.openCommunityHangout.mockReset();
  });

  afterEach(() => {
    document.body.replaceChildren();
  });

  it('shows runtime identity and keeps the community action usable', async () => {
    appInfo.loadRuntimeAppInfo.mockResolvedValue({ name: 'Verus Wallet', version: '9.8.7' });
    externalLinks.openCommunityHangout.mockResolvedValue(undefined);
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(AboutSupportSettings, {
      target,
      props: { onBack: vi.fn() },
    });
    await settle();

    expect(document.body.textContent).toContain('Verus Wallet');
    expect(document.body.textContent).toContain('Version 9.8.7');
    expect(target.querySelector('img')?.getAttribute('src')).toBe('/images/verus-express-icon.png');
    const communityButton = [...document.body.querySelectorAll<HTMLButtonElement>('button')].find(
      (button) => button.textContent?.includes('Join the community on Discord')
    );
    communityButton?.click();
    expect(externalLinks.openCommunityHangout).toHaveBeenCalledOnce();

    await unmount(component);
  });

  it('shows a finite loading state while runtime metadata is pending', async () => {
    let resolveInfo: ((value: { name: string; version: string }) => void) | undefined;
    appInfo.loadRuntimeAppInfo.mockReturnValue(
      new Promise((resolve) => {
        resolveInfo = resolve;
      })
    );
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(AboutSupportSettings, {
      target,
      props: { onBack: vi.fn() },
    });
    await settle();

    expect(document.body.textContent).toContain('—');
    expect(document.body.textContent).not.toContain('Loading…');
    resolveInfo?.({ name: 'Verus Wallet', version: '1.2.3' });
    await settle();
    expect(document.body.textContent).toContain('Version 1.2.3');

    await unmount(component);
  });

  it('settles into an unavailable version state after a runtime failure', async () => {
    appInfo.loadRuntimeAppInfo.mockRejectedValue(new Error('unavailable'));
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(AboutSupportSettings, {
      target,
      props: { onBack: vi.fn() },
    });
    await settle();

    expect(document.body.textContent).toContain('Version unavailable');
    expect(document.body.textContent).toContain('Join the community on Discord');

    await unmount(component);
  });
});
