import { helpPlainText } from './links';

type Translate = (key: string) => string;

export const helpCategories = [
  'basics',
  'payments',
  'conversions',
  'identity',
  'security',
  'data',
] as const;
export type HelpCategory = (typeof helpCategories)[number];

// Stable IDs are shared by navigation, related articles and search. Content ships
// with the app so recovery guidance is available offline and before unlocking.
const definitions = [
  ['wallet', 'basics', ['recovery', 'networks']],
  ['assets', 'basics', ['networks', 'receive']],
  ['networks', 'basics', ['receive', 'cross-chain']],
  ['receive', 'payments', ['networks', 'pending']],
  ['send', 'payments', ['fees', 'uncertain']],
  ['pending', 'payments', ['cross-chain', 'uncertain']],
  ['fees', 'payments', ['conversion-estimate', 'ethereum']],
  ['uncertain', 'payments', ['ethereum', 'pending']],
  ['convert', 'conversions', ['conversion-estimate', 'cross-chain']],
  ['conversion-estimate', 'conversions', ['fees', 'convert']],
  ['cross-chain', 'conversions', ['ethereum', 'pending']],
  ['ethereum', 'conversions', ['uncertain', 'fees']],
  ['verusid', 'identity', ['link-identity', 'public-profile']],
  ['link-identity', 'identity', ['verusid', 'profile-unavailable']],
  ['public-profile', 'identity', ['publish-profile', 'local-data']],
  ['publish-profile', 'identity', ['profile-pending', 'public-profile']],
  ['profile-pending', 'identity', ['publish-profile', 'uncertain']],
  ['profile-unavailable', 'identity', ['link-identity', 'public-profile']],
  ['recovery', 'security', ['forgot-password', 'private-verus']],
  ['forgot-password', 'security', ['recovery', 'guard']],
  ['guard', 'security', ['recovery', 'requests']],
  ['private-verus', 'security', ['private-send', 'recovery']],
  ['private-send', 'security', ['private-verus', 'pending']],
  ['requests', 'security', ['verusid', 'support']],
  ['contacts', 'data', ['local-data', 'verusid']],
  ['watchlist', 'data', ['networks', 'local-data']],
  ['local-data', 'data', ['recovery', 'public-profile']],
  ['settings', 'data', ['private-verus', 'support']],
  ['support', 'data', ['pending', 'forgot-password']],
] as const;

export type HelpArticleId = (typeof definitions)[number][0];
export type HelpArticle = {
  id: HelpArticleId;
  category: HelpCategory;
  title: string;
  summary: string;
  paragraphs: string[];
  keywords: string;
  related: readonly HelpArticleId[];
  source?: { label: string; url: string };
};

const sources: Partial<Record<HelpArticleId, { labelKey: string; url: string }>> = {
  verusid: { labelKey: 'helpCenter.source.identity', url: 'https://verus.io/verusid' },
  guard: { labelKey: 'helpCenter.source.identity', url: 'https://verus.io/verusid' },
  convert: {
    labelKey: 'helpCenter.source.conversions',
    url: 'https://verus.io/build/defi-payments',
  },
  'cross-chain': { labelKey: 'helpCenter.source.bridge', url: 'https://verus.io/ethereum-bridge' },
};

export const suggestedArticles: readonly HelpArticleId[] = [
  'pending',
  'forgot-password',
  'public-profile',
  'convert',
  'recovery',
];

export function buildHelpArticles(t: Translate): HelpArticle[] {
  return definitions.map(([id, category, related]) => {
    const prefix = `helpCenter.article.${id}`;
    const source = sources[id];
    return {
      id,
      category,
      related,
      title: t(`${prefix}.title`),
      summary: t(`${prefix}.summary`),
      paragraphs: t(`${prefix}.body`).split('\n\n'),
      keywords: t(`${prefix}.keywords`),
      source: source ? { label: t(source.labelKey), url: source.url } : undefined,
    };
  });
}

function normalize(text: string): string {
  return text
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase();
}

export function searchHelpArticles(articles: HelpArticle[], query: string): HelpArticle[] {
  const terms = normalize(query).trim().split(/\s+/).filter(Boolean);
  if (!terms.length) return articles;
  return articles
    .map((article) => {
      const title = normalize(article.title);
      const keywords = normalize(article.keywords);
      const body = normalize(helpPlainText(`${article.summary} ${article.paragraphs.join(' ')}`));
      const score = terms.reduce((total, term) => {
        const weight = title.includes(term)
          ? 5
          : keywords.includes(term)
            ? 3
            : body.includes(term)
              ? 1
              : 0;
        return total === 0 || weight === 0 ? 0 : total + weight;
      }, 1);
      return { article, score };
    })
    .filter(({ score }) => score > 0)
    .sort((a, b) => b.score - a.score)
    .map(({ article }) => article);
}
