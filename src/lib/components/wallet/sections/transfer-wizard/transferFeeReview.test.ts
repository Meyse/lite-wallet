import { describe, expect, it } from 'vitest';
import { shouldShowNonDirectTotalDebited, summarizeReviewFees } from './transferFeeReview';

describe('review total debited eligibility', () => {
  it('keeps the total for an ordinary or private non-conversion send', () => {
    expect(
      shouldShowNonDirectTotalDebited({
        conversionEnabled: false,
        hasDistinctBridgeFee: false,
      })
    ).toBe(true);
  });

  it('hides an incomplete total when a distinct bridge fee can apply', () => {
    expect(
      shouldShowNonDirectTotalDebited({
        conversionEnabled: false,
        hasDistinctBridgeFee: true,
      })
    ).toBe(false);
  });
});

describe('review fee summaries', () => {
  it('keeps a single private network fee itemized without a duplicate total', () => {
    expect(summarizeReviewFees([{ kind: 'network', amount: '0.0001', rate: 100 }])).toEqual({
      componentCount: 1,
      totalFiat: 0.01,
      showTotal: false,
    });
  });

  it('shows an aggregate for multiple distinct fee types', () => {
    expect(
      summarizeReviewFees([
        { kind: 'network', amount: '0.0001', rate: 100 },
        { kind: 'conversion', amount: '0.00025', rate: 100 },
      ])
    ).toEqual({
      componentCount: 2,
      totalFiat: 0.035,
      showTotal: true,
    });
  });

  it('does not imply a complete aggregate when an expected fee cannot be valued', () => {
    expect(
      summarizeReviewFees([
        { kind: 'network', amount: '0.0001', rate: 100 },
        { kind: 'bridge', amount: null, rate: null },
      ])
    ).toEqual({
      componentCount: 2,
      totalFiat: null,
      showTotal: false,
    });
  });

  it('deduplicates repeated fee kinds before deciding whether to aggregate', () => {
    expect(
      summarizeReviewFees([
        { kind: 'network', amount: '0.0001', rate: 100 },
        { kind: 'network', amount: '0.0002', rate: 100 },
      ])
    ).toEqual({
      componentCount: 1,
      totalFiat: 0.02,
      showTotal: false,
    });
  });
});
