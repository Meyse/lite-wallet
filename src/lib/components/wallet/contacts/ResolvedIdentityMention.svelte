<script lang="ts">
  import type { ContactIdentity } from '$lib/types/addressBook';
  import { contactChainId } from '$lib/contacts/identity';
  import { contactSession } from '$lib/contacts/session';
  import { resolveContactIdentity } from '$lib/contacts/service';
  import IdentityMention from './IdentityMention.svelte';
  let {
    value,
    chainId,
    delay = 350,
  }: { value: string; chainId: string; delay?: number } = $props();
  let result = $state<{ key: string; identity: ContactIdentity } | null>(null);
  const requestKey = $derived(
    JSON.stringify([$contactSession?.sessionId, $contactSession?.network, chainId, value.trim()])
  );
  const identity = $derived(result?.key === requestKey ? result.identity : null);
  $effect(() => {
    const session = $contactSession;
    const key = requestKey;
    const address = value.trim();
    if (
      !session ||
      chainId !== contactChainId(session.network) ||
      !(/^i[1-9A-HJ-NP-Za-km-z]{24,60}$/.test(address) || /^[A-Za-z0-9._-]+@$/.test(address))
    )
      return undefined;
    let cancelled = false;
    const timer = setTimeout(() => {
      void resolveContactIdentity(address)
        .then((identity) => {
          if (!cancelled) result = { key, identity };
        })
        .catch(() => {});
    }, delay);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  });
</script>

{#if identity}<IdentityMention {identity} />{/if}
