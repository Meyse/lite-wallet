<script lang="ts">
  let { requestedView = 'home' }: { requestedView?: string } = $props();
</script>

<div data-settings-view={requestedView}></div>
