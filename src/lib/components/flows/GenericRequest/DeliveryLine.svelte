<script lang="ts">
  import InfoIcon from '@lucide/svelte/icons/info';
  import SendIcon from '@lucide/svelte/icons/send';

  /** One line that says where the response goes, or that none is sent. */
  let { text, sends }: { text: string; sends: boolean } = $props();
</script>

<p class="flex items-center gap-1.5 text-[12.5px] text-muted-foreground">
  {#if sends}<SendIcon class="size-3.5 shrink-0" />{:else}<InfoIcon
      class="size-3.5 shrink-0"
    />{/if}
  <span class="min-w-0">{text}</span>
</p>
