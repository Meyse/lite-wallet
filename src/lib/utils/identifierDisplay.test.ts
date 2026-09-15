import { describe, expect, it } from 'vitest';
import { formatIdentifierForDisplay } from './identifierDisplay.js';

describe('identifier display formatting', () => {
  const identifier = 'R1234567890abcdefghijklmnopqrstuvXYZ9876543210';

  it('uses six characters on each side in compact mode', () => {
    expect(formatIdentifierForDisplay(identifier, 'compact')).toBe('R12345…543210');
  });

  it('uses twelve characters on each side in review mode', () => {
    expect(formatIdentifierForDisplay(identifier, 'review')).toBe('R1234567890a…YZ9876543210');
  });

  it('keeps the complete identifier in full mode', () => {
    expect(formatIdentifierForDisplay(identifier, 'full')).toBe(identifier);
  });

  it('does not truncate values that already fit', () => {
    expect(formatIdentifierForDisplay('R12345X654321', 'compact')).toBe('R12345X654321');
  });

  it('trims surrounding whitespace without changing identifier casing', () => {
    expect(formatIdentifierForDisplay('  0xAbCdEf  ', 'full')).toBe('0xAbCdEf');
  });
});
