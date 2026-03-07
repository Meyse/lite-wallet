<!--
  Component: PasswordConfirmOverlay
  Purpose: Reusable full-screen in-app password confirmation overlay.
-->

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { i18nStore } from '$lib/i18n';

  type PasswordConfirmOverlayProps = {
    isOpen?: boolean;
    password?: string;
    loading?: boolean;
    errorMessage?: string;
    placeholder?: string;
    confirmLabel?: string;
    loadingLabel?: string;
    cancelLabel?: string;
    onConfirm?: () => void;
    onCancel?: () => void;
  };

   
  let {
    isOpen = $bindable(false),
    password = $bindable(''),
    loading = false,
    errorMessage = '',
    placeholder = '',
    confirmLabel = '',
    loadingLabel = '',
    cancelLabel = '',
    onConfirm = () => {},
    onCancel = () => {}
  }: PasswordConfirmOverlayProps = $props();
   

  const i18n = $derived($i18nStore);
  const resolvedConfirmLabel = $derived(confirmLabel || i18n.t('wallet.settings.recovery.revealConfirm'));
  const resolvedLoadingLabel = $derived(loadingLabel || i18n.t('wallet.settings.recovery.revealLoading'));
  const resolvedCancelLabel = $derived(cancelLabel || i18n.t('common.cancel'));
  const canSubmit = $derived(password.trim().length > 0 && !loading);

  function closeOverlay(): void {
    if (loading) return;
    isOpen = false;
    onCancel();
  }

  function confirm(): void {
    if (!canSubmit) return;
    onConfirm();
  }

</script>

{#if isOpen}
  <div class="fixed inset-0 z-[70] flex items-center justify-center bg-background px-6">
    <div
      class="w-full max-w-sm space-y-3"
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      onkeydown={(event) => {
        if (event.key === 'Enter') {
          confirm();
        }
        if (event.key === 'Escape') {
          closeOverlay();
        }
      }}
    >
      <Input
        type="password"
        bind:value={password}
        {placeholder}
        autofocus
      />

      {#if errorMessage}
        <p class="text-destructive mt-2 text-xs">{errorMessage}</p>
      {/if}

      <div class="mt-3 flex justify-end gap-2">
        <Button variant="secondary" size="sm" onclick={closeOverlay}>
          {resolvedCancelLabel}
        </Button>
        <Button size="sm" onclick={confirm} disabled={!canSubmit}>
          {loading ? resolvedLoadingLabel : resolvedConfirmLabel}
        </Button>
      </div>
    </div>
  </div>
{/if}
