import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type {
  IdentityDetails,
  IdentityProfileLoadResult,
  PendingIdentityProfileUpdate,
} from '$lib/types/wallet';

vi.mock('$lib/services/identityLinkService.js', () => ({
  preflightIdentityProfileUpdate: vi.fn(),
}));
vi.mock('$lib/services/identityService.js', () => ({ sendIdentityUpdate: vi.fn() }));
vi.mock('$lib/services/walletService.js', () => ({ getAddresses: vi.fn() }));

import IdentityDetailView from './IdentityDetailView.svelte';

const TXID = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';

const details: IdentityDetails = {
  identityAddress: 'iSduGc7La416e3SfLD17tCe4Qvreg2i6br',
  name: 'Profile test',
  fullyQualifiedName: 'profile.test@',
  status: 'active',
  systemDisplayName: 'Verus Testnet',
  primaryAddresses: ['RTest'],
  ownedByPrimaryAddress: true,
  minimumSignatures: 1,
  tokenizedControl: false,
  revocationAuthority: 'iSduGc7La416e3SfLD17tCe4Qvreg2i6br',
  recoveryAuthority: 'iSduGc7La416e3SfLD17tCe4Qvreg2i6br',
  profileEditable: true,
  warnings: [],
};

const profile: IdentityProfileLoadResult = {
  state: 'ready',
  avatar: null,
  description: {
    value: 'Current public description',
    source: {
      systemId: 'i-system',
      txid: TXID,
      vout: 0,
      height: 1,
      blockhash: 'block',
      digest: 'description-digest',
    },
  },
  issues: [],
  readHeight: 1,
  revisionTxid: TXID,
};

const pendingRemoval: PendingIdentityProfileUpdate = {
  identityAddress: details.identityAddress,
  txid: TXID,
  submittedAt: 1,
  previousProfile: {
    description: 'Current public description',
    descriptionDigest: 'description-digest',
  },
  proposedProfile: {},
};

describe('identity profile pending notice', () => {
  afterEach(() => {
    document.documentElement.classList.remove('dark');
  });

  it.each(['light', 'dark'])(
    'shows removal-specific pending copy and submitted details in %s mode',
    async (theme) => {
      document.documentElement.classList.toggle('dark', theme === 'dark');
      const target = document.createElement('div');
      document.body.append(target);
      const component = mount(IdentityDetailView, {
        target,
        props: { details, profile, pendingProfile: pendingRemoval },
      });
      try {
        flushSync();
        expect(target.textContent).toContain('Profile removal pending');
        expect(target.textContent).toContain(
          'Your current profile remains visible until this transaction is confirmed.'
        );
        expect(target.textContent).toContain('Current public description');
        expect(target.textContent).not.toContain('Profile updated');

        const viewChanges = [...target.querySelectorAll('button')].find((button) =>
          button.textContent?.includes('View submitted changes')
        );
        viewChanges?.click();
        flushSync();
        expect(target.textContent).toContain('Not confirmed');
        expect(target.textContent).toContain(TXID);
      } finally {
        await unmount(component);
        target.remove();
      }
    }
  );

  it('keeps the prepared previous profile visible when the latest read is unavailable', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(IdentityDetailView, {
      target,
      props: { details, profile: null, pendingProfile: pendingRemoval },
    });
    try {
      flushSync();
      expect(target.textContent).toContain('Current public description');
      expect(target.textContent).toContain('Profile removal pending');
    } finally {
      await unmount(component);
      target.remove();
    }
  });
});
