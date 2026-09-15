export type IdentifierDisplayMode = 'compact' | 'review' | 'full';

const IDENTIFIER_SEGMENTS: Record<
  Exclude<IdentifierDisplayMode, 'full'>,
  readonly [number, number]
> = {
  compact: [6, 6],
  review: [12, 12],
};

export function formatIdentifierForDisplay(
  value: string,
  mode: IdentifierDisplayMode = 'full'
): string {
  const normalized = value.trim();
  if (!normalized || mode === 'full') return normalized;

  const [startLength, endLength] = IDENTIFIER_SEGMENTS[mode];
  if (normalized.length <= startLength + endLength + 1) return normalized;

  return `${normalized.slice(0, startLength)}…${normalized.slice(-endLength)}`;
}
