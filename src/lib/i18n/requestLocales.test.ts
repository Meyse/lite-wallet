import { describe, expect, it } from 'vitest';
import { de } from './locales/de';
import { en } from './locales/en';
import { es } from './locales/es';
import { nl } from './locales/nl';

// The German and Spanish dictionaries start from English, and every lookup
// falls back to English, so an untranslated request string still shows as
// English text. The plan asks for a test that catches it
// (generic-requests-guided-flow.md, phase 1, step 10).
const REQUEST_PREFIX = 'genericRequest.';
const translations = { nl, de, es };

/** Request strings whose correct translation is the English text. */
const SAME_AS_ENGLISH: Record<keyof typeof translations, string[]> = {
  nl: [
    'genericRequest.flow.feeValue',
    'genericRequest.flow.encryptionKey.app',
    'genericRequest.flow.signData.bytes',
    'genericRequest.flow.signData.size',
    'genericRequest.flow.signIn.site',
  ],
  de: [
    'genericRequest.flow.feeValue',
    'genericRequest.flow.encryptionKey.app',
    'genericRequest.flow.signData.bytes',
  ],
  es: [
    'genericRequest.flow.feeValue',
    'genericRequest.flow.invoice.total',
    'genericRequest.flow.signData.bytes',
    'genericRequest.flow.signData.size',
  ],
};

function placeholders(text: string): string {
  return [...text.matchAll(/\{([a-zA-Z0-9_]+)\}/g)]
    .map((match) => match[1])
    .sort()
    .join();
}

const requestKeys = Object.keys(en).filter((key) => key.startsWith(REQUEST_PREFIX));
const locales = Object.keys(translations) as (keyof typeof translations)[];

describe('request copy in every language', () => {
  it.each(locales)('%s translates every request string', (locale) => {
    const dictionary = translations[locale];
    const untranslated = requestKeys.filter(
      (key) => dictionary[key] === undefined || dictionary[key] === en[key]
    );
    expect(untranslated.sort()).toEqual([...SAME_AS_ENGLISH[locale]].sort());
  });

  it.each(locales)('%s has no request string that English lacks', (locale) => {
    const extra = Object.keys(translations[locale]).filter(
      (key) => key.startsWith(REQUEST_PREFIX) && !(key in en)
    );
    expect(extra).toEqual([]);
  });

  it.each(locales)('%s keeps the placeholders of each request string', (locale) => {
    const dictionary = translations[locale];
    const changed = requestKeys.filter(
      (key) => key in dictionary && placeholders(en[key]) !== placeholders(dictionary[key])
    );
    expect(changed).toEqual([]);
  });
});
