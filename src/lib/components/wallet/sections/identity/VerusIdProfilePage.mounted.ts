// @vitest-environment jsdom

import { mount, tick, unmount } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ResolvedContactIdentity } from '$lib/types/addressBook';

const mocks = vi.hoisted(() => ({
  addIdentityContact: vi.fn(),
  loadContacts: vi.fn(),
  loadIdentityProfile: vi.fn(),
}));

vi.mock('$lib/contacts/service', () => ({
  addIdentityContact: mocks.addIdentityContact,
  loadContacts: mocks.loadContacts,
}));

vi.mock('$lib/contacts/profiles', async () => {
  const { writable } = await import('svelte/store');
  return {
    identityProfiles: writable({}),
    loadIdentityProfile: mocks.loadIdentityProfile,
    profileImage: () => null,
  };
});

import { identityKey } from '$lib/contacts/identity';
import { identityProfiles } from '$lib/contacts/profiles';
import { contactsLoadState } from '$lib/contacts/session';
import { localeStore } from '$lib/i18n';
import { addressBookStore } from '$lib/stores/addressBook';
import VerusIdProfilePage from './VerusIdProfilePage.svelte';

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

function button(target: HTMLElement, label: string): HTMLButtonElement {
  const match = Array.from(target.querySelectorAll('button')).find(
    (candidate) => candidate.textContent?.trim() === label
  );
  if (!(match instanceof HTMLButtonElement)) throw new Error(`Missing ${label} button`);
  return match;
}

beforeEach(() => {
  document.body.innerHTML = '';
  localeStore.set('en');
  addressBookStore.set([]);
  contactsLoadState.set('ready');
  identityProfiles.set({
    [identityKey(identity)]: {
      profile: {
        state: 'ready',
        avatar: null,
        description: {
          value: 'Building things.',
          source: {
            systemId: identity.chainId,
            txid: 'revision',
            vout: 0,
            height: 1,
            blockhash: 'blockhash',
            digest: 'digest',
          },
        },
        issues: [],
        readHeight: 1,
        revisionTxid: 'revision',
      },
      loading: false,
      unavailable: false,
      checkedAt: Date.now(),
    },
  });
  mocks.addIdentityContact.mockReset();
  mocks.loadContacts.mockReset().mockResolvedValue([]);
  mocks.loadIdentityProfile.mockReset().mockResolvedValue({
    state: 'empty',
    avatar: null,
    description: null,
    issues: [],
    readHeight: null,
    revisionTxid: null,
  });
});

describe('mounted public VerusID profile', () => {
  it('saves once, keeps the profile open, and starts Send with the canonical identity', async () => {
    const save = deferred<unknown>();
    mocks.addIdentityContact.mockReturnValue(save.promise);
    const sent: ResolvedContactIdentity[] = [];
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(VerusIdProfilePage, {
      target,
      props: {
        identity,
        onSend: (recipient: ResolvedContactIdentity) => sent.push(recipient),
      },
    });

    try {
      await settle();
      expect(target.textContent).toContain('Building things.');
      expect(target.textContent).not.toContain('Website');
      expect(mocks.loadContacts).toHaveBeenCalledOnce();

      const add = button(target, 'Add to contacts');
      add.click();
      add.click();
      await settle();
      expect(mocks.addIdentityContact).toHaveBeenCalledOnce();
      expect(mocks.addIdentityContact).toHaveBeenCalledWith(identity);
      expect(target.textContent).toContain('Saving');

      save.resolve({});
      await settle();
      expect(target.textContent).toContain('Saved');

      button(target, 'Send').click();
      expect(sent).toEqual([identity]);
      expect(target.textContent).toContain('alex.example@');
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('reports contact membership failure as retryable instead of assuming absence', async () => {
    contactsLoadState.set('error');
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(VerusIdProfilePage, { target, props: { identity } });

    try {
      await settle();
      button(target, 'Try again').click();
      await settle();
      expect(mocks.loadContacts).toHaveBeenLastCalledWith(true);
      expect(mocks.addIdentityContact).not.toHaveBeenCalled();
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('keeps the profile open and offers a retry when durable contact saving fails', async () => {
    mocks.addIdentityContact.mockRejectedValueOnce(new Error('storage failed'));
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(VerusIdProfilePage, { target, props: { identity } });

    try {
      await settle();
      button(target, 'Add to contacts').click();
      await settle();
      expect(target.textContent).toContain("Couldn't save contact.");
      expect(target.textContent).toContain('alex.example@');

      mocks.addIdentityContact.mockResolvedValueOnce({});
      button(target, 'Try again').click();
      await settle();
      expect(mocks.addIdentityContact).toHaveBeenCalledTimes(2);
      expect(target.textContent).toContain('Saved');
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('recognizes existing identity membership without offering a duplicate save', async () => {
    addressBookStore.set([
      {
        id: 'existing-contact',
        displayName: 'Alex',
        note: 'Private note',
        createdAt: 1,
        updatedAt: 1,
        endpoints: [],
        identities: [identity],
        profileIdentity: identity,
      },
    ]);
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(VerusIdProfilePage, { target, props: { identity } });

    try {
      await settle();
      expect(button(target, 'In contacts').disabled).toBe(true);
      expect(mocks.addIdentityContact).not.toHaveBeenCalled();
      expect(target.textContent).not.toContain('Private note');
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('disables Send when the resolved identity is known to be revoked', async () => {
    const revoked = { ...identity, status: 'revoked' };
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(VerusIdProfilePage, { target, props: { identity: revoked } });

    try {
      await settle();
      expect(button(target, 'Send').disabled).toBe(true);
      expect(target.textContent).toContain('This VerusID is Revoked');
    } finally {
      await unmount(component);
      target.remove();
    }
  });
});
