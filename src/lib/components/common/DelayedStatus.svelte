<script lang="ts">
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
  import { cn } from '$lib/utils.js';

  let {
    active,
    label,
    idleLabel = '',
    class: className,
  }: {
    active: boolean;
    label: string;
    idleLabel?: string;
    class?: string;
  } = $props();

  let visible = $state(false);
  $effect(() => {
    visible = false;
    if (!active) return undefined;
    // Passive reads should not flash a status for a brief local/cache lookup.
    const timer = setTimeout(() => (visible = true), 1_000);
    return () => clearTimeout(timer);
  });
</script>

<span
  class={cn('inline-flex min-h-4 items-center gap-1.5 text-xs text-muted-foreground', className)}
  role="status"
  aria-live="polite"
  aria-atomic="true"
>
  {#if active && visible}
    <LoaderCircleIcon
      class="size-3.5 shrink-0 animate-spin motion-reduce:animate-none"
      aria-hidden="true"
    />
    <span>{label}</span>
  {:else}
    {idleLabel}
  {/if}
</span>
