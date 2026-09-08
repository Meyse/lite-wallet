<!--
  Component: StandardRightSheet
  Purpose: Shared right-side sheet shell with fixed width and consistent title/body spacing.
-->

<script lang="ts">
  import type { Snippet } from 'svelte';
  import * as Sheet from '$lib/components/ui/sheet';
  import { cn } from '$lib/utils.js';

  type StandardRightSheetProps = {
    isOpen?: boolean;
    title: string;
    hideTitle?: boolean;
    onOpenAutoFocus?: (event: Event) => void;
    titleClass?: string;
    bodyClass?: string;
    children?: Snippet;
  };

   
  let {
    isOpen = $bindable(false),
    title,
    hideTitle = false,
    onOpenAutoFocus = undefined,
    titleClass = '',
    bodyClass = '',
    children
  }: StandardRightSheetProps = $props();
   
</script>

<Sheet.Root bind:open={isOpen}>
  <Sheet.Content
    side="right"
    class="w-[378px] max-w-[92vw] p-6"
    {onOpenAutoFocus}
  >
    {#snippet children()}
      <div class="flex h-full flex-col">
        {#if !hideTitle}
          <Sheet.Header class="gap-1 p-0 pr-8 pt-4">
            <Sheet.Title class={cn('text-base', titleClass)}>{title}</Sheet.Title>
          </Sheet.Header>
        {/if}

        <div class={cn(hideTitle ? 'mt-8' : 'mt-5', 'flex min-h-0 flex-1 flex-col', bodyClass)}>
          {@render children?.()}
        </div>
      </div>
    {/snippet}
  </Sheet.Content>
</Sheet.Root>
