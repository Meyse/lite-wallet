import type {
  IdentityProfileLoadResult,
  IdentityProfileSnapshot,
  PendingIdentityProfileUpdate,
} from '$lib/types/wallet.js';

function hasAvatar(snapshot: IdentityProfileSnapshot): boolean {
  return Boolean(snapshot.avatarDigest || snapshot.avatarBase64);
}

function hasDescription(snapshot: IdentityProfileSnapshot): boolean {
  return Boolean(snapshot.descriptionDigest || snapshot.description?.trim());
}

function validTxid(value: string | null | undefined): string | null {
  const normalized = value?.trim().toLowerCase() ?? '';
  return /^[0-9a-f]{64}$/.test(normalized) ? normalized : null;
}

export function isCompleteProfileRemoval(
  current: IdentityProfileSnapshot,
  proposed: IdentityProfileSnapshot
): boolean {
  return (
    (hasAvatar(current) || hasDescription(current)) &&
    !hasAvatar(proposed) &&
    !hasDescription(proposed)
  );
}

export function profileUpdateRemovesData(
  current: IdentityProfileSnapshot,
  proposed: IdentityProfileSnapshot
): boolean {
  return (
    (hasAvatar(current) && !hasAvatar(proposed)) ||
    (hasDescription(current) && !hasDescription(proposed))
  );
}

export function pendingProfileMatches(
  pending: PendingIdentityProfileUpdate,
  profile: IdentityProfileLoadResult
): boolean {
  return (
    validTxid(profile.revisionTxid) === validTxid(pending.txid) &&
    validTxid(pending.txid) !== null &&
    profileMatchesSnapshot(pending.proposedProfile, profile)
  );
}

export function profileMatchesSnapshot(
  snapshot: IdentityProfileSnapshot,
  profile: IdentityProfileLoadResult
): boolean {
  const hasResolvedProfileData = Boolean(profile.avatar || profile.description);
  if (
    profile.state === 'unavailable' ||
    (profile.state === 'empty' && hasResolvedProfileData) ||
    (profile.state === 'ready' && !hasResolvedProfileData) ||
    profile.issues.length > 0
  ) {
    return false;
  }

  const avatarDigestMatches = snapshot.avatarDigest
    ? profile.avatar?.source.digest === snapshot.avatarDigest
    : true;
  const avatarValueMatches = snapshot.avatarBase64
    ? profile.avatar?.value.base64 === snapshot.avatarBase64
    : true;
  const avatarAbsenceMatches = hasAvatar(snapshot) || !profile.avatar;

  const descriptionDigestMatches = snapshot.descriptionDigest
    ? profile.description?.source.digest === snapshot.descriptionDigest
    : true;
  const descriptionValueMatches = snapshot.description
    ? profile.description?.value === snapshot.description
    : true;
  const descriptionAbsenceMatches = hasDescription(snapshot) || !profile.description;

  return (
    avatarDigestMatches &&
    avatarValueMatches &&
    avatarAbsenceMatches &&
    descriptionDigestMatches &&
    descriptionValueMatches &&
    descriptionAbsenceMatches
  );
}
