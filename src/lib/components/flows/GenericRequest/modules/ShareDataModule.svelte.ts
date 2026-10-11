import type { RequestStepId } from '$lib/genericRequest/flowPlan';
import { rejectionCode } from '$lib/genericRequest/rejections';
import type { UserDataDetailSession } from '$lib/genericRequest/session';
import * as genericRequestService from '$lib/services/genericRequestService.js';
import * as walletService from '$lib/services/walletService.js';
import type { GenericCredentialReview } from '$lib/types/wallet.js';
import { formatIdentityDisplayName } from '$lib/utils/identityDisplay.js';
import type { PlanLine, RequestFact } from '../types';
import type { DetailModule, RequestContext } from './types';

/**
 * Share data (spec "Steps per detail"): Rust reads and decrypts the
 * responder's credentials that the app asks for. Each credential has its own
 * consent, and the response shares exactly the credentials shown here.
 */
export class ShareDataModule implements DetailModule {
  readonly steps: readonly RequestStepId[] = ['shareData'];

  /** Whether this wallet has private addresses; null while it is checked. */
  privateReady = $state<boolean | null>(null);
  review = $state.raw<GenericCredentialReview | null>(null);
  /** The VerusID that `review` belongs to. */
  reviewedFor = $state('');
  loading = $state(false);
  consent = $state<boolean[]>([]);
  error = $state('');
  private sequence = 0;

  constructor(
    private readonly ctx: RequestContext,
    readonly detail: UserDataDetailSession
  ) {}

  async load(): Promise<void> {
    try {
      this.privateReady = (await walletService.getDlightSeedStatus()).configured;
    } catch (error) {
      this.privateReady = false;
      this.error = this.ctx.errorMessage(error, 'genericRequest.flow.encryptionKey.error.status');
    }
  }

  /** Reads the credentials of `identityId`, unless they are already shown. */
  async lookUp(identityId: string, force = false): Promise<void> {
    if (!this.privateReady) return;
    if (!force && identityId.toLowerCase() === this.reviewedFor.toLowerCase()) return;
    const sequence = ++this.sequence;
    this.review = null;
    this.reviewedFor = identityId;
    this.consent = [];
    this.error = '';
    this.loading = true;
    try {
      const review = await genericRequestService.reviewGenericCredentials(
        this.ctx.session.requestHex,
        identityId
      );
      if (sequence !== this.sequence) return;
      this.review = review;
      this.consent = review.credentials.map(() => false);
    } catch (error) {
      if (sequence !== this.sequence) return;
      // A failed read is never an empty answer (spec "Share data").
      if (rejectionCode(error) === 'privateNotSetUp') this.privateReady = false;
      this.error = this.ctx.errorMessage(error, 'genericRequest.flow.shareData.error.lookup');
    } finally {
      if (sequence === this.sequence) this.loading = false;
    }
  }

  setConsent(index: number, value: boolean): void {
    this.consent = this.consent.map((current, at) => (at === index ? value : current));
  }

  /** The number of credentials the response shares. */
  get count(): number {
    return this.review?.credentials.length ?? 0;
  }

  /** Whether another detail answers the request when no credential matches. */
  get answersOtherwise(): boolean {
    return this.ctx.session.details.some((detail) => detail.type !== 'userData');
  }

  title(): string {
    return this.ctx.t('genericRequest.flow.title.shareData', { app: this.ctx.appLabel() });
  }

  ready(): boolean {
    const responder = this.ctx.responder();
    if (!responder || this.loading || this.error || !this.review) return false;
    if (responder.identityAddress.toLowerCase() !== this.reviewedFor.toLowerCase()) return false;
    // As on Verus Mobile, a request with only this detail and no matching
    // credential has nothing to send.
    if (this.count === 0) return this.answersOtherwise;
    return this.consent.every(Boolean);
  }

  private responderName(): string {
    const responder = this.ctx.responder();
    return responder ? formatIdentityDisplayName(responder) : '';
  }

  planLines(): PlanLine[] {
    if (this.count === 0) return [];
    return [
      {
        id: `shareData:${this.detail.index}`,
        icon: 'shareData',
        text: this.ctx.t(
          `genericRequest.flow.plan.shareData.${this.count === 1 ? 'one' : 'other'}`,
          { count: this.count, app: this.ctx.appLabel(), identity: this.responderName() }
        ),
        impact: 'shares',
      },
    ];
  }

  confirmFacts(): RequestFact[] {
    return [];
  }

  actionLabel(withSignIn: boolean): string {
    return withSignIn
      ? this.ctx.t('genericRequest.flow.action.signInAndShare')
      : this.ctx.t('genericRequest.flow.action.share');
  }

  outcomeTitle(): string {
    return this.ctx.t(
      `genericRequest.flow.outcome.sharedData.${this.count === 1 ? 'one' : 'other'}`,
      { count: this.count, app: this.ctx.appLabel() }
    );
  }

  receiptFacts(): RequestFact[] {
    return [
      { label: this.ctx.t('genericRequest.flow.receipt.sharedAs'), value: this.responderName() },
    ];
  }
}
