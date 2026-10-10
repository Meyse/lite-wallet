<script lang="ts">
  import ContactAvatar from '$lib/components/wallet/contacts/ContactAvatar.svelte';
  import RequesterTrust from './RequesterTrust.svelte';
  import type { RequesterView } from './types';

  /** The large header of a single-screen request: who asks and what for. */
  let {
    requester,
    title,
    timeLeftLabel = '',
    expired = false,
  }: {
    requester: RequesterView;
    title: string;
    timeLeftLabel?: string;
    expired?: boolean;
  } = $props();
</script>

<div class="flex items-start gap-3">
  {#if requester.identity}
    <ContactAvatar
      identity={requester.identity}
      name={requester.identityLabel}
      class="size-12 shrink-0"
    />
  {:else}
    <span
      class="grid size-12 shrink-0 place-items-center rounded-xl bg-muted text-lg font-semibold text-foreground uppercase"
      aria-hidden="true"
    >
      {requester.appLabel.charAt(0)}
    </span>
  {/if}
  <div class="min-w-0 flex-1">
    <h1 class="text-2xl leading-tight font-semibold tracking-tight text-balance text-foreground">
      {title}
    </h1>
    <RequesterTrust {requester} {timeLeftLabel} {expired} />
  </div>
</div>
