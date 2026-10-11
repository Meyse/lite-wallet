import type { RequestStepId } from '$lib/genericRequest/flowPlan';
import * as genericRequestService from '$lib/services/genericRequestService.js';
import type {
  CoinDefinition,
  GenericInvoicePreflightResult,
  GenericInvoiceReview,
} from '$lib/types/wallet.js';
import { formatIdentityDisplayName } from '$lib/utils/identityDisplay.js';
import { extractWalletErrorType } from '$lib/utils/walletErrors.js';
import type { FundingSource, PlanLine, RequestFact } from '../types';
import { loadFundingSources } from './fundingSources';
import type { DetailModule, RequestContext } from './types';

const MAINNET_SYSTEM_ID = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
const TESTNET_SYSTEM_ID = 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq';
/** Rust keeps a payment preflight for 15 minutes; refresh an older one on Confirm. */
const PREFLIGHT_REFRESH_MS = 12 * 60 * 1000;
/** Verus produces about one block a minute. */
const SECONDS_PER_BLOCK = 60;

/**
 * Pay invoice (spec "Steps per detail"). Rust reads the amount, currency, and
 * destination from the verified request. The user picks the source; nothing is
 * preselected, the fee shows after the choice, and the amount is never lowered
 * to cover the fee.
 */
export class PayInvoiceModule implements DetailModule {
  readonly steps: readonly RequestStepId[] = ['payInvoice'];

  review = $state<GenericInvoiceReview | null>(null);
  reviewLoading = $state(false);
  sources = $state<FundingSource[]>([]);
  sourcesLoading = $state(false);
  selectedChannelId = $state('');
  amountInput = $state('');
  preflight = $state<GenericInvoicePreflightResult | null>(null);
  preflightLoading = $state(false);
  error = $state('');
  private reviewedAt = 0;
  private preflightAt = 0;
  private sequence = 0;

  constructor(private readonly ctx: RequestContext) {}

  get acceptsAnyAmount(): boolean {
    return this.review !== null && this.review.amount === null;
  }

  get rootSystemId(): string {
    return this.ctx.session.testnet ? TESTNET_SYSTEM_ID : MAINNET_SYSTEM_ID;
  }

  coin(): CoinDefinition | null {
    const currencyId = this.review?.currencyId.toLowerCase();
    if (!currencyId) return null;
    return (
      this.ctx
        .coins()
        .find(
          (coin) =>
            coin.currencyId.toLowerCase() === currencyId && coin.compatibleChannels.includes('vrpc')
        ) ?? null
    );
  }

  ticker(): string {
    return this.coin()?.displayTicker || this.review?.currencyName || '';
  }

  private feeTicker(): string {
    const root = this.rootSystemId.toLowerCase();
    return (
      this.ctx.coins().find((coin) => coin.currencyId.toLowerCase() === root)?.displayTicker ??
      (this.ctx.session.testnet ? 'VRSCTEST' : 'VRSC')
    );
  }

  private format(value: string | number, ticker: string): string {
    const amount = this.ctx.formatNumber(Number(value), { maximumFractionDigits: 8 });
    return ticker ? `${amount} ${ticker}` : amount;
  }

  /** The amount to pay, as text, once it is known. */
  amountText(): string {
    const value = this.review?.amount ?? this.preflight?.amount ?? null;
    return value === null ? '' : this.format(value, this.ticker());
  }

  feeText(): string {
    return this.preflight ? this.format(this.preflight.fee, this.feeTicker()) : '';
  }

  totalText(): string {
    if (!this.preflight) return '';
    const sameCurrency =
      this.preflight.currencyId.toLowerCase() === this.preflight.feeCurrencyId.toLowerCase();
    if (!sameCurrency) {
      return this.ctx.t('genericRequest.flow.invoice.totalSplit', {
        amount: this.amountText(),
        fee: this.feeText(),
      });
    }
    return this.format(Number(this.preflight.amount) + Number(this.preflight.fee), this.ticker());
  }

  destinationLabel(): string {
    if (!this.review) return '';
    const name = this.review.destinationName
      ? formatIdentityDisplayName({
          identityAddress: this.review.destination,
          fullyQualifiedName: this.review.destinationName,
        })
      : '';
    return name || this.review.destination;
  }

  /** When the invoice stops being payable, in Unix seconds. */
  expiresAt(): number | null {
    if (this.review?.blocksLeft == null) return null;
    return Math.floor(this.reviewedAt / 1000) + this.review.blocksLeft * SECONDS_PER_BLOCK;
  }

