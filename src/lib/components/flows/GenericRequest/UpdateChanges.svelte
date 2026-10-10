<script lang="ts">
  import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
  import GlobeIcon from '@lucide/svelte/icons/globe';
  import ShieldAlertIcon from '@lucide/svelte/icons/shield-alert';
  import WalletIcon from '@lucide/svelte/icons/wallet';
  import InlineTextActionButton from '$lib/components/common/InlineTextActionButton.svelte';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Label } from '$lib/components/ui/label';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { i18nStore } from '$lib/i18n';
  import type {
    IdentityUpdateContentItem,
    IdentityUpdateReviewModel,
  } from '$lib/genericRequest/identityUpdateReview';
  import type { IdentityWarning } from '$lib/types/wallet.js';

  /**
   * Update identity, step 1: every change in one list. High-risk changes add an
   * outcome summary and an acknowledgement that Continue needs.
   */
  let {
    review,
    warnings = [],
    acknowledged = $bindable(false),
  }: {
    review: IdentityUpdateReviewModel;
    warnings?: IdentityWarning[];
    acknowledged?: boolean;
  } = $props();

  const i18n = $derived($i18nStore);
  let inspected = $state<IdentityUpdateContentItem | null>(null);
  let inspectOpen = $state(false);

  type Row = {
    id: string;
    label: string;
    before?: string | null;
    after?: string | null;
    identifier: boolean;
    note?: string;
    highRisk: boolean;
    badge?: string;
    content?: IdentityUpdateContentItem;
  };

  const rows = $derived<Row[]>([
    ...review.authorityChanges.map((change) => ({
      id: change.id,
      label: change.title,
      before: change.currentValue,
      after: change.nextValue,
      identifier: true,
      highRisk: true,
    })),
    ...review.primaryAddressChanges.map((change) => ({
      id: change.id,
      label: change.title,
      after: change.address,
      identifier: true,
      highRisk: true,
    })),
    ...review.otherHighRiskItems.map((item) => ({
      id: item.id,
      label: item.title,
      before: item.currentValue,
      after: item.nextValue,
      identifier: item.id === 'private-address',
      note: item.nextValue ? undefined : item.description,
      highRisk: true,
    })),
    ...review.contentChanges.map((change) => ({
      id: change.id,
      label: change.label || change.title,
      after: change.nextPreview,
      identifier: false,
      highRisk: false,
      badge: change.badges[0]?.label,
      content: change,
    })),
  ]);

  // Authority-only updates have no outcome summary; say what the new authority can do.
  const authorityKey = $derived(
    review.authorityChanges.length > 1
      ? 'Both'
      : review.authorityChanges[0]?.id === 'recovery'
        ? 'Recovery'
        : review.authorityChanges[0]?.id === 'revocation'
          ? 'Revocation'
          : ''
  );
  // A single other high-risk change names itself and its consequence.
  const onlyRisk = $derived(
    review.highRiskCount === 1 &&
      review.otherHighRiskItems.length === 1 &&
      !review.isContentClearOnly
      ? review.otherHighRiskItems[0]
      : null
  );
  const riskTitle = $derived(
    onlyRisk?.title ??
      review.outcome?.title ??
      (authorityKey
        ? i18n.t(`genericRequest.update.authority.infoTitle${authorityKey}`)
        : i18n.t('genericRequest.update.highRiskTitle'))
  );
  const riskDescription = $derived(
    onlyRisk?.description ??
      review.outcome?.description ??
      (authorityKey
        ? i18n.t(`genericRequest.update.authority.infoBody${authorityKey}`)
        : review.isContentClearOnly
          ? i18n.t('genericRequest.update.highRiskDescriptionContentClear')
          : i18n.t('genericRequest.update.highRiskDescription'))
  );

  function inspect(change: IdentityUpdateContentItem) {
    inspected = change;
    inspectOpen = true;
  }

  function formatInspectValue(value: unknown): string {
    if (value === undefined || value === null) return i18n.t('genericRequest.update.emptyValue');
    if (typeof value === 'string') return value;
    try {
      return JSON.stringify(value, null, 2);
    } catch {
      return String(value);
    }
  }
</script>

