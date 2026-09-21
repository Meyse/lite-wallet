import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setContactSession } from '$lib/contacts/session';
import { emptyProfileDraft, loadProfileDraft, saveProfileDraft } from '$lib/identity/profileDrafts';
import { ratesStore } from '$lib/stores/rates';
import { settingsStore } from '$lib/stores/settings';
import type {
  IdentityDetails,
  IdentityProfileLoadResult,
  IdentityProfilePreflightResult,
  ProfilePublicationState,
} from '$lib/types/wallet';
import { ProfilePublicationController } from '$lib/identity/profilePublication';
import IdentityProfileEditor from './IdentityProfileEditor.svelte';

const mocks = vi.hoisted(() => ({
  preflight: vi.fn(),
  send: vi.fn(),
  addresses: vi.fn(),
  resume: vi.fn(),
  discard: vi.fn(),
  read: vi.fn(),
  decode: vi.fn(),
  encode: vi.fn(),
}));
vi.mock('$lib/services/identityLinkService.js', () => ({
  preflightIdentityProfileUpdate: mocks.preflight,
  reviewIdentityProfilePublication: mocks.resume,
  discardIdentityProfilePublication: mocks.discard,
}));
vi.mock('$lib/identity/profileImages', async (original) => ({
  ...(await original<object>()),
  loadProfileImage: mocks.decode,
  encodeProfileImage: mocks.encode,
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
const empty: IdentityProfileLoadResult = { state: 'empty', issues: [], revisionTxid: null };
const session = { sessionId: 'profile-test-session', network: 'testnet' as const };
const source = {
  systemId: 'testnet',
  txid: 'a'.repeat(64),
  vout: 0,
  height: 1,
  blockhash: 'b'.repeat(64),
  digest: 'old',
};
const published: IdentityProfileLoadResult = {
  ...empty,
  state: 'ready',
  description: { value: 'Published description', source },
};
function result(
  proposedProfile: IdentityProfilePreflightResult['proposedProfile'],
  fields = ['description']
): IdentityProfilePreflightResult {
  return {
    preflightId: 'prepared-id',
    expiresAt: Date.now() / 1000 + 60,
    currentProfile: {},
    proposedProfile,
    feeSats: '1230000',
    feeDisplay: '0.0123',
    fundingSummary: 'RTest',
    evidenceBytes: 100,
    changedFields: fields,
    publication: {
      planId: 'plan',
      step: 1,
      totalSteps: 1,
      nextFeeSats: null,
      estimatedTotalFeeSats: '1230000',
      earlierFeeSats: null,
      quoteHeight: 1,
      quoteTime: 1,
      availableSats: '100000000',
      proposedProfile,
      changedFields: fields,
      evidenceGroups: [],
      optimization: null,
    },
  };
}
function button(target: HTMLElement, text: string): HTMLButtonElement {
  const found = [...target.querySelectorAll('button')].find(
    (b) => b.textContent?.trim() === text || b.getAttribute('aria-label') === text
  );
  if (!found) throw new Error(`Missing button: ${text}`);
  return found;
}
async function click(target: HTMLElement, text: string) {
  button(target, text).click();
  flushSync();
  await tick();
}
function type(target: HTMLElement, value: string) {
  const textarea = target.querySelector('textarea');
  if (!textarea) throw new Error('Description editor unavailable');
  textarea.value = value;
  textarea.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
}
async function harness(profile = empty, continuation: ProfilePublicationState | null = null) {
  const target = document.createElement('div');
  document.body.append(target);
  const submitted = vi.fn();
  const cancel = vi.fn();
  mocks.read.mockResolvedValue(continuation);
  const owner = new ProfilePublicationController(details.identityAddress, {
    current: () => true,
    read: mocks.read,
    confirm: async () => null,
    submitted,
    settled: () => {},
    confirmed: () => {},
  });
  await owner.refresh();
  const component = mount(IdentityProfileEditor, {
    target,
    props: { details, profile, publication: owner, onCancel: cancel },
  });
  flushSync();
  return {
    target,
    submitted,
    cancel,
    owner,
    cleanup: async () => {
      owner.dispose();
      await unmount(component);
      target.remove();
    },
  };
}
function continuation(
  status: ProfilePublicationState['status'] = 'ready'
): ProfilePublicationState {
  return {
    planId: 'saved-plan',
    identityAddress: details.identityAddress,
    status,
    step: 2,
    totalSteps: 2,
    request: {
      coinId: 'VRSCTEST',
      channelId: 'test',
      identityAddress: details.identityAddress,
      avatar: { action: 'keep' },
      header: { action: 'set', value: 'AQ==', mimeType: 'image/webp' },
      description: { action: 'keep' },
    },
    pending: null,
    settledTxids: [],
  };
}
function required<T>(value: T | null | undefined): T {
  if (value == null) throw Error('Expected element or fixture value');
  return value;
}
async function menu(h: { target: HTMLElement }, text: string) {
  const trigger = required(
    h.target.querySelector<HTMLButtonElement>('[data-slot="dropdown-menu-trigger"]')
  );
  trigger.click();
  await vi.waitFor(() => expect(document.body.querySelector('[role=menuitem]')).not.toBeNull());
  flushSync();
  const item = [...document.body.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(
    (el) => el.textContent?.trim() === text
  );
  if (!item) throw Error(`Missing menu ${text}`);
  item.click();
  await tick();
  flushSync();
}
async function selectFile(
  h: { target: HTMLElement },
  file = new File(['image'], 'test.png', { type: 'image/png' })
) {
  const input = required(h.target.querySelector<HTMLInputElement>('input[type="file"]'));
  Object.defineProperty(input, 'files', { configurable: true, writable: true, value: [file] });
  input.dispatchEvent(new Event('change', { bubbles: true }));
  await tick();
  flushSync();
}
beforeEach(() => {
  setContactSession(null);
  setContactSession(session);
  mocks.addresses.mockResolvedValue({ vrsc_address: 'RTest' });
  ratesStore.set({});
  settingsStore.update((s) => ({ ...s, displayCurrency: 'EUR' }));
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({
    fillStyle: '',
    fillRect: vi.fn(),
    drawImage: vi.fn(),
  } as unknown as CanvasRenderingContext2D);
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.resetAllMocks();
  setContactSession(null);
  document.documentElement.classList.remove('dark');
});
describe('visual profile draft and explicit approvals', () => {
  it.each(['light', 'dark'])(
    'edits description inline with one draft status and no inactive sections (%s)',
    async (theme) => {
      document.documentElement.classList.toggle('dark', theme === 'dark');
      const h = await harness();
      try {
        expect(h.target.textContent).not.toContain('Connections');
        expect(h.target.querySelector('canvas')).toBeNull();
        expect(button(h.target, 'Review changes').disabled).toBe(true);
        type(h.target, 'A new introduction');
        expect(h.target.querySelector('textarea')?.value).toBe('A new introduction');
        expect(h.target.textContent?.match(/Unpublished changes/g)).toHaveLength(1);
        expect(mocks.preflight).not.toHaveBeenCalled();
        mocks.preflight.mockResolvedValue(result({ description: 'A new introduction' }));
        await click(h.target, 'Review changes');
        await vi.waitFor(() =>
          expect(h.target.querySelector('[data-publication-costs]')).not.toBeNull()
        );
        expect(mocks.preflight).toHaveBeenCalledWith(
          expect.objectContaining({
            avatar: { action: 'keep' },
            header: { action: 'keep' },
            description: { action: 'set', value: 'A new introduction' },
          })
        );
        expect(h.target.querySelector('[data-profile-preview]')?.textContent).toContain(
          'A new introduction'
        );
        expect(h.target.querySelector('[aria-label="Fiat estimate unavailable"]')).toBeNull();
        expect(h.target.textContent?.match(/These changes are public/g)).toHaveLength(1);
        await click(h.target, 'Back to edit profile');
        expect(h.target.querySelector('textarea')?.value).toBe('A new introduction');
      } finally {
        await h.cleanup();
      }
    }
  );
  it('retains invalid and raw description text across navigation, disables review and supports undo', async () => {
    let h = await harness(published);
    type(h.target, 'x'.repeat(161));
    expect(button(h.target, 'Review changes').disabled).toBe(true);
    await h.cleanup();
    h = await harness(published);
    try {
      expect(h.target.querySelector('textarea')?.value).toBe('x'.repeat(161));
      await click(h.target, 'Undo description change');
      expect(h.target.querySelector('textarea')?.value).toBe('Published description');
      type(h.target, '');
      mocks.preflight.mockResolvedValue({
        ...result({}),
        currentProfile: { description: 'Published description' },
      });
      await click(h.target, 'Review changes');
      await vi.waitFor(() => expect(h.target.textContent).toContain('Description will be removed'));
      expect(mocks.send).not.toHaveBeenCalled();
    } finally {
      await h.cleanup();
    }
  });
  it.each(['avatar', 'header'] as const)(
    'reviews an independent %s change and removal/undo has explicit semantics',
    async (field) => {
      saveProfileDraft(session, details.identityAddress, {
        ...emptyProfileDraft(),
        [field]: { action: 'set', value: 'AQ==', mimeType: 'image/webp' },
      });
      const h = await harness();
      try {
        expect(h.target.querySelectorAll('img')).toHaveLength(1);
        await menu(h, 'Remove image');
        expect(button(h.target, 'Review changes').disabled).toBe(true);
        expect(loadProfileDraft(session, details.identityAddress)[field].action).toBe('keep');
      } finally {
        await h.cleanup();
      }
    }
  );
  it('chooser and crop cancellation plus decode/encode errors preserve an existing image', async () => {
    saveProfileDraft(session, details.identityAddress, {
      ...emptyProfileDraft(),
      avatar: { action: 'set', value: 'AQ==', mimeType: 'image/webp' },
    });
    const h = await harness();
    try {
      const original = h.target.querySelector('img')?.src;
      const chooser = vi.spyOn(
        required(h.target.querySelector<HTMLInputElement>('input[type=file]')),
        'click'
      );
      await click(h.target, 'Change avatar');
      expect(chooser).toHaveBeenCalledOnce();
      expect(h.target.querySelector('canvas')).toBeNull();
      required(h.target.querySelector('input')).dispatchEvent(new Event('cancel'));
      expect(h.target.querySelector('img')?.src).toBe(original);
      mocks.decode.mockRejectedValueOnce(new Error('source_pixels'));
      await selectFile(h);
      await vi.waitFor(() => expect(h.target.textContent).toContain('20 megapixels'));
      expect(h.target.querySelector('img')?.src).toBe(original);
      const bitmap = { width: 100, height: 100, close: vi.fn() };
      mocks.decode.mockResolvedValue(bitmap);
      await selectFile(h);
      await vi.waitFor(() => expect(h.target.querySelector('canvas')).not.toBeNull());
      expect(h.target.querySelectorAll('[data-navigation-back]')).toHaveLength(0);
      mocks.encode.mockRejectedValueOnce(new Error('too_large'));
      await click(h.target, 'Use image');
      await vi.waitFor(() => expect(h.target.textContent).toContain('32 KiB'));
      expect(loadProfileDraft(session, details.identityAddress).avatar).toEqual({
        action: 'set',
        value: 'AQ==',
        mimeType: 'image/webp',
      });
      await click(h.target, 'Cancel');
      expect(h.target.querySelector('img')?.src).toBe(original);
      expect(document.activeElement).toBe(button(h.target, 'Change avatar'));
      expect(bitmap.close).toHaveBeenCalledOnce();
    } finally {
      await h.cleanup();
    }
  });
  it('uses image only after successful crop encoding and keeps the smaller candidate optional', async () => {
    const h = await harness();
    try {
      mocks.decode.mockResolvedValue({ width: 400, height: 400, close: vi.fn() });
      mocks.encode.mockResolvedValue({
        balanced: { base64: 'AQ==', mimeType: 'image/webp', byteLength: 1 },
        smaller: null,
      });
      await click(h.target, 'Add avatar');
      await selectFile(h);
      await vi.waitFor(() => expect(h.target.querySelector('canvas')).not.toBeNull());
      await click(h.target, 'Use image');
      await vi.waitFor(() => expect(h.target.querySelector('img')).not.toBeNull());
      expect(loadProfileDraft(session, details.identityAddress).avatar).toEqual({
        action: 'set',
        value: 'AQ==',
        mimeType: 'image/webp',
      });
      expect(mocks.send).not.toHaveBeenCalled();
    } finally {
      await h.cleanup();
    }
  });
  it('reviews all three fields and double click consumes one preflight', async () => {
    saveProfileDraft(session, details.identityAddress, {
      avatar: { action: 'set', value: 'AQ==', mimeType: 'image/webp' },
      header: { action: 'set', value: 'Ag==', mimeType: 'image/webp' },
      description: { action: 'set', value: 'All three' },
    });
    mocks.preflight.mockResolvedValue(
      result({ avatarBase64: 'AQ==', headerBase64: 'Ag==', description: 'All three' }, [
        'avatar',
        'header',
        'description',
      ])
    );
    let finish!: (v: { txid: string }) => void;
    mocks.send.mockImplementation(() => new Promise((r) => (finish = r)));
    const h = await harness();
    try {
      await click(h.target, 'Review changes');
      await vi.waitFor(() =>
        expect(h.target.querySelector('[data-publication-costs]')).not.toBeNull()
      );
      expect(h.target.querySelectorAll('img')).toHaveLength(2);
      const publish = button(h.target, 'Publish profile');
      publish.click();
      publish.click();
      flushSync();
      expect(mocks.send).toHaveBeenCalledExactlyOnceWith({ preflightId: 'prepared-id' });
      finish({ txid: 'c'.repeat(64) });
      await vi.waitFor(() => expect(h.target.textContent).toContain('Waiting for confirmation'));
      expect(h.target.textContent).not.toContain('next update');
      expect(h.target.textContent).not.toContain('Saved for later');
      expect(loadProfileDraft(session, details.identityAddress)).toEqual(emptyProfileDraft());
    } finally {
      await h.cleanup();
    }
  });
  it('expires reviews without sending and ignores preparation after a wallet switch', async () => {
    const h = await harness();
    try {
      type(h.target, 'Valid');
      mocks.preflight.mockResolvedValue({ ...result({ description: 'Valid' }), expiresAt: 1 });
      await click(h.target, 'Review changes');
      await vi.waitFor(() =>
        expect(h.target.querySelector('[data-publication-costs]')).not.toBeNull()
      );
      await click(h.target, 'Publish profile');
      expect(h.target.textContent).toContain('Review expired. Review the fee again.');
      expect(mocks.send).not.toHaveBeenCalled();
      let finish!: (v: IdentityProfilePreflightResult) => void;
      mocks.resume.mockImplementation(() => new Promise((r) => (finish = r)));
      await click(h.target, 'Review fee');
      setContactSession({ sessionId: 'other', network: 'testnet' });
      finish(result({ description: 'Wrong wallet' }));
      await tick();
      expect(h.target.textContent).not.toContain('Wrong wallet');
    } finally {
      await h.cleanup();
    }
  });
  it('restores only retained intent and requires a fresh second fee and explicit approval', async () => {
    const state = continuation();
    const prepared = result({ headerBase64: 'AQ==', headerMimeType: 'image/webp' }, ['header']);
    prepared.publication = {
      ...prepared.publication,
      step: 2,
      totalSteps: 2,
      earlierFeeSats: '1000000',
    };
    mocks.resume.mockResolvedValue(prepared);
    const h = await harness(empty, state);
    try {
      expect(mocks.send).not.toHaveBeenCalled();
      await click(h.target, 'Review header fee');
      await vi.waitFor(() => expect(h.target.textContent).toContain('Transaction fee'));
      expect(h.target.textContent).toContain('Update 2 of 2');
      expect(h.target.textContent).toContain('network fee changed');
      await click(h.target, 'Back to publication');
      expect(h.cancel).not.toHaveBeenCalled();
      expect(h.target.textContent).toContain('Header image');
      await click(h.target, 'Review header fee');
      await vi.waitFor(() => expect(mocks.resume).toHaveBeenCalledTimes(2));
      expect(mocks.send).not.toHaveBeenCalled();
    } finally {
      await h.cleanup();
    }
  });
  it('requires explicit discard confirmation and retains remainder after failure', async () => {
    const h = await harness(empty, continuation());
    try {
      await click(h.target, 'Discard remaining changes…');
      expect(mocks.discard).not.toHaveBeenCalled();
      expect(h.target.textContent).toContain('Header image');
      mocks.discard.mockRejectedValueOnce({ type: 'NetworkError' });
      await click(h.target, 'Discard changes');
      await vi.waitFor(() => expect(h.target.querySelector('[role=alert]')).not.toBeNull());
      expect(h.cancel).not.toHaveBeenCalled();
      await click(h.target, 'Cancel');
      expect(h.target.textContent).toContain('Review header fee');
    } finally {
      await h.cleanup();
    }
  });
});
function splitReview() {
  const prepared = result({ avatarBase64: 'AQ==', avatarMimeType: 'image/webp' }, ['avatar']);
  prepared.publication = {
    ...prepared.publication,
    totalSteps: 2,
    nextFeeSats: '2000000',
    estimatedTotalFeeSats: '3230000',
    changedFields: ['avatar', 'header'],
    proposedProfile: {
      ...prepared.proposedProfile,
      headerBase64: 'Ag==',
      headerMimeType: 'image/webp',
    },
    optimization: {
      field: 'header',
      image: { action: 'set', value: 'Aw==', mimeType: 'image/webp' },
      originalBytes: 2,
      smallerBytes: 1,
      totalSteps: 1,
      estimatedTotalFeeSats: '2000000',
      savingSats: '1230000',
    },
  };
  return prepared;
}
describe('earlier one-update comparison and exact fees', () => {
  async function ready(prepared = splitReview()) {
    saveProfileDraft(session, details.identityAddress, {
      ...emptyProfileDraft(),
      avatar: { action: 'set', value: 'AQ==', mimeType: 'image/webp' },
      header: { action: 'set', value: 'Ag==', mimeType: 'image/webp' },
    });
    mocks.preflight.mockResolvedValue(prepared);
    const h = await harness();
    await click(h.target, 'Review changes');
    await vi.waitFor(() =>
      expect(h.target.querySelector('[data-publication-costs]')).not.toBeNull()
    );
    return h;
  }
  it('offers a beneficial comparison before costs, preserves original on failure and freshly reviews either choice', async () => {
    const h = await ready();
    try {
      const option = required(h.target.querySelector('[data-one-update-option]'));
      const costs = required(h.target.querySelector('[data-publication-costs]'));
      expect(option.compareDocumentPosition(costs) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(h.target.textContent).toContain('0.0323 VRSCTEST');
      expect(button(h.target, 'Publish avatar')).toBeDefined();
      await click(h.target, 'Compare images');
      expect(h.target.textContent).toContain('Current image');
      expect(h.target.textContent).toContain('Smaller image');
      expect(mocks.send).not.toHaveBeenCalled();
      mocks.resume.mockRejectedValueOnce({ type: 'NetworkError' });
      await click(h.target, 'Use smaller image');
      await vi.waitFor(() => expect(h.target.querySelector('[role=alert]')).not.toBeNull());
      expect(loadProfileDraft(session, details.identityAddress).header).toEqual({
        action: 'set',
        value: 'Ag==',
        mimeType: 'image/webp',
      });
      mocks.resume.mockResolvedValue(splitReview());
      await click(h.target, 'Keep current image');
      await vi.waitFor(() =>
        expect(h.target.querySelector('[data-publication-costs]')).not.toBeNull()
      );
      expect(mocks.resume).toHaveBeenLastCalledWith(details.identityAddress, 'plan', undefined);
      await click(h.target, 'Compare images');
      mocks.resume.mockResolvedValue(
        result({ avatarBase64: 'AQ==', headerBase64: 'Aw==' }, ['avatar', 'header'])
      );
      await click(h.target, 'Use smaller image');
      await vi.waitFor(() =>
        expect(h.target.querySelector('[data-publication-costs]')).not.toBeNull()
      );
      expect(mocks.resume).toHaveBeenLastCalledWith(details.identityAddress, 'plan', 'header');
      expect(button(h.target, 'Publish profile')).toBeDefined();
      expect(mocks.send).not.toHaveBeenCalled();
    } finally {
      await h.cleanup();
    }
  });
  it.each(['single', 'second', 'still-two', 'no-saving'])(
    'hides ineligible alternative: %s',
    async (kind) => {
      const p = splitReview();
      if (kind === 'single') p.publication.totalSteps = 1;
      if (kind === 'second') p.publication.step = 2;
      if (kind === 'still-two') required(p.publication.optimization).totalSteps = 2;
      if (kind === 'no-saving')
        required(p.publication.optimization).estimatedTotalFeeSats =
          p.publication.estimatedTotalFeeSats;
      const h = await ready(p);
      try {
        expect(h.target.querySelector('[data-one-update-option]')).toBeNull();
      } finally {
        await h.cleanup();
      }
    }
  );
  it('preserves one-satoshi precision, trims zeros, omits missing fiat and uses only actual asset rates', async () => {
    ratesStore.set({ VRSC: { rates: { EUR: 100 }, usdChange24hPct: null } });
    const p = result({ description: 'test' });
    p.feeSats = '8522001';
    p.feeDisplay = 'wrong legacy display';
    const h = await ready(p);
    try {
      expect(h.target.textContent).toContain('0.08522001 VRSCTEST');
      expect(h.target.textContent).not.toContain('wrong legacy');
      expect(h.target.textContent).not.toContain('€');
      ratesStore.set({ VRSCTEST: { rates: { EUR: 2 }, usdChange24hPct: null } });
      await tick();
      expect(h.target.textContent).toContain('€0.17');
    } finally {
      await h.cleanup();
    }
  });
});
