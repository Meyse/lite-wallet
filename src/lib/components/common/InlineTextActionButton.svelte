<script lang="ts">
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import type { Snippet } from 'svelte';
  import { cn, type WithElementRef } from '$lib/utils.js';

  type InlineTextActionTone = 'accent' | 'muted' | 'destructive';

  type InlineTextActionButtonProps = WithElementRef<HTMLButtonAttributes> & {
    children?: Snippet;
    tone?: InlineTextActionTone;
  };

  const toneClass = {
    accent: 'text-text-action hover:text-text-action',
    muted: 'text-muted-foreground hover:text-foreground',
    destructive: 'text-destructive hover:text-destructive',
  } satisfies Record<InlineTextActionTone, string>;

  let {
    class: className = '',
    type = 'button',
    disabled = false,
    tone = 'accent',
    children,
    ...restProps
  }: InlineTextActionButtonProps = $props();
</script>

<button
  {type}
  {disabled}
  class={cn(
    'inline-flex items-center gap-1 rounded-sm text-xs underline-offset-4 transition-colors outline-none hover:underline focus-visible:ring-2 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 disabled:hover:no-underline',
    toneClass[tone],
    className
  )}
  {...restProps}
>
  {@render children?.()}
</button>
