import type { RequestStepId } from '$lib/genericRequest/flowPlan';
import type { AppEncryptionDetailSession } from '$lib/genericRequest/session';
import * as walletService from '$lib/services/walletService.js';
import { formatIdentityDisplayName } from '$lib/utils/identityDisplay.js';
import type { PlanLine, RequestFact } from '../types';
import type { DetailModule, RequestContext } from './types';

/**
 * Encryption key (spec "Steps per detail"): the responder is chosen here, and
 * no sign-in response is sent. Rust derives a viewing key and a private
 * address for this app from the wallet's private account after Approval.
 */
export class EncryptionKeyModule implements DetailModule {
  readonly steps: readonly RequestStepId[] = ['encryptionKey'];

  /** Whether this wallet has private addresses; null while it is checked. */
  privateReady = $state<boolean | null>(null);
  spendingKeyAcknowledged = $state(false);
  error = $state('');

  constructor(
    private readonly ctx: RequestContext,
    readonly detail: AppEncryptionDetailSession
  ) {}

  async load(): Promise<void> {
    try {
      this.privateReady = (await walletService.getDlightSeedStatus()).configured;
    } catch (error) {
      this.privateReady = false;
      this.error = this.ctx.errorMessage(error, 'genericRequest.flow.encryptionKey.error.status');
    }
  }

  title(): string {
    return this.ctx.t('genericRequest.flow.title.encryptionKey', { app: this.ctx.appLabel() });
  }

  ready(): boolean {
    return (
      this.privateReady === true &&
      !!this.ctx.responder() &&
      (!this.detail.returnsSpendingKey || this.spendingKeyAcknowledged)
    );
  }

  private responderName(): string {
    const responder = this.ctx.responder();
    return responder ? formatIdentityDisplayName(responder) : '';
  }

  facts(): RequestFact[] {
    return [
      { label: this.ctx.t('genericRequest.flow.encryptionKey.app'), value: this.ctx.appLabel() },
      {
        label: this.ctx.t('genericRequest.flow.encryptionKey.number'),
        value: String(this.detail.derivationNumber),
      },
      {
        label: this.ctx.t('genericRequest.flow.encryptionKey.shares'),
        value: this.detail.returnsSpendingKey
          ? this.ctx.t('genericRequest.flow.encryptionKey.sharesWithSpendingKey')
          : this.ctx.t('genericRequest.flow.encryptionKey.sharesViewing'),
      },
    ];
  }

  planLines(): PlanLine[] {
    return [
      {
        id: 'encryptionKey',
        icon: 'encryptionKey',
        text: this.ctx.t('genericRequest.flow.plan.encryptionKey', {
          app: this.ctx.appLabel(),
          identity: this.responderName(),
        }),
        detail: this.detail.returnsSpendingKey
          ? this.ctx.t('genericRequest.flow.encryptionKey.sharesWithSpendingKey')
          : undefined,
        impact: 'shares',
      },
    ];
  }

  confirmFacts(): RequestFact[] {
    return [];
  }

  actionLabel(): string {
    return this.ctx.t('genericRequest.flow.action.shareKey');
  }

  outcomeTitle(): string {
    return this.ctx.t('genericRequest.flow.outcome.keyShared', { app: this.ctx.appLabel() });
  }

  receiptFacts(): RequestFact[] {
    return [
      { label: this.ctx.t('genericRequest.flow.receipt.sharedAs'), value: this.responderName() },
    ];
  }
}
