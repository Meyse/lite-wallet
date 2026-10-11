import { Buffer } from 'buffer';
import { describe, expect, it, vi } from 'vitest';
import vectors from './fixtures/request-vectors.json';
import { legacyLoginWebhookBody } from './legacy';

vi.mock('buffer', () => import('node:buffer'));

describe('legacy login webhook body', () => {
  // Rust posts the body only when it repeats these fields of the signed
  // response (`check_webhook_body`).
  it('repeats the signed response that Rust checks', () => {
    const { login, signature } = vectors.legacy;
    const body = JSON.parse(legacyLoginWebhookBody(login.responseHex));
    expect(body.system_id).toBe('iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq');
    expect(body.signing_id).toBe(login.responder);
    expect(body.signature.signature).toBe(Buffer.from(signature, 'hex').toString('base64'));
    expect(body.decision.decision_id).toBe(login.challengeId);
    expect(body.decision.created_at).toBe(login.responseCreatedAt);
    expect(body.decision.request.challenge.challenge_id).toBe(login.challengeId);
  });
});
