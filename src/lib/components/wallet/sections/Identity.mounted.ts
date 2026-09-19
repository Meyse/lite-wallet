// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  IdentityProfileLoadResult,
  LinkedIdentity,
  PendingIdentityProfileUpdate,
} from '$lib/types/wallet';

const mocks = vi.hoisted(() => ({
  getLinkedIdentities: vi.fn(),
  getIdentityProfile: vi.fn(),
  getPendingIdentityProfileUpdates: vi.fn(),
  clearPendingIdentityProfileUpdate: vi.fn(),
  setLinkedIdentityFavorite: vi.fn(),
  listIdentityProvisioningJobs: vi.fn(),
  toastError: vi.fn(),
  toastSuccess: vi.fn(),
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
    success: mocks.toastSuccess,
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

const TXID = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
const OTHER_TXID = 'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff';

const pendingRemoval: PendingIdentityProfileUpdate = {
  identityAddress: favoriteIdentity.identityAddress,
  txid: TXID,
  submittedAt: 1,
  previousProfile: { description: 'Previous', descriptionDigest: 'previous-digest' },
  proposedProfile: {},
};

function pendingSessionState(profile: IdentityProfileLoadResult) {
  return {
    ...initialSessionState(),
    profilesByAddress: { [favoriteIdentity.identityAddress.toLowerCase()]: profile },
    pendingProfilesByAddress: {
      [favoriteIdentity.identityAddress.toLowerCase()]: pendingRemoval,
    },
  };
}

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
    state: 'empty',
    avatar: null,
    description: null,
    issues: [],
    readHeight: null,
    revisionTxid: null,
  });
  mocks.getPendingIdentityProfileUpdates.mockReset().mockResolvedValue([]);
  mocks.clearPendingIdentityProfileUpdate.mockReset().mockResolvedValue(true);
  mocks.setLinkedIdentityFavorite.mockReset();
  mocks.listIdentityProvisioningJobs.mockReset().mockResolvedValue([]);
  mocks.toastError.mockReset();
  mocks.toastSuccess.mockReset();
});

