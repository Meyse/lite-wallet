<script lang="ts">
  import type { Snippet } from 'svelte';
  import InlineTextActionButton from '$lib/components/common/InlineTextActionButton.svelte';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Label } from '$lib/components/ui/label';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import type { SignableItemView } from '$lib/genericRequest/session';
  import { i18nStore } from '$lib/i18n';
  import type { SignDataModule } from './modules/SignDataModule.svelte';

  /** Sign data: one consent per item and per statement (spec "Sign data"). */
  let {
    module,
    appLabel,
    responder,
  }: {
    module: SignDataModule;
    appLabel: string;
    /** The responder picker, when no sign in before this step chose one. */
    responder?: Snippet;
  } = $props();

  const i18n = $derived($i18nStore);
  let inspected = $state<SignableItemView | null>(null);
  let inspectOpen = $state(false);

  function itemTitle(item: SignableItemView): string {
    return (
      item.label ??
      (item.kind === 'text'
        ? i18n.t('genericRequest.flow.signData.message')
        : i18n.t('genericRequest.flow.signData.data'))
    );
  }

  function itemMeta(item: SignableItemView): string {
    const size = i18n.t('genericRequest.flow.signData.size', {
      size: i18n.formatNumber(item.size),
    });
    return item.mimeType ? `${item.mimeType} · ${size}` : size;
  }

  function preview(item: SignableItemView): string {
    return item.text ?? item.hex;
  }

  function inspect(item: SignableItemView) {
    inspected = item;
    inspectOpen = true;
  }
</script>

{#if responder}
  {@render responder()}
{/if}

<p class="text-[13.5px] text-foreground/80">
  {i18n.t('genericRequest.flow.signData.intro', { app: appLabel })}
</p>

<ul class="overflow-hidden rounded-xl border border-border/70">
  {#each module.detail.items as item, index (index)}
    <li class="flex items-start gap-3 border-b border-border/70 px-3.5 py-3 last:border-b-0">
      <Checkbox
        id={`request-sign-item-${index}`}
        checked={module.consent[index]}
        onCheckedChange={(checked) => module.setConsent(index, checked === true)}
        class="mt-0.5"
        aria-label={i18n.t('genericRequest.flow.signData.consent', { item: itemTitle(item) })}
      />
      <div class="flex min-w-0 flex-1 flex-col gap-1">
        <Label for={`request-sign-item-${index}`} class="leading-snug text-foreground">
          {itemTitle(item)}
        </Label>
        <p
          class={`line-clamp-3 text-[13px] break-words text-foreground/80 ${item.text === null ? 'font-mono' : ''}`}
        >
          {preview(item)}
        </p>
        <span class="flex items-center gap-3 text-xs text-muted-foreground">
          <span>{itemMeta(item)}</span>
          <InlineTextActionButton class="w-fit" onclick={() => inspect(item)}>
            {i18n.t('genericRequest.update.content.viewDetails')}
          </InlineTextActionButton>
        </span>
      </div>
    </li>
  {/each}
  {#each module.detail.statements as statement, at (at)}
    {@const index = module.detail.items.length + at}
    <li class="flex items-start gap-3 border-b border-border/70 px-3.5 py-3 last:border-b-0">
      <Checkbox
        id={`request-sign-item-${index}`}
        checked={module.consent[index]}
        onCheckedChange={(checked) => module.setConsent(index, checked === true)}
        class="mt-0.5"
      />
      <Label
        for={`request-sign-item-${index}`}
        class="flex min-w-0 flex-1 flex-col items-start gap-1 leading-normal font-normal"
      >
        <span class="text-xs font-medium text-muted-foreground">
          {i18n.t('genericRequest.flow.signData.statement')}
        </span>
        <span class="text-[13.5px] break-words text-foreground">{statement}</span>
      </Label>
    </li>
  {/each}
</ul>

{#if !module.consent.every(Boolean)}
  <p class="text-xs text-muted-foreground">{i18n.t('genericRequest.flow.signData.allNeeded')}</p>
{/if}

<StandardRightSheet
  bind:isOpen={inspectOpen}
  title={inspected ? itemTitle(inspected) : i18n.t('genericRequest.flow.signData.data')}
>
  {#if inspected}
    <ScrollArea.Root class="min-h-0 flex-1">
      <ScrollArea.Viewport class="h-full pr-1">
        <div class="space-y-4 pb-4">
          {#if inspected.text !== null}
            <pre
              class="overflow-x-auto rounded-lg bg-muted/45 px-3 py-3 text-xs whitespace-pre-wrap text-foreground dark:bg-muted/30">{inspected.text}</pre>
          {/if}
          <div class="space-y-1.5">
            <p class="text-xs font-medium text-muted-foreground">
              {i18n.t('genericRequest.flow.signData.bytes')}
            </p>
            <pre
              class="overflow-x-auto rounded-lg bg-muted/45 px-3 py-3 font-mono text-xs break-all whitespace-pre-wrap text-foreground dark:bg-muted/30">{inspected.hex}</pre>
          </div>
          <div class="space-y-1.5">
            <p class="text-xs font-medium text-muted-foreground">
              {i18n.t('genericRequest.flow.signData.hash')}
            </p>
            <p class="font-mono text-xs break-all text-foreground">{inspected.sha256}</p>
          </div>
        </div>
      </ScrollArea.Viewport>
      <ScrollArea.Scrollbar orientation="vertical" />
    </ScrollArea.Root>
  {/if}
</StandardRightSheet>
