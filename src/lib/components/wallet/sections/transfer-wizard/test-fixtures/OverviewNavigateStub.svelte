<script lang="ts">
  let {
    showAddAssetSheet = $bindable(false),
    onNavigateToSend,
    onNavigateToConvert,
  }: {
    showAddAssetSheet?: boolean;
    onNavigateToSend: () => void;
    onNavigateToConvert: () => void;
  } = $props();
</script>

<button data-open-production-send type="button" onclick={onNavigateToSend}> Send </button>
<button data-open-production-convert type="button" onclick={onNavigateToConvert}> Convert </button>

{#if showAddAssetSheet}<div data-manage-assets>Manage assets</div>{/if}
