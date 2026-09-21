import { get } from 'svelte/store';
import { contactSession, type ContactSession } from '$lib/contacts/session';
import type {
  IdentityProfileAvatarChange,
  IdentityProfileDescriptionChange,
  IdentityProfileLoadResult,
} from '$lib/types/wallet';

export const PROFILE_FIELDS = ['avatar', 'header', 'description'] as const;
export type ProfileField = (typeof PROFILE_FIELDS)[number];
export type ProfileDraft = {
  descriptionText?: string;
  avatar: IdentityProfileAvatarChange;
  header: IdentityProfileAvatarChange;
  description: IdentityProfileDescriptionChange;
  smallerAvatar?: IdentityProfileAvatarChange | null;
  smallerHeader?: IdentityProfileAvatarChange | null;
};
export function emptyProfileDraft(): ProfileDraft {
  return {
    avatar: { action: 'keep' },
    header: { action: 'keep' },
    description: { action: 'keep' },
  };
}

// Unpublished content stays in memory, like the existing contact/profile session caches.
// It survives route/component navigation, but is discarded on lock, wallet/network change,
// or app exit. It is never written to plaintext localStorage. Only Review sends
// the selected changes to the backend for transaction preparation.
const drafts = new Map<string, ProfileDraft>();
contactSession.subscribe(() => drafts.clear());

function current(session: ContactSession | null): boolean {
  return session !== null && get(contactSession) === session;
}
export function loadProfileDraft(session: ContactSession | null, identity: string): ProfileDraft {
  return structuredClone(
    current(session) ? (drafts.get(identity) ?? emptyProfileDraft()) : emptyProfileDraft()
  );
}
export function saveProfileDraft(
  session: ContactSession | null,
  identity: string,
  draft: ProfileDraft
): void {
  if (!current(session)) return;
  if (
    PROFILE_FIELDS.every((field) => draft[field].action === 'keep') &&
    draft.descriptionText === undefined
  )
    drafts.delete(identity);
  else drafts.set(identity, structuredClone(draft));
}
export function clearProfileDraft(session: ContactSession | null, identity: string): void {
  if (current(session)) drafts.delete(identity);
}
export function publishedProfileValue(
  profile: IdentityProfileLoadResult | null,
  field: ProfileField
): string | null {
  return field === 'description'
    ? (profile?.description?.value ?? null)
    : (profile?.[field]?.value.base64 ?? null);
}
export function effectiveProfileValue(
  profile: IdentityProfileLoadResult | null,
  draft: ProfileDraft,
  field: ProfileField
): string | null {
  const change = draft[field];
  return change.action === 'set'
    ? change.value
    : change.action === 'remove'
      ? null
      : publishedProfileValue(profile, field);
}
export function stageProfileValue(
  profile: IdentityProfileLoadResult | null,
  draft: ProfileDraft,
  field: ProfileField,
  value: string | null
): ProfileDraft {
  const published = publishedProfileValue(profile, field);
  return {
    ...draft,
    [field]:
      value === published || (!value && !published)
        ? { action: 'keep' }
        : value
          ? { action: 'set', value, ...(field === 'description' ? {} : { mimeType: 'image/webp' }) }
          : { action: 'remove' },
  };
}
export function removeProfileValue(
  profile: IdentityProfileLoadResult | null,
  draft: ProfileDraft,
  field: ProfileField
): ProfileDraft {
  return clearCandidate(stageProfileValue(profile, draft, field, null), field);
}
export function undoProfileValue(draft: ProfileDraft, field: ProfileField): ProfileDraft {
  return clearCandidate({ ...draft, [field]: { action: 'keep' } }, field);
}
function clearCandidate(draft: ProfileDraft, field: ProfileField): ProfileDraft {
  if (field === 'description') {
    const { descriptionText: _text, ...rest } = draft;
    return rest;
  }
  return { ...draft, [field === 'avatar' ? 'smallerAvatar' : 'smallerHeader']: null };
}

export function effectiveProfileMime(
  profile: IdentityProfileLoadResult | null,
  draft: ProfileDraft,
  field: 'avatar' | 'header'
): string | null {
  const change = draft[field];
  return change.action === 'set' ? change.mimeType : (profile?.[field]?.value.mimeType ?? null);
}
