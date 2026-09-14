<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { resolveCoinPresentationById } from '$lib/coins/presentation.js';
  import { i18nStore } from '$lib/i18n';
  import type { EthPendingSubmissionReview } from '$lib/types/wallet.js';

  type Props = {
    review: EthPendingSubmissionReview;
    recovering: boolean;
    onrecover: () => void;
  };

  let { review, recovering, onrecover }: Props = $props();
  const i18n = $derived($i18nStore);
  const presentation = $derived(resolveCoinPresentationById(review.context.coinId));
  const ticker = $derived(presentation?.displayTicker?.trim() || review.context.coinId);

  function truncateAddress(value: string): string {
    return value.length <= 23 ? value : `${value.slice(0, 10)}…${value.slice(-10)}`;
  }

  function stageLabel(stage: string): string {
    if (
      stage === 'eth' ||
      stage === 'erc20' ||
      stage === 'bridge_zero_approval' ||
      stage === 'bridge_approval' ||
      stage === 'bridge_transfer'
    ) {
      return i18n.t(`wallet.transfer.ethRecovery.stage.${stage}`);
    }
    return i18n.t('wallet.transfer.ethRecovery.stage.unknown');
  }
</script>

<section
  class="rounded-lg border border-amber-300/70 bg-amber-50 px-3 py-3 text-amber-950 dark:border-amber-500/35 dark:bg-amber-500/12 dark:text-amber-100"
  aria-labelledby="eth-recovery-title"
>
  <div class="space-y-3">
    <div>
      <p id="eth-recovery-title" class="text-sm font-semibold">
        {i18n.t('wallet.transfer.ethRecovery.title')}
      </p>
      <p class="mt-0.5 text-xs opacity-80">
        {i18n.t('wallet.transfer.ethRecovery.description', {
          value: `${review.context.value} ${ticker}`,
          recipient: truncateAddress(review.context.toAddress),
        })}
      </p>
    </div>
    <dl
      class="grid gap-x-5 gap-y-2 rounded-md bg-white/55 px-3 py-2.5 text-xs sm:grid-cols-2 dark:bg-black/15"
    >
      <div>
        <dt class="opacity-65">{i18n.t('wallet.transfer.summary.amount')}</dt>
        <dd class="mt-0.5 font-medium tabular-nums">{review.context.value} {ticker}</dd>
      </div>
      <div>
        <dt class="opacity-65">{i18n.t('wallet.transfer.ethRecovery.network')}</dt>
        <dd class="mt-0.5 font-medium">
          {i18n.t(`common.network.${review.context.walletNetwork}`)} · {i18n.t(
            'wallet.transfer.ethRecovery.chain',
            { value: review.context.chainId }
          )}
        </dd>
      </div>
      <div class="sm:col-span-2">
        <dt class="opacity-65">{i18n.t('wallet.transfer.summary.recipient')}</dt>
        <dd class="identifier-text mt-0.5 font-medium break-all">{review.context.toAddress}</dd>
      </div>
      <div>
        <dt class="opacity-65">{i18n.t('wallet.transfer.summary.networkFee')}</dt>
        <dd class="mt-0.5 font-medium tabular-nums">
          {review.context.fee}
          {review.context.feeCurrency}
        </dd>
      </div>
      <div>
        <dt class="opacity-65">{i18n.t('wallet.transfer.ethRecovery.stage')}</dt>
        <dd class="mt-0.5 font-medium">{stageLabel(review.stage)}</dd>
      </div>
      {#if review.context.contractAddress}
        <div class="sm:col-span-2">
          <dt class="opacity-65">{i18n.t('wallet.transfer.ethRecovery.assetContract')}</dt>
          <dd class="identifier-text mt-0.5 font-medium break-all">
            {review.context.contractAddress}
          </dd>
        </div>
      {/if}
      {#if review.context.bridgeContractAddress}
        <div class="sm:col-span-2">
          <dt class="opacity-65">{i18n.t('wallet.transfer.ethRecovery.bridgeContract')}</dt>
          <dd class="identifier-text mt-0.5 font-medium break-all">
            {review.context.bridgeContractAddress}
          </dd>
        </div>
      {/if}
      {#if review.context.mappedCurrencyId}
        <div>
          <dt class="opacity-65">{i18n.t('wallet.transfer.ethRecovery.mappedCurrency')}</dt>
          <dd class="identifier-text mt-0.5 font-medium break-all">
            {review.context.mappedCurrencyId}
          </dd>
        </div>
      {/if}
      {#if review.context.destinationSystemId}
        <div>
          <dt class="opacity-65">{i18n.t('wallet.transfer.ethRecovery.destinationSystem')}</dt>
          <dd class="identifier-text mt-0.5 font-medium break-all">
            {review.context.destinationSystemId}
          </dd>
        </div>
      {/if}
    </dl>
    <Button class="w-full sm:w-auto" onclick={onrecover} disabled={recovering}>
      {recovering
        ? i18n.t('wallet.transfer.ethRecovery.recovering')
        : review.requiresResume
          ? i18n.t('wallet.transfer.ethRecovery.continue')
          : i18n.t('wallet.transfer.ethRecovery.showResult')}
    </Button>
  </div>
</section>
