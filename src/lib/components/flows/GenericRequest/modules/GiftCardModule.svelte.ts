import type { RequestStepId } from '$lib/genericRequest/flowPlan';
import { rejectionCode } from '$lib/genericRequest/rejections';
import type { GiftCardDetailSession } from '$lib/genericRequest/session';
import * as genericRequestService from '$lib/services/genericRequestService.js';
import type { GiftCardClaimPreflight, GiftCardReview } from '$lib/types/wallet.js';
import { formatIdentityDisplayName } from '$lib/utils/identityDisplay.js';
import { extractWalletErrorType } from '$lib/utils/walletErrors.js';
import type { PlanLine, RequestFact } from '../types';
import type { DetailModule, RequestContext } from './types';

/** Rust keeps a claim preflight for 15 minutes; refresh an older one on Confirm. */
const PREFLIGHT_REFRESH_MS = 12 * 60 * 1000;

/**
 * Gift card claim (Verus Mobile `SpendableKeyRequestInfo.js`): Rust reads the
 * card's key, with its password when it has one, and shows what the key
 * holds. The claim moves its VerusIDs and funds to this wallet's address.
 * Nothing is sent back to the requester.
 */
export class GiftCardModule implements DetailModule {
  readonly steps: readonly RequestStepId[] = ['giftCard'];

  password = $state('');
  review = $state.raw<GiftCardReview | null>(null);
  reviewLoading = $state(false);
  preflight = $state.raw<GiftCardClaimPreflight | null>(null);
  preflightLoading = $state(false);
  /** The claim's transaction IDs, in broadcast order. */
  txids = $state.raw<string[]>([]);
  error = $state('');
  private preflightAt = 0;

  constructor(
    private readonly ctx: RequestContext,
    readonly detail: GiftCardDetailSession
  ) {}

  async load(): Promise<void> {
    if (!this.detail.requiresPassword) await this.unlock();
  }

  /** Reads the card with its password, then prepares the claim. */
  async unlock(): Promise<void> {
    if (this.reviewLoading) return;
    this.reviewLoading = true;
    this.error = '';
    try {
      this.review = await genericRequestService.reviewGiftCard(
        this.ctx.session.requestHex,
        this.detail.requiresPassword ? this.password : null
      );
    } catch (error) {
      this.error =
        rejectionCode(error) === 'giftCardPassword'
          ? this.ctx.t('genericRequest.rejected.giftCardPassword')
          : this.ctx.errorMessage(error, 'genericRequest.flow.giftCard.error.review');
      return;
    } finally {
      this.reviewLoading = false;
    }
    if (this.claimable()) await this.prepare();
  }

  /** Something can be claimed, and every VerusID on the card can move. */
  claimable(): boolean {
    const review = this.review;
    if (!review) return false;
    const holds = review.currencies.length > 0 || review.identities.length > 0;
    return holds && review.identities.every((identity) => identity.claimable);
  }

  async prepare(): Promise<void> {
    this.preflightLoading = true;
    this.error = '';
    try {
      this.preflight = await genericRequestService.preflightGiftCardClaim(
        this.ctx.session.requestHex
      );
      this.preflightAt = Date.now();
    } catch (error) {
      this.preflight = null;
      this.error =
        extractWalletErrorType(error) === 'InsufficientFunds'
          ? this.ctx.t('genericRequest.flow.giftCard.error.fee')
          : this.ctx.errorMessage(error, 'genericRequest.flow.giftCard.error.preflight');
    } finally {
      this.preflightLoading = false;
    }
  }

  private feeTicker(): string {
    const id = this.preflight?.feeCurrencyId.toLowerCase();
    return (
      this.ctx.coins().find((coin) => coin.currencyId.toLowerCase() === id)?.displayTicker ??
      (this.ctx.session.testnet ? 'VRSCTEST' : 'VRSC')
    );
  }

  private amount(value: string, ticker: string): string {
    const amount = this.ctx.formatNumber(Number(value), { maximumFractionDigits: 8 });
    return `${amount} ${ticker}`;
  }

  /** Each currency on the card, as text. */
  currencyTexts(): string[] {
    return (this.review?.currencies ?? []).map((currency) => {
      const coin = this.ctx
        .coins()
        .find((entry) => entry.currencyId.toLowerCase() === currency.currencyId.toLowerCase());
      return this.amount(
        currency.amount,
        coin?.displayTicker || currency.name || currency.currencyId
      );
    });
  }

  identityName(identity: GiftCardReview['identities'][number]): string {
    return formatIdentityDisplayName({
      identityAddress: identity.identityAddress,
      fullyQualifiedName: identity.fullyQualifiedName,
    });
  }

  private summary(): string {
    const parts = [
      ...this.currencyTexts(),
      ...(this.review?.identities ?? []).map((identity) => this.identityName(identity)),
    ];
    return parts.join(', ');
  }

  feeText(): string {
    return this.preflight ? this.amount(this.preflight.fee, this.feeTicker()) : '';
  }

  title(): string {
    return this.ctx.t('genericRequest.flow.title.giftCard');
  }

  ready(): boolean {
    return (
      this.claimable() &&
      !!this.preflight &&
      !this.preflightLoading &&
      !this.reviewLoading &&
      !this.error
    );
  }

  planLines(): PlanLine[] {
    return [
      {
        id: `giftCard:${this.detail.index}`,
        icon: 'giftCard',
        text: this.ctx.t('genericRequest.flow.plan.giftCard', { items: this.summary() }),
        impact: 'spends',
      },
    ];
  }

  confirmFacts(): RequestFact[] {
    if (!this.preflight) return [];
    return [{ label: this.ctx.t('genericRequest.flow.networkFee'), value: this.feeText() }];
  }

  actionLabel(): string {
    return this.ctx.t('genericRequest.flow.action.claim');
  }

  outcomeTitle(): string {
    return this.ctx.t('genericRequest.flow.outcome.claimed');
  }

  receiptFacts(): RequestFact[] {
    const rows: RequestFact[] = [
      { label: this.ctx.t('genericRequest.flow.giftCard.claimed'), value: this.summary() },
    ];
    if ((this.review?.identities.length ?? 0) > 0) {
      rows.push({
        label: this.ctx.t('genericRequest.flow.giftCard.next'),
        value: this.ctx.t('genericRequest.flow.giftCard.linkAfter'),
      });
    }
    return rows;
  }

  transaction = async (): Promise<string> => {
    if (!this.preflight) throw new Error('genericRequest.flow.giftCard.error.preflight');
    const result = await genericRequestService.sendGiftCardClaim(
      this.ctx.session.requestHex,
      this.preflight.preflightId
    );
    this.txids = result.txids;
    return result.txids[result.txids.length - 1] ?? '';
  };

  beforeConfirm = async (): Promise<void> => {
    if (this.preflight && Date.now() - this.preflightAt > PREFLIGHT_REFRESH_MS) {
      await this.prepare();
    }
  };
}
