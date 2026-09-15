type IdentityDisplayInput = {
  identityAddress: string;
  fullyQualifiedName?: string | null;
  name?: string | null;
};

function normalizeNonEmpty(value: string | null | undefined): string | null {
  const trimmed = value?.trim() ?? '';
  return trimmed.length > 0 ? trimmed : null;
}

function ensureHandleSuffix(value: string): string {
  return value.endsWith('@') ? value : `${value}@`;
}

function isLikelySystemSuffix(value: string): boolean {
  return value.length > 0 && [...value].every((char) => /[A-Z0-9]/.test(char));
}

export function formatIdentityFullyQualifiedName(
  rawValue: string | null | undefined
): string | null {
  const normalized = normalizeNonEmpty(rawValue);
  if (!normalized) return null;

  const withAt = ensureHandleSuffix(normalized);
  const withoutAt = withAt.slice(0, -1);
  const lastDotIndex = withoutAt.lastIndexOf('.');

  if (lastDotIndex <= 0) return withAt;

  const suffix = withoutAt.slice(lastDotIndex + 1);
  if (!isLikelySystemSuffix(suffix)) return withAt;

  const withoutSystem = withoutAt.slice(0, lastDotIndex);
  if (!withoutSystem) return withAt;

  return `${withoutSystem}@`;
}

export function formatIdentityDisplayName(input: IdentityDisplayInput): string {
  const formattedFqn = formatIdentityFullyQualifiedName(input.fullyQualifiedName);
  if (formattedFqn) return formattedFqn;

  const normalizedName = normalizeNonEmpty(input.name);
  if (normalizedName) return ensureHandleSuffix(normalizedName);

  return input.identityAddress;
}
