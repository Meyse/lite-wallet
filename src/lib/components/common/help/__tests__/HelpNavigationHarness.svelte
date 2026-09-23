<script lang="ts">
  import HelpCenterDialog from '../HelpCenterDialog.svelte';
  let open = $state(false);
  let destination = $state('');
</script>

<button
  onclick={() => {
    open = true;
  }}>Open help</button
>
<div data-destination>{destination}</div>
<HelpCenterDialog
  bind:open
  backLabel="Back to wallet"
  onNavigate={(value) => {
    destination = value;
    open = false;
  }}
/>
