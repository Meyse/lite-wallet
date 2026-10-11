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

  it('shows the text of a string content value instead of its JSON', () => {
    const stringType = 'iK7a5JNJnbeuYWVHCDRpJosj3irGJ5Qa8c';
    const review = buildGenericIdentityUpdateReview({
      ...defaults,
      currentIdentity: { contentmultimap: {} },
      requestedIdentity: { contentmultimap: { [key]: [{ [stringType]: 'Disposable QA note' }] } },
      rawRequestedIdentity: {
        contentmultimap: { [key]: [{ [stringType]: 'Disposable QA note' }] },
      },
    });
    expect(review.contentChanges[0].nextPreview).toBe('Disposable QA note');
  });

  it('names the credentials an update stores by their labels', () => {
    const credentialKey = 'iMRGoNZvSuayeu17PmbXv2CA6eRL4xfmhm';
    const credential = (label: string | null, credentialkey: string) => ({
      iDTG49YLqmkHMYRyuQBYgEyTByQwAzqGd6: {
        version: 1,
        credentialkey,
        credential: { displayName: 'Test user' },
        scopes: [key],
        label,
      },
    });
    const values = [credential('Test profile', unrelated), credential(null, key)];
    const review = buildGenericIdentityUpdateReview({
      ...defaults,
      currentIdentity: { contentmultimap: {} },
      requestedIdentity: { contentmultimap: { [credentialKey]: values } },
      rawRequestedIdentity: { contentmultimap: { [credentialKey]: values } },
    });
    expect(review.contentChanges[0]).toMatchObject({
      label: 'genericRequest.update.content.credentials',
      nextPreview: `Test profile, ${key}`,
    });
  });

  it('treats a private address change as high risk, as Verus Mobile does', () => {
    const next = 'zs13akhfdhypx9a8t2efkfgxpd5d9h3glz7d5pug2ys65mrzt4mw5vjqx0vw29gx82y3e0xjjuhrwn';
    const review = buildGenericIdentityUpdateReview({
      ...defaults,
      currentIdentity: { name: 'player7' },
      requestedIdentity: { name: 'player7', privateaddress: next },
      rawRequestedIdentity: { privateaddress: next },
    });
    expect(review.contentChanges).toHaveLength(0);
    expect(review.hasHighRisk).toBe(true);
    expect(review.otherHighRiskItems).toEqual([
      {
        id: 'private-address',
        title: 'genericRequest.update.privateAddressChange.title',
        description: 'genericRequest.update.privateAddressChange.description',
        currentValue: null,
        nextValue: next,
      },
    ]);
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
