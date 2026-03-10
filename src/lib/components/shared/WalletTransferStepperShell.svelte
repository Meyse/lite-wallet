<script lang="ts">
  import type { Snippet } from 'svelte';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Button } from '$lib/components/ui/button';
  import StepperWithAsideLayout from '$lib/components/shared/StepperWithAsideLayout.svelte';
  import { i18nStore } from '$lib/i18n';
  import type { StepStatus } from '$lib/components/wallet/sections/transfer-wizard/types';

  type WalletTransferStepperShellProps = {
    currentStep: number;
    totalSteps: number;
    steps?: { id: string; label: string; status: StepStatus }[];
    onClose?: () => void;
    closeDisabled?: boolean;
    showCloseButton?: boolean;
    dirty?: boolean;
    showAside?: boolean;
    mobileAsideLabel?: string;
    mobileAsideTitle?: string;
    children?: Snippet;
    aside?: Snippet;
    footer?: Snippet;
    footerAside?: Snippet;
  };

  const defaultCloseHandler = () => {};

   
  let {
    currentStep,
    totalSteps,
    steps = [],
    onClose = defaultCloseHandler,
    closeDisabled = false,
    showCloseButton = true,
    dirty = false,
    showAside = true,
    mobileAsideLabel = '',
    mobileAsideTitle = '',
    children,
    aside,
    footer,
    footerAside
  }: WalletTransferStepperShellProps = $props();
   

  const i18n = $derived($i18nStore);
  const asideSnippet = $derived(aside);
  const footerSnippet = $derived(footer);
  const footerAsideSnippet = $derived(footerAside);
  let showDiscardDialog = $state(false);

  function requestClose() {
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
    showDiscardDialog = false;
    onClose();
  }
</script>

<StepperWithAsideLayout
  {currentStep}
  {totalSteps}
  {steps}
  onClose={requestClose}
  {closeDisabled}
  {showCloseButton}
  {showAside}
  {mobileAsideLabel}
  {mobileAsideTitle}
>
  {#snippet aside()}
    {@render asideSnippet?.()}
  {/snippet}

  {#snippet footer()}
    {@render footerSnippet?.()}
  {/snippet}

  {#snippet footerAside()}
    {@render footerAsideSnippet?.()}
  {/snippet}

  {@render children?.()}
</StepperWithAsideLayout>

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
