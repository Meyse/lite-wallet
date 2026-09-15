<script lang="ts">
  import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
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

  let { rows, warnings = [], class: className = '' }: TransferSummaryRailProps = $props();

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
        {@const isAssetFlowRow =
          !!assetFlowPair && (index === assetFlowPair.fromIndex || index === assetFlowPair.toIndex)}
        {@const showAssetFlowArrow = !!assetFlowPair && index === assetFlowPair.fromIndex}
        {@const isAmountEstimateRow =
          !!amountEstimatePair &&
          (index === amountEstimatePair.amountIndex || index === amountEstimatePair.estimatedIndex)}
        {@const showAmountEstimateArrow =
          !!amountEstimatePair && index === amountEstimatePair.amountIndex}
        {@const groupId = rowGroupId(row, index)}
        {@const previousGroupId = index > 0 ? rowGroupId(rows[index - 1], index - 1) : ''}
        {@const startsNewGroup = index > 0 && groupId !== previousGroupId}
        <div class={cn(startsNewGroup ? 'mt-6' : '')}>
          <dt
            class={cn(
              'text-xs leading-tight text-muted-foreground',
              isAssetFlowRow || isAmountEstimateRow ? 'sr-only' : ''
            )}
          >
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
                  {#if row.primaryIdentifier}
                    <IdentifierText
                      value={row.primary}
                      mode="compact"
                      class="block truncate text-sm leading-tight font-semibold"
                    />
                  {:else}
                    <p
                      class={cn(
                        'truncate text-sm leading-tight font-semibold',
                        row.breakAll ? 'break-all' : ''
                      )}
                    >
                      {row.primary}
                    </p>
                  {/if}
                  {#if row.secondary}
                    {#if row.secondaryIdentifier}
                      <IdentifierText
                        value={row.secondary}
                        mode="compact"
                        class="mt-0.5 block truncate text-xs text-muted-foreground"
                      />
                    {:else}
                      <p class="mt-0.5 truncate text-xs text-muted-foreground">
                        {row.secondary}
                      </p>
                    {/if}
                  {/if}
                </div>
              </div>
            </dd>
            {#if showAssetFlowArrow}
              <div
                class="my-2 flex h-4 w-[18px] items-center justify-center text-muted-foreground/70"
              >
                <ArrowDownIcon class="size-3.5" />
              </div>
            {/if}
          {:else}
            <dd
              class={cn(
                isAmountEstimateRow
                  ? 'mt-0 text-sm leading-tight font-medium'
                  : 'mt-1.5 text-sm leading-tight font-medium',
                row.breakAll ? 'break-all' : ''
              )}
            >
              {#if row.primaryIdentifier}
                <IdentifierText value={row.primary} mode="compact" class="block" />
              {:else}
                <p>{row.primary}</p>
              {/if}
              {#if row.secondary}
                {#if row.secondaryIdentifier}
                  <IdentifierText
                    value={row.secondary}
                    mode="compact"
                    class="mt-0.5 block text-xs text-muted-foreground"
                  />
                {:else}
                  <p class="mt-0.5 text-xs text-muted-foreground">{row.secondary}</p>
                {/if}
              {/if}
            </dd>
            {#if showAmountEstimateArrow}
              <div
                class="my-2 flex h-4 w-[18px] items-center justify-center text-muted-foreground/70"
              >
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
        <p class="text-xs text-amber-600 dark:text-amber-400">{warning}</p>
      {/each}
    </div>
  {/if}
</section>
