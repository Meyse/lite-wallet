export const APPEARANCE_PREFERENCES = ['light', 'dark', 'system'] as const;

export type AppearancePreference = (typeof APPEARANCE_PREFERENCES)[number];

export const DEFAULT_APPEARANCE_PREFERENCE: AppearancePreference = 'system';

export function normalizeAppearancePreference(value: unknown): AppearancePreference {
  return typeof value === 'string' && APPEARANCE_PREFERENCES.includes(value as AppearancePreference)
    ? (value as AppearancePreference)
    : DEFAULT_APPEARANCE_PREFERENCE;
}
