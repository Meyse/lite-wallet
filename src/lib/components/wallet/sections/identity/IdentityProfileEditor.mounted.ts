import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type {
  IdentityDetails,
  IdentityProfileLoadResult,
  IdentityProfilePreflightResult,
} from '$lib/types/wallet';
import IdentityProfileEditor from './IdentityProfileEditor.svelte';

const mocks = vi.hoisted(() => ({
  preflight: vi.fn(),
  send: vi.fn(),
  addresses: vi.fn(),
}));

vi.mock('$lib/services/identityLinkService.js', () => ({
  preflightIdentityProfileUpdate: mocks.preflight,
}));
vi.mock('$lib/services/identityService.js', () => ({ sendIdentityUpdate: mocks.send }));
vi.mock('$lib/services/walletService.js', () => ({ getAddresses: mocks.addresses }));

const details: IdentityDetails = {
  identityAddress: 'iSduGc7La416e3SfLD17tCe4Qvreg2i6br',
  name: 'Profile test',
  status: 'active',
  primaryAddresses: ['RTest'],
  ownedByPrimaryAddress: true,
  minimumSignatures: 1,
  tokenizedControl: false,
  profileEditable: true,
  warnings: [],
};

const TXID = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';

function loadedProfile({
  avatar = false,
  description = false,
  state = 'ready',
}: {
  avatar?: boolean;
  description?: boolean;
  state?: IdentityProfileLoadResult['state'];
}): IdentityProfileLoadResult {
  return {
    state,
    avatar: avatar
      ? {
          value: {
            base64: 'AQ==',
            mimeType: 'image/jpeg',
            width: 256,
            height: 256,
            byteLength: 1,
          },
          source: {
            systemId: 'i-system',
            txid: TXID,
            vout: 0,
            height: 1,
            blockhash: 'block',
            digest: 'avatar-digest',
          },
        }
      : null,
    description: description
      ? {
          value: 'Existing description',
          source: {
            systemId: 'i-system',
            txid: TXID,
            vout: 0,
            height: 1,
            blockhash: 'block',
            digest: 'description-digest',
          },
        }
      : null,
    issues: [],
    revisionTxid: TXID,
    readHeight: 1,
  };
}

function removalPreflight(): IdentityProfilePreflightResult {
  return {
    preflightId: 'remove-profile',
    expiresAt: 2_000_000_000,
    currentProfile: {
      avatarBase64: 'AQ==',
      avatarDigest: 'avatar-digest',
      description: 'Existing description',
      descriptionDigest: 'description-digest',
    },
    proposedProfile: {},
    feeSats: '10000',
    feeDisplay: '0.00010000',
    fundingSummary: 'RTest',
    evidenceBytes: 100,
    changedFields: ['avatar', 'description'],
  };
}

async function settle(): Promise<void> {
  for (let index = 0; index < 4; index += 1) await Promise.resolve();
  await tick();
}

