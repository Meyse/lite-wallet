// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  IdentityProfileLoadResult,
  LinkedIdentity,
  PendingIdentityProfileUpdate,
  ProvisioningJobRecord,
} from '$lib/types/wallet';

class ResizeObserverStub {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}

vi.stubGlobal('ResizeObserver', ResizeObserverStub);

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

const provisioningJobs: ProvisioningJobRecord[] = Array.from({ length: 6 }, (_, index) => ({
  jobId: `provisioning-${index}`,
  requestType: 'identity_provisioning',
  requestHex: `request-${index}`,
  requestedIdentityAddress: null,
  requestedFqn: `pending-${index + 1}.example@`,
  signingId: `service-${index + 1}@`,
  hasResponseUris: false,
  infoUri: null,
  status: index === 0 ? 'ready' : 'pending',
  createdAt: index + 1,
  error: null,
}));

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
    profilesByAddress: { [favoriteIdentity.identityAddress]: profile },
    pendingProfilesByAddress: {
      [favoriteIdentity.identityAddress]: pendingRemoval,
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
        expect(target.querySelector('header')).toBeNull();
        expect(target.querySelector('[data-identity-search-toolbar]')).toBeNull();
        expect(target.querySelector('input[placeholder="Search linked identities"]')).toBeNull();
        expect(
          target
            .querySelector('[data-identity-linked-scroll] [data-slot="scroll-area-viewport"]')
            ?.classList.contains('pr-1')
        ).toBe(false);
        expect(emptyState.querySelector('h3')?.textContent?.trim()).toBe('Link your VerusID');
        expect(emptyState.parentElement?.classList.contains('-mt-14')).toBe(true);
        expect(
          emptyState.querySelector('[data-testid="wallet-empty-eyebrow-slot"]')
        ).not.toBeNull();
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
  it('keeps provisioning jobs and linked rows in one scroll region below the fixed toolbar', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(Identity, {
      target,
      props: {
        sessionState: {
          ...initialSessionState(),
          provisioningJobs,
        },
      },
    });

    try {
      await settle();
      const scrollHost = target.querySelector('[data-identity-linked-scroll]');
      const viewport = scrollHost?.querySelector<HTMLElement>('[data-slot="scroll-area-viewport"]');
      const search = target.querySelector<HTMLInputElement>(
        'input[placeholder="Search linked identities"]'
      );
      const layout = target.querySelector('[data-identity-layout]');
      expect(scrollHost).not.toBeNull();
      expect(scrollHost?.querySelectorAll('[data-slot="scroll-area-viewport"]')).toHaveLength(1);
      expect(viewport?.contains(search ?? null)).toBe(false);
      expect(layout?.classList.contains('max-w-6xl')).toBe(true);
      expect(layout?.classList.contains('px-5')).toBe(true);
      expect(layout?.classList.contains('pt-5')).toBe(true);
      expect(search?.classList.contains('h-9')).toBe(true);
      expect(search?.classList.contains('focus-visible:ring-[3px]')).toBe(true);
      expect(viewport?.textContent).toContain('Pending provisioning');
      expect(viewport?.textContent).toContain('pending-6.example@');
      expect(viewport?.textContent).toContain('favorite@');
      expect(viewport?.textContent).toContain('other@');
      expect(viewport?.textContent).toContain('Link identity');
      expect(target.querySelector('[data-linked-identity-manage]')?.textContent?.trim()).toBe('');
      expect(
        target.querySelector('[data-linked-identity-manage]')?.getAttribute('aria-label')
      ).toBe('Manage');

      if (!viewport) throw new Error('Missing linked identity viewport');
      Object.defineProperties(viewport, {
        clientHeight: { configurable: true, value: 200 },
        scrollHeight: { configurable: true, value: 800 },
      });
      viewport.scrollTop = 0;
      viewport.dispatchEvent(new Event('scroll'));
      await settle();
      expect(target.querySelector('[data-identity-scroll-fade="top"]')).toBeNull();
      expect(target.querySelector('[data-identity-scroll-fade="bottom"]')).not.toBeNull();

      viewport.scrollTop = 300;
      viewport.dispatchEvent(new Event('scroll'));
      await settle();
      expect(target.querySelector('[data-identity-scroll-fade="top"]')).not.toBeNull();
      expect(target.querySelector('[data-identity-scroll-fade="bottom"]')).not.toBeNull();

      viewport.scrollTop = 600;
      viewport.dispatchEvent(new Event('scroll'));
      await settle();
      expect(target.querySelector('[data-identity-scroll-fade="top"]')).not.toBeNull();
      expect(target.querySelector('[data-identity-scroll-fade="bottom"]')).toBeNull();
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('uses shared search inputs with separate primary Link and Find actions', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(Identity, {
      target,
      props: { sessionState: initialSessionState() },
    });

    try {
      await settle();
      const linkedInput = target.querySelector<HTMLInputElement>(
        'input[placeholder="Search linked identities"]'
      );
      const linkButton = Array.from(target.querySelectorAll<HTMLButtonElement>('button')).find(
        (button) => button.textContent?.trim() === 'Link VerusID'
      );
      const linkAction = target.querySelector('[data-identity-link-action]');
      expect(linkedInput?.classList.contains('h-9')).toBe(true);
      expect(linkedInput?.classList.contains('focus-visible:ring-[3px]')).toBe(true);
      expect(linkAction?.classList.contains('absolute')).toBe(true);
      expect(linkAction?.classList.contains('top-5')).toBe(true);
      expect(linkAction?.classList.contains('right-5')).toBe(true);
      expect(linkAction?.contains(linkButton ?? null)).toBe(true);
      expect(linkButton?.classList.contains('bg-primary')).toBe(true);
      expect(
        target.querySelector('[data-identity-search-toolbar]')?.contains(linkButton ?? null)
      ).toBe(false);

      const lookupTab = Array.from(target.querySelectorAll<HTMLButtonElement>('[role="tab"]')).find(
        (button) => button.textContent?.trim() === 'Find a VerusID'
      );
      lookupTab?.click();
      await settle();

      const lookupInput = target.querySelector<HTMLInputElement>('[data-verusid-lookup-input]');
      const lookupButton = target.querySelector<HTMLButtonElement>(
        '[data-verusid-lookup-toolbar] button[type="submit"]'
      );
      expect(target.querySelector('[data-identity-link-action]')).toBeNull();
      expect(lookupInput?.classList.contains('h-9')).toBe(true);
      expect(lookupInput?.classList.contains('focus-visible:ring-[3px]')).toBe(true);
      expect(lookupButton?.classList.contains('h-9')).toBe(true);
      expect(lookupButton?.classList.contains('bg-primary')).toBe(true);
      expect(lookupButton?.textContent?.trim()).toBe('Find VerusID');
      expect(target.textContent).not.toContain('Verus Mainnet');
      expect(target.textContent).not.toContain('Full VerusID');
      expect(target.textContent).not.toContain('For example');
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('keeps name-only rows stable while optional profile enrichment loads', async () => {
    const profile = deferred<IdentityProfileLoadResult>();
    mocks.getIdentityProfile.mockReturnValue(profile.promise);
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(Identity, {
      target,
      props: { sessionState: initialSessionState() },
    });

    try {
      await settle();
      expect(target.querySelector('[data-linked-identity-profile-skeleton]')).toBeNull();
      expect(target.textContent).not.toContain('Loading profile…');

      profile.resolve({
        state: 'empty',
        avatar: null,
        description: null,
        issues: [],
        readHeight: null,
        revisionTxid: null,
      });
      await settle();
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('draws dividers only between rows within the same visible group', async () => {
    const identities = [{ ...favoriteIdentity, favorite: false }, otherIdentity];
    mocks.getLinkedIdentities.mockResolvedValue(identities);
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(Identity, {
      target,
      props: {
        sessionState: {
          ...initialSessionState(),
          linkedIdentities: identities,
        },
      },
    });

    try {
      await settle();
      const rows = target.querySelectorAll('[data-linked-identity-row]');
      expect(rows).toHaveLength(2);
      expect(rows[0].getAttribute('data-divider')).toBe('between');
      expect(rows[1].getAttribute('data-divider')).toBe('none');

      const search = target.querySelector<HTMLInputElement>(
        'input[placeholder="Search linked identities"]'
      );
      if (!search) throw new Error('Missing linked identity search');
      search.value = 'other@';
      search.dispatchEvent(new Event('input', { bubbles: true }));
      await new Promise((resolve) => setTimeout(resolve, 175));
      await settle();
      const filteredRows = target.querySelectorAll('[data-linked-identity-row]');
      expect(filteredRows).toHaveLength(1);
      expect(filteredRows[0].getAttribute('data-divider')).toBe('none');
    } finally {
      await unmount(component);
      target.remove();
    }
  });

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
        expect(target.querySelector('header h2')).toBeNull();
        expect(target.querySelector('[role="tab"][data-state="active"]')?.textContent?.trim()).toBe(
          'Linked IDs'
        );
        const rows = target.querySelectorAll('[data-linked-identity-row]');
        expect(rows).toHaveLength(2);
        expect(rows[0].getAttribute('data-divider')).toBe('none');
        expect(rows[1].getAttribute('data-divider')).toBe('none');
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
      const retained = latestState.profilesByAddress[favoriteIdentity.identityAddress];
      expect(retained.description?.value).toBe('Previous');
      expect(latestState.pendingProfilesByAddress).toHaveProperty(favoriteIdentity.identityAddress);
      expect(mocks.toastSuccess).not.toHaveBeenCalled();
    } finally {
      await unmount(component);
      target.remove();
    }
  });
});
