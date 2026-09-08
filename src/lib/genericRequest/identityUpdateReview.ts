import { ContentMultiMapRemoveKey } from 'verus-typescript-primitives';
import type { TranslationParams } from '$lib/i18n';
import { formatIdentityFullyQualifiedName } from '$lib/utils/identityDisplay';
import type {
  GenericIdentityAuthorities,
  GenericIdentityPrimaryAddressInfo,
} from '$lib/types/wallet.js';

type TranslateFn = (key: string, params?: TranslationParams) => string;

type ReviewTone = 'primary' | 'info' | 'warning';
type ReviewBadgeTone = 'wallet' | 'external';

export type IdentityUpdateOutcome = {
  title: string;
  description: string;
  tone: ReviewTone;
};

export type IdentityUpdateAuthorityChange = {
  id: 'revocation' | 'recovery';
  title: string;
  currentValue: string | null;
  nextValue: string;
};

export type IdentityUpdateBadge = {
  label: string;
  tone: ReviewBadgeTone;
};

export type IdentityUpdatePrimaryAddressChange = {
  id: string;
  title: string;
  description: string;
  address: string;
  badge?: IdentityUpdateBadge;
};

export type IdentityUpdateHighRiskItem = {
  id: string;
  title: string;
  description: string;
  currentValue?: string | null;
  nextValue?: string | null;
};

export type IdentityUpdateContentItem = {
  id: string;
  kind: 'cmm' | 'generic';
  changeType: 'added' | 'appended' | 'removed' | 'updated';
  label: string;
  title: string;
  badges: Array<{
    id: string;
    label: string;
    tone: 'add' | 'remove' | 'info';
  }>;
  currentLabel: string | null;
  currentPreview: string | null;
  nextLabel: string | null;
  nextPreview: string | null;
  rawCurrentValue?: unknown;
  rawRequestedValue?: unknown;
  detailBody?: string | null;
  effectNote?: string | null;
  historyNote?: string | null;
  inspectTitle?: string | null;
  inspectPayload?: {
    current?: unknown;
    requested?: unknown;
  } | null;
  isInspectable: boolean;
};

export type IdentityUpdateReviewModel = {
  highRiskCount: number;
  contentCount: number;
  hasHighRisk: boolean;
  hasContent: boolean;
  isAuthorityOnly: boolean;
  isContentClearOnly: boolean;
  outcome: IdentityUpdateOutcome | null;
  authorityChanges: IdentityUpdateAuthorityChange[];
  primaryAddressesAfterUpdate: Array<{
    address: string;
    displayAddress: string;
    badge: IdentityUpdateBadge;
  }>;
  primaryAddressChanges: IdentityUpdatePrimaryAddressChange[];
  otherHighRiskItems: IdentityUpdateHighRiskItem[];
  contentChanges: IdentityUpdateContentItem[];
};

type BuildIdentityUpdateReviewParams = {
  currentIdentity: Record<string, unknown>;
  requestedIdentity: Record<string, unknown>;
  rawRequestedIdentity: Record<string, unknown>;
  friendlyNames: Record<string, string>;
  signerCmmKeyLabels: Record<string, string>;
  primaryAddressAfterUpdateInfo: GenericIdentityPrimaryAddressInfo;
  currentAuthorities: GenericIdentityAuthorities;
  t: TranslateFn;
};

type ContentChangeAccumulator = {
  contentChanges: IdentityUpdateContentItem[];
  highRiskItems: IdentityUpdateHighRiskItem[];
};

type ContentMultiMapRemoveMeta = {
  action: number;
  entryKey: string | null;
  valueHash: string | null;
};

const IDENTITY_FLAG_REVOKED = 0x8000;

const HIGH_RISK_ROOT_KEYS = new Set([
  'primaryaddresses',
  'recoveryauthority',
  'revocationauthority',
  'flags',
  'minimumsignatures',
]);

const SKIPPED_ROOT_KEYS = new Set([
  'identityaddress',
  'name',
  'fullyqualifiedname',
  'friendlyname',
  'systemid',
  'timelock',
  'txid',
  'vout',
]);

