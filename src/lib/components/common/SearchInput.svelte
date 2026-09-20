<!--
  Component: SearchInput
  Purpose: Shared search field with consistent magnifier icon treatment.
-->

<script lang="ts">
  import SearchIcon from '@lucide/svelte/icons/search';
  import XIcon from '@lucide/svelte/icons/x';
  import { Input } from '$lib/components/ui/input';
  import { cn, type WithElementRef } from '$lib/utils.js';
  import type { HTMLInputAttributes } from 'svelte/elements';

  type SearchInputProps = WithElementRef<Omit<HTMLInputAttributes, 'type' | 'files'>> & {
    type?: 'text' | 'search';
    inputClass?: string;
    iconClass?: string;
    clearLabel?: string;
    showFocusRing?: boolean;
  };

  let {
    ref = $bindable(null),
    value = $bindable(''),
    type = 'text',
    placeholder = '',
    inputClass = '',
    iconClass = '',
    clearLabel = '',
    showFocusRing = false,
    class: className = '',
    ...restProps
  }: SearchInputProps = $props();

  function clearSearch(): void {
    value = '';
    ref?.focus();
  }
</script>

<div class={cn('relative', className)}>
  <SearchIcon
    class={cn(
      'pointer-events-none absolute top-1/2 left-3.5 z-10 h-[18px] w-[18px] -translate-y-1/2 text-foreground/35 dark:text-foreground/40',
      iconClass
    )}
    strokeWidth={2.2}
    absoluteStrokeWidth
  />
  <Input
    {type}
    bind:ref
    bind:value
    {placeholder}
    class={cn(
      'pl-10',
      clearLabel && value ? 'pr-9' : '',
      showFocusRing
        ? 'focus-visible:ring-[3px] focus-visible:ring-ring/60'
        : 'focus-visible:ring-0 focus-visible:ring-transparent',
      inputClass
    )}
    {...restProps}
  />
  {#if clearLabel && value}
    <button
      type="button"
      aria-label={clearLabel}
      class="absolute top-1/2 right-2 inline-flex size-7 -translate-y-1/2 items-center justify-center rounded-sm text-settings-muted-foreground outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
      onclick={clearSearch}
    >
      <XIcon class="size-3.5" />
    </button>
  {/if}
</div>
