import { describe, expect, it } from 'vitest';
import { get } from 'svelte/store';
import { i18nStore, setLocale, SUPPORTED_LOCALES } from '$lib/i18n';
import { buildHelpArticles } from './catalog';
import { helpDestinations, helpPlainText, isHelpDestination, parseHelpText } from './links';

describe('Help screen links', () => {
  it('keeps destination IDs independent of visible labels and never interprets HTML or URLs', () => {
    expect(parseHelpText('Use [[manage-assets|Currencies]] here.')).toEqual([
      { text: 'Use ' },
      { text: 'Currencies', destination: 'manage-assets' },
      { text: ' here.' },
    ]);
    for (const value of [
      '[[recovery-keys|Secret]]',
      '[[javascript:alert(1)|Run]]',
      '<script>alert(1)</script>',
      '[[send|]]',
      '[[send|Line\nbreak]]',
    ]) {
      expect(parseHelpText(value)).toEqual([{ text: value }]);
    }
    expect(isHelpDestination('__proto__')).toBe(false);
    expect(isHelpDestination('toString')).toBe(false);
  });

  it.each(SUPPORTED_LOCALES)(
    'has localized or fallback links for every allowed screen in %s',
    (locale) => {
      setLocale(locale);
      const articles = buildHelpArticles(get(i18nStore).t);
      const destinations = new Set(
        articles
          .flatMap((a) => [a.summary, ...a.paragraphs])
          .flatMap(parseHelpText)
          .flatMap((part) => (part.destination ? [part.destination] : []))
      );
      expect(destinations).toEqual(new Set(Object.keys(helpDestinations)));
      expect(
        helpPlainText(required(articles.find((a) => a.id === 'assets')).summary)
      ).not.toContain('[[');
      expect(get(i18nStore).t('helpCenter.unlockRequired')).not.toContain('helpCenter.');
      setLocale('en');
    }
  );
});

function required<T>(value: T | null | undefined): T {
  if (value === null || value === undefined) throw new Error('Expected test element or value');
  return value;
}
