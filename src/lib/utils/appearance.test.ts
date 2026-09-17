import { describe, expect, it } from 'vitest';
import { DEFAULT_APPEARANCE_PREFERENCE, normalizeAppearancePreference } from './appearance.js';

describe('appearance preferences', () => {
  it.each(['light', 'dark', 'system'] as const)('keeps the supported %s preference', (value) => {
    expect(normalizeAppearancePreference(value)).toBe(value);
  });

  it('falls back to system for missing or unsupported values', () => {
    expect(normalizeAppearancePreference(undefined)).toBe(DEFAULT_APPEARANCE_PREFERENCE);
    expect(normalizeAppearancePreference('sepia')).toBe(DEFAULT_APPEARANCE_PREFERENCE);
  });
});