describe('profile publication review', () => {
  afterEach(() => {
    vi.clearAllMocks();
    document.documentElement.classList.remove('dark');
  });

  it.each(['light', 'dark'])(
    'reviews the prepared description when the draft changes during preflight (%s)',
    async (theme) => {
      document.documentElement.classList.toggle('dark', theme === 'dark');
      mocks.addresses.mockResolvedValue({ vrsc_address: 'RTest' });
      let finishPreflight!: (result: IdentityProfilePreflightResult) => void;
      mocks.preflight.mockImplementation(
        () => new Promise<IdentityProfilePreflightResult>((resolve) => (finishPreflight = resolve))
      );
      const target = document.createElement('div');
      document.body.append(target);
      const component = mount(IdentityProfileEditor, {
        target,
        props: { details, profile: { state: 'empty', issues: [], revisionTxid: null } },
      });
      try {
        flushSync();
        const description = target.querySelector('textarea');
        if (!description) throw new Error('Description input was not rendered');
        description.value = 'Prepared description';
        description.dispatchEvent(new Event('input', { bubbles: true }));
        flushSync();
        const review = [...target.querySelectorAll('button')].find((button) =>
          button.textContent?.includes('Review publication')
        );
        if (!review) throw new Error('Review action was not rendered');
        review.click();
        await vi.waitFor(() => expect(mocks.preflight).toHaveBeenCalledOnce());

        description.value = 'Later draft that was never prepared';
        description.dispatchEvent(new Event('input', { bubbles: true }));
        flushSync();
        finishPreflight({
          preflightId: 'prepared-profile',
          expiresAt: 2_000_000_000,
          currentProfile: {},
          proposedProfile: { description: 'Prepared description', descriptionDigest: 'digest' },
          feeSats: '10000',
          feeDisplay: '0.00010000',
          fundingSummary: 'RTest',
          evidenceBytes: 100,
          changedFields: ['description'],
        });
        await vi.waitFor(() => {
          flushSync();
          expect(target.textContent).toContain('Review profile publication');
        });
        expect(target.textContent).toContain('Prepared description');
        expect(target.textContent).not.toContain('Later draft that was never prepared');
        expect(mocks.send).not.toHaveBeenCalled();
      } finally {
        await unmount(component);
        target.remove();
      }
    }
  );

  it.each([
    ['avatar only', loadedProfile({ avatar: true }), true],
    ['description only', loadedProfile({ description: true }), true],
    ['avatar and description', loadedProfile({ avatar: true, description: true }), true],
    ['empty', loadedProfile({ state: 'empty' }), false],
    ['unavailable', loadedProfile({ avatar: true, state: 'unavailable' }), false],
    [
      'issue-bearing',
      {
        ...loadedProfile({ description: true }),
        issues: [{ field: 'avatar', code: 'invalid_or_unavailable' }],
      },
      false,
    ],
  ])('shows the combined removal action for %s profiles only', async (_label, profile, visible) => {
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(IdentityProfileEditor, { target, props: { details, profile } });
    try {
      flushSync();
      const action = [...target.querySelectorAll('button')].find((button) =>
        button.textContent?.includes('Remove profile data…')
      );
      expect(Boolean(action)).toBe(visible);
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it.each(['light', 'dark'])(
    'prepares and submits a complete removal in %s mode',
    async (theme) => {
      document.documentElement.classList.toggle('dark', theme === 'dark');
      mocks.addresses.mockResolvedValue({ vrsc_address: 'RTest' });
      mocks.preflight.mockResolvedValue(removalPreflight());
      let finishSend!: (value: { txid: string }) => void;
      mocks.send.mockImplementation(
        () => new Promise<{ txid: string }>((resolve) => (finishSend = resolve))
      );
      const submitted = vi.fn();
      const target = document.createElement('div');
      document.body.append(target);
      const component = mount(IdentityProfileEditor, {
        target,
        props: {
          details,
          profile: loadedProfile({ avatar: true, description: true }),
          onSubmitted: submitted,
        },
      });

      try {
        flushSync();
        const remove = [...target.querySelectorAll('button')].find((button) =>
          button.textContent?.includes('Remove profile data…')
        );
        remove?.click();
        await vi.waitFor(() => expect(mocks.preflight).toHaveBeenCalledOnce());
        expect(mocks.preflight).toHaveBeenCalledWith({
          coinId: 'VRSCTEST',
          channelId: 'vrpc.RTest.iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq',
          identityAddress: details.identityAddress,
          avatar: { action: 'remove' },
          description: { action: 'remove' },
        });
        await vi.waitFor(() => expect(target.textContent).toContain('Review profile removal'));
        expect(target.textContent).toContain('Check the removal and network fee before signing.');
        expect(target.textContent).toContain('The VerusID remains active');
        expect(target.textContent).toContain('Publish removal');

        const publish = [...target.querySelectorAll('button')].find((button) =>
          button.textContent?.includes('Publish removal')
        );
        publish?.click();
        await settle();
        expect(target.textContent).toContain('Publishing profile removal…');

        finishSend({ txid: TXID });
        await vi.waitFor(() => expect(target.textContent).toContain('Profile removal submitted'));
        expect(target.textContent).toContain('The transaction was submitted but is not confirmed.');
        expect(submitted).toHaveBeenCalledWith(
          expect.objectContaining({ txid: TXID, proposedProfile: {} })
        );
      } finally {
        await unmount(component);
        target.remove();
      }
    }
  );

  it.each(['light', 'dark'])(
    'preserves the editor draft when returning from a complete removal review (%s)',
    async (theme) => {
      document.documentElement.classList.toggle('dark', theme === 'dark');
      mocks.addresses.mockResolvedValue({ vrsc_address: 'RTest' });
      mocks.preflight.mockResolvedValue(removalPreflight());
      const target = document.createElement('div');
      document.body.append(target);
      const component = mount(IdentityProfileEditor, {
        target,
        props: { details, profile: loadedProfile({ avatar: true, description: true }) },
      });

      try {
        flushSync();
        const description = target.querySelector('textarea');
        if (!description) throw new Error('Description input was not rendered');
        description.value = 'Unpublished description draft';
        description.dispatchEvent(new Event('input', { bubbles: true }));
        flushSync();

        expect(target.querySelector('img')?.getAttribute('src')).toBe(
          'data:image/jpeg;base64,AQ=='
        );
        const removal = [...target.querySelectorAll('button')].find((button) =>
          button.textContent?.includes('Remove profile data…')
        );
        removal?.click();
        await vi.waitFor(() => expect(target.textContent).toContain('Review profile removal'));

        const back = [...target.querySelectorAll('button')].find((button) =>
          button.textContent?.includes('Back to editing')
        );
        back?.click();
        await vi.waitFor(() => expect(target.textContent).toContain('Edit profile'));

        expect((target.querySelector('textarea') as HTMLTextAreaElement | null)?.value).toBe(
          'Unpublished description draft'
        );
        expect(target.querySelector('img')?.getAttribute('src')).toBe(
          'data:image/jpeg;base64,AQ=='
        );
        expect(target.textContent).toContain('Replace image');
      } finally {
        await unmount(component);
        target.remove();
      }
    }
  );

  it('asks the backend to remove both supported fields from a description-only snapshot', async () => {
    mocks.addresses.mockResolvedValue({ vrsc_address: 'RTest' });
    mocks.preflight.mockResolvedValue({
      ...removalPreflight(),
      currentProfile: {
        description: 'Existing description',
        descriptionDigest: 'description-digest',
      },
      changedFields: ['description'],
    });
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(IdentityProfileEditor, {
      target,
      props: { details, profile: loadedProfile({ description: true }) },
    });
    try {
      flushSync();
      const removal = [...target.querySelectorAll('button')].find((button) =>
        button.textContent?.includes('Remove profile data…')
      );
      removal?.click();
      await vi.waitFor(() => expect(mocks.preflight).toHaveBeenCalledOnce());
      expect(mocks.preflight).toHaveBeenCalledWith(
        expect.objectContaining({
          avatar: { action: 'remove' },
          description: { action: 'remove' },
        })
      );
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('keeps ordinary update copy and adds history disclosure for a partial removal', async () => {
    mocks.addresses.mockResolvedValue({ vrsc_address: 'RTest' });
    mocks.preflight.mockResolvedValue({
      ...removalPreflight(),
      preflightId: 'remove-avatar',
      proposedProfile: {
        description: 'Existing description',
        descriptionDigest: 'description-digest',
      },
      changedFields: ['avatar'],
    });
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(IdentityProfileEditor, {
      target,
      props: { details, profile: loadedProfile({ avatar: true, description: true }) },
    });
    try {
      flushSync();
      const removeAvatar = [...target.querySelectorAll('button')].find(
        (button) => button.textContent?.trim() === 'Remove'
      );
      removeAvatar?.click();
      flushSync();
      const review = [...target.querySelectorAll('button')].find((button) =>
        button.textContent?.includes('Review publication')
      );
      review?.click();
      await vi.waitFor(() => expect(target.textContent).toContain('Review profile publication'));
      expect(target.textContent).toContain('Removed data remains public in blockchain history.');
      expect(target.textContent).not.toContain('Review profile removal');
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('returns safely from preflight and send failures without claiming removal', async () => {
    mocks.addresses.mockResolvedValue({ vrsc_address: 'RTest' });
    mocks.preflight.mockRejectedValueOnce({ type: 'NetworkError' });
    const submitted = vi.fn();
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(IdentityProfileEditor, {
      target,
      props: {
        details,
        profile: loadedProfile({ avatar: true, description: true }),
        onSubmitted: submitted,
      },
    });
    try {
      flushSync();
      let removal = [...target.querySelectorAll('button')].find((button) =>
        button.textContent?.includes('Remove profile data…')
      );
      removal?.click();
      await vi.waitFor(() => expect(target.textContent).toContain('Network error'));
      expect(target.textContent).not.toContain('Profile removal submitted');

      mocks.preflight.mockResolvedValueOnce(removalPreflight());
      removal = [...target.querySelectorAll('button')].find((button) =>
        button.textContent?.includes('Remove profile data…')
      );
      removal?.click();
      await vi.waitFor(() => expect(target.textContent).toContain('Review profile removal'));
      mocks.send.mockRejectedValueOnce({ type: 'NetworkError' });
      const publish = [...target.querySelectorAll('button')].find((button) =>
        button.textContent?.includes('Publish removal')
      );
      publish?.click();
      await vi.waitFor(() => expect(target.textContent).toContain('Review profile removal'));
      expect(target.textContent).not.toContain('Profile removal submitted');
      expect(submitted).not.toHaveBeenCalled();
    } finally {
      await unmount(component);
      target.remove();
    }
  });
});
