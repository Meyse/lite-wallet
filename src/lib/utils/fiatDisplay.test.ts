import { describe, expect, it } from 'vitest';
import {
  filterFiatCurrencyOptions,
  getFiatCurrencyOptions,
  normalizeDisplayCurrency,
} from './fiatDisplay.js';

describe('fiat display catalog', () => {
  const options = getFiatCurrencyOptions();

  it('sources supported display currencies from the fiat catalog', () => {
    expect(options.length).toBeGreaterThan(3);
    expect(options.find((option) => option.code === 'EUR')?.name).toBe('Euro');
    expect(options.find((option) => option.code === 'USD')?.name).toBe('US Dollar');
  });

  it('filters by code or currency name and supports an empty result', () => {
    expect(filterFiatCurrencyOptions(options, 'dollar').map((option) => option.code)).toEqual([
      'AUD',
      'CAD',
      'HKD',
      'NZD',
      'SGD',
      'USD',
    ]);
    expect(filterFiatCurrencyOptions(options, 'eur').map((option) => option.code)).toContain('EUR');
    expect(filterFiatCurrencyOptions(options, 'not-a-currency')).toEqual([]);
  });

  it('normalizes persisted values against the same catalog', () => {
    expect(normalizeDisplayCurrency(' eur ')).toBe('EUR');
    expect(normalizeDisplayCurrency('not-a-currency')).toBe('USD');
  });
});
