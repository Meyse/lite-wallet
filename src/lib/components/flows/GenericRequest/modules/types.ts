import type { RequestStepId } from '$lib/genericRequest/flowPlan';
import type { GenericRequestFlowSession } from '$lib/genericRequest/session';
import type { CoinDefinition, LinkedIdentity } from '$lib/types/wallet.js';
import type { PlanLine, RequestFact } from '../types';

/**
 * What a detail module gets from the shell. The shell owns the layout, the
 * rail, the responder, Confirm, the stages, and the outcome (plan step 7).
 */
export type RequestContext = {
  session: GenericRequestFlowSession;
  t: (key: string, params?: Record<string, string | number>) => string;
  formatNumber: (value: number, options?: Intl.NumberFormatOptions) => string;
  appLabel: () => string;
  responder: () => LinkedIdentity | null;
  coins: () => CoinDefinition[];
  /** Turns a wallet error into copy, with a fallback key for unknown errors. */
  errorMessage: (error: unknown, fallbackKey: string) => string;
};

/**
 * One detail type in the guided flow. A module provides its steps, their
 * screens, the Confirm line, and, when it broadcasts, the on-chain stage.
 * Nothing in a module signs, broadcasts, or sends before Approval (ADR 0001).
 */
export interface DetailModule {
  readonly steps: readonly RequestStepId[];
  /** Starts the review reads for this detail. */
  load(): Promise<void>;
  title(step: RequestStepId): string;
  ready(step: RequestStepId): boolean;
  planLines(): PlanLine[];
  /** Facts that Confirm lists after the plan lines, such as a fee and a total. */
  confirmFacts(): RequestFact[];
  /** The commit button's wording when this detail leads, for example "Pay". */
  actionLabel(withSignIn: boolean): string;
  /** The Done title when this detail leads. */
  outcomeTitle(): string;
  receiptFacts(): RequestFact[];
  /** The on-chain stage after Approval. Returns the transaction ID. */
  transaction?: () => Promise<string>;
  /** Refreshes anything that may be stale when Confirm opens. */
  beforeConfirm?: () => Promise<void>;
}
