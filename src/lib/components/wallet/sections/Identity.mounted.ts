// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { LinkedIdentity } from '$lib/types/wallet';

const mocks = vi.hoisted(() => ({
  getLinkedIdentities: vi.fn(),
  getIdentityProfile: vi.fn(),
  getPendingIdentityProfileUpdates: vi.fn(),
  clearPendingIdentityProfileUpdate: vi.fn(),
  setLinkedIdentityFavorite: vi.fn(),
  listIdentityProvisioningJobs: vi.fn(),
  toastError: vi.fn(),
}));

vi.mock('$lib/services/identityLinkService.js', () => ({
  getLinkedIdentities: mocks.getLinkedIdentities,
  getIdentityDetails: vi.fn(),
  getIdentityProfile: mocks.getIdentityProfile,
  getPendingIdentityProfileUpdates: mocks.getPendingIdentityProfileUpdates,
  clearPendingIdentityProfileUpdate: mocks.clearPendingIdentityProfileUpdate,
  setLinkedIdentityFavorite: mocks.setLinkedIdentityFavorite,
  unlinkIdentity: vi.fn(),
}));

vi.mock('$lib/services/genericRequestService.js', () => ({
  listIdentityProvisioningJobs: mocks.listIdentityProvisioningJobs,
  refreshIdentityProvisioningJobs: vi.fn(),
  linkReadyIdentityProvisioning: vi.fn(),
}));

vi.mock('$lib/services/walletLockCoordinator.js', () => ({
  isForcedWalletLockError: () => false,
}));

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: vi.fn(),
}));

vi.mock('svelte-sonner', () => ({
  toast: {
    error: mocks.toastError,
    success: vi.fn(),
  },
}));

vi.mock('./identity/LinkIdentitySheet.svelte', async () => ({
  default: (await import('./transfer-wizard/test-fixtures/EmptyWalletChild.svelte')).default,
}));

import { localeStore } from '$lib/i18n';
import Identity from './Identity.svelte';
import { createIdentitySectionSessionState } from './identity/identitySectionSessionState';

const favoriteIdentity: LinkedIdentity = {
  identityAddress: 'iFavoriteIdentity',
  name: 'Favorite',
  fullyQualifiedName: 'favorite@',
  status: 'active',
  systemId: null,
  favorite: true,
};

const otherIdentity: LinkedIdentity = {
  identityAddress: 'iOtherIdentity',
  name: 'Other',
  fullyQualifiedName: 'other@',
  status: 'active',
  systemId: null,
  favorite: false,
};

function deferred<T>(): {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (error: unknown) => void;
} {
  let resolve: (value: T) => void = () => {};
  let reject: (error: unknown) => void = () => {};
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

async function settle(): Promise<void> {
  for (let index = 0; index < 4; index += 1) await Promise.resolve();
  await tick();
}

function initialSessionState() {
  return {
    ...createIdentitySectionSessionState(),
    linkedIdentities: [favoriteIdentity, otherIdentity],
    hasLoadedLinkedIdentitiesOnce: true,
    hasLoadedProvisioningOnce: true,
  };
}

beforeEach(() => {
  document.body.innerHTML = '';
  document.documentElement.classList.remove('dark');
  localeStore.set('en');
  mocks.getLinkedIdentities.mockReset().mockResolvedValue([favoriteIdentity, otherIdentity]);
  mocks.getIdentityProfile.mockReset().mockResolvedValue({
    state: 'missing',
    avatar: null,
    description: null,
    issues: [],
    readHeight: null,
  });
  mocks.getPendingIdentityProfileUpdates.mockReset().mockResolvedValue([]);
  mocks.clearPendingIdentityProfileUpdate.mockReset().mockResolvedValue(true);
  mocks.setLinkedIdentityFavorite.mockReset();
  mocks.listIdentityProvisioningJobs.mockReset().mockResolvedValue([]);
  mocks.toastError.mockReset();
});

describe('mounted identity favorite toggle', () => {
  it.each(['light', 'dark'] as const)(
    'shows a single-flight saving state and applies persisted success in %s mode',
    async (theme) => {
      document.documentElement.classList.toggle('dark', theme === 'dark');
      const target = document.createElement('div');
      document.body.append(target);
      const save = deferred<LinkedIdentity[]>();
      mocks.setLinkedIdentityFavorite.mockReturnValue(save.promise);
      const component = mount(Identity, {
        target,
        props: { sessionState: initialSessionState() },
      });

      try {
        await settle();
        const favoriteButton = target.querySelector(
          '[data-favorite-state="favorite"]'
        ) as HTMLButtonElement | null;
        expect(favoriteButton).not.toBeNull();

        favoriteButton?.click();
        await settle();

        const savingButton = target.querySelector(
          '[data-favorite-state="saving"]'
        ) as HTMLButtonElement | null;
        expect(savingButton).not.toBeNull();
        expect(savingButton?.disabled).toBe(true);
        expect(savingButton?.getAttribute('aria-label')).toBe('Saving change…');
        expect(
          Array.from(target.querySelectorAll('[data-favorite-state]')).every(
            (button) => (button as HTMLButtonElement).disabled
          )
        ).toBe(true);

        savingButton?.click();
        expect(mocks.setLinkedIdentityFavorite).toHaveBeenCalledTimes(1);
        expect(mocks.setLinkedIdentityFavorite).toHaveBeenCalledWith({
          identityAddress: favoriteIdentity.identityAddress,
          favorite: false,
        });

        save.resolve([{ ...favoriteIdentity, favorite: false }, otherIdentity]);
        await settle();

        expect(target.querySelector('[data-favorite-state="saving"]')).toBeNull();
        expect(target.querySelector('[data-favorite-state="favorite"]')).toBeNull();
        expect(target.querySelectorAll('[data-favorite-state="not-favorite"]')).toHaveLength(2);
        expect(
          Array.from(target.querySelectorAll('[data-favorite-state]')).every(
            (button) => !(button as HTMLButtonElement).disabled
          )
        ).toBe(true);
      } finally {
        await unmount(component);
        target.remove();
      }
    }
  );

  it('restores the previous star state and reports a failed save', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    const save = deferred<LinkedIdentity[]>();
    mocks.setLinkedIdentityFavorite.mockReturnValue(save.promise);
    const component = mount(Identity, {
      target,
      props: { sessionState: initialSessionState() },
    });

    try {
      await settle();
      const favoriteButton = target.querySelector(
        '[data-favorite-state="favorite"]'
      ) as HTMLButtonElement | null;
      favoriteButton?.click();
      await settle();
      expect(target.querySelector('[data-favorite-state="saving"]')).not.toBeNull();

      save.reject({ type: 'OperationFailed' });
      await settle();

      const restoredButton = target.querySelector(
        '[data-favorite-state="favorite"]'
      ) as HTMLButtonElement | null;
      expect(restoredButton).not.toBeNull();
      expect(restoredButton?.disabled).toBe(false);
      expect(mocks.toastError).toHaveBeenCalledOnce();
    } finally {
      await unmount(component);
      target.remove();
    }
  });
});
