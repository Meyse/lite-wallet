<script lang="ts">
  import type { ContactIdentity } from '$lib/types/addressBook';
  import { identityKey } from '$lib/contacts/identity';
  import { contactSession } from '$lib/contacts/session';
  import IdentityPreview from './IdentityPreview.svelte';
  let { identity, iconOnly = false }: { identity: ContactIdentity; iconOnly?: boolean } = $props();
</script>

{#if $contactSession}
  {#key `${$contactSession.sessionId}:${$contactSession.network}:${identityKey(identity)}`}
    <IdentityPreview {identity} {iconOnly} />
  {/key}
{:else}<span>{identity.fullyQualifiedName}</span>{/if}
