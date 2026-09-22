export const DEFAULT_REQUEST_TIMEOUT_MS = 20_000;
export const EXTENDED_REQUEST_TIMEOUT_MS = 45_000;

// Tauri invocations cannot be cancelled here. Stop waiting and ignore late
// results; callers remain responsible for rejecting stale session results.
export async function withRequestTimeout<T>(
  request: Promise<T>,
  timeoutMs = DEFAULT_REQUEST_TIMEOUT_MS,
  message = 'Request timed out'
): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      request,
      new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(new DOMException(message, 'TimeoutError')), timeoutMs);
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}
