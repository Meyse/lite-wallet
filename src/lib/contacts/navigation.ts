import { getContext, setContext } from 'svelte';
import type { ContactIdentity } from '$lib/types/addressBook';

export type ContactReturnState = { contactId: string; searchTerm: string };

type OpenContact = (identity: ContactIdentity, returnFocus: HTMLElement | null) => void;
const navigationContext = Symbol('contacts-navigation');

export function provideContactNavigation(open: OpenContact): void {
  setContext(navigationContext, open);
}

export function getContactNavigation(): OpenContact | undefined {
  return getContext<OpenContact | undefined>(navigationContext);
}
