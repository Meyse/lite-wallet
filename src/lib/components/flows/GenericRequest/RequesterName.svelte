<script lang="ts">
  import BadgeCheckIcon from '@lucide/svelte/icons/badge-check';
  import IdentityMention from '$lib/components/wallet/contacts/IdentityMention.svelte';
  import { i18nStore } from '$lib/i18n';
  import type { RequesterView } from './types';

  /**
   * Who asks, named once (spec "Requester header"): the signing VerusID with a
   * quiet check after the signature check, else the requester's address. A
   * signed request is the normal case, so it gets no colour; a missing
   * signature shows as NotSignedChip instead.
   */
  let { requester }: { requester: RequesterView } = $props();

  const i18n = $derived($i18nStore);
</script>

{#if requester.signed && (requester.identity || requester.identityLabel)}
  <span class="inline-flex min-w-0 items-center gap-1">
    {#if requester.identity}
      <IdentityMention identity={requester.identity} nameOnly />
    {:else}
      <span class="min-w-0 truncate">{requester.identityLabel}</span>
    {/if}
    <span
      class="inline-flex shrink-0 text-muted-foreground"
      title={i18n.t('genericRequest.flow.signatureChecked')}
    >
      <BadgeCheckIcon class="size-3.5" aria-hidden="true" />
      <span class="sr-only">{i18n.t('genericRequest.flow.signatureChecked')}</span>
    </span>
  </span>
{:else}
  <span class="min-w-0 truncate">{requester.appLabel}</span>
{/if}
