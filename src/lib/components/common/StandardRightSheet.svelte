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
    closeLabel?: string;
    onOpenAutoFocus?: (event: Event) => void;
    titleClass?: string;
    bodyClass?: string;
    children?: Snippet;
  };

  let {
    isOpen = $bindable(false),
    title,
    hideTitle = false,
    closeLabel = 'Close',
    onOpenAutoFocus = undefined,
    titleClass = '',
    bodyClass = '',
    children,
  }: StandardRightSheetProps = $props();
</script>

<Sheet.Root bind:open={isOpen}>
  <Sheet.Content
    side="right"
    class="settings-sheet-panel w-[378px] max-w-[92vw] border-0 bg-settings-sheet-surface p-6 data-[state=closed]:duration-150 data-[state=open]:duration-200"
    overlayClass="bg-black/15 dark:bg-black/45"
    closeClass="end-6 top-9 size-7 rounded-md text-settings-muted-foreground hover:text-foreground focus-visible:ring-settings-focus-ring"
    {closeLabel}
    {onOpenAutoFocus}
  >
    {#snippet children()}
      <div class="flex h-full flex-col">
        {#if !hideTitle}
          <Sheet.Header class="mt-3 h-7 gap-1 p-0 pr-8">
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
