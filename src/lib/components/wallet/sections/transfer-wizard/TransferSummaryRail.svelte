<script lang="ts">
  import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import { i18nStore } from '$lib/i18n';
  import { cn } from '$lib/utils.js';
  import { getTransferSummaryLabels } from '$lib/transfer/transferWizardCopy';

  type SummaryRow = {
    label: string;
    primary: string;
    secondary?: string;
    breakAll?: boolean;
    primaryIdentifier?: boolean;
    secondaryIdentifier?: boolean;
    iconCoinId?: string;
    iconCoinName?: string;
  };

  type TransferSummaryRailProps = {
    rows: SummaryRow[];
    warnings?: string[];
    class?: string;
  };

   
  let {
    rows,
    warnings = [],
    class: className = ''
  }: TransferSummaryRailProps = $props();
   

  const i18n = $derived($i18nStore);
  const labels = $derived(getTransferSummaryLabels(i18n.t));
  const assetFlowPair = $derived(
    (() => {
      const fromIndex = rows.findIndex((row) => !!row.iconCoinId && row.label === labels.from);
      if (fromIndex === -1) return null;

      const toIndex = rows.findIndex(
        (row, index) => index > fromIndex && !!row.iconCoinId && row.label === labels.to
      );
      if (toIndex !== fromIndex + 1) return null;

      return { fromIndex, toIndex };
    })()
  );
  const amountEstimatePair = $derived(
    (() => {
      const amountIndex = rows.findIndex((row) => row.label === labels.amount);
      if (amountIndex === -1) return null;

      const estimatedIndex = rows.findIndex(
        (row, index) => index > amountIndex && row.label === labels.estimatedReceive
      );
      if (estimatedIndex !== amountIndex + 1) return null;

      return { amountIndex, estimatedIndex };
    })()
  );

  function rowGroupId(row: SummaryRow, index: number): string {
    if (assetFlowPair && (index === assetFlowPair.fromIndex || index === assetFlowPair.toIndex)) {
      return 'asset-flow';
    }
    if (
      amountEstimatePair &&
      (index === amountEstimatePair.amountIndex || index === amountEstimatePair.estimatedIndex)
    ) {
      return 'amount-estimate';
    }
    if (row.label === labels.route) return 'route';
    if (row.label === labels.recipient) return 'recipient';
    if (row.label === labels.networkFee) return 'network-fee';
    return `row-${index}`;
  }
</script>

<section class={cn('h-full p-1', className)}>
  <div class="h-5" aria-hidden="true"></div>

  {#if rows.length > 0}
    <dl class="mt-3">
      {#each rows as row, index}
        {@const isAssetFlowRow = !!assetFlowPair && (index === assetFlowPair.fromIndex || index === assetFlowPair.toIndex)}
        {@const showAssetFlowArrow = !!assetFlowPair && index === assetFlowPair.fromIndex}
        {@const isAmountEstimateRow = !!amountEstimatePair && (index === amountEstimatePair.amountIndex || index === amountEstimatePair.estimatedIndex)}
        {@const showAmountEstimateArrow = !!amountEstimatePair && index === amountEstimatePair.amountIndex}
        {@const groupId = rowGroupId(row, index)}
        {@const previousGroupId = index > 0 ? rowGroupId(rows[index - 1], index - 1) : ''}
        {@const startsNewGroup = index > 0 && groupId !== previousGroupId}
        <div class={cn(startsNewGroup ? 'mt-6' : '')}>
          <dt class={cn('text-muted-foreground text-xs leading-tight', isAssetFlowRow || isAmountEstimateRow ? 'sr-only' : '')}>
            {row.label}
          </dt>
          {#if row.iconCoinId}
            <dd class={cn(isAssetFlowRow ? 'mt-0' : 'mt-1.5')}>
              <div class="flex items-center gap-2.5">
                <CoinIcon
                  coinId={row.iconCoinId}
                  coinName={row.iconCoinName ?? row.primary}
                  size={18}
                  decorative={true}
                />
                <div class="min-w-0">
                  <p
                    class={cn(
                      'truncate text-sm leading-tight font-semibold',
                      row.breakAll ? 'break-all' : '',
                      row.primaryIdentifier ? 'identifier-text' : ''
                    )}
                  >
                    {row.primary}
                  </p>
                  {#if row.secondary}
                    <p class={cn('text-muted-foreground mt-0.5 truncate text-xs', row.secondaryIdentifier ? 'identifier-text' : '')}>
                      {row.secondary}
                    </p>
                  {/if}
                </div>
              </div>
            </dd>
            {#if showAssetFlowArrow}
              <div class="text-muted-foreground/70 my-2 flex h-4 w-[18px] items-center justify-center">
                <ArrowDownIcon class="size-3.5" />
              </div>
            {/if}
          {:else}
            <dd
              class={cn(
                isAmountEstimateRow ? 'mt-0 text-sm leading-tight font-medium' : 'mt-1.5 text-sm leading-tight font-medium',
                row.breakAll ? 'break-all' : '',
                row.primaryIdentifier ? 'identifier-text' : ''
              )}
            >
              <p>{row.primary}</p>
              {#if row.secondary}
                <p class={cn('text-muted-foreground mt-0.5 text-xs', row.secondaryIdentifier ? 'identifier-text' : '')}>
                  {row.secondary}
                </p>
              {/if}
            </dd>
            {#if showAmountEstimateArrow}
              <div class="text-muted-foreground/70 my-2 flex h-4 w-[18px] items-center justify-center">
                <ArrowDownIcon class="size-3.5" />
              </div>
            {/if}
          {/if}
        </div>
      {/each}
    </dl>
  {/if}

  {#if warnings.length > 0}
    <div class="mt-4 space-y-1.5">
      <p class="text-xs font-medium">{labels.warnings}</p>
      {#each warnings as warning}
        <p class="text-amber-600 dark:text-amber-400 text-xs">{warning}</p>
      {/each}
    </div>
  {/if}
</section>
