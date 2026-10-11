<script lang="ts">
  import ContactAvatar from '$lib/components/wallet/contacts/ContactAvatar.svelte';
  import type { RequesterView } from './types';

  /**
   * The requester's picture: the signing VerusID's avatar, else the first
   * letter of the name shown next to it. `tile` sets the fill of the letter
   * tile, which differs on the rail and the page.
   */
  let {
    requester,
    class: className = '',
    tile = 'bg-muted',
  }: { requester: RequesterView; class?: string; tile?: string } = $props();

  // The letter belongs to the name shown next to it: the signer when signed.
  const initial = $derived(
    (requester.signed && requester.identityLabel
      ? requester.identityLabel
      : requester.appLabel
    ).charAt(0)
  );
</script>

{#if requester.signed && requester.identity}
  <ContactAvatar
    identity={requester.identity}
    name={requester.identityLabel}
    class={`shrink-0 ${className}`}
  />
{:else}
  <span
    class={`grid shrink-0 place-items-center rounded-[10px] font-semibold text-foreground uppercase ${tile} ${className}`}
    aria-hidden="true"
  >
    {initial}
  </span>
{/if}
