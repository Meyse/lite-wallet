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
    'text-muted-foreground inline-flex items-center gap-1 text-xs underline-offset-4 transition-colors hover:text-foreground hover:underline disabled:pointer-events-none disabled:opacity-50',
    className
  )}
  {...restProps}
>
  {@render children?.()}
</button>
