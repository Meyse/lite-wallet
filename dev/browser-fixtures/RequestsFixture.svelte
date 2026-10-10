<script lang="ts">
  import GenericRequestFlowHost from '$lib/components/flows/GenericRequest/GenericRequestFlowHost.svelte';
  import type { GenericRequestFlowSession } from '$lib/genericRequest/session';

  let { session }: { session: GenericRequestFlowSession } = $props();
  let isOpen = $state(true);
</script>

<div class="relative h-screen overflow-hidden bg-background">
  {#if !isOpen}
    <p class="p-6 text-sm text-muted-foreground">The request closed. Reload to open it again.</p>
  {/if}
  <GenericRequestFlowHost bind:isOpen {session} />
</div>
