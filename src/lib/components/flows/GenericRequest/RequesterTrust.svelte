<script lang="ts">
  import ClockIcon from '@lucide/svelte/icons/clock';
  import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
  import ShieldQuestionIcon from '@lucide/svelte/icons/shield-question';
  import IdentityMention from '$lib/components/wallet/contacts/IdentityMention.svelte';
  import { i18nStore } from '$lib/i18n';
  import type { RequesterView } from './types';

  /** "Signed by" names the requester only after the signature check (spec "Requester header"). */
  let {
    requester,
    compact = false,
    timeLeftLabel = '',
    expired = false,
  }: {
    requester: RequesterView;
    compact?: boolean;
    timeLeftLabel?: string;
    expired?: boolean;
  } = $props();

  const i18n = $derived($i18nStore);
</script>

<p
  class={`flex flex-wrap items-center text-muted-foreground ${compact ? 'mt-0.5 gap-1 text-xs' : 'mt-1.5 gap-x-3 gap-y-1 text-[12.5px]'}`}
>
  {#if requester.signed}
    <span
      class="inline-flex min-w-0 flex-wrap items-center gap-x-1 text-emerald-700 dark:text-emerald-400"
    >
      <ShieldCheckIcon class="size-3.5 shrink-0" />
      <span class="shrink-0">{i18n.t('genericRequest.flow.signedBy')}</span>
      <span class="min-w-0 font-medium break-all">
        {#if requester.identity}<IdentityMention
            identity={requester.identity}
          />{:else}{requester.identityLabel}{/if}
      </span>
    </span>
  {:else}
    <span class="inline-flex items-center gap-1">
      <ShieldQuestionIcon class="size-3.5 shrink-0" />
      {i18n.t('genericRequest.flow.notSigned')}
    </span>
  {/if}
  {#if !compact && timeLeftLabel}
    <span
      class={`inline-flex items-center gap-1 ${expired ? 'text-amber-700 dark:text-amber-300' : ''}`}
    >
      <ClockIcon class="size-3.5 shrink-0" />
      {timeLeftLabel}
    </span>
  {/if}
</p>
