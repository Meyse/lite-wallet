<!--
  Component: PrivateVerusSettings
  Purpose: Focused settings detail page for private Verus setup and status.
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import CopyButton from '$lib/components/ui/copy-button/copy-button.svelte';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { Button } from '$lib/components/ui/button';
  import { Label } from '$lib/components/ui/label';
  import { Textarea } from '$lib/components/ui/textarea';
  import { i18nStore } from '$lib/i18n';
  import * as walletService from '$lib/services/walletService';
  import type { WalletNetwork } from '$lib/types/wallet';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import { extractWalletErrorType } from '$lib/utils/walletErrors';

  type PrivateVerusSettingsProps = {
    walletNetwork: WalletNetwork;
    onBack: () => void;
    onOpenRecovery: () => void;
  };

  const { walletNetwork, onBack, onOpenRecovery }: PrivateVerusSettingsProps = $props();

  const i18n = $derived($i18nStore);
  const networkLabel = $derived(
    i18n.t(walletNetwork === 'testnet' ? 'common.network.testnet' : 'common.network.mainnet')
  );
  const addressCopyFeedback = new TimedValueState<'copied' | 'failed'>();

  let statusState = $state<'loading' | 'configured' | 'unconfigured' | 'error'>('loading');
  let shieldedAddress = $state('');
  let setupError = $state('');
  let activationMessage = $state('');
  let importText = $state('');
  let showImportSheet = $state(false);
  let showAdvanced = $state(false);
  let showBackupSheet = $state(false);
  let generatedSeedPhrase = $state('');
  let backupAcknowledged = $state(false);
  let submittingMode = $state<'reuse_primary' | 'create_new' | 'import_text' | null>(null);
  let requestGeneration = 0;
  let disposed = false;

  function invalidateRequests(): void {
    requestGeneration += 1;
  }

  function clearImport(): void {
    importText = '';
  }

  function clearBackup(): void {
    generatedSeedPhrase = '';
    backupAcknowledged = false;
  }

  async function loadStatus(): Promise<void> {
    const generation = ++requestGeneration;
    statusState = 'loading';
    setupError = '';
    try {
      const status = await walletService.getDlightSeedStatus();
      if (disposed || generation !== requestGeneration) return;
      statusState = status.configured ? 'configured' : 'unconfigured';
      shieldedAddress = status.shieldedAddress ?? '';
    } catch {
      if (disposed || generation !== requestGeneration) return;
      statusState = 'error';
      shieldedAddress = '';
    }
  }

  function setupErrorFor(error: unknown): string {
    const errorType = extractWalletErrorType(error);
    if (errorType === 'InvalidSeedPhrase') {
      return i18n.t('wallet.settings.privateVerus.error.invalidPrimary');
    }
    if (errorType === 'InvalidImportText') {
      return i18n.t('wallet.settings.privateVerus.error.invalidImport');
    }
    return i18n.t('wallet.settings.privateVerus.error.generic');
  }

  async function setup(mode: 'reuse_primary' | 'create_new' | 'import_text'): Promise<void> {
    if (submittingMode) return;

    const generation = ++requestGeneration;
    submittingMode = mode;
    setupError = '';
    activationMessage = '';

    try {
      const result = await walletService.setupDlightSeed({
        mode,
        importText: mode === 'import_text' ? importText.trim() : undefined,
      });
      if (disposed || generation !== requestGeneration) return;

      statusState = result.configured ? 'configured' : 'unconfigured';
      showAdvanced = false;
      activationMessage = i18n.t(
        result.requiresRelogin
          ? 'wallet.settings.privateVerus.activationRelogin'
          : 'wallet.settings.privateVerus.activationReady'
      );

      if (mode === 'create_new' && result.generatedSeedPhrase?.trim()) {
        generatedSeedPhrase = result.generatedSeedPhrase.trim();
        backupAcknowledged = false;
        showBackupSheet = true;
      }

      if (mode === 'import_text') {
        showImportSheet = false;
        clearImport();
      }
    } catch (error) {
      if (disposed || generation !== requestGeneration) return;
      setupError = setupErrorFor(error);
    } finally {
      if (!disposed && generation === requestGeneration) {
        submittingMode = null;
      }
    }
  }

  async function copyAddress(): Promise<void> {
    const copied = await writeClipboardText(shieldedAddress);
    addressCopyFeedback.set(copied ? 'copied' : 'failed');
  }

  function openImport(): void {
    setupError = '';
    clearImport();
    showImportSheet = true;
  }

  function handleImportOpenChange(open: boolean): void {
    showImportSheet = open;
    if (!open) {
      invalidateRequests();
      submittingMode = null;
      clearImport();
    }
  }

  function handleBackupOpenChange(open: boolean): void {
    showBackupSheet = open;
    if (!open) clearBackup();
  }

  function closeBackup(): void {
    showBackupSheet = false;
    clearBackup();
  }

  function preventBackupDismiss(event: Event): void {
    if (!backupAcknowledged) event.preventDefault();
  }

  onMount(() => {
    disposed = false;
    void loadStatus();
    return () => {
      disposed = true;
      invalidateRequests();
      submittingMode = null;
      clearImport();
      clearBackup();
    };
  });
