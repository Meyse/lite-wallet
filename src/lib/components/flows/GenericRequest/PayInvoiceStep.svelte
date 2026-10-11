<script lang="ts">
  import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Spinner } from '$lib/components/ui/spinner';
  import { i18nStore } from '$lib/i18n';
  import type { PayInvoiceModule } from './modules/PayInvoiceModule.svelte';
  import NetworkFeePicker from './NetworkFeePicker.svelte';
  import RequestFacts from './RequestFacts.svelte';
  import type { RequestFact } from './types';

  /** Pay invoice: the amount, who receives it, and who pays. */
  let {
    module,
    networkLabel,
  }: {
    module: PayInvoiceModule;
    networkLabel: (systemId: string) => string;
  } = $props();

  const i18n = $derived($i18nStore);
  let amountTimer: ReturnType<typeof setTimeout> | undefined;

  const facts = $derived.by<RequestFact[]>(() => {
    const rows: RequestFact[] = [];
    if (!module.acceptsAnyAmount) {
      rows.push({
        label: i18n.t('genericRequest.flow.invoice.amount'),
        value: module.amountText(),
      });
    }
    rows.push({ label: i18n.t('genericRequest.flow.invoice.to'), content: destinationValue });
    return rows;
  });

  const totalRows = $derived<RequestFact[]>(
    module.preflight
      ? [{ label: i18n.t('genericRequest.flow.invoice.total'), value: module.totalText() }]
      : []
  );

  function onAmountInput(event: Event) {
    module.setAmount((event.target as HTMLInputElement).value);
    clearTimeout(amountTimer);
    amountTimer = setTimeout(() => void module.prepare(), 500);
  }
</script>

{#snippet destinationValue()}
  <span class="flex flex-col items-end gap-0.5">
    {#if module.review?.destinationName}
      <span class="font-medium">{module.destinationLabel()}</span>
    {/if}
    <IdentifierText value={module.review?.destination ?? ''} mode="compact" />
  </span>
{/snippet}

{#if module.reviewLoading}
  <p class="flex items-center gap-2 text-sm text-muted-foreground">
    <Spinner class="size-4" />
    {i18n.t('genericRequest.flow.invoice.loading')}
  </p>
{:else if module.review}
  {#if module.review.alreadyPaid}
    <p class="rounded-lg bg-muted/60 px-3 py-2.5 text-[13.5px] text-foreground">
      {i18n.t('genericRequest.rejected.alreadyPaid')}
    </p>
  {/if}

  {#if module.acceptsAnyAmount}
    <div class="flex flex-col gap-1.5">
      <Label for="request-invoice-amount" class="text-xs font-medium text-muted-foreground">
        {i18n.t('genericRequest.flow.invoice.amountInput', { currency: module.ticker() })}
      </Label>
      <Input
        id="request-invoice-amount"
        value={module.amountInput}
        oninput={onAmountInput}
        inputmode="decimal"
        autocomplete="off"
        placeholder="0.00"
        aria-invalid={module.amountInput.trim() !== '' && !module.amountIsValid()}
      />
    </div>
  {/if}

  <RequestFacts rows={facts} />

  <NetworkFeePicker
    sources={module.sources}
    selectedChannelId={module.selectedChannelId}
    loading={module.sourcesLoading}
    feeLoading={module.preflightLoading}
    feeLabel={module.feeText()}
    heading={i18n.t('genericRequest.flow.invoice.payFrom')}
    emptyText={i18n.t('genericRequest.flow.invoice.noSource', { currency: module.ticker() })}
    extraRows={totalRows}
    disabled={module.review.alreadyPaid || (module.acceptsAnyAmount && !module.amountIsValid())}
    networkLabel={(source) => networkLabel(source.systemId)}
    onSelect={(channelId) => void module.choose(channelId)}
  />
{/if}

{#if module.error}
  <div
    class="flex items-start gap-2 rounded-xl border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
  >
    <AlertCircleIcon class="mt-0.5 size-4 shrink-0" />
    <p>{module.error}</p>
  </div>
{/if}
