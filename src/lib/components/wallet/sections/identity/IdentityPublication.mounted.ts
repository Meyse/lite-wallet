import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setContactSession } from '$lib/contacts/session';
import type {
  IdentityDetails,
  IdentityProfileLoadResult,
  IdentityProfilePreflightResult,
  ProfilePublicationState,
} from '$lib/types/wallet';
import IdentityDetailView from './IdentityDetailView.svelte';
const mocks = vi.hoisted(() => ({
  read: vi.fn(),
  confirm: vi.fn(),
  review: vi.fn(),
  send: vi.fn(),
}));
vi.mock('$lib/services/identityLinkService', () => ({
  getIdentityProfilePublication: mocks.read,
  confirmIdentityProfileUpdate: mocks.confirm,
  reviewIdentityProfilePublication: mocks.review,
}));
vi.mock('$lib/services/identityService.js', () => ({ sendIdentityUpdate: mocks.send }));
vi.mock('$lib/services/invokeWalletCommand', () => ({ invokeSessionBoundWalletCommand: vi.fn() }));
const identity = 'iSduGc7La416e3SfLD17tCe4Qvreg2i6br';
const details: IdentityDetails = {
  identityAddress: identity,
  fullyQualifiedName: 'player.VRSCTEST@',
  status: 'active',
  primaryAddresses: ['RTest'],
  ownedByPrimaryAddress: true,
  minimumSignatures: 1,
  tokenizedControl: false,
  profileEditable: true,
  warnings: [],
};
const empty: IdentityProfileLoadResult = {
  state: 'empty',
  issues: [],
  revisionTxid: 'b'.repeat(64),
};
const receipt = {
  identityAddress: identity,
  txid: 'a'.repeat(64),
  submittedAt: 1,
  previousProfile: {},
  proposedProfile: { description: 'The confirmed result' },
};
const canonical: IdentityProfileLoadResult = {
  state: 'ready',
  issues: [],
  revisionTxid: receipt.txid,
  description: {
    value: 'The confirmed result',
    source: {
      systemId: 'test',
      txid: receipt.txid,
      height: 1,
      blockhash: 'block',
      vout: 0,
      digest: 'digest',
    },
  },
};
function plan(step: 1 | 2, totalSteps: 1 | 2): ProfilePublicationState {
  return {
    planId: 'plan',
    identityAddress: identity,
    status: 'ready',
    step,
    totalSteps,
    request: {
      coinId: 'VRSCTEST',
      channelId: 'test',
      identityAddress: identity,
      avatar: { action: 'keep' },
      description: { action: 'set', value: 'The confirmed result' },
    },
    pending: null,
    settledTxids: [],
  };
}
function prepared(step: 1 | 2, totalSteps: 1 | 2): IdentityProfilePreflightResult {
  return {
    preflightId: 'fresh',
    expiresAt: Date.now() / 1000 + 300,
    currentProfile: {},
    proposedProfile: receipt.proposedProfile,
    feeSats: '1',
    feeDisplay: '0.00000001',
    fundingSummary: 'RTest',
    evidenceBytes: 1,
    changedFields: ['description'],
    publication: {
      planId: 'plan',
      step,
      totalSteps,
      nextFeeSats: '1',
      estimatedTotalFeeSats: '2',
      earlierFeeSats: null,
      quoteHeight: 1,
      quoteTime: 1,
      availableSats: '100',
      proposedProfile: receipt.proposedProfile,
      changedFields: ['description'],
      evidenceGroups: [],
      optimization: null,
    },
  };
}
async function settle() {
  for (let i = 0; i < 10; i++) {
    await tick();
    flushSync();
  }
}
function click(target: HTMLElement, text: string) {
  const b = [...target.querySelectorAll('button')].find((b) => b.textContent?.trim() === text);
  if (!b) throw Error(`Missing ${text}: ${target.textContent}`);
  b.click();
  flushSync();
}
async function harness(step: 1 | 2, totalSteps: 1 | 2) {
  mocks.read.mockResolvedValue(plan(step, totalSteps));
  mocks.review.mockResolvedValue(prepared(step, totalSteps));
  mocks.send.mockResolvedValue({ txid: receipt.txid, profileUpdate: receipt });
  mocks.confirm.mockResolvedValue(canonical);
  const target = document.createElement('div');
  document.body.append(target);
  const confirmed = vi.fn();
  const submitted = vi.fn();
  const settled = vi.fn();
  const component = mount(IdentityDetailView, {
    target,
    props: {
      details,
      profile: empty,
      onProfileConfirmed: confirmed,
      onProfileSubmitted: submitted,
      onProfileSettled: settled,
    },
  });
  await settle();
  return {
    target,
    confirmed,
    submitted,
    settled,
    close: async () => {
      await unmount(component);
      target.remove();
    },
  };
}
beforeEach(() => {
  vi.useFakeTimers();
  setContactSession({ sessionId: 'publication-composition', network: 'testnet' });
});
afterEach(() => {
  setContactSession(null);
  vi.useRealTimers();
  vi.resetAllMocks();
});
describe('real detail/editor publication composition', () => {
  it.each([
    [1, 1],
    [1, 2],
    [2, 2],
  ] as const)(
    'keeps step %i/%i pending when an old ready poll resolves after send',
    async (step, total) => {
      const h = await harness(step, total);
      try {
        click(h.target, step === 2 ? 'Review header fee' : 'Continue profile update');
        await settle();
        click(h.target, step === 2 ? 'Review header fee' : 'Review fee');
        await settle();
        let finish!: (state: ProfilePublicationState) => void;
        mocks.read
          .mockReturnValueOnce(new Promise((r) => (finish = r)))
          .mockResolvedValue({ ...plan(step, total), status: 'waiting', pending: receipt });
        await vi.advanceTimersByTimeAsync(10000);
        click(h.target, step === 2 ? 'Publish header' : 'Publish description');
        await settle();
        expect(h.submitted).toHaveBeenCalledOnce();
        expect(h.target.textContent).not.toContain('Discard remaining changes');
        expect(h.target.textContent).not.toContain('Review fee');
        finish(plan(step, total));
        await settle();
        expect(h.target.textContent).toContain('Transaction submitted');
        expect(h.target.textContent).not.toContain('Discard remaining changes');
        expect(mocks.send).toHaveBeenCalledOnce();
      } finally {
        await h.close();
      }
    }
  );
  it.each(['complete', 'consumed'])(
    'finishes automatically with %s evidence, once, and never reopens on null',
    async (kind) => {
      const h = await harness(1, 1);
      try {
        click(h.target, 'Continue profile update');
        await settle();
        click(h.target, 'Review fee');
        await settle();
        mocks.read.mockResolvedValue({ ...plan(1, 1), status: 'waiting', pending: receipt });
        click(h.target, 'Publish description');
        await settle();
        mocks.read.mockResolvedValue(
          kind === 'complete'
            ? { ...plan(1, 1), status: 'complete', settledTxids: [receipt.txid] }
            : null
        );
        await vi.advanceTimersByTimeAsync(10000);
        await settle();
        expect(h.target.querySelector('[data-profile-editor]')).toBeNull();
        expect(h.target.textContent).toContain('The confirmed result');
        expect(h.target.textContent).toContain('Profile updated');
        expect(h.confirmed).toHaveBeenCalledOnce();
        expect(h.settled).toHaveBeenCalledOnce();
        await vi.advanceTimersByTimeAsync(5000);
        await settle();
        expect(h.target.textContent).not.toContain('Profile updated');
        mocks.read.mockResolvedValue(null);
        await vi.advanceTimersByTimeAsync(10000);
        expect(h.confirmed).toHaveBeenCalledOnce();
        expect(h.target.querySelector('[data-profile-editor]')).toBeNull();
      } finally {
        await h.close();
      }
    }
  );
  it('keeps the pending view when another observer consumed Complete but evidence is unavailable', async () => {
    const h = await harness(1, 1);
    try {
      click(h.target, 'Continue profile update');
      await settle();
      click(h.target, 'Review fee');
      await settle();
      mocks.read.mockResolvedValue({ ...plan(1, 1), status: 'waiting', pending: receipt });
      click(h.target, 'Publish description');
      await settle();
      mocks.read.mockResolvedValue(null);
      mocks.confirm.mockResolvedValue(null);
      await vi.advanceTimersByTimeAsync(10000);
      await settle();
      expect(h.target.querySelector('[data-profile-editor]')).not.toBeNull();
      expect(h.confirmed).not.toHaveBeenCalled();
    } finally {
      await h.close();
    }
  });

  it('queues a reconnect refresh behind an active poll', async () => {
    const h = await harness(1, 1);
    try {
      click(h.target, 'Continue profile update');
      await settle();
      click(h.target, 'Review fee');
      await settle();
      mocks.read.mockResolvedValue({ ...plan(1, 1), status: 'waiting', pending: receipt });
      click(h.target, 'Publish description');
      await settle();
      let finish!: (state: ProfilePublicationState) => void;
      mocks.read.mockReturnValueOnce(new Promise((r) => (finish = r))).mockResolvedValue(null);
      const count = mocks.read.mock.calls.length;
      await vi.advanceTimersByTimeAsync(10000);
      window.dispatchEvent(new Event('online'));
      finish({ ...plan(1, 1), status: 'waiting', pending: receipt });
      await settle();
      expect(mocks.read).toHaveBeenCalledTimes(count + 2);
      expect(h.confirmed).toHaveBeenCalledOnce();
    } finally {
      await h.close();
    }
  });

  it.each(['session', 'unmount'])('ignores late completion after %s changes', async (kind) => {
    const h = await harness(1, 1);
    let closed = false;
    try {
      click(h.target, 'Continue profile update');
      await settle();
      click(h.target, 'Review fee');
      await settle();
      mocks.read.mockResolvedValue({ ...plan(1, 1), status: 'waiting', pending: receipt });
      click(h.target, 'Publish description');
      await settle();
      let finish!: (state: ProfilePublicationState | null) => void;
      mocks.read.mockReturnValueOnce(new Promise((r) => (finish = r)));
      await vi.advanceTimersByTimeAsync(10000);
      if (kind === 'session') {
        setContactSession({ sessionId: 'other-wallet', network: 'testnet' });
        mocks.read.mockResolvedValue(null);
        await settle();
      } else {
        await h.close();
        closed = true;
      }
      finish({ ...plan(1, 1), status: 'complete', settledTxids: [receipt.txid] });
      await settle();
      expect(h.confirmed).not.toHaveBeenCalled();
      expect(h.settled).not.toHaveBeenCalled();
    } finally {
      if (!closed) await h.close();
    }
  });

  it('retains a pending view after an ambiguous send resolves to backend waiting', async () => {
    const h = await harness(1, 1);
    try {
      click(h.target, 'Continue profile update');
      await settle();
      click(h.target, 'Review fee');
      await settle();
      mocks.send.mockRejectedValue({ type: 'NetworkError' });
      mocks.read.mockResolvedValue({ ...plan(1, 1), status: 'waiting', pending: receipt });
      click(h.target, 'Publish description');
      await settle();
      expect(h.target.textContent).toContain('Transaction submitted');
      expect(h.target.textContent).not.toContain('Discard remaining changes');
      expect(h.target.textContent).not.toContain('Review fee');
      expect(h.confirmed).not.toHaveBeenCalled();
    } finally {
      await h.close();
    }
  });

  it('returns to the last confirmed profile with a submitted notice and automatic readback', async () => {
    const h = await harness(1, 1);
    try {
      click(h.target, 'Continue profile update');
      await settle();
      click(h.target, 'Review fee');
      await settle();
      mocks.read.mockResolvedValue({ ...plan(1, 1), status: 'waiting', pending: receipt });
      click(h.target, 'Publish description');
      await settle();
      expect(h.target.textContent).toContain('Transaction submitted');
      expect(h.target.textContent).not.toContain('Check confirmation');
      click(h.target, 'Back to profile');
      await settle();
      expect(h.target.querySelector('[data-profile-editor]')).toBeNull();
      expect(h.target.textContent).toContain('Description update submitted');
      expect(h.target.textContent).not.toContain('The confirmed result');
      mocks.read.mockResolvedValue(null);
      await vi.advanceTimersByTimeAsync(10000);
      await settle();
      expect(h.target.textContent).toContain('The confirmed result');
      expect(h.target.textContent).not.toContain('Description update submitted');
    } finally {
      await h.close();
    }
  });

  it('keeps an uncertain send on the profile without a duplicate publish action', async () => {
    const h = await harness(1, 1);
    try {
      click(h.target, 'Continue profile update');
      await settle();
      click(h.target, 'Review fee');
      await settle();
      mocks.send.mockRejectedValue({ type: 'NetworkError' });
      mocks.read.mockRejectedValue(new Error('offline'));
      click(h.target, 'Publish description');
      await settle();
      expect(h.target.textContent).toContain('Submission not yet verified');
      expect(h.target.textContent).toContain('New description');
      expect(h.target.textContent).not.toContain('Submitted description');
      click(h.target, 'Back to profile');
      await settle();
      expect(h.target.textContent).toContain('Submission not yet verified');
      expect(h.target.textContent).not.toContain('Publish description');
      expect(mocks.send).toHaveBeenCalledOnce();
    } finally {
      await h.close();
    }
  });

  it('does not offer header review again while its submission is uncertain', async () => {
    const h = await harness(2, 2);
    try {
      click(h.target, 'Review header fee');
      await settle();
      click(h.target, 'Review header fee');
      await settle();
      mocks.send.mockRejectedValue({ type: 'NetworkError' });
      mocks.read.mockRejectedValue(new Error('offline'));
      click(h.target, 'Publish header');
      await settle();
      expect(h.target.textContent).toContain('Submission not yet verified');
      click(h.target, 'Back to profile');
      await settle();
      expect(h.target.textContent).toContain('Submission not yet verified');
      expect(h.target.textContent).not.toContain('Review header fee');
      expect(h.target.textContent).not.toContain('Discard unpublished header');
      expect(mocks.send).toHaveBeenCalledOnce();
    } finally {
      await h.close();
    }
  });

  it('checks at 10 seconds, backs off after connection failures, and checks on reconnect', async () => {
    const h = await harness(1, 1);
    try {
      const initialReads = mocks.read.mock.calls.length;
      mocks.read.mockRejectedValue(new Error('offline'));
      await vi.advanceTimersByTimeAsync(10000);
      await settle();
      expect(mocks.read).toHaveBeenCalledTimes(initialReads + 1);
      await vi.advanceTimersByTimeAsync(10000);
      expect(mocks.read).toHaveBeenCalledTimes(initialReads + 1);
      await vi.advanceTimersByTimeAsync(10000);
      await settle();
      expect(mocks.read).toHaveBeenCalledTimes(initialReads + 2);
      window.dispatchEvent(new Event('online'));
      await settle();
      expect(mocks.read).toHaveBeenCalledTimes(initialReads + 3);
    } finally {
      await h.close();
    }
  });

  it('requires a fresh header review and a second explicit publish', async () => {
    const h = await harness(1, 2);
    try {
      click(h.target, 'Continue profile update');
      await settle();
      click(h.target, 'Review fee');
      await settle();
      mocks.read.mockResolvedValue({ ...plan(1, 2), status: 'waiting', pending: receipt });
      click(h.target, 'Publish description');
      await settle();
      click(h.target, 'Back to profile');
      await settle();
      mocks.read.mockResolvedValue({ ...plan(2, 2), firstReceipt: receipt });
      mocks.review.mockResolvedValue(prepared(2, 2));
      await vi.advanceTimersByTimeAsync(10000);
      await settle();
      expect(h.target.textContent).toContain('Header image ready to publish');
      expect(mocks.send).toHaveBeenCalledOnce();
      click(h.target, 'Review header fee');
      await settle();
      click(h.target, 'Review header fee');
      await settle();
      expect(mocks.review).toHaveBeenCalledTimes(2);
      expect(mocks.review).toHaveBeenLastCalledWith(identity, 'plan', undefined);
      expect(mocks.send).toHaveBeenCalledOnce();
      expect(h.target.textContent).toContain('Publish header');
    } finally {
      await h.close();
    }
  });
});
