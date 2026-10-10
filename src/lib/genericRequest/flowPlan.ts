import type { SupportedGenericRequestDetail } from './session';

/** What a detail can do to the user (CONTEXT.md "Impact"). */
export type RequestImpact = 'signs' | 'shares' | 'spends' | 'control';

/** One screen in the guided flow. Update identity takes two (spec "Steps per detail"). */
export type RequestStepId = 'signIn' | 'newIdentity' | 'updateChanges' | 'networkFee' | 'confirm';

export type RequestFlowPlan = {
  /** Single screen for one detail that only signs as the user; otherwise the rail. */
  layout: 'single' | 'rail';
  steps: RequestStepId[];
};

/** Stages after Approval run in this order (ADR 0001). */
export type ApprovalStageId = 'transaction' | 'signing' | 'delivery';

export const DETAIL_IMPACT: Record<SupportedGenericRequestDetail['type'], RequestImpact> = {
  authentication: 'signs',
  provisionIdentity: 'signs',
  identityUpdate: 'control',
};

/**
 * Builds the steps for a request. A provisioning detail becomes its own step only
 * when the user asks for a new VerusID; otherwise the sign in answers it, as in
 * Verus Mobile, where the sign in marks the provisioning detail handled.
 */
export function planRequestFlow(
  details: readonly SupportedGenericRequestDetail[],
  options: { newIdentity: boolean }
): RequestFlowPlan {
  const steps: RequestStepId[] = [];
  const answered: SupportedGenericRequestDetail[] = [];
  for (const detail of details) {
    if (detail.type === 'authentication') {
      steps.push('signIn');
      answered.push(detail);
    } else if (detail.type === 'provisionIdentity') {
      if (options.newIdentity) {
        steps.push('newIdentity');
        answered.push(detail);
      }
    } else if (detail.type === 'identityUpdate') {
      steps.push('updateChanges', 'networkFee');
      answered.push(detail);
    }
  }

  const single =
    answered.length === 1 && DETAIL_IMPACT[answered[0].type] === 'signs' && steps.length === 1;
  return single ? { layout: 'single', steps } : { layout: 'rail', steps: [...steps, 'confirm'] };
}

/** Stages that apply to a request, in their fixed order. */
export function approvalStages(input: {
  hasTransaction: boolean;
  signsResponse: boolean;
  delivers: boolean;
}): ApprovalStageId[] {
  const stages: ApprovalStageId[] = [];
  if (input.hasTransaction) stages.push('transaction');
  if (input.signsResponse) stages.push('signing');
  if (input.signsResponse && input.delivers) stages.push('delivery');
  return stages;
}

export type ApprovalStageStatus = 'pending' | 'running' | 'done' | 'failed';

/**
 * Runs the stages in order and stops at the first failure. A finished stage never
 * runs again, so a retry after a failed signing or delivery cannot broadcast the
 * same transaction twice within one open request.
 */
export class ApprovalRun {
  readonly status: Record<ApprovalStageId, ApprovalStageStatus>;
  readonly errors: Partial<Record<ApprovalStageId, unknown>> = {};
  private running = false;

  constructor(
    readonly stages: readonly ApprovalStageId[],
    private readonly handlers: Record<ApprovalStageId, () => Promise<void>>,
    private readonly onChange: () => void = () => {}
  ) {
    this.status = { transaction: 'pending', signing: 'pending', delivery: 'pending' };
  }

  get finished(): boolean {
    return this.stages.every((stage) => this.status[stage] === 'done');
  }

  get failedStage(): ApprovalStageId | null {
    return this.stages.find((stage) => this.status[stage] === 'failed') ?? null;
  }

  get isRunning(): boolean {
    return this.running;
  }

  /** Runs every stage that is not done yet. Returns true when all stages are done. */
  async run(): Promise<boolean> {
    if (this.running) return false;
    this.running = true;
    try {
      for (const stage of this.stages) {
        if (this.status[stage] === 'done') continue;
        this.status[stage] = 'running';
        delete this.errors[stage];
        this.onChange();
        try {
          await this.handlers[stage]();
          this.status[stage] = 'done';
          this.onChange();
        } catch (error) {
          this.status[stage] = 'failed';
          this.errors[stage] = error;
          this.onChange();
          return false;
        }
      }
      return true;
    } finally {
      this.running = false;
      this.onChange();
    }
  }
}

export type TimeLeft =
  { kind: 'expired' } | { kind: 'minutes'; minutes: number } | { kind: 'hours'; hours: number };

/** One expiry format for every request (spec "Requester header"). */
export function timeLeft(expiresAtSeconds: number, nowSeconds: number): TimeLeft {
  const remaining = expiresAtSeconds - nowSeconds;
  if (remaining <= 0) return { kind: 'expired' };
  const minutes = Math.ceil(remaining / 60);
  if (minutes < 90) return { kind: 'minutes', minutes };
  return { kind: 'hours', hours: Math.round(minutes / 60) };
}