function stableStringify(value: unknown): string | null {
  if (value === undefined || value === null) return null;
  if (typeof value === 'string') return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function stablePreview(value: unknown, maxLength = 120): string | null {
  const stringified = stableStringify(value);
  if (!stringified) return null;

  const collapsed = stringified.replace(/\s+/g, ' ').trim();
  if (!collapsed) return null;

  return collapsed.length > maxLength ? `${collapsed.slice(0, maxLength - 3)}...` : collapsed;
}

function normalizeString(value: unknown): string | null {
  if (typeof value !== 'string') return null;
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function getField(record: Record<string, unknown>, ...keys: string[]): unknown {
  for (const key of keys) {
    if (key in record) return record[key];
  }
  return undefined;
}

function getStringField(record: Record<string, unknown>, ...keys: string[]): string | null {
  return normalizeString(getField(record, ...keys));
}

function getStringArrayField(record: Record<string, unknown>, ...keys: string[]): string[] {
  const value = getField(record, ...keys);
  if (!Array.isArray(value)) return [];
  return value
    .map((entry) => normalizeString(entry))
    .filter((entry): entry is string => entry !== null);
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function isRevokedIdentity(identity: Record<string, unknown>): boolean {
  const flags = getField(identity, 'flags');
  const numericFlags =
    typeof flags === 'number'
      ? flags
      : typeof flags === 'string'
        ? Number(flags)
        : NaN;
  return Number.isFinite(numericFlags) && (numericFlags & IDENTITY_FLAG_REVOKED) === IDENTITY_FLAG_REVOKED;
}

function normalizeFriendlyLabel(value: string | null, friendlyNames: Record<string, string>): string | null {
  if (!value) return null;
  const friendly = friendlyNames[value];
  if (!friendly) return value;
  return formatIdentityFullyQualifiedName(friendly) ?? friendly;
}

function formatContentKeyLabel(
  key: string,
  signerCmmKeyLabels: Record<string, string>,
  t: TranslateFn,
): string {
  const explicitLabel = normalizeString(signerCmmKeyLabels[key]);
  if (explicitLabel) return explicitLabel;
  if (key === ContentMultiMapRemoveKey.vdxfid) {
    return t('genericRequest.update.content.currentIdentityContent');
  }
  if (key.length <= 12) return key;
  return `${key.slice(0, 6)}...${key.slice(-6)}`;
}

function hasReadableContentKeyLabel(
  key: string,
  signerCmmKeyLabels: Record<string, string>,
): boolean {
  const explicitLabel = normalizeString(signerCmmKeyLabels[key]);
  if (explicitLabel) return true;
  if (key === ContentMultiMapRemoveKey.vdxfid) return true;
  return key.length <= 12;
}

function walkNestedContentDiff(
  currentValue: unknown,
  requestedValue: unknown,
  path: string,
  output: IdentityUpdateContentItem[],
  t: TranslateFn,
): void {
  if (JSON.stringify(currentValue) === JSON.stringify(requestedValue)) {
    return;
  }

  const currentIsObject = isPlainObject(currentValue);
  const requestedIsObject = isPlainObject(requestedValue);

  if (currentIsObject && requestedIsObject) {
    const keys = new Set([...Object.keys(currentValue), ...Object.keys(requestedValue)]);
    for (const key of [...keys].sort()) {
      const nextPath = path ? `${path}.${key}` : key;
      walkNestedContentDiff(currentValue[key], requestedValue[key], nextPath, output, t);
    }
    return;
  }

  output.push({
    id: path,
    kind: 'generic',
    changeType: 'updated',
    label: humanizeContentPath(path, t),
    title: humanizeContentPath(path, t),
    badges: [],
    currentLabel: t('genericRequest.update.beforeValue'),
    currentPreview: stablePreview(currentValue, 160),
    nextLabel: t('genericRequest.update.afterValue'),
    nextPreview: stablePreview(requestedValue, 160),
    rawCurrentValue: currentValue,
    rawRequestedValue: requestedValue,
    inspectTitle: humanizeContentPath(path, t),
    inspectPayload: buildInspectPayload(currentValue, requestedValue),
    isInspectable:
      isInspectableValue(currentValue) || isInspectableValue(requestedValue),
  });
}

function humanizeContentPath(path: string, t: TranslateFn): string {
  if (path === 'privateaddress') {
    return t('genericRequest.update.privateAddress');
  }

  return path
    .replace(/^contentmultimap\./i, `${t('genericRequest.update.contentPrefix')}: `)
    .replace(/\./g, ' / ');
}

function normalizeContentMultiMapValues(value: unknown): unknown[] {
  if (Array.isArray(value)) return value;
  if (value == null) return [];
  return [value];
}

function isInspectableValue(value: unknown): boolean {
  if (value === undefined || value === null) return false;
  if (Array.isArray(value)) return value.length > 0;
  if (isPlainObject(value)) return Object.keys(value).length > 0;

  const stringified = stableStringify(value);
  return Boolean(stringified && stringified.length > 120);
}

function buildInspectPayload(
  currentValue: unknown,
  requestedValue: unknown,
): IdentityUpdateContentItem['inspectPayload'] {
  const payload: NonNullable<IdentityUpdateContentItem['inspectPayload']> = {};
  if (currentValue !== undefined) {
    payload.current = currentValue;
  }
  if (requestedValue !== undefined) {
    payload.requested = requestedValue;
  }

  return Object.keys(payload).length > 0 ? payload : null;
}

function makeBadge(
  id: string,
  label: string,
  tone: 'add' | 'remove' | 'info',
): IdentityUpdateContentItem['badges'][number] {
  return { id, label, tone };
}

function extractContentMultiMapRemoveMeta(value: unknown): ContentMultiMapRemoveMeta | null {
  const candidates = Array.isArray(value) ? value : [value];

  for (const candidate of candidates) {
    if (!isPlainObject(candidate)) continue;

    const topLevel = candidate[ContentMultiMapRemoveKey.vdxfid];
    if (!isPlainObject(topLevel)) continue;

    const nested = topLevel[ContentMultiMapRemoveKey.vdxfid];
    const payload = isPlainObject(nested) ? nested : topLevel;
    const action = Number(payload.action);
    if (!Number.isFinite(action)) continue;

    return {
      action,
      entryKey: normalizeString(payload.entrykey),
      valueHash: normalizeString(payload.valuehash),
    };
  }

  return null;
}

function buildContentMultiMapRemoval(
  key: string,
  removeMeta: ContentMultiMapRemoveMeta,
  currentContentMap: Record<string, unknown>,
  signerCmmKeyLabels: Record<string, string>,
  t: TranslateFn,
): IdentityUpdateContentItem | IdentityUpdateHighRiskItem {
  const targetKey = removeMeta.action === 4 ? null : (removeMeta.entryKey ?? key);
  const targetLabel = targetKey
    ? formatContentKeyLabel(targetKey, signerCmmKeyLabels, t)
    : t('genericRequest.update.content.currentIdentityContent');
  const useGenericKeyTitle =
    targetKey !== null && !hasReadableContentKeyLabel(targetKey, signerCmmKeyLabels);

  if (removeMeta.action === 4) {
    return {
      id: `content-clear:${key}`,
      title: t('genericRequest.update.contentClearTitle'),
      description: t('genericRequest.update.contentClearDescription'),
      currentValue: stableStringify(currentContentMap),
      nextValue: t('genericRequest.update.content.noneAfterUpdate'),
    };
  }

  let titleKey = 'genericRequest.update.contentRemoveValueTitle';
  let titleKeyGeneric = 'genericRequest.update.contentRemoveValueTitleGeneric';
  let previewKey = 'genericRequest.update.content.removeValuePreview';
  if (removeMeta.action === 3) {
    titleKey = 'genericRequest.update.contentRemoveAllValuesTitle';
    titleKeyGeneric = 'genericRequest.update.contentRemoveAllValuesTitleGeneric';
    previewKey = 'genericRequest.update.content.removeAllValuesPreview';
  } else if (removeMeta.action === 2) {
    titleKey = 'genericRequest.update.contentRemoveMatchingValuesTitle';
    titleKeyGeneric = 'genericRequest.update.contentRemoveMatchingValuesTitleGeneric';
    previewKey = 'genericRequest.update.content.removeMatchingValuesPreview';
  }

  const currentValue = currentContentMap[targetKey ?? key] ?? null;
  const requestedValue = removeMeta;
  const inspectPayload = buildInspectPayload(currentValue, requestedValue);
  const detailBody =
    useGenericKeyTitle
      ? t('genericRequest.update.content.keyLine', { label: targetLabel })
      : null;
  const currentLabel =
    removeMeta.action === 1
      ? t('genericRequest.update.content.currentValueLabel')
      : t('genericRequest.update.content.currentValuesLabel');

  return {
    id: `content-remove:${key}:${removeMeta.action}`,
    kind: 'cmm',
    changeType: 'removed',
    label: targetLabel,
    title: t(useGenericKeyTitle ? titleKeyGeneric : titleKey, { label: targetLabel }),
    badges: [
      makeBadge('remove', t('genericRequest.update.content.badge.remove'), 'remove'),
    ],
    currentLabel,
    currentPreview: stablePreview(currentValue),
    nextLabel: t('genericRequest.update.content.afterUpdateLabel'),
    nextPreview: t(previewKey, { label: targetLabel }),
    rawCurrentValue: currentValue,
    rawRequestedValue: requestedValue,
    detailBody,
    effectNote: null,
    historyNote:
      removeMeta.valueHash != null
        ? t('genericRequest.update.contentRemoveHistoryHash', {
            hash: `${removeMeta.valueHash.slice(0, 10)}...`,
          })
        : t('genericRequest.update.contentRemoveHistory'),
    inspectTitle: t('genericRequest.update.content.inspectTitle', { label: targetLabel }),
    inspectPayload,
    isInspectable: inspectPayload !== null,
  };
}

function buildContentChanges(
  currentIdentity: Record<string, unknown>,
  requestedIdentity: Record<string, unknown>,
  rawRequestedIdentity: Record<string, unknown>,
  signerCmmKeyLabels: Record<string, string>,
  t: TranslateFn,
): ContentChangeAccumulator {
  const contentChanges: IdentityUpdateContentItem[] = [];
  const highRiskItems: IdentityUpdateHighRiskItem[] = [];
  const currentContentMapRoot = getField(currentIdentity, 'contentmultimap', 'contentMultiMap');
  const requestedContentMapRoot = getField(requestedIdentity, 'contentmultimap', 'contentMultiMap');
  const rawRequestedContentMapRoot = getField(
    rawRequestedIdentity,
    'contentmultimap',
    'contentMultiMap',
  );
  const keys = new Set([...Object.keys(currentIdentity), ...Object.keys(requestedIdentity)]);
  let contentMapHandled = false;

  for (const key of [...keys].sort()) {
    const normalizedKey = key.toLowerCase();
    if (HIGH_RISK_ROOT_KEYS.has(normalizedKey) || SKIPPED_ROOT_KEYS.has(normalizedKey)) {
      continue;
    }

    if (normalizedKey === 'contentmultimap') {
      if (contentMapHandled) {
        continue;
      }
      contentMapHandled = true;

      const currentMap = isPlainObject(currentContentMapRoot) ? currentContentMapRoot : {};
      const requestedMap = isPlainObject(requestedContentMapRoot) ? requestedContentMapRoot : {};
      const rawRequestedMap = isPlainObject(rawRequestedContentMapRoot) ? rawRequestedContentMapRoot : {};
      const contentKeys = new Set([
        ...Object.keys(currentMap),
        ...Object.keys(requestedMap),
        ...Object.keys(rawRequestedMap),
      ]);

      for (const contentKey of [...contentKeys].sort()) {
        const beforeEntry = currentMap[contentKey];
        const afterEntry = requestedMap[contentKey];
        if (JSON.stringify(beforeEntry) === JSON.stringify(afterEntry)) {
          continue;
        }

        const removeMeta = extractContentMultiMapRemoveMeta(rawRequestedMap[contentKey]);
        if (removeMeta) {
          const change = buildContentMultiMapRemoval(
            contentKey,
            removeMeta,
            currentMap,
            signerCmmKeyLabels,
            t,
          );
          if ('description' in change) {
            highRiskItems.push(change);
          } else {
            contentChanges.push(change);
          }
          continue;
        }

        const contentLabel = formatContentKeyLabel(contentKey, signerCmmKeyLabels, t);
        const rawRequestedEntry = rawRequestedMap[contentKey];
        const hasExistingValue = normalizeContentMultiMapValues(beforeEntry).length > 0;
        const requestedPreviewSource = rawRequestedEntry ?? afterEntry;
        const inspectPayload = buildInspectPayload(beforeEntry, requestedPreviewSource);

        contentChanges.push({
          id: `content:${contentKey}`,
          kind: 'cmm',
          changeType: hasExistingValue ? 'appended' : 'added',
          label: contentLabel,
          title: t('genericRequest.update.contentKeyTitle', {
            label: contentLabel,
          }),
          badges: [
            makeBadge(
              hasExistingValue ? 'append' : 'add',
              hasExistingValue
                ? t('genericRequest.update.content.badge.append')
                : t('genericRequest.update.content.badge.add'),
              'add',
            ),
          ],
          currentLabel: hasExistingValue ? t('genericRequest.update.content.existingLabel') : null,
          currentPreview: hasExistingValue ? stablePreview(beforeEntry) : null,
          nextLabel: hasExistingValue
            ? t('genericRequest.update.content.addingLabel')
            : t('genericRequest.update.content.newLabel'),
          nextPreview: stablePreview(requestedPreviewSource),
          rawCurrentValue: beforeEntry,
          rawRequestedValue: requestedPreviewSource,
          inspectTitle: t('genericRequest.update.content.inspectTitle', { label: contentLabel }),
          inspectPayload,
          isInspectable: inspectPayload !== null,
        });
      }
      continue;
    }

    const currentValue = currentIdentity[key];
    const requestedValue = requestedIdentity[key];
    if (JSON.stringify(currentValue) === JSON.stringify(requestedValue)) {
      continue;
    }

    walkNestedContentDiff(currentValue, requestedValue, key, contentChanges, t);
  }

  return { contentChanges, highRiskItems };
}

function buildPrimaryAddressOutcome(
  info: GenericIdentityPrimaryAddressInfo,
  t: TranslateFn,
): IdentityUpdateOutcome | null {
  if (!info.addresses.length) return null;

  if (info.walletCount === 0) {
    return {
      title: t('genericRequest.update.outcome.loseControlTitle'),
      description: t('genericRequest.update.outcome.loseControlDescription'),
      tone: 'warning',
    };
  }

  if (info.externalCount > 0) {
    return {
      title: t('genericRequest.update.outcome.shareControlTitle'),
      description: t('genericRequest.update.outcome.shareControlDescription'),
      tone: 'info',
    };
  }

  return {
    title: t('genericRequest.update.outcome.keepControlTitle'),
    description: t('genericRequest.update.outcome.keepControlDescription'),
    tone: 'primary',
  };
}

export function buildGenericIdentityUpdateReview(
  params: BuildIdentityUpdateReviewParams,
): IdentityUpdateReviewModel {
  const {
    currentIdentity,
    requestedIdentity,
    rawRequestedIdentity,
    friendlyNames,
    signerCmmKeyLabels,
    primaryAddressAfterUpdateInfo,
    currentAuthorities,
    t,
  } = params;

  const authorityChanges: IdentityUpdateAuthorityChange[] = [];
  const nextRevocation = getStringField(requestedIdentity, 'revocationauthority', 'revocationAuthority');
  const nextRecovery = getStringField(requestedIdentity, 'recoveryauthority', 'recoveryAuthority');

  if (normalizeString(currentAuthorities.revocation) !== nextRevocation && nextRevocation) {
    authorityChanges.push({
      id: 'revocation',
      title: t('genericRequest.update.authority.changeRevocation'),
      currentValue: normalizeFriendlyLabel(normalizeString(currentAuthorities.revocation), friendlyNames),
      nextValue: normalizeFriendlyLabel(nextRevocation, friendlyNames) ?? nextRevocation,
    });
  }

  if (normalizeString(currentAuthorities.recovery) !== nextRecovery && nextRecovery) {
    authorityChanges.push({
      id: 'recovery',
      title: t('genericRequest.update.authority.changeRecovery'),
      currentValue: normalizeFriendlyLabel(normalizeString(currentAuthorities.recovery), friendlyNames),
      nextValue: normalizeFriendlyLabel(nextRecovery, friendlyNames) ?? nextRecovery,
    });
  }

  const currentPrimaryAddresses = getStringArrayField(currentIdentity, 'primaryaddresses', 'primaryAddresses');
  const nextPrimaryAddresses = primaryAddressAfterUpdateInfo.addresses.map((entry) => entry.address);
  const currentPrimarySet = new Set(currentPrimaryAddresses);
  const nextPrimarySet = new Set(nextPrimaryAddresses);
  const hasPrimaryAddressChanges =
    currentPrimaryAddresses.length !== nextPrimaryAddresses.length ||
    currentPrimaryAddresses.some((address) => !nextPrimarySet.has(address));

  const primaryAddressesAfterUpdate = primaryAddressAfterUpdateInfo.addresses.map((entry) => ({
    address: entry.address,
    displayAddress: normalizeFriendlyLabel(entry.address, friendlyNames) ?? entry.address,
    badge: entry.inWallet
      ? { label: t('genericRequest.update.badge.inWallet'), tone: 'wallet' as const }
      : { label: t('genericRequest.update.badge.external'), tone: 'external' as const },
  }));

  const primaryAddressChanges: IdentityUpdatePrimaryAddressChange[] = [];
  if (hasPrimaryAddressChanges) {
    const hasWalletAddressAfterUpdate = primaryAddressAfterUpdateInfo.walletCount > 0;

    for (const address of nextPrimaryAddresses) {
      if (currentPrimarySet.has(address)) continue;
      const inWallet = primaryAddressAfterUpdateInfo.addresses.some(
        (entry) => entry.address === address && entry.inWallet,
      );
      primaryAddressChanges.push({
        id: `primary:add:${address}`,
        title: t('genericRequest.update.primaryAddress.addTitle'),
        description: inWallet
          ? t('genericRequest.update.primaryAddress.addWalletDescription')
          : hasWalletAddressAfterUpdate
            ? t('genericRequest.update.primaryAddress.addSharedDescription')
            : t('genericRequest.update.primaryAddress.addLoseControlDescription'),
        address: normalizeFriendlyLabel(address, friendlyNames) ?? address,
        badge: inWallet
          ? { label: t('genericRequest.update.badge.inWallet'), tone: 'wallet' }
          : { label: t('genericRequest.update.badge.external'), tone: 'external' },
      });
    }

    for (const address of currentPrimaryAddresses) {
      if (nextPrimarySet.has(address)) continue;
      primaryAddressChanges.push({
        id: `primary:remove:${address}`,
        title: t('genericRequest.update.primaryAddress.removeTitle'),
        description: t('genericRequest.update.primaryAddress.removeDescription'),
        address: normalizeFriendlyLabel(address, friendlyNames) ?? address,
      });
    }
  }

  const otherHighRiskItems: IdentityUpdateHighRiskItem[] = [];
  const { contentChanges, highRiskItems: contentRiskItems } = buildContentChanges(
    currentIdentity,
    requestedIdentity,
    rawRequestedIdentity,
    signerCmmKeyLabels,
    t,
  );
  otherHighRiskItems.push(...contentRiskItems);

  const revokedBefore = isRevokedIdentity(currentIdentity);
  const revokedAfter = isRevokedIdentity(requestedIdentity);
  if (revokedBefore !== revokedAfter) {
    otherHighRiskItems.push({
      id: 'status:revoked',
      title: t('genericRequest.update.status.title'),
      description: revokedAfter
        ? t('genericRequest.update.status.revokedDescription')
        : t('genericRequest.update.status.activeDescription'),
      currentValue: revokedBefore
        ? t('genericRequest.update.status.revoked')
        : t('genericRequest.update.status.active'),
      nextValue: revokedAfter
        ? t('genericRequest.update.status.revoked')
        : t('genericRequest.update.status.active'),
    });
  }

  const isContentClearOnly =
    !authorityChanges.length &&
    !primaryAddressChanges.length &&
    otherHighRiskItems.length === 1 &&
    otherHighRiskItems[0].id.startsWith('content-clear:');

  const isAuthorityOnly =
    authorityChanges.length > 0 &&
    !primaryAddressChanges.length &&
    !otherHighRiskItems.length;

  let outcome: IdentityUpdateOutcome | null = null;
  if (hasPrimaryAddressChanges) {
    outcome = buildPrimaryAddressOutcome(primaryAddressAfterUpdateInfo, t);
  } else if (isContentClearOnly) {
    outcome = {
      title: t('genericRequest.update.outcome.clearContentTitle'),
      description: t('genericRequest.update.outcome.clearContentDescription'),
      tone: 'warning',
    };
  } else if (!isAuthorityOnly && (authorityChanges.length > 0 || otherHighRiskItems.length > 0)) {
    outcome = {
      title: t('genericRequest.update.outcome.reviewRequiredTitle'),
      description: t('genericRequest.update.outcome.reviewRequiredDescription'),
      tone: 'info',
    };
  }

  const highRiskCount =
    authorityChanges.length + primaryAddressChanges.length + otherHighRiskItems.length;

  return {
    highRiskCount,
    contentCount: contentChanges.length,
    hasHighRisk: highRiskCount > 0,
    hasContent: contentChanges.length > 0,
    isAuthorityOnly,
    isContentClearOnly,
    outcome,
    authorityChanges,
    primaryAddressesAfterUpdate,
    primaryAddressChanges,
    otherHighRiskItems,
    contentChanges,
  };
}
