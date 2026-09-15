import { type FiatComponent, sumFiatComponents } from './transferDisplay.js';

export type ReviewFeeKind = 'network' | 'conversion' | 'bridge';

export type ReviewFeeComponent = FiatComponent & {
  kind: ReviewFeeKind;
};

export type ReviewFeeSummary = {
  componentCount: number;
  totalFiat: number | null;
  showTotal: boolean;
};

export function shouldShowNonDirectTotalDebited({
  conversionEnabled,
  hasDistinctBridgeFee,
}: {
  conversionEnabled: boolean;
  hasDistinctBridgeFee: boolean;
}): boolean {
  // The non-direct total currently covers only the sent amount and network fee.
  return !conversionEnabled && !hasDistinctBridgeFee;
}

export function summarizeReviewFees(components: ReviewFeeComponent[]): ReviewFeeSummary {
  const distinctComponents = Array.from(
    new Map(components.map((component) => [component.kind, component])).values()
  );
  const totalFiat = distinctComponents.length > 0 ? sumFiatComponents(distinctComponents) : null;

  return {
    componentCount: distinctComponents.length,
    totalFiat,
    showTotal:
      distinctComponents.length >= 2 &&
      totalFiat !== null &&
      Number.isFinite(totalFiat) &&
      totalFiat > 0,
  };
}
