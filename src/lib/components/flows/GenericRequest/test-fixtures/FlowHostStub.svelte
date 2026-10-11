<script lang="ts">
  /** Stands in for the request flow: it shows it is open and can approve. */
  let {
    isOpen = $bindable(false),
    approved = $bindable(false),
  }: { isOpen?: boolean; approved?: boolean; [key: string]: unknown } = $props();
</script>

{#if isOpen}
  <div data-request-flow-stub>
    <button type="button" onclick={() => (approved = true)}>Approve request</button>
  </div>
{/if}