  async load(): Promise<void> {
    this.reviewLoading = true;
    this.error = '';
    try {
      this.review = await genericRequestService.reviewGenericInvoice(this.ctx.session.requestHex);
      this.reviewedAt = Date.now();
    } catch (error) {
      this.error = this.ctx.errorMessage(error, 'genericRequest.flow.invoice.error.review');
      return;
    } finally {
      this.reviewLoading = false;
    }
    await this.loadSources();
  }

  private async loadSources(): Promise<void> {
    const coin = this.coin();
    if (!coin) {
      this.sources = [];
      return;
    }
    this.sourcesLoading = true;
    try {
      // Rust pays invoices from the wallet's own address on the Verus chain.
      this.sources = await loadFundingSources({
        coin,
        systemId: this.rootSystemId,
        primaryOnly: true,
        formatBalance: (value, sourceCoin) =>
          this.format(value, sourceCoin.displayTicker || this.ticker()),
      });
    } catch (error) {
      this.sources = [];
      this.error = this.ctx.errorMessage(error, 'genericRequest.error.fundingSources');
    } finally {
      this.sourcesLoading = false;
    }
  }

  async choose(channelId: string): Promise<void> {
    if (channelId === this.selectedChannelId && this.preflight) return;
    this.selectedChannelId = channelId;
    this.preflight = null;
    await this.prepare();
  }

  setAmount(value: string): void {
    if (value === this.amountInput) return;
    this.amountInput = value;
    this.sequence += 1;
    this.preflight = null;
    this.preflightLoading = false;
  }

  amountIsValid(): boolean {
    const value = Number(this.amountInput.trim());
    return /^\d+(\.\d{1,8})?$/.test(this.amountInput.trim()) && value > 0;
  }

  /** Prepares the payment from the chosen source; the fee shows after this. */
  async prepare(): Promise<void> {
    if (!this.selectedChannelId || !this.review || this.review.alreadyPaid) return;
    if (this.acceptsAnyAmount && !this.amountIsValid()) return;
    const sequence = ++this.sequence;
    this.preflightLoading = true;
    this.error = '';
    try {
      const result = await genericRequestService.preflightGenericInvoice(
        this.ctx.session.requestHex,
        this.selectedChannelId,
        this.acceptsAnyAmount ? this.amountInput.trim() : null
      );
      if (sequence !== this.sequence) return;
      this.preflight = result;
      this.preflightAt = Date.now();
    } catch (error) {
      if (sequence !== this.sequence) return;
      this.preflight = null;
      this.error =
        extractWalletErrorType(error) === 'InsufficientFunds'
          ? this.ctx.t('genericRequest.flow.invoice.error.funds', { currency: this.ticker() })
          : this.ctx.errorMessage(error, 'genericRequest.flow.invoice.error.preflight');
    } finally {
      if (sequence === this.sequence) this.preflightLoading = false;
    }
  }

  title(): string {
    return this.ctx.t('genericRequest.flow.title.payInvoice', { app: this.ctx.appLabel() });
  }

  ready(): boolean {
    return !!this.review && !this.review.alreadyPaid && !!this.preflight && !this.preflightLoading;
  }

  planLines(): PlanLine[] {
    return [
      {
        id: 'invoice',
        icon: 'payInvoice',
        text: this.ctx.t('genericRequest.flow.plan.pay', {
          amount: this.amountText(),
          to: this.destinationLabel(),
        }),
        impact: 'spends',
      },
    ];
  }

  confirmFacts(): RequestFact[] {
    if (!this.preflight) return [];
    return [
      { label: this.ctx.t('genericRequest.flow.networkFee'), value: this.feeText() },
      { label: this.ctx.t('genericRequest.flow.invoice.total'), value: this.totalText() },
    ];
  }

  actionLabel(withSignIn: boolean): string {
    return withSignIn
      ? this.ctx.t('genericRequest.flow.action.signInAndPay')
      : this.ctx.t('genericRequest.flow.action.pay', { amount: this.amountText() });
  }

  outcomeTitle(): string {
    return this.ctx.t('genericRequest.flow.outcome.paid', {
      amount: this.amountText(),
      to: this.destinationLabel(),
    });
  }

  receiptFacts(): RequestFact[] {
    return [
      { label: this.ctx.t('genericRequest.flow.invoice.paidTo'), value: this.destinationLabel() },
      { label: this.ctx.t('genericRequest.flow.invoice.total'), value: this.totalText() },
    ];
  }

  transaction = async (): Promise<string> => {
    if (!this.preflight) throw new Error('genericRequest.flow.invoice.error.preflight');
    const result = await genericRequestService.sendGenericInvoicePayment(
      this.ctx.session.requestHex,
      this.preflight.preflightId
    );
    return result.txid;
  };

  beforeConfirm = async (): Promise<void> => {
    if (this.preflight && Date.now() - this.preflightAt > PREFLIGHT_REFRESH_MS) {
      await this.prepare();
    }
  };
}
