import '../../src/app.css';
import { mount } from 'svelte';
import { installPublicationFixture } from './profile-publication';
import ProfileFixture from './ProfileFixture.svelte';
import { setContactSession } from '$lib/contacts/session';
import { setLocale } from '$lib/i18n';
import type { ResolvedContactIdentity } from '$lib/types/addressBook';
import type { IdentityProfilePreflightRequest } from '$lib/types/wallet';
import type { PublicProfileContent } from '$lib/components/wallet/sections/identity/publicProfileContent';

const nativeInternals = window.__TAURI_INTERNALS__;
const nativeInvoke = nativeInternals?.invoke.bind(nativeInternals);
const params = new URLSearchParams(location.search);
const identity: ResolvedContactIdentity = {
  identityAddress: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
  fullyQualifiedName: params.has('long')
    ? 'averylongcanonicalidentityname.withparent.namespace@'
    : 'alex.example@',
  network: 'mainnet',
  chainId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
  status: params.has('revoked') ? 'revoked' : 'active',
};
const source = {
  systemId: identity.chainId,
  txid: 'fixture',
  vout: 0,
  height: 1,
  blockhash: 'fixture',
  digest: 'fixture',
};
let contacts: unknown[] = params.has('saved') ? [makeContact()] : [];
function makeContact() {
  return {
    id: 'fixture-contact',
    displayName: identity.fullyQualifiedName,
    note: 'Private note not for profile',
    identities: [identity],
    profileIdentity: identity,
    endpoints: [],
    createdAt: 1,
    updatedAt: 1,
  };
}
let saveAttempts = 0;
const fixtureInternals = {
  ...nativeInternals,
  transformCallback: nativeInternals?.transformCallback ?? (() => 1),
  unregisterCallback: nativeInternals?.unregisterCallback ?? (() => {}),
  invoke: async (command: string, args?: Record<string, unknown>) => {
    if (command === 'encode_identity_profile_image') {
      if (nativeInvoke) return nativeInvoke(command, args);
      const { encodeProfileCandidates } = await import('$lib/identity/profileImages');
      const blob = await (await fetch(`data:image/png;base64,${args?.png_base64}`)).blob();
      const bitmap = await createImageBitmap(blob);
      const canvas = document.createElement('canvas');
      canvas.width = bitmap.width;
      canvas.height = bitmap.height;
      canvas.getContext('2d')!.drawImage(bitmap, 0, 0);
      bitmap.close();
      return encodeProfileCandidates(canvas);
    }
    if (command === 'plugin:os|type') return 'macos';
    if (params.has('editor') && command === 'get_addresses') return { vrsc_address: 'RFixture' };
    if (params.has('editor') && command === 'preflight_identity_profile_update') {
      const request = args?.request as IdentityProfilePreflightRequest;
      const changedFields = (['avatar', 'header', 'description'] as const).filter(
        (field) => request[field]?.action !== 'keep'
      );
      const proposedProfile = Object.fromEntries(
        changedFields.flatMap((field) => {
          const change = request[field];
          return change?.action === 'set'
            ? [
                [field === 'description' ? field : `${field}Base64`, change.value],
                ...(field === 'description' ? [] : [[`${field}MimeType`, 'image/webp']]),
              ]
            : [];
        })
      );
      return {
        publication: {
          planId: 'fixture-plan',
          step: 1,
          totalSteps: params.has('split') ? 2 : 1,
          nextFeeSats: params.has('split') ? '14342000' : null,
          estimatedTotalFeeSats: params.has('split') ? '15572000' : '1230000',
          earlierFeeSats: null,
          quoteHeight: 1234567,
          quoteTime: Date.now() / 1000,
          availableSats: '1234567890',
          proposedProfile,
          changedFields,
          evidenceGroups: [],
          optimization: null,
        },
        preflightId: 'synthetic-profile-review',
        expiresAt: Math.floor(Date.now() / 1000) + 300,
        currentProfile: {},
        proposedProfile: Object.fromEntries(
          changedFields.flatMap((field) => {
            const change = request[field];
            return change?.action === 'set'
              ? [[field === 'description' ? field : `${field}Base64`, change.value]]
              : [];
          })
        ),
        changedFields,
        feeSats: '1230000',
        feeDisplay: '0.0123',
        fundingSummary: 'Synthetic fixture',
        evidenceBytes: 100,
      };
    }
    if (command === 'get_identity_profile_publication') return null;
    if (command === 'get_identity_profile')
      return {
        state: params.has('unavailable') ? 'unavailable' : params.has('empty') ? 'empty' : 'ready',
        description:
          params.has('empty') || params.has('unavailable')
            ? null
            : {
                value: params.has('long')
                  ? 'Building tools for a more open internet.\n'.repeat(12)
                  : 'Building tools for a more open internet.\nCurious about identity, privacy and everything in between.',
                source,
              },
        avatar: null,
        issues: [],
        revisionTxid: 'fixture',
        readHeight: 1,
      };
    if (command === 'get_identity_details') {
      if (params.has('detailsError')) throw new Error('Synthetic details unavailable');
      return {
        identityAddress: identity.identityAddress,
        status: identity.status,
        system: identity.chainId,
        systemDisplayName: 'Verus',
        minimumSignatures: 1,
        primaryAddresses: ['R9dV5nznVjjrGn43jo69DzrxQ3AJXCRHzK'],
        revocationAuthorityName: identity.fullyQualifiedName,
        recoveryAuthorityName: identity.fullyQualifiedName,
        ownedByPrimaryAddress: false,
        tokenizedControl: false,
        profileEditable: false,
        warnings: [],
      };
    }
    if (command === 'list_address_book_contacts') return contacts;
    if (command === 'save_address_book_contact') {
      await new Promise((resolve) => setTimeout(resolve, 900));
      if (params.has('failOnce') && saveAttempts++ === 0)
        throw new Error('Synthetic storage failure');
      const contact = makeContact();
      contacts = [contact];
      return contact;
    }
    if (command === 'plugin:opener|open_url') return null;
    throw new Error(`Unsupported fixture command: ${command}`);
  },
};
window.__PROFILE_FIXTURE_INVOKE__ = fixtureInternals.invoke;
// Native Tauri internals are immutable. Fixture service calls use a dev-only
// module adapter; encoding still invokes the real local Rust command.
if (!nativeInternals) window.__TAURI_INTERNALS__ = fixtureInternals;
const content: PublicProfileContent = params.has('rich')
  ? {
      websites: [
        { url: 'https://alex.example', verification: 'verified' },
        { url: 'https://verus.io', verification: 'unverified' },
        { url: 'https://github.com/example', verification: 'verified' },
      ],
      socials: [
        {
          platform: 'linkedin',
          profileUrl: 'https://www.linkedin.com/in/example',
          proofUrl: 'https://www.linkedin.com/posts/example',
          verification: 'verified',
        },
        { platform: 'x', profileUrl: 'https://x.com/example', verification: 'unverified' },
      ],
      addresses: [
        {
          network: 'Ethereum',
          address: '0x71C7656EC7ab88b098defB751B7401B5f6d8976F',
          verification: 'verified',
        },
      ],
    }
  : {};
