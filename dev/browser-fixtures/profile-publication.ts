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
  const scenario = params.get('scenario');
  const split =
    params.has('split') ||
    scenario === 'split' ||
    state.includes('first') ||
    state.includes('header');
  const second = state.includes('header');
  let currentStep: 1 | 2 = second ? 2 : 1;
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
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('Synthetic canvas is unavailable');
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
  const existing = Boolean(scenario && scenario !== 'addition') || params.has('published');
  const initialDraft = (() => {
    const keep = { action: 'keep' as const };
    const changedDescription = {
      action: 'set' as const,
      value: 'A simpler profile for our community. I share wallet updates and design notes.',
    };
    const changedAvatar = {
      action: 'set' as const,
      value: media(false, 0.5),
      mimeType: 'image/webp',
    };
    const changedHeader = {
      action: 'set' as const,
      value: media(true, 0.5),
      mimeType: 'image/webp',
    };
    switch (scenario) {
      case 'addition':
        return { ...emptyProfileDraft(), description: original.description };
      case 'change':
        return { ...emptyProfileDraft(), description: changedDescription };
      case 'removal':
        return { ...emptyProfileDraft(), description: { action: 'remove' as const } };
      case 'two-removals':
        return {
          ...emptyProfileDraft(),
          avatar: { action: 'remove' as const },
          header: { action: 'remove' as const },
        };
      case 'three-removals':
        return {
          ...emptyProfileDraft(),
          avatar: { action: 'remove' as const },
          header: { action: 'remove' as const },
          description: { action: 'remove' as const },
        };
      case 'two-changes':
        return { ...emptyProfileDraft(), avatar: changedAvatar, description: changedDescription };
      case 'three-changes':
      case 'split':
        return {
          ...emptyProfileDraft(),
          avatar: changedAvatar,
          header: changedHeader,
          description: changedDescription,
          smallerHeader: original.smallerHeader,
        };
      case 'mixed':
        return {
          ...emptyProfileDraft(),
          avatar: { action: 'remove' as const },
          description: changedDescription,
        };
      default:
        return { ...emptyProfileDraft(), ...original, header: keep };
    }
  })();
  let firstReceipt: PendingIdentityProfileUpdate = {
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
    second ? firstReceipt.proposedProfile : existing ? full : {},
    source.txid
  );
  let request = second
    ? {
        ...initialDraft,
        avatar: { action: 'keep' as const },
        description: { action: 'keep' as const },
      }
    : initialDraft;
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
  if (params.has('draft') || scenario)
    saveProfileDraft(get(contactSession), identityAddress, initialDraft);
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
    const reviewingSecond = currentStep === 2;
    const proposed: IdentityProfileSnapshot = reviewingSecond
      ? { ...firstReceipt.proposedProfile }
      : existing
        ? { ...full }
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
    const fee = reviewingSecond ? '8642000' : split && !selectedSmaller ? '13305000' : '8522000';
    const current = reviewingSecond ? firstReceipt.proposedProfile : existing ? full : {};
    const immediate =
      totalSteps === 2 && !reviewingSecond
        ? {
            ...proposed,
            headerBase64: current.headerBase64,
            headerMimeType: current.headerMimeType,
          }
        : proposed;
    review = {
      preflightId: 'synthetic-review',
      expiresAt: params.has('expired') ? 1 : Date.now() / 1000 + 300,
      currentProfile: current,
      proposedProfile: immediate,
      changedFields:
        totalSteps === 2 && !reviewingSecond ? fields.filter((f) => f !== 'header') : fields,
      feeSats: fee,
      feeDisplay: 'fixture display must not be used',
      fundingSummary: 'Synthetic fixture',
      evidenceBytes: 100,
      publication: {
        planId: 'fixture-plan',
        step: currentStep,
        totalSteps,
        nextFeeSats: totalSteps === 2 && !reviewingSecond ? '8642000' : null,
        estimatedTotalFeeSats: totalSteps === 2 ? '21947000' : fee,
        earlierFeeSats: reviewingSecond ? (params.has('feeChanged') ? '8000000' : '8642000') : null,
        quoteHeight: 1234567,
        quoteTime: 1,
        availableSats: params.has('insufficient') ? '1' : '100000000',
        proposedProfile: proposed,
        changedFields: fields,
        evidenceGroups: [],
        optimization:
          totalSteps === 2 && !reviewingSecond && !params.has('noAlternative')
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
      step: currentStep,
      totalSteps,
      request,
      pending: null,
      firstReceipt: reviewingSecond ? firstReceipt : null,
      settledTxids: reviewingSecond ? [firstReceipt.txid] : [],
    };
    return review;
  }
  let reads = 0;
  const fallback = window.__PROFILE_FIXTURE_INVOKE__;
  if (!fallback) throw new Error('Synthetic wallet command handler is unavailable');
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
    if (command === 'confirm_identity_profile_update') {
      const receipt =
        plan?.pending?.txid === args?.txid
          ? plan.pending
          : plan?.firstReceipt?.txid === args?.txid
            ? plan.firstReceipt
            : null;
      return params.has('confirmed') && receipt
        ? loaded(receipt.proposedProfile, receipt.txid)
        : null;
    }
    if (command === 'send_identity_update') {
      await new Promise((r) => setTimeout(r, 600));
      const pending = {
        identityAddress,
        txid: (currentStep === 2 ? 'e' : 'a').repeat(64),
        submittedAt: 1,
        previousProfile: review.currentProfile,
        proposedProfile: review.proposedProfile,
      };
      if (!plan) throw new Error('Synthetic publication plan is unavailable');
      plan = { ...plan, status: 'waiting', pending };
      if (params.has('ambiguous')) throw new Error('Synthetic ambiguous transport');
      if (params.has('confirmed'))
        setTimeout(() => {
          if (!plan) return;
          if (split && currentStep === 1 && !selectedSmaller) {
            firstReceipt = pending;
            currentStep = 2;
            plan = {
              ...plan,
              status: 'ready',
              step: 2,
              request: { ...request, avatar: { action: 'keep' }, description: { action: 'keep' } },
              pending: null,
              firstReceipt: pending,
              settledTxids: [pending.txid],
            };
          } else {
            plan = { ...plan, status: 'complete' };
          }
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
