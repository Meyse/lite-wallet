export type IdentityUpdateContentChange = {
  path: string;
  beforeValue: string | null;
  afterValue: string | null;
};

const HIGH_RISK_ROOT_KEYS = new Set([
  'primaryaddresses',
  'recoveryauthority',
  'revocationauthority',
  'flags',
  'minimumsignatures'
]);

const SKIPPED_ROOT_KEYS = new Set([
  'identityaddress',
  'name',
  'fullyqualifiedname',
  'friendlyname',
  'systemid',
  'timelock',
  'txid',
  'vout'
]);

function stableStringify(value: unknown): string | null {
  if (value === undefined || value === null) return null;
  if (typeof value === 'string') return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function walkDiff(
  currentValue: unknown,
  requestedValue: unknown,
  path: string,
  output: IdentityUpdateContentChange[]
): void {
  if (JSON.stringify(currentValue) === JSON.stringify(requestedValue)) {
    return;
  }

  const currentIsObject =
    currentValue !== null && typeof currentValue === 'object' && !Array.isArray(currentValue);
  const requestedIsObject =
    requestedValue !== null && typeof requestedValue === 'object' && !Array.isArray(requestedValue);

  if (currentIsObject && requestedIsObject) {
    const currentRecord = currentValue as Record<string, unknown>;
    const requestedRecord = requestedValue as Record<string, unknown>;
    const keys = new Set([...Object.keys(currentRecord), ...Object.keys(requestedRecord)]);

    for (const key of [...keys].sort()) {
      const nextPath = path ? `${path}.${key}` : key;
      walkDiff(currentRecord[key], requestedRecord[key], nextPath, output);
    }
    return;
  }

  output.push({
    path,
    beforeValue: stableStringify(currentValue),
    afterValue: stableStringify(requestedValue)
  });
}

export function collectIdentityUpdateContentChanges(
  currentIdentity: Record<string, unknown>,
  requestedIdentity: Record<string, unknown>
): IdentityUpdateContentChange[] {
  const output: IdentityUpdateContentChange[] = [];
  const keys = new Set([...Object.keys(currentIdentity), ...Object.keys(requestedIdentity)]);

  for (const key of [...keys].sort()) {
    const normalizedKey = key.toLowerCase();
    if (HIGH_RISK_ROOT_KEYS.has(normalizedKey) || SKIPPED_ROOT_KEYS.has(normalizedKey)) {
      continue;
    }

    walkDiff(currentIdentity[key], requestedIdentity[key], key, output);
  }

  return output;
}

export function humanizeIdentityUpdatePath(path: string): string {
  return path
    .replace(/^contentmultimap\./i, 'Content: ')
    .replace(/^privateaddress$/i, 'Private address')
    .replace(/\./g, ' / ');
}