</script>

<div
  class="mx-auto flex h-full min-h-0 w-full max-w-5xl flex-col bg-app-canvas px-6 pt-0 pb-6 sm:px-8"
>
  <section class="flex min-h-0 flex-1 flex-col overflow-auto pt-2">
    <header class="flex h-[60px] shrink-0 flex-col gap-3">
      <button
        type="button"
        class="inline-flex h-5 w-fit items-center gap-1 text-[13px] leading-5 text-settings-muted-foreground outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
        onclick={onBack}
      >
        <ChevronLeftIcon class="size-4" />
        {i18n.t('wallet.settings.backLabel')}
      </button>
      <h2 class="text-xl leading-7 font-semibold tracking-[-0.015em]">
        {i18n.t('wallet.settings.privateVerus.title')}
      </h2>
    </header>

    {#if statusState === 'loading'}
      <div class="mt-5 rounded-lg bg-settings-surface p-4">
        <p class="text-[13px] leading-5 text-settings-muted-foreground">
          {i18n.t('common.loading')}
        </p>
      </div>
    {:else if statusState === 'error'}
      <div class="mt-5 rounded-lg bg-settings-surface p-4">
        <p class="text-sm leading-5 font-medium">
          {i18n.t('wallet.settings.privateVerus.statusUnavailable')}
        </p>
        <p class="mt-1 text-[13px] leading-5 text-settings-muted-foreground">
          {i18n.t('wallet.settings.privateVerus.statusLoadError')}
        </p>
        <Button class="mt-4" size="sm" variant="secondary" onclick={() => void loadStatus()}>
          {i18n.t('common.retry')}
        </Button>
      </div>
    {:else if statusState === 'configured' && !showAdvanced}
      <div class="mt-5 rounded-lg bg-settings-surface p-4">
        <div class="flex items-start justify-between gap-4">
          <div class="min-w-0">
            <p class="text-sm leading-5 font-medium">
              {i18n.t('wallet.settings.privateVerus.statusConfigured')}
            </p>
            <p class="mt-1 text-[13px] leading-5 text-settings-muted-foreground">{networkLabel}</p>
          </div>
          <span class="mt-1 size-2 shrink-0 rounded-full bg-emerald-500"></span>
        </div>

        {#if shieldedAddress}
          <div class="mt-4 border-t border-border/50 pt-3">
            <p class="text-xs leading-5 text-settings-muted-foreground">
              {i18n.t('wallet.settings.privateVerus.statusAddress')}
            </p>
            <div class="mt-1 flex items-center gap-2">
              <p class="min-w-0 flex-1 truncate font-mono text-xs leading-5">{shieldedAddress}</p>
              <CopyButton
                size="xs"
                copied={addressCopyFeedback.current === 'copied'}
                aria-label={i18n.t(
                  addressCopyFeedback.current === 'copied' ? 'common.copied' : 'common.copy'
                )}
                onclick={() => void copyAddress()}
              />
            </div>
            {#if addressCopyFeedback.current === 'failed'}
              <p class="mt-1 text-xs text-destructive" role="status">
                {i18n.t('common.copyFailed')}
              </p>
            {/if}
          </div>
        {/if}

        {#if activationMessage}
          <p class="mt-3 text-[13px] leading-5 text-settings-muted-foreground" role="status">
            {activationMessage}
          </p>
        {/if}
      </div>

      <div class="mt-3 overflow-hidden rounded-lg bg-settings-surface">
        <button
          type="button"
          class="flex min-h-16 w-full items-center gap-4 px-4 py-3 text-left outline-none hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:ring-inset"
          onclick={onOpenRecovery}
        >
          <span class="min-w-0 flex-1">
            <span class="block text-sm leading-5 font-medium">
              {i18n.t('wallet.settings.privateVerus.recoveryTitle')}
            </span>
            <span class="mt-0.5 block text-xs leading-5 text-settings-muted-foreground">
              {i18n.t('wallet.settings.privateVerus.recoveryDescription')}
            </span>
          </span>
          <ChevronRightIcon class="size-4 shrink-0 text-settings-muted-foreground" />
        </button>

        <button
          type="button"
          class="flex min-h-16 w-full items-center gap-4 border-t border-border/50 px-4 py-3 text-left outline-none hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:ring-inset"
          onclick={() => {
            setupError = '';
            showAdvanced = true;
          }}
        >
          <span class="min-w-0 flex-1">
            <span class="block text-sm leading-5 font-medium">
              {i18n.t('wallet.settings.privateVerus.advanced')}
            </span>
            <span class="mt-0.5 block text-xs leading-5 text-settings-muted-foreground">
              {i18n.t('wallet.settings.privateVerus.advancedDescription')}
            </span>
          </span>
          <ChevronRightIcon class="size-4 shrink-0 text-settings-muted-foreground" />
        </button>
      </div>
    {:else}
      {#if showAdvanced}
        <div class="mt-5 rounded-lg bg-amber-50 p-4 dark:bg-amber-500/10">
          <p class="text-sm leading-5 font-medium">
            {i18n.t('wallet.settings.privateVerus.replaceTitle')}
          </p>
          <p class="mt-1 text-[13px] leading-5 text-settings-muted-foreground">
            {i18n.t('wallet.settings.privateVerus.advancedWarning')}
          </p>
        </div>
      {:else}
        <div class="mt-5 rounded-lg bg-settings-surface p-4">
          <p class="text-sm leading-5 font-medium">
            {i18n.t('wallet.settings.privateVerus.statusNotConfigured')}
          </p>
          <p class="mt-1 text-[13px] leading-5 text-settings-muted-foreground">
            {i18n.t('wallet.settings.privateVerus.statusHelp')}
          </p>
        </div>
      {/if}

      <div class="mt-3 overflow-hidden rounded-lg bg-settings-surface">
        <button
          type="button"
          class="flex min-h-16 w-full items-center gap-4 px-4 py-3 text-left outline-none hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:ring-inset disabled:opacity-50"
          disabled={submittingMode !== null}
          onclick={() => void setup('reuse_primary')}
        >
          <span class="min-w-0 flex-1">
            <span class="block text-sm leading-5 font-medium">
              {submittingMode === 'reuse_primary'
                ? i18n.t('wallet.settings.privateVerus.settingUp')
                : i18n.t('wallet.settings.privateVerus.reusePrimary')}
            </span>
            <span class="mt-0.5 block text-xs leading-5 text-settings-muted-foreground">
              {i18n.t('wallet.settings.privateVerus.reuseDescription')}
            </span>
          </span>
          <ChevronRightIcon class="size-4 shrink-0 text-settings-muted-foreground" />
        </button>

        <button
          type="button"
          class="flex min-h-16 w-full items-center gap-4 border-t border-border/50 px-4 py-3 text-left outline-none hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:ring-inset disabled:opacity-50"
          disabled={submittingMode !== null}
          onclick={() => void setup('create_new')}
        >
          <span class="min-w-0 flex-1">
            <span class="block text-sm leading-5 font-medium">
              {submittingMode === 'create_new'
                ? i18n.t('wallet.settings.privateVerus.settingUp')
                : i18n.t('wallet.settings.privateVerus.createNew')}
            </span>
            <span class="mt-0.5 block text-xs leading-5 text-settings-muted-foreground">
              {i18n.t('wallet.settings.privateVerus.createDescription')}
            </span>
          </span>
          <ChevronRightIcon class="size-4 shrink-0 text-settings-muted-foreground" />
        </button>

        <button
          type="button"
          class="flex min-h-16 w-full items-center gap-4 border-t border-border/50 px-4 py-3 text-left outline-none hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:ring-inset disabled:opacity-50"
          disabled={submittingMode !== null}
          onclick={openImport}
        >
          <span class="min-w-0 flex-1">
            <span class="block text-sm leading-5 font-medium">
              {i18n.t('wallet.settings.privateVerus.importAction')}
            </span>
            <span class="mt-0.5 block text-xs leading-5 text-settings-muted-foreground">
              {i18n.t('wallet.settings.privateVerus.importDescription')}
            </span>
          </span>
          <ChevronRightIcon class="size-4 shrink-0 text-settings-muted-foreground" />
        </button>
      </div>

      {#if setupError}
        <p class="mt-3 text-[13px] leading-5 text-destructive" role="alert">{setupError}</p>
      {/if}

      {#if showAdvanced}
        <Button
          class="mt-4 w-fit"
          size="sm"
          variant="secondary"
          disabled={submittingMode !== null}
          onclick={() => {
            showAdvanced = false;
            setupError = '';
          }}
        >
          {i18n.t('common.cancel')}
        </Button>
      {/if}
    {/if}
  </section>
</div>

<StandardRightSheet
  bind:isOpen={showImportSheet}
  title={i18n.t('wallet.settings.privateVerus.importSheetTitle')}
  closeLabel={i18n.t('common.close')}
  onOpenChange={handleImportOpenChange}
>
  <div class="flex h-full flex-col">
    <Label for="private-verus-import" class="text-[13px] leading-5">
      {i18n.t('wallet.settings.privateVerus.importLabel')}
    </Label>
    <Textarea
      id="private-verus-import"
      variant="surface"
      class="mt-2 min-h-32 resize-none text-[13px] leading-5"
      placeholder={i18n.t('wallet.settings.privateVerus.importPlaceholder')}
      bind:value={importText}
    ></Textarea>
    <p class="mt-2 text-xs leading-5 text-settings-muted-foreground">
      {i18n.t('wallet.settings.privateVerus.importHelp')}
    </p>
    {#if setupError}
      <p class="mt-3 text-[13px] leading-5 text-destructive" role="alert">{setupError}</p>
    {/if}
    <div class="mt-auto flex justify-end gap-2 pt-6">
      <Button
        size="sm"
        variant="secondary"
        disabled={submittingMode !== null}
        onclick={() => handleImportOpenChange(false)}
      >
        {i18n.t('common.cancel')}
      </Button>
      <Button
        size="sm"
        disabled={submittingMode !== null || !importText.trim()}
        onclick={() => void setup('import_text')}
      >
        {submittingMode === 'import_text'
          ? i18n.t('wallet.settings.privateVerus.settingUp')
          : i18n.t('wallet.settings.privateVerus.importAction')}
      </Button>
    </div>
  </div>
</StandardRightSheet>

<StandardRightSheet
  bind:isOpen={showBackupSheet}
  title={i18n.t('wallet.settings.privateVerus.backupTitle')}
  closeLabel={i18n.t('common.close')}
  onOpenChange={handleBackupOpenChange}
  onEscapeKeydown={preventBackupDismiss}
  onInteractOutside={preventBackupDismiss}
>
  <div class="flex h-full flex-col">
    <p class="text-[13px] leading-5 text-settings-muted-foreground">
      {i18n.t('wallet.settings.privateVerus.backupDescription')}
    </p>
    <div class="mt-4 rounded-lg bg-settings-surface p-4">
      <p class="font-mono text-[13px] leading-6 break-words">{generatedSeedPhrase}</p>
      <Button
        class="mt-3"
        size="sm"
        variant="secondary"
        onclick={() => void writeClipboardText(generatedSeedPhrase)}
      >
        {i18n.t('wallet.settings.privateVerus.copyPhrase')}
      </Button>
    </div>
    <Label class="mt-4 flex items-start gap-3 text-[13px] leading-5 font-normal">
      <Checkbox bind:checked={backupAcknowledged} class="mt-0.5" />
      <span>{i18n.t('wallet.settings.privateVerus.savedAcknowledgement')}</span>
    </Label>
    <Button class="mt-auto" disabled={!backupAcknowledged} onclick={closeBackup}>
      {i18n.t('common.done')}
    </Button>
  </div>
</StandardRightSheet>