{#if rows.length === 0}
  <p class="text-[13.5px] text-foreground/80">{i18n.t('genericRequest.flow.noChanges')}</p>
{/if}
<ul class="overflow-hidden rounded-xl border border-border/70" class:hidden={rows.length === 0}>
  {#each rows as row (row.id)}
    <li
      class={`grid grid-cols-[128px_minmax(0,1fr)_auto] items-start gap-3 border-b border-border/70 px-3.5 py-2.5 text-[13.5px] last:border-b-0 ${
        row.highRisk ? 'bg-amber-50/70 dark:bg-amber-950/20' : ''
      }`}
    >
      <span class="text-muted-foreground">{row.label}</span>
      <span class="flex min-w-0 flex-col gap-1">
        {#if row.before}
          <span class="flex min-w-0 items-center gap-1.5 text-muted-foreground line-through">
            {#if row.identifier}<IdentifierText
                value={row.before}
                mode="review"
              />{:else}{row.before}{/if}
          </span>
        {/if}
        {#if row.after}
          <span class="flex min-w-0 items-center gap-1.5 text-foreground">
            {#if row.before}<ArrowRightIcon class="size-3.5 shrink-0 text-muted-foreground" />{/if}
            {#if row.identifier}
              <IdentifierText value={row.after} mode="review" />
            {:else}
              <span class="line-clamp-2 break-words">{row.after}</span>
            {/if}
          </span>
        {:else if row.note}
          <span class="text-foreground">{row.note}</span>
        {/if}
        {#if row.content?.isInspectable}
          {@const content = row.content}
          <InlineTextActionButton class="w-fit" onclick={() => inspect(content)}>
            {i18n.t('genericRequest.update.content.viewDetails')}
          </InlineTextActionButton>
        {/if}
      </span>
      {#if row.highRisk}
        <span
          class="inline-flex h-5 items-center rounded-full bg-amber-100 px-2 text-[11px] leading-none font-medium text-amber-800 dark:bg-amber-900/40 dark:text-amber-200"
        >
          {i18n.t('genericRequest.flow.highRisk')}
        </span>
      {:else}
        <span
          class="inline-flex h-5 items-center rounded-full bg-muted px-2 text-[11px] leading-none font-medium text-foreground/80"
        >
          {row.badge ?? i18n.t('genericRequest.update.contentPrefix')}
        </span>
      {/if}
    </li>
  {/each}
</ul>

{#if review.primaryAddressChanges.length > 0}
  <div class="flex flex-col gap-1.5">
    <span class="text-xs font-medium text-muted-foreground">
      {i18n.t('genericRequest.update.primaryAddress.afterUpdateTitle')}
    </span>
    <ul class="flex flex-col gap-1.5">
      {#each review.primaryAddressesAfterUpdate as entry (entry.address)}
        <li
          class="flex items-center justify-between gap-3 rounded-lg bg-muted/45 px-3 py-2 text-sm dark:bg-muted/30"
        >
          <IdentifierText
            value={entry.address}
            mode="review"
            class="min-w-0 truncate text-foreground"
          />
          <span class="inline-flex shrink-0 items-center gap-1 text-xs text-muted-foreground">
            {#if entry.badge.tone === 'wallet'}<WalletIcon class="size-3.5" />{:else}<GlobeIcon
                class="size-3.5"
              />{/if}
            {entry.badge.label}
          </span>
        </li>
      {/each}
    </ul>
  </div>
{/if}

{#if warnings.length > 0}
  <ul class="flex flex-col gap-1.5 text-sm">
    {#each warnings as warning (`${warning.warningType}:${warning.message}`)}
      <li class="rounded-lg bg-muted/45 px-3 py-2 text-foreground dark:bg-muted/30">
        {warning.message}
      </li>
    {/each}
  </ul>
{/if}

{#if review.hasHighRisk}
  <div
    class="flex gap-2.5 rounded-xl border border-amber-300/70 bg-amber-50/80 px-3.5 py-3 dark:border-amber-500/30 dark:bg-amber-950/20"
  >
    <ShieldAlertIcon class="mt-0.5 size-[18px] shrink-0 text-amber-700 dark:text-amber-300" />
    <div class="min-w-0">
      <p class="text-sm font-semibold text-foreground">{riskTitle}</p>
      <p class="mt-0.5 text-[13px] text-foreground/80">{riskDescription}</p>
    </div>
  </div>

  <Label
    for="request-high-risk-ack"
    class="flex items-start gap-2.5 text-[13.5px] leading-snug font-normal"
  >
    <Checkbox
      id="request-high-risk-ack"
      checked={acknowledged}
      onCheckedChange={(checked) => (acknowledged = checked === true)}
      class="mt-0.5"
    />
    <span>{i18n.t('genericRequest.update.highRiskAcknowledge')}</span>
  </Label>
{/if}

<StandardRightSheet
  bind:isOpen={inspectOpen}
  title={inspected?.inspectTitle || i18n.t('genericRequest.update.content.viewDetails')}
>
  {#if inspected}
    <ScrollArea.Root class="min-h-0 flex-1">
      <ScrollArea.Viewport class="h-full pr-1">
        <div class="space-y-4 pb-4">
          {#if inspected.inspectPayload?.current !== undefined}
            <div class="space-y-1.5">
              <p class="text-xs font-medium text-muted-foreground">
                {inspected.currentLabel || i18n.t('genericRequest.update.currentValue')}
              </p>
              <pre
                class="overflow-x-auto rounded-lg bg-muted/45 px-3 py-3 text-xs whitespace-pre-wrap text-foreground dark:bg-muted/30">{formatInspectValue(
                  inspected.inspectPayload.current
                )}</pre>
            </div>
          {/if}
          {#if inspected.inspectPayload?.requested !== undefined}
            <div class="space-y-1.5">
              <p class="text-xs font-medium text-muted-foreground">
                {inspected.nextLabel || i18n.t('genericRequest.update.content.requestedValue')}
              </p>
              <pre
                class="overflow-x-auto rounded-lg bg-muted/45 px-3 py-3 text-xs whitespace-pre-wrap text-foreground dark:bg-muted/30">{formatInspectValue(
                  inspected.inspectPayload.requested
                )}</pre>
            </div>
          {/if}
          {#each [inspected.detailBody, inspected.effectNote, inspected.historyNote].filter(Boolean) as note (note)}
            <p class="text-sm text-muted-foreground">{note}</p>
          {/each}
        </div>
      </ScrollArea.Viewport>
      <ScrollArea.Scrollbar orientation="vertical" />
    </ScrollArea.Root>
  {/if}
</StandardRightSheet>
