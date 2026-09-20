// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ResolvedContactIdentity } from '$lib/types/addressBook';

const mocks = vi.hoisted(() => ({
  resolveContactIdentity: vi.fn(),
  loadIdentityProfile: vi.fn(),
}));

vi.mock('$lib/contacts/service', () => ({
  resolveContactIdentity: mocks.resolveContactIdentity,
}));

vi.mock('$lib/contacts/profiles', async () => {
  const { writable } = await import('svelte/store');
  return {
    identityProfiles: writable({}),
    loadIdentityProfile: mocks.loadIdentityProfile,
    profileImage: () => null,
  };
});

import { localeStore } from '$lib/i18n';
import { identityProfiles } from '$lib/contacts/profiles';
import VerusIdLookup from './VerusIdLookup.svelte';
import { createVerusIdLookupState, type VerusIdLookupState } from './verusIdPublicProfile';

const identity: ResolvedContactIdentity = {
  identityAddress: 'iCanonicalIdentityAddress',
  fullyQualifiedName: 'alex.example@',
  network: 'mainnet',
  chainId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
  status: 'active',
};

function deferred<T>() {
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

function enter(target: HTMLElement, value: string): HTMLInputElement {
  const input = target.querySelector('input') as HTMLInputElement;
  input.value = value;
  input.dispatchEvent(new Event('input', { bubbles: true }));
  return input;
}

function submit(target: HTMLElement): void {
  target
    .querySelector('form')
    ?.dispatchEvent(new SubmitEvent('submit', { bubbles: true, cancelable: true }));
}

beforeEach(() => {
  document.body.innerHTML = '';
  localeStore.set('en');
  identityProfiles.set({});
  mocks.resolveContactIdentity.mockReset();
  mocks.loadIdentityProfile.mockReset().mockResolvedValue({
    state: 'empty',
    avatar: null,
    description: null,
    issues: [],
    readHeight: null,
    revisionTxid: null,
  });
});

describe('mounted public VerusID lookup', () => {
  it('resolves exactly once without loading Contacts and opens the canonical result', async () => {
    mocks.resolveContactIdentity.mockResolvedValue(identity);
    let latest = createVerusIdLookupState();
    const opened: ResolvedContactIdentity[] = [];
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(VerusIdLookup, {
      target,
      props: {
        walletNetwork: 'mainnet',
        initialState: latest,
        onStateChange: (state: VerusIdLookupState) => {
          latest = state;
        },
        onOpenProfile: (resolved: ResolvedContactIdentity) => opened.push(resolved),
      },
    });

    try {
      enter(target, '  alex.example@  ');
      await settle();
      const findButton = Array.from(target.querySelectorAll('button')).find(
        (candidate) => candidate.type === 'submit'
      );
      findButton?.click();
      findButton?.click();
      await settle();

      expect(mocks.resolveContactIdentity).toHaveBeenCalledOnce();
      expect(mocks.resolveContactIdentity).toHaveBeenCalledWith('alex.example@');
      expect(latest.status).toBe('resolved');
      expect(latest.result).toEqual(identity);
      expect(mocks.loadIdentityProfile).toHaveBeenCalledWith(identity, false, true);
      expect(target.textContent).toContain('View profile');

      Array.from(target.querySelectorAll<HTMLButtonElement>('button'))
        .find((candidate) => candidate.textContent?.trim() === 'View profile')
        ?.click();
      expect(opened).toEqual([identity]);
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('invalidates a pending result as soon as the query changes', async () => {
    const pending = deferred<ResolvedContactIdentity>();
    mocks.resolveContactIdentity.mockReturnValue(pending.promise);
    let latest = createVerusIdLookupState();
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(VerusIdLookup, {
      target,
      props: {
        walletNetwork: 'mainnet',
        initialState: latest,
        onStateChange: (state: VerusIdLookupState) => {
          latest = state;
        },
      },
    });

    try {
      enter(target, 'alex.example@');
      submit(target);
      await settle();
      expect(latest.status).toBe('looking-up');

      enter(target, 'morgan.example@');
      await settle();
      expect(latest.status).toBe('idle');
      expect(latest.result).toBeNull();

      pending.resolve(identity);
      await settle();
      expect(latest.status).toBe('idle');
      expect(target.textContent).not.toContain('View profile');
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('keeps the active request alive through whitespace-only query edits', async () => {
    const pending = deferred<ResolvedContactIdentity>();
    mocks.resolveContactIdentity.mockReturnValue(pending.promise);
    let latest = createVerusIdLookupState();
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(VerusIdLookup, {
      target,
      props: {
        walletNetwork: 'mainnet',
        initialState: latest,
        onStateChange: (state: VerusIdLookupState) => {
          latest = state;
        },
      },
    });

    try {
      enter(target, 'alex.example@');
      submit(target);
      await settle();
      expect(latest.status).toBe('looking-up');

      enter(target, 'alex.example@ ');
      await settle();
      pending.resolve(identity);
      await settle();

      expect(latest.status).toBe('resolved');
      expect(latest.result).toEqual(identity);
      expect(target.textContent).toContain('View profile');
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('restores an interrupted lookup as retryable without submitting on remount', async () => {
    const pending = deferred<ResolvedContactIdentity>();
    mocks.resolveContactIdentity
      .mockReturnValueOnce(pending.promise)
      .mockResolvedValueOnce(identity);
    let latest = createVerusIdLookupState();
    const target = document.createElement('div');
    document.body.append(target);
    const props = () => ({
      walletNetwork: 'mainnet' as const,
      initialState: latest,
      onStateChange: (state: VerusIdLookupState) => {
        latest = state;
      },
    });
    let component = mount(VerusIdLookup, { target, props: props() });

    enter(target, 'alex.example@');
    submit(target);
    await settle();
    expect(latest.status).toBe('looking-up');

    await unmount(component);
    pending.resolve(identity);
    await settle();

    component = mount(VerusIdLookup, { target, props: props() });
    try {
      await settle();
      expect(latest.status).toBe('idle');
      expect(latest.result).toBeNull();
      expect(mocks.resolveContactIdentity).toHaveBeenCalledOnce();

      const findButton = Array.from(target.querySelectorAll<HTMLButtonElement>('button')).find(
        (candidate) => candidate.type === 'submit'
      );
      expect(findButton?.disabled).toBe(false);
      findButton?.click();
      await settle();

      expect(mocks.resolveContactIdentity).toHaveBeenCalledTimes(2);
      expect(latest.status).toBe('resolved');
      expect(target.textContent).toContain('View profile');
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('normalizes a persisted interrupted snapshot without automatically retrying', async () => {
    let latest: VerusIdLookupState = {
      query: 'alex.example@',
      submittedQuery: 'alex.example@',
      status: 'looking-up',
      result: null,
      scrollTop: 18,
    };
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(VerusIdLookup, {
      target,
      props: {
        walletNetwork: 'mainnet',
        initialState: latest,
        onStateChange: (state: VerusIdLookupState) => {
          latest = state;
        },
      },
    });

    try {
      await settle();
      expect(latest).toMatchObject({
        query: 'alex.example@',
        submittedQuery: '',
        status: 'idle',
        result: null,
      });
      expect(mocks.resolveContactIdentity).not.toHaveBeenCalled();
      const findButton = Array.from(target.querySelectorAll<HTMLButtonElement>('button')).find(
        (candidate) => candidate.type === 'submit'
      );
      expect(findButton?.disabled).toBe(false);
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it.each([
    [{ type: 'IdentityNotFound' }, 'not-found', 'No VerusID found.'],
    [new Error('provider offline'), 'unavailable', "Couldn't look up this VerusID."],
  ] as const)('keeps absence distinct from provider failure', async (error, status, message) => {
    mocks.resolveContactIdentity.mockRejectedValue(error);
    let latest = createVerusIdLookupState();
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(VerusIdLookup, {
      target,
      props: {
        walletNetwork: 'mainnet',
        initialState: latest,
        onStateChange: (state: VerusIdLookupState) => {
          latest = state;
        },
      },
    });

    try {
      enter(target, 'missing@');
      submit(target);
      await settle();
      expect(latest.status).toBe(status);
      expect(target.textContent).toContain(message);
      expect((target.querySelector('input') as HTMLInputElement).value).toBe('missing@');
    } finally {
      await unmount(component);
      target.remove();
    }
  });
});