if (params.has('header')) {
  // Synthetic graphic used only in this fixture; never a public identity claim.
  const canvas = document.createElement('canvas');
  canvas.width = 960;
  canvas.height = 160;
  const ctx = canvas.getContext('2d')!;
  ctx.fillStyle = '#8eacb3';
  ctx.fillRect(0, 0, 960, 160);
  ctx.fillStyle = '#d6d8ca';
  ctx.beginPath();
  ctx.arc(710, 45, 23, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = '#3e6872';
  ctx.beginPath();
  ctx.moveTo(0, 125);
  ctx.lineTo(250, 38);
  ctx.lineTo(620, 160);
  ctx.lineTo(0, 160);
  ctx.fill();
  ctx.fillStyle = '#234b55';
  ctx.beginPath();
  ctx.moveTo(250, 160);
  ctx.lineTo(680, 58);
  ctx.lineTo(960, 142);
  ctx.lineTo(960, 160);
  ctx.fill();
  const base64 = canvas.toDataURL('image/jpeg', 0.8).split(',')[1];
  content.header = {
    base64,
    mimeType: 'image/jpeg',
    width: 960,
    height: 160,
    byteLength: Math.ceil(base64.length * 0.75),
  };
}
if (params.get('theme') === 'dark') document.documentElement.classList.add('dark');
setLocale(params.get('locale') === 'nl' ? 'nl' : 'en');
setContactSession({
  sessionId: 'profile-fixture',
  network: params.has('editor') ? 'testnet' : 'mainnet',
});
const initialProfile = params.has('editor')
  ? installPublicationFixture(params, identity.identityAddress)
  : undefined;
mount(ProfileFixture, {
  target: document.getElementById('fixture')!,
  props: { identity, content, initialProfile },
});
