<script lang="ts">
  import HelpLink from './HelpLink.svelte';
  import HelpCenterDialog from './help/HelpCenterDialog.svelte';

  const { linkText, backLabel }: { linkText: string; backLabel: string } = $props();
  let open = $state(false);
</script>

<HelpLink
  text={linkText}
  onclick={() => {
    open = true;
  }}
/>
<HelpCenterDialog bind:open {backLabel} />
