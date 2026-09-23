import type { HelpArticleId, HelpCategory } from './catalog';
import type { HelpDestination } from './links';

export type HelpVisit = {
  articleId: HelpArticleId | null;
  category: HelpCategory | null;
  query: string;
  scroll: number;
};
export type HelpReaderState = HelpVisit & {
  history: HelpVisit[];
  focusDestination: HelpDestination | null;
};

// Owned by the dialog host so closing/unmounting the reader retains the visit.
// Never persisted to disk and reset with the wallet unlock session.
export function createHelpReaderState(articleId: HelpArticleId | null = null): HelpReaderState {
  return { articleId, category: null, query: '', scroll: 0, history: [], focusDestination: null };
}
