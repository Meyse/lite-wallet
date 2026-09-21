import { describe, expect, it } from 'vitest';
import type {
  IdentityProfileLoadResult,
  IdentityProfileSnapshot,
  PendingIdentityProfileUpdate,
} from '$lib/types/wallet.js';
import {
  isCompleteProfileRemoval,
  pendingProfileMatches,
  profileUpdateRemovesData,
} from './identityProfileUpdate';

const TXID = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
const OTHER_TXID = 'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff';

function pending(proposedProfile: IdentityProfileSnapshot): PendingIdentityProfileUpdate {
  return {
    identityAddress: 'i-test',
    txid: TXID,
    submittedAt: 1,
    previousProfile: { avatarDigest: 'old-avatar', descriptionDigest: 'old-description' },
    proposedProfile,
  };
}

function profile(overrides: Partial<IdentityProfileLoadResult> = {}): IdentityProfileLoadResult {
  return {
    state: 'empty',
    avatar: null,
    description: null,
    issues: [],
    revisionTxid: TXID,
    readHeight: 10,
    ...overrides,
  };
}

describe('pendingProfileMatches', () => {
  it('requires the confirmed header bytes and digest and checks header removal', () => {
    const header = {
      value: {
        base64: 'header-jpeg',
        mimeType: 'image/jpeg',
        width: 960,
        height: 160,
        byteLength: 8,
      },
      source: {
        systemId: 'i-system',
        txid: TXID,
        vout: 0,
        height: 10,
        blockhash: 'block',
        digest: 'header-digest',
      },
    };
    const proposed = { headerBase64: 'header-jpeg', headerDigest: 'header-digest' };
    expect(pendingProfileMatches(pending(proposed), profile({ state: 'ready', header }))).toBe(
      true
    );
    expect(pendingProfileMatches(pending(proposed), profile())).toBe(false);
    expect(
      pendingProfileMatches(
        pending(proposed),
        profile({
          state: 'ready',
          header: { ...header, value: { ...header.value, base64: 'other' } },
        })
      )
    ).toBe(false);
    expect(
      pendingProfileMatches(
        pending(proposed),
        profile({
          state: 'ready',
          header: { ...header, source: { ...header.source, digest: 'other' } },
        })
      )
    ).toBe(false);
    expect(pendingProfileMatches(pending({}), profile({ state: 'ready', header }))).toBe(false);
    expect(isCompleteProfileRemoval(proposed, {})).toBe(true);
    expect(profileUpdateRemovesData(proposed, {})).toBe(true);
  });

  it('accepts a confirmed empty profile at the submitted revision', () => {
    expect(pendingProfileMatches(pending({}), profile())).toBe(true);
  });

  it.each([
    ['unavailable state', { state: 'unavailable' as const }],
    ['unresolved issues', { issues: [{ field: 'avatar', code: 'invalid' }] }],
    ['missing revision', { revisionTxid: null }],
    ['wrong revision', { revisionTxid: OTHER_TXID }],
    ['an inconsistent ready state', { state: 'ready' as const }],
  ])('rejects %s', (_label, overrides) => {
    expect(pendingProfileMatches(pending({}), profile(overrides))).toBe(false);
  });

  it('rejects a snapshot mismatch even at the submitted revision', () => {
    expect(
      pendingProfileMatches(
        pending({ description: 'New description', descriptionDigest: 'new-description' }),
        profile({
          state: 'ready',
          description: {
            value: 'Different description',
            source: {
              systemId: 'i-system',
              txid: TXID,
              vout: 0,
              height: 10,
              blockhash: 'block',
              digest: 'new-description',
            },
          },
        })
      )
    ).toBe(false);
  });

  it('requires both the prepared value and digest to match for an updated field', () => {
    const proposed = { description: 'New description', descriptionDigest: 'new-description' };
    const matchingDescription = {
      value: 'New description',
      source: {
        systemId: 'i-system',
        txid: TXID,
        vout: 0,
        height: 10,
        blockhash: 'block',
        digest: 'new-description',
      },
    };

    expect(
      pendingProfileMatches(
        pending(proposed),
        profile({ state: 'ready', description: matchingDescription })
      )
    ).toBe(true);
    expect(
      pendingProfileMatches(
        pending(proposed),
        profile({
          state: 'ready',
          description: {
            ...matchingDescription,
            source: { ...matchingDescription.source, digest: 'different-digest' },
          },
        })
      )
    ).toBe(false);
  });
});

describe('profile update classification', () => {
  it('classifies only a non-empty to empty transition as a complete removal', () => {
    expect(isCompleteProfileRemoval({ description: 'Profile' }, {})).toBe(true);
    expect(isCompleteProfileRemoval({}, {})).toBe(false);
    expect(isCompleteProfileRemoval({ description: 'Profile' }, { avatarDigest: 'new' })).toBe(
      false
    );
  });

  it('detects partial and complete field removal', () => {
    expect(
      profileUpdateRemovesData(
        { avatarDigest: 'old', description: 'Profile' },
        { description: 'Profile' }
      )
    ).toBe(true);
    expect(profileUpdateRemovesData({ description: 'Profile' }, { description: 'Updated' })).toBe(
      false
    );
  });
});
