<!--
  Component: NavigationBackButton
  Purpose: Shared contextual back action for full-page detail views and nested sheet views.
-->

<script lang="ts">
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import type { ComponentProps } from 'svelte';
  import { Button } from '$lib/components/ui/button';
  import { cn } from '$lib/utils.js';

  type NavigationBackButtonProps = Omit<
    ComponentProps<typeof Button>,
    'children' | 'size' | 'variant'
  > & {
    label: string;
    tone?: 'default' | 'settings';
  };

  let {
    label,
    tone = 'default',
    class: className,
    ...restProps
  }: NavigationBackButtonProps = $props();
</script>

<Button
  variant="ghost"
  size="sm"
  class={cn(
    '-ml-1 h-8 gap-1.5 px-1 text-[13px] leading-5 font-normal hover:bg-transparent focus-visible:ring-2 has-[>svg]:px-1 dark:hover:bg-transparent',
    tone === 'settings'
      ? 'text-settings-muted-foreground hover:text-foreground focus-visible:ring-settings-focus-ring'
      : 'text-muted-foreground hover:text-foreground',
    className
  )}
  {...restProps}
>
  <ArrowLeftIcon class="size-4" aria-hidden="true" />
  {label}
</Button>
