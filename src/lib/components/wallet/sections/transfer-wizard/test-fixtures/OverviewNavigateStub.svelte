<script lang="ts">
  let {
    onNavigateToSend,
    onNavigateToConvert,
  }: {
    onNavigateToSend: () => void;
    onNavigateToConvert: () => void;
  } = $props();
</script>

<button data-open-production-send type="button" onclick={onNavigateToSend}> Send </button>
<button data-open-production-convert type="button" onclick={onNavigateToConvert}> Convert </button>
