import { Buffer } from 'buffer';
import { LoginConsentResponse } from 'verus-typescript-primitives';
import { hexToBytes } from '$lib/utils/bytes.js';

/**
 * The webhook body of a legacy login. Verus Mobile posts the response object
 * itself, which axios sends as JSON (`LoginRequestComplete.js`). Rust signs
 * the response and checks that this body repeats it before it posts it.
 */
export function legacyLoginWebhookBody(responseHex: string): string {
  const response = new LoginConsentResponse();
  response.fromBuffer(Buffer.from(hexToBytes(responseHex)), 0);
  return JSON.stringify(response);
}
