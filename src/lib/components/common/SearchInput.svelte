<!--
  Component: SearchInput
  Purpose: Shared search field with consistent magnifier icon treatment.
-->

<script lang="ts">
  import SearchIcon from '@lucide/svelte/icons/search';
  import { Input } from '$lib/components/ui/input';
  import { cn, type WithElementRef } from '$lib/utils.js';
  import type { HTMLInputAttributes } from 'svelte/elements';

  type SearchInputProps = WithElementRef<Omit<HTMLInputAttributes, 'type' | 'files'>> & {
    inputClass?: string;
    iconClass?: string;
  };

   
  let {
    ref = $bindable(null),
    value = $bindable(''),
    placeholder = '',
    inputClass = '',
    iconClass = '',
    class: className = '',
    ...restProps
  }: SearchInputProps = $props();
   
</script>

<div class={cn('relative', className)}>
  <SearchIcon
    class={cn(
      'text-foreground/35 dark:text-foreground/40 pointer-events-none absolute top-1/2 left-3.5 z-10 h-[18px] w-[18px] -translate-y-1/2',
      iconClass
    )}
    strokeWidth={2.2}
    absoluteStrokeWidth
  />
  <Input
    type="text"
    bind:ref
    bind:value
    {placeholder}
    class={cn('pl-10 focus-visible:ring-0 focus-visible:ring-transparent', inputClass)}
    {...restProps}
  />
</div>
