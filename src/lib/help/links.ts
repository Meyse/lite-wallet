import destinations from './destinations.json';

// Navigation only: this allowlist deliberately contains no signing, recovery
// disclosure, import, or submission actions. The editor embeds a checked snapshot.
export const helpDestinations = destinations;
export type HelpDestination = keyof typeof destinations;
export type HelpSettingsView =
  'private-verus' | 'display-language' | 'profile-security' | 'about-support';
export type HelpTextPart = { text: string; destination?: HelpDestination };

export function isHelpDestination(value: unknown): value is HelpDestination {
  return typeof value === 'string' && Object.hasOwn(destinations, value);
}

// A tiny text format, not HTML or Markdown: [[stable-destination|localized label]].
// Unsupported or malformed tokens stay inert text. Svelte escapes every text part.
export function parseHelpText(value: string): HelpTextPart[] {
  const parts: HelpTextPart[] = [];
  const pattern = /\[\[([a-z-]+)\|([^\]\r\n[]+)\]\]/g;
  let start = 0;
  for (const match of value.matchAll(pattern)) {
    if (!isHelpDestination(match[1]) || !match[2].trim()) continue;
    if (match.index > start) parts.push({ text: value.slice(start, match.index) });
    parts.push({ text: match[2], destination: match[1] });
    start = match.index + match[0].length;
  }
  if (start < value.length) parts.push({ text: value.slice(start) });
  return parts;
}

export function helpPlainText(value: string): string {
  return parseHelpText(value)
    .map((part) => part.text)
    .join('');
}
