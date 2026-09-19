<script lang="ts">
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import type { Snippet } from 'svelte';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Button } from '$lib/components/ui/button';
  import StepperWithAsideLayout from '$lib/components/shared/StepperWithAsideLayout.svelte';
  import { i18nStore } from '$lib/i18n';
  import type { StepStatus } from '$lib/components/wallet/sections/transfer-wizard/types';

  type WalletTransferStepperShellProps = {
    active?: boolean;
    embedded?: boolean;
    header?: Snippet;
    currentStep: number;
    totalSteps: number;
    steps?: { id: string; label: string; status: StepStatus }[];
    onClose?: () => void;
    closeDisabled?: boolean;
    showCloseButton?: boolean;
    showProgress?: boolean;
    dirty?: boolean;
    showAside?: boolean;
    mobileAsideLabel?: string;
    mobileAsideTitle?: string;
    children?: Snippet;
    aside?: Snippet;
    footer?: Snippet<[{ requestClose: () => void }]>;
    footerAside?: Snippet;
  };

  const defaultCloseHandler = () => {};

  let {
    active = true,
    embedded = false,
    header,
    currentStep,
    totalSteps,
    steps = [],
    onClose = defaultCloseHandler,
    closeDisabled = false,
    showCloseButton = true,
    showProgress = true,
    dirty = false,
    showAside = true,
    mobileAsideLabel = '',
    mobileAsideTitle = '',
    children,
    aside,
    footer,
    footerAside,
  }: WalletTransferStepperShellProps = $props();

  const i18n = $derived($i18nStore);
  const asideSnippet = $derived(aside);
  const footerSnippet = $derived(footer);
  const footerAsideSnippet = $derived(footerAside);
  let showDiscardDialog = $state(false);

  $effect(() => {
    if (!active) showDiscardDialog = false;
  });

  function handleEmbeddedEscape(event: KeyboardEvent) {
    if (!embedded || !active || event.key !== 'Escape' || event.defaultPrevented) return;
    // Portal dialogs and pickers own Escape before the underlying transfer.
    if (document.querySelector('[role="dialog"], [role="alertdialog"]')) return;
    event.preventDefault();
    requestClose();
  }

  function requestClose() {
    if (!active) return;
    if (showDiscardDialog) {
      showDiscardDialog = false;
      return;
    }
    if (closeDisabled) return;
    if (dirty) {
      showDiscardDialog = true;
      return;
    }
    onClose();
  }

  function confirmDiscard() {
    if (!active || closeDisabled) return;
    showDiscardDialog = false;
    onClose();
  }
</script>

<svelte:window onkeydown={handleEmbeddedEscape} />

{#if embedded}
  <section class="@container/transfer flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
    <header class="shrink-0 px-6" data-transfer-heading tabindex="-1">
      {@render header?.()}
    </header>
    <ScrollArea.Root class="min-h-0 flex-1">
      <ScrollArea.Viewport class="h-full">
        <div class="mx-auto w-full max-w-[888px] px-6 py-5">{@render children?.()}</div>
      </ScrollArea.Viewport>
      <ScrollArea.Scrollbar orientation="vertical" />
    </ScrollArea.Root>
    <footer class="shrink-0 border-t border-border/70 bg-background px-6 py-3">
      <div class="mx-auto w-full max-w-[840px]">{@render footerSnippet?.({ requestClose })}</div>
    </footer>
  </section>
{:else}
  <StepperWithAsideLayout
    {active}
    {currentStep}
    {totalSteps}
    {steps}
    onClose={requestClose}
    {closeDisabled}
    {showCloseButton}
    {showProgress}
    {showAside}
    {mobileAsideLabel}
    {mobileAsideTitle}
  >
    {#snippet aside()}
      {@render asideSnippet?.()}
    {/snippet}

    {#snippet footer()}
      {@render footerSnippet?.({ requestClose })}
    {/snippet}

    {#snippet footerAside()}
      {@render footerAsideSnippet?.()}
    {/snippet}

    {@render children?.()}
  </StepperWithAsideLayout>
{/if}

<Dialog.Root
  open={showDiscardDialog}
  onOpenChange={(open) => {
    if (!open) showDiscardDialog = false;
  }}
>
  <Dialog.Content class="max-w-md">
    <Dialog.Header>
      <Dialog.Title>{i18n.t('wallet.transfer.closeDiscardTitle')}</Dialog.Title>
      <Dialog.Description>{i18n.t('wallet.transfer.closeDiscardDescription')}</Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer class="flex justify-end gap-3">
      <Button variant="secondary" onclick={() => (showDiscardDialog = false)}>
        {i18n.t('common.cancel')}
      </Button>
      <Button variant="destructive" onclick={confirmDiscard}>
        {i18n.t('wallet.transfer.closeDiscardConfirm')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
