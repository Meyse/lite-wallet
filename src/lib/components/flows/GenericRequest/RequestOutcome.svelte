<script lang="ts">
  import CheckIcon from '@lucide/svelte/icons/check';
  import CircleIcon from '@lucide/svelte/icons/circle';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';
  import CircleCheckIcon from '@lucide/svelte/icons/circle-check';
  import ClockIcon from '@lucide/svelte/icons/clock';
  import XIcon from '@lucide/svelte/icons/x';
  import InlineTextActionButton from '$lib/components/common/InlineTextActionButton.svelte';
  import { Button } from '$lib/components/ui/button';
  import { CopyButton } from '$lib/components/ui/copy-button';
  import { Spinner } from '$lib/components/ui/spinner';
  import { i18nStore } from '$lib/i18n';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import RequestFacts from './RequestFacts.svelte';
  import type { RequestFact, StageView } from './types';

  /**
   * What the wallet did for a request: the receipt facts and one line per stage.
   * A failed stage offers Try again; a failed delivery also offers the signed
   * response to copy, and the site when a redirect address exists.
   */
  let {
    kind,
    title,
    body = '',
    facts = [],
    stages = [],
    retrying = false,
    copyValue = '',
    onRetry,
    onOpenSite,
  }: {
    kind: 'done' | 'failed' | 'declined' | 'expired';
    title: string;
    body?: string;
    facts?: RequestFact[];
    stages?: StageView[];
    retrying?: boolean;
    copyValue?: string;
    onRetry?: () => void;
    onOpenSite?: () => void;
  } = $props();

  const i18n = $derived($i18nStore);
  const copied = new TimedValueState<'response'>();

  async function copyResponse() {
    if (copyValue && (await writeClipboardText(copyValue))) copied.set('response');
  }
</script>

<div class="flex flex-col gap-4" role="status" aria-live="polite">
  <span
    class={`grid size-11 place-items-center rounded-full ${
      kind === 'done'
        ? 'bg-emerald-50 text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-400'
        : kind === 'failed' || kind === 'expired'
          ? 'bg-amber-50 text-amber-700 dark:bg-amber-950/40 dark:text-amber-300'
          : 'bg-muted text-muted-foreground'
    }`}
    aria-hidden="true"
  >
    {#if kind === 'done'}
      <CheckIcon class="size-5" />
    {:else if kind === 'failed'}
      <CircleAlertIcon class="size-5" />
    {:else if kind === 'expired'}
      <ClockIcon class="size-5" />
    {:else}
      <XIcon class="size-5" />
    {/if}
  </span>

  <h2 class="text-xl font-semibold tracking-tight text-balance text-foreground">{title}</h2>
  {#if body}<p class="text-[13.5px] text-foreground/80">{body}</p>{/if}

  {#if facts.length > 0}
    <RequestFacts rows={facts} />
  {/if}

  {#if stages.length > 0}
    <ul class="flex flex-col gap-2 text-[13.5px]">
      {#each stages as stage (stage.id)}
        <li class="flex flex-col gap-1">
          <span
            class={`flex items-center gap-2 ${stage.status === 'pending' ? 'text-muted-foreground' : 'text-foreground'}`}
          >
            {#if stage.status === 'done'}
              <CircleCheckIcon class="size-4 shrink-0 text-emerald-600 dark:text-emerald-400" />
            {:else if stage.status === 'failed'}
              <CircleAlertIcon class="size-4 shrink-0 text-amber-600 dark:text-amber-300" />
            {:else if stage.status === 'running'}
              <Spinner class="size-4 shrink-0" />
            {:else}
              <CircleIcon class="size-4 shrink-0" />
            {/if}
            {stage.label}
          </span>
          {#if stage.error}<span class="pl-6 text-xs text-muted-foreground">{stage.error}</span
            >{/if}
        </li>
      {/each}
    </ul>
  {/if}

  {#if kind === 'failed' && onRetry}
    <div class="flex flex-wrap items-center gap-2">
      <Button variant="secondary" onclick={onRetry} disabled={retrying}>
        {#if retrying}<Spinner class="size-4" />{/if}
        {i18n.t('common.retry')}
      </Button>
      {#if copyValue}
        <CopyButton
          copied={copied.current === 'response'}
          onclick={() => void copyResponse()}
          title={i18n.t('genericRequest.flow.copyResponse')}
          aria-label={i18n.t('genericRequest.flow.copyResponse')}
        />
      {/if}
      {#if onOpenSite}
        <InlineTextActionButton onclick={onOpenSite}
          >{i18n.t('genericRequest.flow.openSite')}</InlineTextActionButton
        >
      {/if}
    </div>
  {/if}
</div>