describe('mounted identity empty state', () => {
  it.each(['light', 'dark'] as const)(
    'matches the shared empty-state hierarchy in %s mode',
    async (theme) => {
      document.documentElement.classList.toggle('dark', theme === 'dark');
      mocks.getLinkedIdentities.mockResolvedValue([]);
      const target = document.createElement('div');
      document.body.append(target);
      const component = mount(Identity, {
        target,
        props: {
          sessionState: {
            ...createIdentitySectionSessionState(),
            hasLoadedLinkedIdentitiesOnce: true,
            hasLoadedProvisioningOnce: true,
          },
        },
      });

      try {
        await settle();
        const emptyState = target.querySelector('[data-testid="identity-empty"]');
        expect(emptyState).not.toBeNull();
        if (!emptyState) throw new Error('Missing identity empty state');
        expect(target.querySelector('header h2')?.textContent?.trim()).toBe('VerusID');
        expect(target.querySelectorAll('header button')).toHaveLength(0);
        expect(emptyState.querySelector('h3')?.textContent?.trim()).toBe(
          'Link a VerusID to this wallet'
        );
        expect(emptyState.querySelector('[data-testid="wallet-empty-eyebrow"]')).toBeNull();
        expect(emptyState.textContent).not.toContain('controlled by this wallet address');
        expect([...emptyState.children].some((child) => child.tagName === 'svg')).toBe(false);
        expect(emptyState.querySelector('button svg')).not.toBeNull();
      } finally {
        await unmount(component);
        target.remove();
      }
    }
  );
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

describe('mounted profile confirmation reconciliation', () => {
  it('shows one removal toast and clears the marker only for the matching canonical revision', async () => {
    const confirmedEmpty: IdentityProfileLoadResult = {
      state: 'empty',
      avatar: null,
      description: null,
      issues: [],
      revisionTxid: TXID,
      readHeight: 10,
    };
    mocks.getPendingIdentityProfileUpdates.mockResolvedValue([pendingRemoval]);
    mocks.getIdentityProfile.mockResolvedValue(confirmedEmpty);
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(Identity, {
      target,
      props: { sessionState: pendingSessionState(confirmedEmpty) },
    });
    try {
      await settle();
      await vi.waitFor(() => expect(mocks.clearPendingIdentityProfileUpdate).toHaveBeenCalled());
      expect(mocks.toastSuccess).toHaveBeenCalledOnce();
      expect(mocks.toastSuccess).toHaveBeenCalledWith(
        'Profile data removed from the current profile.'
      );
      expect(target.textContent).not.toContain('Profile updated');
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it.each([
    [
      'an unavailable read',
      {
        state: 'unavailable',
        avatar: null,
        description: null,
        issues: [],
        revisionTxid: TXID,
        readHeight: 10,
      } satisfies IdentityProfileLoadResult,
    ],
    [
      'an issue-bearing read',
      {
        state: 'empty',
        avatar: null,
        description: null,
        issues: [{ field: 'description', code: 'invalid_or_unavailable' }],
        revisionTxid: TXID,
        readHeight: 10,
      } satisfies IdentityProfileLoadResult,
    ],
    [
      'the wrong revision',
      {
        state: 'empty',
        avatar: null,
        description: null,
        issues: [],
        revisionTxid: OTHER_TXID,
        readHeight: 10,
      } satisfies IdentityProfileLoadResult,
    ],
    [
      'a mismatched snapshot',
      {
        state: 'ready',
        avatar: null,
        description: {
          value: 'Previous',
          source: {
            systemId: 'i-system',
            txid: TXID,
            vout: 0,
            height: 10,
            blockhash: 'block',
            digest: 'previous-digest',
          },
        },
        issues: [],
        revisionTxid: TXID,
        readHeight: 10,
      } satisfies IdentityProfileLoadResult,
    ],
  ])('keeps the pending marker for %s', async (_label, profile) => {
    mocks.getPendingIdentityProfileUpdates.mockResolvedValue([pendingRemoval]);
    mocks.getIdentityProfile.mockResolvedValue(profile);
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(Identity, {
      target,
      props: { sessionState: pendingSessionState(profile) },
    });
    try {
      await settle();
      expect(mocks.toastSuccess).not.toHaveBeenCalled();
      expect(mocks.clearPendingIdentityProfileUpdate).not.toHaveBeenCalled();
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('keeps the previous confirmed profile visible when a refresh cannot prove the removal', async () => {
    const previousProfile: IdentityProfileLoadResult = {
      state: 'ready',
      avatar: null,
      description: {
        value: 'Previous',
        source: {
          systemId: 'i-system',
          txid: OTHER_TXID,
          vout: 0,
          height: 9,
          blockhash: 'previous-block',
          digest: 'previous-digest',
        },
      },
      issues: [],
      revisionTxid: OTHER_TXID,
      readHeight: 9,
    };
    const unprovenEmpty: IdentityProfileLoadResult = {
      state: 'empty',
      avatar: null,
      description: null,
      issues: [],
      revisionTxid: null,
      readHeight: 10,
    };
    mocks.getPendingIdentityProfileUpdates.mockResolvedValue([pendingRemoval]);
    mocks.getIdentityProfile.mockResolvedValue(unprovenEmpty);
    let latestState = pendingSessionState(previousProfile);
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(Identity, {
      target,
      props: {
        sessionState: latestState,
        onSessionStateChange: (nextState) => (latestState = nextState),
      },
    });
    try {
      await settle();
      const retained =
        latestState.profilesByAddress[favoriteIdentity.identityAddress.toLowerCase()];
      expect(retained.description?.value).toBe('Previous');
      expect(latestState.pendingProfilesByAddress).toHaveProperty(
        favoriteIdentity.identityAddress.toLowerCase()
      );
      expect(mocks.toastSuccess).not.toHaveBeenCalled();
    } finally {
      await unmount(component);
      target.remove();
    }
  });
});
