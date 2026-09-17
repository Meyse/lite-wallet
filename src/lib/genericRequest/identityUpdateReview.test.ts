import { describe, expect, it } from 'vitest';
import { ContentMultiMapRemoveKey } from 'verus-typescript-primitives';
import { buildGenericIdentityUpdateReview } from './identityUpdateReview';

const key = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
const unrelated = 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq';
const defaults = {
  friendlyNames: {},
  signerCmmKeyLabels: {},
  primaryAddressAfterUpdateInfo: { addresses: [], walletCount: 0, externalCount: 0 },
  currentAuthorities: {},
  t: (value: string) => value,
};

describe('identity content review compatibility', () => {
  it('shows only appended content while retaining unrelated unknown values', () => {
    const currentIdentity = { contentmultimap: { [key]: ['deadbeef'], [unrelated]: ['cafe'] } };
    const requestedIdentity = {
      contentmultimap: { [key]: ['deadbeef', 'abcd'], [unrelated]: ['cafe'] },
    };
    const review = buildGenericIdentityUpdateReview({
      ...defaults,
      currentIdentity,
      requestedIdentity,
      rawRequestedIdentity: { contentmultimap: { [key]: ['abcd'] } },
    });
    expect(review.contentChanges).toHaveLength(1);
    expect(review.contentChanges[0]).toMatchObject({
      changeType: 'appended',
      inspectPayload: { current: ['deadbeef'], requested: ['abcd'] },
    });
    expect(currentIdentity.contentmultimap[unrelated]).toEqual(['cafe']);
  });

  it('recognizes the nested VDXF remove operation in review JSON', () => {
    const removal = { [ContentMultiMapRemoveKey.vdxfid]: { version: 1, action: 3, entrykey: key } };
    const review = buildGenericIdentityUpdateReview({
      ...defaults,
      currentIdentity: { contentmultimap: { [key]: ['deadbeef'], [unrelated]: ['cafe'] } },
      requestedIdentity: { contentmultimap: { [unrelated]: ['cafe'] } },
      rawRequestedIdentity: { contentmultimap: { [key]: [removal] } },
    });
    expect(review.contentChanges).toHaveLength(1);
    expect(review.contentChanges[0]).toMatchObject({
      changeType: 'removed',
      title: 'genericRequest.update.contentRemoveAllValuesTitleGeneric',
    });
  });
});
