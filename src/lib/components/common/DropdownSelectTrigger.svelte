<!--
  Component: DropdownSelectTrigger
  Purpose: Shared borderless trigger for compact settings and regular onboarding menus.
-->

<script lang="ts">
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import { cn } from '$lib/utils.js';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type DropdownSelectTriggerProps = HTMLButtonAttributes & {
    value: string;
    size?: 'compact' | 'regular';
  };

  let {
    value,
    size = 'compact',
    class: className = '',
    type = 'button',
    ...restProps
  }: DropdownSelectTriggerProps = $props();
</script>

<button
  {type}
  class={cn(
    'inline-flex items-center justify-between gap-3 border-0 text-left font-normal text-foreground outline-none focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-settings-focus-ring',
    size === 'compact'
      ? 'h-7 w-[152px] rounded-[5px] bg-settings-control-surface px-2.5 text-[13px] leading-[18px]'
      : 'h-9 w-full rounded-md bg-settings-surface px-3 text-sm leading-5',
    className
  )}
  {...restProps}
>
  <span class="min-w-0 flex-1 truncate">{value}</span>
  <ChevronDownIcon class={size === 'compact' ? 'size-3.5 shrink-0' : 'size-4 shrink-0'} />
</button>
