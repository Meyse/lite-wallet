import type { RequestStepId } from '$lib/genericRequest/flowPlan';
import type { DataPacketDetailSession } from '$lib/genericRequest/session';
import { formatIdentityDisplayName } from '$lib/utils/identityDisplay.js';
import type { PlanLine, RequestFact } from '../types';
import type { DetailModule, RequestContext } from './types';

/**
 * Sign data (spec "Steps per detail"): every item and statement has its own
 * consent, and Continue needs all of them. Rust builds each hash from the
 * verified request and signs it after Approval.
 */
export class SignDataModule implements DetailModule {
  readonly steps: readonly RequestStepId[] = ['signData'];

  /** Consent per item, then per statement, in request order. */
  consent = $state<boolean[]>([]);

  constructor(
    private readonly ctx: RequestContext,
    readonly detail: DataPacketDetailSession
  ) {
    this.consent = Array.from({ length: this.count }, () => false);
  }

  get count(): number {
    return this.detail.items.length + this.detail.statements.length;
  }

  setConsent(index: number, value: boolean): void {
    this.consent = this.consent.map((current, at) => (at === index ? value : current));
  }

  async load(): Promise<void> {}

  title(): string {
    return this.ctx.t('genericRequest.flow.title.signData', { app: this.ctx.appLabel() });
  }

  ready(): boolean {
    return this.consent.length > 0 && this.consent.every(Boolean) && !!this.ctx.responder();
  }

  private responderName(): string {
    const responder = this.ctx.responder();
    return responder ? formatIdentityDisplayName(responder) : '';
  }

  planLines(): PlanLine[] {
    return [
      {
        id: `signData:${this.detail.index}`,
        icon: 'signData',
        text: this.ctx.t(
          `genericRequest.flow.plan.signData.${this.count === 1 ? 'one' : 'other'}`,
          {
            count: this.count,
            app: this.ctx.appLabel(),
            identity: this.responderName(),
          }
        ),
        impact: 'signs',
      },
    ];
  }

  confirmFacts(): RequestFact[] {
    return [];
  }

  actionLabel(withSignIn: boolean): string {
    return withSignIn
      ? this.ctx.t('genericRequest.flow.action.signInAndSign')
      : this.ctx.t('genericRequest.flow.action.sign');
  }

  outcomeTitle(): string {
    return this.ctx.t(
      `genericRequest.flow.outcome.signedData.${this.count === 1 ? 'one' : 'other'}`,
      {
        count: this.count,
        app: this.ctx.appLabel(),
      }
    );
  }

  receiptFacts(): RequestFact[] {
    return [
      { label: this.ctx.t('genericRequest.flow.receipt.signedAs'), value: this.responderName() },
    ];
  }
}
