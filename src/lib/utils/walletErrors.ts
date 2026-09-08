function parseWalletErrorString(error: string): Record<string, unknown> | null {
  try {
    const parsed = JSON.parse(error) as unknown;
    if (!parsed || typeof parsed !== 'object') return null;
    return parsed as Record<string, unknown>;
  } catch {
    return null;
  }
}

function toWalletErrorObject(error: unknown): Record<string, unknown> | null {
  if (typeof error === 'string') return parseWalletErrorString(error);
  if (!error || typeof error !== 'object') return null;
  return error as Record<string, unknown>;
}

export function extractWalletErrorType(error: unknown): string | null {
  const object = toWalletErrorObject(error);
  if (!object) return null;

  if (typeof object.type === 'string' && object.type.trim()) {
    return object.type.trim();
  }

  if (object.data && typeof object.data === 'object') {
    const data = object.data as Record<string, unknown>;
    if (typeof data.type === 'string' && data.type.trim()) {
      return data.type.trim();
    }
  }

  return null;
}

export function isWalletLockedError(error: unknown): boolean {
  return extractWalletErrorType(error) === 'WalletLocked';
}

export function extractWalletErrorMessage(error: unknown): string | null {
  const object = toWalletErrorObject(error);
  if (object) {
    if (typeof object.message === 'string' && object.message.trim()) {
      return object.message.trim();
    }

    if (object.data && typeof object.data === 'object') {
      const data = object.data as Record<string, unknown>;
      if (typeof data.message === 'string' && data.message.trim()) {
        return data.message.trim();
      }
    }
  }

  if (error instanceof Error && error.message.trim()) {
    return error.message.trim();
  }

  return null;
}
