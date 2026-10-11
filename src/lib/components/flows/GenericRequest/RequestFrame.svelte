<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import XIcon from '@lucide/svelte/icons/x';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { i18nStore } from '$lib/i18n';

  /**
   * The window a request takes over: an optional step rail from the top of the
   * window, like the wallet sidebar, and one main column of at most 560px with
   * Close above it and the step actions below. The requester is named in the
   * rail or the header, so the top bar holds no text; `label` names the region
   * for assistive technology.
   */
  let {
    label,
    onClose,
    closeDisabled = false,
    rail,
    children,
    footer,
  }: {
    label: string;
    onClose: () => void;
    closeDisabled?: boolean;
    rail?: Snippet;
    children: Snippet;
    footer: Snippet;
  } = $props();

  const i18n = $derived($i18nStore);

  onMount(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || event.defaultPrevented || closeDisabled) return;
      // A right sheet opened from a step closes itself first.
      if (document.querySelector('[role="dialog"]')) return;
      event.preventDefault();
      onClose();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  });
</script>

<section class="absolute inset-0 z-50 flex bg-background dark:bg-app-canvas" aria-label={label}>
  {#if rail}
    <aside
      class="relative flex w-60 shrink-0 flex-col gap-6 overflow-y-auto border-r border-sidebar-divider bg-sidebar-surface px-3 pt-16 pb-4"
    >
      <div
        class="absolute inset-x-0 top-0 h-[52px]"
        data-tauri-drag-region
        aria-hidden="true"
      ></div>
      {@render rail()}
    </aside>
  {/if}

  <div class="flex min-w-0 flex-1 flex-col">
    <header
      class={`relative flex h-[52px] shrink-0 items-center justify-end pr-3.5 ${rail ? 'pl-3.5' : 'pl-[88px]'}`}
    >
      <div class="absolute inset-0" data-tauri-drag-region aria-hidden="true"></div>
      <button
        type="button"
        class="relative inline-flex size-8 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-hidden disabled:opacity-50"
        onclick={onClose}
        disabled={closeDisabled}
        aria-label={i18n.t('genericRequest.flow.close')}
      >
        <XIcon class="size-4" />
      </button>
    </header>

    <ScrollArea.Root class="min-h-0 flex-1">
      <ScrollArea.Viewport class="h-full">
        <div class="px-7 pt-4 pb-8">
          <div class="mx-auto flex w-full max-w-[560px] flex-col gap-5">
            {@render children()}
          </div>
        </div>
      </ScrollArea.Viewport>
      <ScrollArea.Scrollbar orientation="vertical" />
    </ScrollArea.Root>

    <footer
      class="flex shrink-0 items-center justify-between gap-3 border-t border-border/70 px-6 py-3"
    >
      {@render footer()}
    </footer>
  </div>
</section>
