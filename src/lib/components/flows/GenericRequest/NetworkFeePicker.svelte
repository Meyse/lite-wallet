<script lang="ts">
  import CheckIcon from '@lucide/svelte/icons/check';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import WalletIcon from '@lucide/svelte/icons/wallet';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { Spinner } from '$lib/components/ui/spinner';
  import { i18nStore } from '$lib/i18n';
  import RequestFacts from './RequestFacts.svelte';
  import type { FundingSource, RequestFact } from './types';

  /**
   * Who pays: the fee of an update, or an invoice and its fee. Nothing is
   * preselected; the fee shows after the choice (spec "Network fee").
   */
  let {
    sources,
    selectedChannelId,
    loading = false,
    feeLoading = false,
    feeLabel = '',
    heading = '',
    emptyText = '',
    extraRows = [],
    disabled = false,
    networkLabel,
    onSelect,
  }: {
    sources: FundingSource[];
    selectedChannelId: string;
    loading?: boolean;
    feeLoading?: boolean;
    feeLabel?: string;
    /** The picker's heading; "Pay the fee from" when empty. */
    heading?: string;
    emptyText?: string;
    /** Rows after the fee, such as a total, once the fee is known. */
    extraRows?: RequestFact[];
    disabled?: boolean;
    networkLabel: (source: FundingSource) => string;
    onSelect: (channelId: string) => void;
  } = $props();

  const pickerLabel = $derived(heading || $i18nStore.t('genericRequest.flow.payFeeFrom'));

  const i18n = $derived($i18nStore);
  let sheetOpen = $state(false);
  const selected = $derived(
    sources.find((source) => source.channelId === selectedChannelId) ?? null
  );

  function label(source: FundingSource): string {
    const value = source.addressLabel.trim();
    return value && value.toLowerCase() !== source.address.toLowerCase() ? value : '';
  }

  function choose(channelId: string) {
    sheetOpen = false;
    onSelect(channelId);
  }
</script>

{#snippet sourceText(source: FundingSource)}
  <span class="flex min-w-0 flex-1 flex-col leading-tight">
    {#if label(source)}
      <span class="truncate text-sm font-medium text-foreground">{label(source)}</span>
      <IdentifierText value={source.address} mode="compact" class="text-xs text-muted-foreground" />
    {:else}
      <IdentifierText
        value={source.address}
        mode="compact"
        class="text-sm font-medium text-foreground"
      />
      <span class="truncate text-xs text-muted-foreground">{networkLabel(source)}</span>
    {/if}
  </span>
  <span class="shrink-0 text-xs text-muted-foreground tabular-nums">{source.balanceDisplay}</span>
{/snippet}

<div class="flex flex-col gap-1.5">
  <span class="text-xs font-medium text-muted-foreground">{pickerLabel}</span>
  {#if loading}
    <div
      class="flex min-h-[52px] items-center gap-2 rounded-lg bg-muted/60 px-3 text-sm text-muted-foreground"
    >
      <Spinner class="size-4" />
      {i18n.t('genericRequest.update.fundingSourcesLoading')}
    </div>
  {:else if sources.length === 0}
    <p
      class="rounded-lg border border-dashed border-border px-3 py-3 text-sm text-muted-foreground"
    >
      {emptyText || i18n.t('genericRequest.update.fundingEmpty')}
    </p>
  {:else}
    <button
      type="button"
      class={`flex min-h-[52px] w-full items-center gap-2.5 rounded-lg px-3 py-2 text-left transition-colors focus-visible:ring-2 focus-visible:ring-ring/60 focus-visible:outline-hidden disabled:opacity-60 ${
        selected
          ? 'bg-muted/70 hover:bg-muted'
          : 'border border-dashed border-border hover:bg-muted/40'
      }`}
      onclick={() => (sheetOpen = true)}
      {disabled}
    >
      <WalletIcon class="size-5 shrink-0 text-muted-foreground" />
      {#if selected}
        {@render sourceText(selected)}
      {:else}
        <span class="flex min-w-0 flex-1 flex-col leading-tight">
          <span class="text-sm font-medium text-foreground"
            >{i18n.t('genericRequest.flow.chooseSource')}</span
          >
          <span class="text-xs text-muted-foreground"
            >{i18n.t('genericRequest.flow.feeAfterChoice')}</span
          >
        </span>
      {/if}
      <ChevronDownIcon class="size-4 shrink-0 text-muted-foreground" />
    </button>
  {/if}
</div>

{#if selected && (feeLoading || feeLabel)}
  {#snippet feeValue()}
    {#if feeLoading}
      <span class="inline-flex items-center gap-1.5 font-normal text-muted-foreground">
        <Spinner class="size-3.5" />
        {i18n.t('genericRequest.update.fundingCalculating')}
      </span>
    {:else}
      {feeLabel}
    {/if}
  {/snippet}
  <RequestFacts
    rows={[
      { label: i18n.t('genericRequest.flow.networkFee'), content: feeValue },
      ...(feeLoading ? [] : extraRows),
    ]}
  />
{/if}

<StandardRightSheet bind:isOpen={sheetOpen} title={pickerLabel}>
  <ScrollArea.Root class="min-h-0 flex-1">
    <ScrollArea.Viewport class="h-full pr-1">
      <ul class="space-y-1.5 pb-4">
        {#each sources as source (source.channelId)}
          <li>
            <button
              type="button"
              class={`flex w-full items-center gap-2.5 rounded-lg px-3 py-2.5 text-left transition-colors focus-visible:ring-2 focus-visible:ring-ring/60 focus-visible:outline-hidden ${
                source.channelId === selectedChannelId
                  ? 'bg-primary/12 dark:bg-primary/24'
                  : 'hover:bg-muted/70'
              }`}
              onclick={() => choose(source.channelId)}
            >
              <WalletIcon class="size-5 shrink-0 text-muted-foreground" />
              {@render sourceText(source)}
              {#if source.channelId === selectedChannelId}<CheckIcon
                  class="size-4 shrink-0 text-primary"
                />{/if}
            </button>
          </li>
        {/each}
      </ul>
    </ScrollArea.Viewport>
    <ScrollArea.Scrollbar orientation="vertical" />
  </ScrollArea.Root>
</StandardRightSheet>
