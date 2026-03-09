<script lang="ts">
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import type { Snippet } from 'svelte';
  import { cn, type WithElementRef } from '$lib/utils.js';

  type InlineTextActionButtonProps = WithElementRef<HTMLButtonAttributes> & {
    children?: Snippet;
  };

   
  let {
    class: className = '',
    type = 'button',
    disabled = false,
    children,
    ...restProps
  }: InlineTextActionButtonProps = $props();
   
</script>

<button
  {type}
  {disabled}
  class={cn(
    'text-muted-foreground focus-visible:ring-ring/50 inline-flex items-center gap-1 rounded-sm text-xs transition-colors outline-none hover:text-foreground focus-visible:ring-2 disabled:pointer-events-none disabled:opacity-50',
    className
  )}
  {...restProps}
>
  {@render children?.()}
</button>
