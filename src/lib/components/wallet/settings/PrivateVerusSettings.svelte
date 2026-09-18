<!--
  Component: PrivateVerusSettings
  Purpose: Focused settings detail page for private Verus setup and status.
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
  import ShieldIcon from '@lucide/svelte/icons/shield';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import CopyButton from '$lib/components/ui/copy-button/copy-button.svelte';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { Button } from '$lib/components/ui/button';
  import { Label } from '$lib/components/ui/label';
  import { Textarea } from '$lib/components/ui/textarea';
  import { i18nStore } from '$lib/i18n';
  import * as walletService from '$lib/services/walletService';
  import type {
    DlightSeedSetupMode,
    SetupDlightSeedResult,
    WalletNetwork,
  } from '$lib/types/wallet';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import {
    type DlightSetupOperation,
    getDlightSetupOperation,
    startDlightSetupOperation,
    waitForDlightSetupOperation,
  } from '$lib/utils/dlightSetupCoordinator';
  import { extractWalletErrorType } from '$lib/utils/walletErrors';

  type PrivateVerusSettingsProps = {
    walletNetwork: WalletNetwork;
    walletSessionKey: string;
    onBack: () => void;
    onOpenRecovery: () => void;
  };

  const { walletNetwork, walletSessionKey, onBack, onOpenRecovery }: PrivateVerusSettingsProps =
    $props();

  const i18n = $derived($i18nStore);
  const networkLabel = $derived(
    i18n.t(walletNetwork === 'testnet' ? 'common.network.testnet' : 'common.network.mainnet')
  );
  const addressCopyFeedback = new TimedValueState<'copied' | 'failed'>();
  const backupCopyFeedback = new TimedValueState<'copied' | 'failed'>();

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
  let submittingMode = $state<DlightSeedSetupMode | null>(null);
  let statusGeneration = 0;
  let backupCopyGeneration = 0;
  let disposed = false;

  function invalidateStatusRequests(): void {
    statusGeneration += 1;
  }

  function clearImport(): void {
    importText = '';
  }

  function clearBackup(): void {
    backupCopyGeneration += 1;
    generatedSeedPhrase = '';
    backupAcknowledged = false;
    backupCopyFeedback.clear();
  }

  async function loadStatus(): Promise<void> {
    const generation = ++statusGeneration;
    statusState = 'loading';
    setupError = '';
    try {
      const status = await walletService.getDlightSeedStatus();
      if (disposed || generation !== statusGeneration) return;
      statusState = status.configured ? 'configured' : 'unconfigured';
      shieldedAddress = status.shieldedAddress ?? '';
    } catch {
      if (disposed || generation !== statusGeneration) return;
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

  async function refreshStatusAfterSetup(fallbackConfigured: boolean): Promise<void> {
    const generation = ++statusGeneration;
    try {
      const status = await walletService.getDlightSeedStatus();
      if (disposed || generation !== statusGeneration) return;
      statusState = status.configured ? 'configured' : 'unconfigured';
      shieldedAddress = status.shieldedAddress ?? '';
    } catch {
      if (disposed || generation !== statusGeneration) return;
      statusState = fallbackConfigured ? 'configured' : 'error';
      shieldedAddress = '';
    }
  }

  async function observeOwnedSetup(
    operation: DlightSetupOperation,
    ownerResult: Promise<SetupDlightSeedResult>
  ): Promise<void> {
    submittingMode = operation.mode;
    setupError = '';
    activationMessage = '';

    let result: SetupDlightSeedResult | null = null;
    try {
      result = await ownerResult;
      if (disposed) return;

      statusState = result.configured ? 'configured' : 'unconfigured';
      shieldedAddress = '';
      showAdvanced = false;
      activationMessage = i18n.t(
        result.requiresRelogin
          ? 'wallet.settings.privateVerus.activationRelogin'
          : 'wallet.settings.privateVerus.activationReady'
      );

      if (operation.mode === 'create_new' && result.generatedSeedPhrase?.trim()) {
        generatedSeedPhrase = result.generatedSeedPhrase.trim();
        backupAcknowledged = false;
        backupCopyFeedback.clear();
        showBackupSheet = true;
      }

      if (operation.mode === 'import_text') {
        showImportSheet = false;
        clearImport();
      }
    } catch (error) {
      if (disposed) return;
      setupError = setupErrorFor(error);
    }

    await refreshStatusAfterSetup(result?.configured ?? statusState === 'configured');
    if (disposed) return;
    submittingMode = null;
  }

  async function observePendingSetup(operation: DlightSetupOperation): Promise<void> {
    submittingMode = operation.mode;
    setupError = '';
    activationMessage = '';

    const settlement = await waitForDlightSetupOperation(operation);
    if (disposed) return;

    if (settlement.status === 'cancelled') {
      submittingMode = null;
      await loadStatus();
      return;
    }

    if (settlement.status === 'success') {
      statusState = settlement.configured ? 'configured' : 'unconfigured';
      shieldedAddress = '';
      showAdvanced = false;
      activationMessage = i18n.t(
        settlement.requiresRelogin
          ? 'wallet.settings.privateVerus.activationRelogin'
          : 'wallet.settings.privateVerus.activationReady'
      );
    } else {
      setupError = i18n.t('wallet.settings.privateVerus.error.generic');
    }

    await refreshStatusAfterSetup(
      settlement.status === 'success' ? settlement.configured : statusState === 'configured'
    );
    if (disposed) return;
    submittingMode = null;
  }

  async function setup(mode: DlightSeedSetupMode): Promise<void> {
    if (submittingMode) return;

    const importValue = mode === 'import_text' ? importText.trim() : undefined;
    invalidateStatusRequests();
    const setupOperation = startDlightSetupOperation(walletSessionKey, mode, () =>
      walletService.setupDlightSeed({
        mode,
        importText: importValue,
      })
    );
    if (!setupOperation.started) {
      void observePendingSetup(setupOperation.operation);
      return;
    }
    void observeOwnedSetup(setupOperation.operation, setupOperation.ownerResult);
  }

  async function copyAddress(): Promise<void> {
    const copied = await writeClipboardText(shieldedAddress);
    addressCopyFeedback.set(copied ? 'copied' : 'failed');
  }

  async function copyGeneratedPhrase(): Promise<void> {
    const generation = backupCopyGeneration;
    const phrase = generatedSeedPhrase;
    const copied = await writeClipboardText(phrase);
    if (
      disposed ||
      generation !== backupCopyGeneration ||
      !showBackupSheet ||
      phrase !== generatedSeedPhrase
    ) {
      return;
    }
    backupCopyFeedback.set(copied ? 'copied' : 'failed');
  }

  function openImport(): void {
    setupError = '';
    clearImport();
    showImportSheet = true;
  }

  function handleImportOpenChange(open: boolean): void {
    showImportSheet = open;
    if (!open) {
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
    const operation = getDlightSetupOperation(walletSessionKey);
    if (operation) {
      statusState = 'loading';
      shieldedAddress = '';
      void observePendingSetup(operation);
    } else {
      void loadStatus();
    }
    return () => {
      disposed = true;
      invalidateStatusRequests();
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
      <div
        class="mt-5 flex h-[74px] shrink-0 items-center gap-3 rounded-lg bg-settings-surface p-4"
      >
        <span
          class="flex size-8 shrink-0 items-center justify-center rounded-full bg-emerald-500/12"
        >
          <ShieldCheckIcon class="size-[18px] text-emerald-600 dark:text-emerald-400" />
        </span>
        <div class="min-w-0 flex-1">
          <p class="text-sm leading-5 font-medium">
            {i18n.t('wallet.settings.privateVerus.statusConfigured')}
          </p>
          <p class="mt-0.5 text-[13px] leading-5 text-settings-muted-foreground">{networkLabel}</p>
        </div>
      </div>

      {#if shieldedAddress}
        <div class="mt-3 h-20 rounded-lg bg-settings-surface p-4">
          <p class="text-xs leading-5 text-settings-muted-foreground">
            {i18n.t('wallet.settings.privateVerus.statusAddress')}
          </p>
          <div class="mt-1 flex items-center gap-2">
            <IdentifierText
              value={shieldedAddress}
              mode="compact"
              class="min-w-0 flex-1 font-mono text-xs leading-5"
            />
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

      <button
        type="button"
        class="mt-3 flex min-h-16 w-full items-center gap-4 rounded-lg bg-settings-surface px-4 py-3 text-left outline-none hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
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
        class="mt-3 flex h-10 w-full items-center gap-2 rounded-md px-1 text-left text-[13px] leading-5 text-settings-muted-foreground outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
        onclick={() => {
          setupError = '';
          showAdvanced = true;
        }}
      >
        <span class="flex-1">{i18n.t('wallet.settings.privateVerus.advanced')}</span>
        <ChevronRightIcon class="size-4 shrink-0" />
      </button>
    {:else}
      {#if showAdvanced}
        <div class="mt-5 rounded-lg bg-settings-surface p-4">
          <p class="text-sm leading-5 font-medium">
            {i18n.t('wallet.settings.privateVerus.replaceTitle')}
          </p>
          <p class="mt-1 text-[13px] leading-5 text-settings-muted-foreground">
            {i18n.t('wallet.settings.privateVerus.advancedWarning')}
          </p>
        </div>
      {:else}
        <div
          class="mt-5 flex h-[74px] shrink-0 items-center gap-3 rounded-lg bg-settings-surface p-4"
        >
          <span class="flex size-8 shrink-0 items-center justify-center rounded-full bg-primary/10">
            <ShieldIcon class="size-[18px] text-primary" />
          </span>
          <div class="min-w-0">
            <p class="text-sm leading-5 font-medium">
              {i18n.t('wallet.settings.privateVerus.statusNotConfigured')}
            </p>
            <p class="mt-0.5 text-[13px] leading-5 text-settings-muted-foreground">
              {i18n.t('wallet.settings.privateVerus.statusHelp')}
            </p>
          </div>
        </div>
      {/if}

      <button
        type="button"
        class="mt-3 flex min-h-[66px] w-full items-center gap-4 rounded-lg bg-settings-surface px-4 py-3 text-left outline-none hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring disabled:opacity-50"
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
        class="mt-3 flex min-h-[66px] w-full items-center gap-4 rounded-lg bg-settings-surface px-4 py-3 text-left outline-none hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring disabled:opacity-50"
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
        class="mt-3 flex min-h-[66px] w-full items-center gap-4 rounded-lg bg-settings-surface px-4 py-3 text-left outline-none hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring disabled:opacity-50"
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
    <p class="mt-1 text-xs leading-5 text-settings-muted-foreground">
      {i18n.t('wallet.settings.privateVerus.importHelp')}
    </p>
    <Textarea
      id="private-verus-import"
      variant="surface"
      class="mt-3 h-36 resize-none text-[13px] leading-5"
      placeholder={i18n.t('wallet.settings.privateVerus.importPlaceholder')}
      bind:value={importText}
    ></Textarea>
    <p class="mt-3 text-xs leading-5 text-settings-muted-foreground">
      {i18n.t('wallet.settings.privateVerus.activationRelogin')}
    </p>
    {#if setupError}
      <p class="mt-3 text-[13px] leading-5 text-destructive" role="alert">{setupError}</p>
    {/if}
    <div class="mt-5 flex justify-end gap-2">
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
    <div class="mt-4 min-h-36 rounded-lg bg-settings-surface p-4 dark:bg-settings-control-surface">
      <p class="font-mono text-[13px] leading-6 break-words">{generatedSeedPhrase}</p>
    </div>
    <Button
      class="mt-3 w-fit"
      size="sm"
      variant="secondary"
      onclick={() => void copyGeneratedPhrase()}
    >
      {i18n.t(
        backupCopyFeedback.current === 'copied'
          ? 'common.copied'
          : 'wallet.settings.privateVerus.copyPhrase'
      )}
    </Button>
    {#if backupCopyFeedback.current === 'failed'}
      <p class="mt-2 text-xs text-destructive" role="status">
        {i18n.t('common.copyFailed')}
      </p>
    {/if}
    <Label class="mt-4 flex items-start gap-3 text-[13px] leading-5 font-normal">
      <Checkbox bind:checked={backupAcknowledged} class="mt-0.5" />
      <span>{i18n.t('wallet.settings.privateVerus.savedAcknowledgement')}</span>
    </Label>
    {#if activationMessage}
      <p class="mt-4 text-[13px] leading-5 text-settings-muted-foreground" role="status">
        {activationMessage}
      </p>
    {/if}
    <div class="mt-5 flex justify-end">
      <Button size="sm" disabled={!backupAcknowledged} onclick={closeBackup}>
        {i18n.t('common.done')}
      </Button>
    </div>
  </div>
</StandardRightSheet>
