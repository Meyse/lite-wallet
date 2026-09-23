<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog';
  import { i18nStore } from '$lib/i18n';
  import HelpCenter from './HelpCenter.svelte';
  import { createHelpReaderState } from '$lib/help/readerState';
  import type { HelpDestination } from '$lib/help/links';
  import type { HelpArticleId } from '$lib/help/catalog';

  let {
    open = $bindable(false),
    backLabel,
    initialArticleId = null,
    onNavigate,
  }: {
    open?: boolean;
    backLabel: string;
    initialArticleId?: HelpArticleId | null;
    onNavigate?: (destination: HelpDestination) => void;
  } = $props();
  // svelte-ignore state_referenced_locally
  let readerState = $state(createHelpReaderState(initialArticleId));
  const i18n = $derived($i18nStore);
  let content = $state<HTMLDivElement | null>(null);
  let returnFocus: HTMLElement | null = null;
</script>

<Dialog.Root bind:open>
  <Dialog.Content
    bind:ref={content}
    showCloseButton={false}
    aria-describedby={undefined}
    class="inset-0 flex h-dvh max-h-dvh w-screen max-w-none translate-x-0 translate-y-0 gap-0 overflow-hidden rounded-none border-0 bg-app-canvas p-0 shadow-none duration-0 data-[state=closed]:animate-none data-[state=open]:animate-none sm:max-w-none"
    onOpenAutoFocus={(event) => {
      event.preventDefault();
      returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      const destination = readerState.focusDestination
        ? content?.querySelector<HTMLElement>(
            `[data-help-destination="${readerState.focusDestination}"]`
          )
        : null;
      const target =
        destination ??
        content?.querySelector<HTMLElement>('[data-help-search]') ??
        content?.querySelector<HTMLElement>('[data-help-heading]');
      target?.focus({ preventScroll: true });
    }}
    onCloseAutoFocus={(event) => {
      if (returnFocus?.isConnected && !returnFocus.closest('[inert]')) {
        event.preventDefault();
        returnFocus.focus();
      }
    }}
  >
    <!-- Match the wallet's title bar and keep the navigation controls below it. -->
    <div class="absolute inset-x-0 top-0 z-40 h-11" data-tauri-drag-region aria-hidden="true"></div>
    <Dialog.Title class="sr-only">{i18n.t('helpCenter.title')}</Dialog.Title>
    <HelpCenter
      {backLabel}
      {initialArticleId}
      bind:readerState
      {onNavigate}
      onClose={() => {
        open = false;
      }}
    />
  </Dialog.Content>
</Dialog.Root>
