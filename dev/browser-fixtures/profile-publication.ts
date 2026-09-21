// Synthetic publication states for UI inspection only. No wallet or network writes.
import { get } from 'svelte/store';
import { contactSession } from '$lib/contacts/session';
import { emptyProfileDraft, saveProfileDraft } from '$lib/identity/profileDrafts';
import type {
  IdentityProfileLoadResult,
  IdentityProfilePreflightRequest,
  IdentityProfilePreflightResult,
  IdentityProfileSnapshot,
  PendingIdentityProfileUpdate,
  ProfilePublicationState,
} from '$lib/types/wallet';

export function installPublicationFixture(params: URLSearchParams, identityAddress: string) {
  const state = params.get('state') ?? 'edit';
  const split = params.has('split') || state.includes('first') || state.includes('header');
  const second = state.includes('header');
  const source = {
    systemId: 'fixture',
    txid: 'b'.repeat(64),
    vout: 0,
    height: 1234567,
    blockhash: 'c'.repeat(64),
    digest: 'fixture',
  };
  function media(header: boolean, quality = 0.86) {
    const canvas = document.createElement('canvas');
    canvas.width = header ? 960 : 256;
    canvas.height = header ? 160 : 256;
    const ctx = canvas.getContext('2d')!;
    const gradient = ctx.createLinearGradient(0, 0, canvas.width, canvas.height);
    gradient.addColorStop(0, header ? '#bfdce0' : '#24618a');
    gradient.addColorStop(1, header ? '#467984' : '#81c2c7');
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    ctx.fillStyle = header ? '#294e66' : '#eaf4ef';
    ctx.beginPath();
    if (header) {
      ctx.moveTo(0, 160);
      ctx.bezierCurveTo(320, -40, 500, 190, 960, 48);
      ctx.lineTo(960, 160);
    } else {
      ctx.arc(128, 128, 64, 0, Math.PI * 2);
    }
    ctx.fill();
    return canvas.toDataURL('image/webp', quality).split(',')[1];
  }
  const avatar = media(false),
    header = media(true),
    smaller = media(true, 0.45);
  const original: IdentityProfilePreflightRequest = {
    coinId: 'VRSCTEST',
    channelId: 'fixture',
    identityAddress,
    avatar: { action: 'set', value: avatar, mimeType: 'image/webp' },
    header: { action: 'set', value: header, mimeType: 'image/webp' },
    smallerHeader: { action: 'set', value: smaller, mimeType: 'image/webp' },
    description: {
      action: 'set',
      value: params.has('long')
        ? 'Ik bouw hulpmiddelen voor een open internet. Iedereen kan meedoen, samenwerken en kennis delen. Nieuwe ideeën zijn welkom, elke dag opnieuw, overal ter wereld.'
        : 'Building tools for a more open internet.',
    },
  };
  const full: IdentityProfileSnapshot = {
    avatarBase64: avatar,
    avatarMimeType: 'image/webp',
    headerBase64: header,
    headerMimeType: 'image/webp',
    description: original.description.action === 'set' ? original.description.value : '',
  };
  const firstReceipt: PendingIdentityProfileUpdate = {
    identityAddress,
    txid: 'd'.repeat(64),
    submittedAt: 1,
    previousProfile: {},
    proposedProfile: {
      avatarBase64: avatar,
      avatarMimeType: 'image/webp',
      description: full.description,
    },
  };
  function loaded(snapshot: IdentityProfileSnapshot, txid: string): IdentityProfileLoadResult {
    return {
      state: Object.values(snapshot).some(Boolean) ? 'ready' : 'empty',
      issues: [],
      revisionTxid: txid,
      readHeight: 1234567,
      avatar: snapshot.avatarBase64
        ? {
            source,
            value: {
              base64: snapshot.avatarBase64,
              mimeType: 'image/webp',
              width: 256,
              height: 256,
              byteLength: 1000,
            },
          }
        : null,
      header: snapshot.headerBase64
        ? {
            source,
            value: {
              base64: snapshot.headerBase64,
              mimeType: 'image/webp',
              width: 960,
              height: 160,
              byteLength: 2000,
            },
          }
        : null,
      description: snapshot.description ? { value: snapshot.description, source } : null,
    };
  }
  const initialProfile = loaded(
    second ? firstReceipt.proposedProfile : params.has('published') ? full : {},
    source.txid
  );
  let request = second
    ? { ...original, avatar: { action: 'keep' as const }, description: { action: 'keep' as const } }
    : original;
  let plan: ProfilePublicationState | null =
    state === 'edit'
      ? null
      : {
          planId: 'fixture-plan',
          identityAddress,
          status: state.includes('pending') ? 'waiting' : state === 'recovery' ? 'stale' : 'ready',
          step: second ? 2 : 1,
          totalSteps: split ? 2 : 1,
          request,
          pending: null,
          firstReceipt: second ? firstReceipt : null,
          settledTxids: second ? [firstReceipt.txid] : [],
        };
  if (plan?.status === 'waiting')
    plan.pending = {
      identityAddress,
      txid: 'a'.repeat(64),
      submittedAt: 1,
      previousProfile: second ? firstReceipt.proposedProfile : {},
      proposedProfile: split && !second ? firstReceipt.proposedProfile : full,
    };
  if (params.has('draft'))
    saveProfileDraft(get(contactSession), identityAddress, { ...emptyProfileDraft(), ...original });
  if (params.has('removal'))
    saveProfileDraft(get(contactSession), identityAddress, {
      ...emptyProfileDraft(),
      avatar: { action: 'remove' },
      header: { action: 'remove' },
      description: { action: 'remove' },
    });
  let review: IdentityProfilePreflightResult;
  let selectedSmaller = false;
  function prepare(next: IdentityProfilePreflightRequest) {
    request = next;
    const fields = (['avatar', 'header', 'description'] as const).filter(
      (f) => (next[f]?.action ?? 'keep') !== 'keep'
    );
    const proposed: IdentityProfileSnapshot = params.has('published')
      ? { ...full }
      : second
        ? { ...firstReceipt.proposedProfile }
        : {};
    for (const f of fields) {
      const change = next[f];
      const key = f === 'description' ? 'description' : (`${f}Base64` as const);
      if (change?.action === 'set') {
        proposed[key] = change.value;
        if (f !== 'description') proposed[`${f}MimeType`] = 'image/webp';
      } else {
        delete proposed[key];
        if (f !== 'description') delete proposed[`${f}MimeType`];
      }
    }
    const totalSteps = split && !selectedSmaller ? 2 : 1;
    const fee = second ? '8642000' : split && !selectedSmaller ? '13305000' : '8522000';
    const current = second ? firstReceipt.proposedProfile : params.has('published') ? full : {};
    const immediate =
      totalSteps === 2 && !second
        ? { ...proposed, headerBase64: undefined, headerMimeType: undefined }
        : proposed;
    review = {
      preflightId: 'synthetic-review',
      expiresAt: params.has('expired') ? 1 : Date.now() / 1000 + 300,
      currentProfile: current,
      proposedProfile: immediate,
      changedFields: totalSteps === 2 && !second ? fields.filter((f) => f !== 'header') : fields,
      feeSats: fee,
      feeDisplay: 'fixture display must not be used',
      fundingSummary: 'Synthetic fixture',
      evidenceBytes: 100,
      publication: {
        planId: 'fixture-plan',
        step: second ? 2 : 1,
        totalSteps,
        nextFeeSats: totalSteps === 2 && !second ? '8642000' : null,
        estimatedTotalFeeSats: totalSteps === 2 ? '21947000' : fee,
        earlierFeeSats: second ? (params.has('feeChanged') ? '8000000' : '8642000') : null,
        quoteHeight: 1234567,
        quoteTime: 1,
        availableSats: params.has('insufficient') ? '1' : '100000000',
        proposedProfile: proposed,
        changedFields: fields,
        evidenceGroups: [],
        optimization:
          totalSteps === 2 && !second && !params.has('noAlternative')
            ? {
                field: 'header',
                image: { action: 'set', value: smaller, mimeType: 'image/webp' },
                originalBytes: 6800,
                smallerBytes: 4700,
                totalSteps: 1,
                estimatedTotalFeeSats: '19221000',
                savingSats: '2726000',
              }
            : null,
      },
    };
    plan = {
      planId: 'fixture-plan',
      identityAddress,
      status: 'ready',
      step: second ? 2 : 1,
      totalSteps,
      request,
      pending: null,
      firstReceipt: second ? firstReceipt : null,
      settledTxids: second ? [firstReceipt.txid] : [],
    };
    return review;
  }
  let reads = 0;
  const fallback = window.__PROFILE_FIXTURE_INVOKE__!;
  window.__PROFILE_FIXTURE_INVOKE__ = async (command, args) => {
    if (command === 'preflight_identity_profile_update')
      return prepare(args?.request as IdentityProfilePreflightRequest);
    if (command === 'review_identity_profile_publication') {
      if (params.has('replanFail')) throw new Error('Synthetic replan failure');
      if (args?.smaller_field) {
        selectedSmaller = true;
        return prepare({
          ...request,
          header: { action: 'set', value: smaller, mimeType: 'image/webp' },
        });
      }
      return prepare(request);
    }
    if (command === 'get_identity_profile_publication') {
      if (params.has('refreshFail') && reads++ > 0)
        throw new Error('Synthetic confirmation unavailable');
      return plan;
    }
    if (command === 'confirm_identity_profile_update')
      return params.has('confirmed') && plan?.pending
        ? loaded(plan.pending.proposedProfile, plan.pending.txid)
        : null;
    if (command === 'send_identity_update') {
      await new Promise((r) => setTimeout(r, 600));
      const pending = {
        identityAddress,
        txid: 'a'.repeat(64),
        submittedAt: 1,
        previousProfile: review.currentProfile,
        proposedProfile: review.proposedProfile,
      };
      plan = { ...plan!, status: 'waiting', pending };
      if (params.has('ambiguous')) throw new Error('Synthetic ambiguous transport');
      if (params.has('confirmed'))
        setTimeout(() => {
          if (plan) plan = { ...plan, status: 'complete' };
        }, 1000);
      return {
        txid: pending.txid,
        operation: 'update',
        targetIdentity: identityAddress,
        fee: review.feeSats,
        fromAddress: 'RFixture',
        profileUpdate: pending,
      };
    }
    if (command === 'discard_identity_profile_publication') {
      plan = null;
      return null;
    }
    return fallback(command, args);
  };
  return initialProfile;
}
