import { Buffer } from 'node:buffer';
import { describe, expect, it } from 'vitest';
import {
  IdentityUpdateRequestDetails,
  LoginConsentProvisioningRequest,
  LoginConsentProvisioningResponse,
} from 'verus-typescript-primitives';
import provisioning from './fixtures/legacy-provisioning.json';
import content from './fixtures/legacy-content.json';

// Captured from baseline 30c9518, not regenerated with the candidate. Synthetic
// public values only. These test persisted protocol representations, not live jobs.
describe('legacy protocol representations', () => {
  it('preserves provisioning request JSON', () => {
    expect(new LoginConsentProvisioningRequest(provisioning.request).toJson()).toEqual(
      provisioning.request
    );
  });
  for (const fixture of provisioning.cases) {
    it(`preserves ${fixture.state} response JSON and decision hash`, () => {
      // @ts-expect-error Upstream constructors accept JSON at runtime but type nested results as class instances.
      const response = new LoginConsentProvisioningResponse(fixture.json);
      expect(response.toJson()).toEqual(fixture.json);
      expect(response.getDecisionHash(965771, 2).toString('hex')).toBe(fixture.hash);
    });
  }
  for (const fixture of content) {
    it(`preserves ${fixture.name} identity content and review JSON`, () => {
      const detail = new IdentityUpdateRequestDetails();
      detail.fromBuffer(Buffer.from(fixture.hex, 'hex'));
      expect(detail.toBuffer().toString('hex')).toBe(fixture.hex);
      expect(detail.toCLIJson()).toEqual(fixture.json);
    });
  }
});
