import { afterEach, describe, expect, it } from 'vitest';
import { setContactSession } from '$lib/contacts/session';
import {
  clearProfileDraft,
  emptyProfileDraft,
  loadProfileDraft,
  removeProfileValue,
  saveProfileDraft,
  stageProfileValue,
  undoProfileValue,
} from './profileDrafts';

describe('private profile drafts', () => {
  afterEach(() => setContactSession(null));
  it('survives navigation, isolates identities and clears across wallet/network/lock boundaries', () => {
    const session = { sessionId: 'wallet-session-a', network: 'testnet' as const };
    setContactSession(session);
    const draft = stageProfileValue(null, emptyProfileDraft(), 'header', 'jpeg');
    saveProfileDraft(session, 'identity-a', draft);
    expect(loadProfileDraft(session, 'identity-a')).toEqual(draft);
    expect(loadProfileDraft(session, 'identity-b')).toEqual(emptyProfileDraft());
    const next = { sessionId: 'wallet-session-b', network: 'testnet' as const };
    setContactSession(next);
    saveProfileDraft(session, 'identity-a', draft);
    expect(loadProfileDraft(next, 'identity-a')).toEqual(emptyProfileDraft());
    saveProfileDraft(next, 'identity-a', draft);
    setContactSession({ ...next, network: 'mainnet' });
    expect(loadProfileDraft(next, 'identity-a')).toEqual(emptyProfileDraft());
    setContactSession(null);
    expect(loadProfileDraft(null, 'identity-a')).toEqual(emptyProfileDraft());
  });
  it('distinguishes removal from undo for new, replaced and published images', () => {
    const source = {
      systemId: 'test',
      txid: 'tx',
      vout: 0,
      height: 1,
      blockhash: 'block',
      digest: 'old',
    };
    const profile = {
      state: 'ready' as const,
      issues: [],
      revisionTxid: 'tx',
      avatar: {
        value: { base64: 'old', mimeType: 'image/jpeg', width: 256, height: 256, byteLength: 3 },
        source,
      },
    };
    const fresh = stageProfileValue(null, emptyProfileDraft(), 'avatar', 'new');
    expect(removeProfileValue(null, fresh, 'avatar').avatar).toEqual({ action: 'keep' });
    const replacement = {
      ...stageProfileValue(profile, emptyProfileDraft(), 'avatar', 'new'),
      smallerAvatar: { action: 'set' as const, value: 'smaller', mimeType: 'image/webp' as const },
    };
    const removal = removeProfileValue(profile, replacement, 'avatar');
    expect(removal.avatar).toEqual({ action: 'remove' });
    expect(removal.smallerAvatar).toBeNull();
    expect(undoProfileValue(removal, 'avatar').avatar).toEqual({ action: 'keep' });
    expect(undoProfileValue(replacement, 'avatar').avatar).toEqual({ action: 'keep' });
  });
  it('does not retain mutation aliases and clears only the submitted identity', () => {
    const session = { sessionId: 'wallet-session', network: 'testnet' as const };
    setContactSession(session);
    const draft = stageProfileValue(null, emptyProfileDraft(), 'description', 'Private draft');
    saveProfileDraft(session, 'a', draft);
    saveProfileDraft(session, 'b', draft);
    draft.description = { action: 'remove' };
    expect(loadProfileDraft(session, 'a').description).toEqual({
      action: 'set',
      value: 'Private draft',
    });
    clearProfileDraft(session, 'a');
    expect(loadProfileDraft(session, 'a')).toEqual(emptyProfileDraft());
    expect(loadProfileDraft(session, 'b').description.action).toBe('set');
  });
});
