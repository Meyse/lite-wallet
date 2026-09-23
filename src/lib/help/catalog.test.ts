import { describe, expect, it } from 'vitest';
import { en } from '$lib/i18n/locales/en';
import { nl } from '$lib/i18n/locales/nl';
import { de } from '$lib/i18n/locales/de';
import { es } from '$lib/i18n/locales/es';
import {
  buildHelpArticles,
  helpCategories,
  searchHelpArticles,
  suggestedArticles,
} from './catalog';

describe('offline help catalog', () => {
  it('keeps localized unlock guidance in languages using English Help articles', () => {
    expect(de['helpCenter.unlockRequired']).toBe(
      'Entsperre deine Wallet, um diesen Bildschirm zu öffnen.'
    );
    expect(es['helpCenter.unlockRequired']).toBe('Desbloquea tu cartera para abrir esta pantalla.');
    expect(de['helpCenter.article.wallet.title']).toBe(en['helpCenter.article.wallet.title']);
    expect(es['helpCenter.article.wallet.title']).toBe(en['helpCenter.article.wallet.title']);
  });

  it.each([
    ['English', en],
    ['Dutch', nl],
  ] as const)('has complete, connected %s articles', (_, dictionary) => {
    const articles = buildHelpArticles((key) => dictionary[key] ?? key);
    const ids = new Set(articles.map((article) => article.id));
    expect(ids.size).toBe(articles.length);
    expect(articles).toHaveLength(29);
    expect([...ids]).not.toContain('balances');
    expect(new Set(articles.map((article) => article.category))).toEqual(new Set(helpCategories));
    for (const article of articles) {
      for (const value of [
        article.title,
        article.summary,
        article.keywords,
        ...article.paragraphs,
      ]) {
        expect(value.trim()).not.toBe('');
        expect(value).not.toContain('helpCenter.');
      }
      for (const id of article.related) expect(ids.has(id)).toBe(true);
      if (article.source) expect(new URL(article.source.url).hostname).toBe('verus.io');
    }
    for (const id of suggestedArticles) expect(ids.has(id)).toBe(true);
  });

  it('finds everyday language, combines terms, and handles case, accents and no matches', () => {
    const articles = buildHelpArticles((key) => en[key]);
    expect(searchHelpArticles(articles, '  STUCK  ')[0].id).toBe('pending');
    expect(searchHelpArticles(articles, 'swap')[0].id).toBe('convert');
    expect(searchHelpArticles(articles, 'forgot password')[0].id).toBe('forgot-password');
    expect(searchHelpArticles(articles, 'seed backup').map((article) => article.id)).toContain(
      'recovery'
    );
    expect(searchHelpArticles(articles, 'nonexistent query')).toEqual([]);
    const dutch = buildHelpArticles((key) => nl[key]);
    expect(searchHelpArticles(dutch, 'prive')).toEqual(searchHelpArticles(dutch, 'privé'));
  });
});
