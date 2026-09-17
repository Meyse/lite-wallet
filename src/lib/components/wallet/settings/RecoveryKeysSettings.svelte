<!--
  Component: RecoveryKeysSettings
  Purpose: Password-gated recovery overview with short-lived secret detail sheets.
-->

<script lang="ts">
  import { onDestroy } from 'svelte';
  import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import EyeIcon from '@lucide/svelte/icons/eye';
  import EyeOffIcon from '@lucide/svelte/icons/eye-off';
  import QrCodeIcon from '@lucide/svelte/icons/qr-code';
  import PasswordConfirmOverlay from '$lib/components/common/PasswordConfirmOverlay.svelte';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { Button } from '$lib/components/ui/button';
  import { CopyButton } from '$lib/components/ui/copy-button';
  import { i18nStore } from '$lib/i18n';
  import * as walletService from '$lib/services/walletService';
  import type {
    DlightRecoverySecretKind,
    RecoverySecretKind,
    WalletNetwork,
    WalletRecoverySecretsResult,
  } from '$lib/types/wallet';
  import { TimedRecordState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import { encodeQrCode, type RenderedQrCode } from '$lib/utils/qrCode';
  import { extractWalletErrorType } from '$lib/utils/walletErrors';

  type RecoveryKeysSettingsProps = {
    walletNetwork: WalletNetwork;
    walletName: string;
    backLabel: string;
    onBack: () => void;
  };

  type DetailKind = 'primary' | 'dlight' | 'keys' | 'addresses';

  type RecoveryEntry = {
    id: string;
    label: string;
    value: string;
    isSecret: boolean;
    supportsQr?: boolean;
  };

  const { walletNetwork, walletName, backLabel, onBack }: RecoveryKeysSettingsProps = $props();
  const i18n = $derived($i18nStore);

  let secrets = $state<WalletRecoverySecretsResult | null>(null);
  let passwordDialogOpen = $state(true);
  let passwordInput = $state('');
  let passwordError = $state('');
  let isLoading = $state(false);
  let activeDetail = $state<DetailKind | null>(null);
  let visibleSecretById = $state<Record<string, boolean>>({});
  let qrEntry = $state<RecoveryEntry | null>(null);
  let qrCode = $state<RenderedQrCode | null>(null);
  let qrState = $state<'idle' | 'loading' | 'ready' | 'error'>('idle');
  let requestGeneration = 0;
  let qrGeneration = 0;
  let disposed = false;

  const copyFeedback = new TimedRecordState<'copied' | 'failed'>();
  const copyStatusById = $derived(copyFeedback.values);
  const networkLabel = $derived(
    i18n.t(walletNetwork === 'testnet' ? 'common.network.testnet' : 'common.network.mainnet')
  );

  const primaryEntries = $derived<RecoveryEntry[]>(
    secrets
      ? [
          {
            id: 'primarySecret',
            label: i18n.t('wallet.settings.recovery.field.primarySecret'),
            value: secrets.primarySecret,
            isSecret: true,
            supportsQr: true,
          },
        ]
      : []
  );

  const privateSpendingKey = $derived(
    secrets?.dlightDerivedSpendingKey?.trim() ||
      (secrets?.dlightSecretKind === 'spending_key' ? secrets.dlightSecret?.trim() : '') ||
      ''
  );

  const dlightEntries = $derived<RecoveryEntry[]>(
    secrets?.dlightSecret
      ? [
          {
            id: 'dlightSecret',
            label: i18n.t('wallet.settings.recovery.field.dlightSecret'),
            value: secrets.dlightSecret,
            isSecret: true,
            supportsQr: true,
          },
          ...(secrets.dlightShieldedAddress
            ? [
                {
                  id: 'dlightShieldedAddress',
                  label: i18n.t('wallet.settings.recovery.field.dlightShieldedAddress'),
                  value: secrets.dlightShieldedAddress,
                  isSecret: false,
                },
              ]
            : []),
        ]
      : []
  );

  const keyEntries = $derived<RecoveryEntry[]>(
    secrets
      ? [
          {
            id: 'verusWif',
            label: i18n.t('wallet.settings.recovery.field.verusWif'),
            value: secrets.verusWif,
            isSecret: true,
            supportsQr: true,
          },
          {
            id: 'btcWif',
            label: i18n.t('wallet.settings.recovery.field.btcWif'),
            value: secrets.btcWif,
            isSecret: true,
            supportsQr: true,
          },
          {
            id: 'ethPrivateKey',
            label: i18n.t('wallet.settings.recovery.field.ethPrivateKey'),
            value: secrets.ethPrivateKey,
            isSecret: true,
            supportsQr: true,
          },
          ...(privateSpendingKey
            ? [
                {
                  id: 'dlightDerivedSpendingKey',
                  label: i18n.t('wallet.settings.recovery.field.dlightDerivedSpendingKey'),
                  value: privateSpendingKey,
                  isSecret: true,
                  supportsQr: true,
                },
              ]
            : []),
        ]
      : []
  );

  const addressEntries = $derived<RecoveryEntry[]>(
    secrets
      ? [
          {
            id: 'verusAddress',
            label: i18n.t('wallet.settings.recovery.field.verusAddress'),
            value: secrets.verusAddress,
            isSecret: false,
          },
          {
            id: 'btcAddress',
            label: i18n.t('wallet.settings.recovery.field.btcAddress'),
            value: secrets.btcAddress,
            isSecret: false,
          },
          {
            id: 'ethAddress',
            label: i18n.t('wallet.settings.recovery.field.ethAddress'),
            value: secrets.ethAddress,
            isSecret: false,
          },
          ...(secrets.dlightShieldedAddress
            ? [
                {
                  id: 'dlightAddress',
                  label: i18n.t('wallet.settings.recovery.field.dlightShieldedAddress'),
                  value: secrets.dlightShieldedAddress,
                  isSecret: false,
                },
              ]
            : []),
        ]
      : []
  );

  const activeEntries = $derived(
    activeDetail === 'primary'
      ? primaryEntries
      : activeDetail === 'dlight'
        ? dlightEntries
        : activeDetail === 'keys'
          ? keyEntries
          : activeDetail === 'addresses'
            ? addressEntries
            : []
  );

  const detailTitle = $derived(
    i18n.t(
      activeDetail === 'primary'
        ? 'wallet.settings.recovery.primarySection'
        : activeDetail === 'dlight'
          ? 'wallet.settings.recovery.dlightSection'
          : activeDetail === 'keys'
            ? 'wallet.settings.recovery.derivedKeysSection'
            : 'wallet.settings.recovery.addressesSection'
    )
  );

  function recoverySecretKindLabel(kind: RecoverySecretKind): string {
    if (kind === 'wif') return i18n.t('wallet.settings.recovery.kind.wif');
    if (kind === 'private_key_hex') {
      return i18n.t('wallet.settings.recovery.kind.privateKeyHex');
    }
    return i18n.t('wallet.settings.recovery.kind.seedText');
  }

  function dlightSecretKindLabel(kind: DlightRecoverySecretKind | null | undefined): string {
    if (kind === 'spending_key') {
      return i18n.t('wallet.settings.recovery.kind.dlightSpendingKey');
    }
    if (kind === 'mnemonic') return i18n.t('wallet.settings.recovery.kind.dlightMnemonic');
    return i18n.t('wallet.settings.recovery.kind.unknown');
  }

  function clearQr(): void {
    qrGeneration += 1;
    qrEntry = null;
    qrCode = null;
    qrState = 'idle';
  }

  function clearDetailState(): void {
    visibleSecretById = {};
    copyFeedback.clearAll();
    clearQr();
  }

  function clearSecrets(): void {
    requestGeneration += 1;
    secrets = null;
    passwordInput = '';
    passwordError = '';
    isLoading = false;
    activeDetail = null;
    clearDetailState();
  }

  function leaveRecovery(): void {
    clearSecrets();
    passwordDialogOpen = false;
    onBack();
  }

  function openDetail(detail: DetailKind): void {
    clearDetailState();
    activeDetail = detail;
  }

  function handleDetailOpenChange(open: boolean): void {
    if (open) return;
    activeDetail = null;
    clearDetailState();
  }

  function toggleSecretVisibility(id: string): void {
    visibleSecretById = { ...visibleSecretById, [id]: !visibleSecretById[id] };
  }

  function renderedValue(entry: RecoveryEntry): string {
    const value = entry.value.trim();
    if (!value) return i18n.t('wallet.settings.recovery.valueUnavailable');
    if (!entry.isSecret || visibleSecretById[entry.id]) return value;
    return '••••••••••••••••••••';
  }

  async function copyValue(entry: RecoveryEntry): Promise<void> {
    const copied = await writeClipboardText(entry.value);
    copyFeedback.set(entry.id, copied ? 'copied' : 'failed');
  }

  async function generateQr(entry: RecoveryEntry): Promise<void> {
    const payload = entry.value.trim();
    if (!payload) return;

    const generation = ++qrGeneration;
    qrEntry = entry;
    qrCode = null;
    qrState = 'loading';

    try {
      const encoded = await encodeQrCode(payload);
      if (disposed || generation !== qrGeneration) return;
      qrCode = encoded;
      qrState = 'ready';
    } catch {
      if (disposed || generation !== qrGeneration) return;
      qrCode = null;
      qrState = 'error';
    }
  }

  function closeQr(): void {
    clearQr();
  }

  async function revealSecrets(): Promise<void> {
    if (isLoading || !passwordInput.trim()) return;

    const password = passwordInput.trim();
    const generation = ++requestGeneration;
    isLoading = true;
    passwordError = '';

    try {
      const result = await walletService.getWalletRecoverySecrets(password);
      if (disposed || generation !== requestGeneration) return;
      secrets = result;
      passwordInput = '';
      passwordDialogOpen = false;
      clearDetailState();
    } catch (error) {
      if (disposed || generation !== requestGeneration) return;
      const errorType = extractWalletErrorType(error);
      passwordError = i18n.t(
        errorType === 'SecureStorageUnavailable'
          ? 'common.error.secureStorageUnavailable'
          : errorType === 'InvalidPassword'
            ? 'wallet.settings.recovery.passwordInvalid'
            : 'wallet.settings.recovery.passwordError'
      );
    } finally {
      if (!disposed && generation === requestGeneration) isLoading = false;
    }
  }

  onDestroy(() => {
    disposed = true;
    clearSecrets();
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
        onclick={leaveRecovery}
      >
        <ChevronLeftIcon class="size-4" />
        {backLabel}
      </button>
      <h2 class="text-xl leading-7 font-semibold tracking-[-0.015em]">
        {i18n.t('wallet.settings.recovery.title')}
      </h2>
    </header>

    {#if secrets}
      <div class="mt-5 rounded-lg bg-amber-50 p-4 dark:bg-amber-500/10">
        <p class="text-sm leading-5 font-medium text-amber-950 dark:text-amber-100">
          {i18n.t('wallet.settings.recovery.warningTitle')}
        </p>
        <p class="mt-1 text-[13px] leading-5 text-amber-900/80 dark:text-amber-100/75">
          {i18n.t('wallet.settings.recovery.warningInline')}
        </p>
      </div>

      <div class="mt-3 overflow-hidden rounded-lg bg-settings-surface">
        <button
          type="button"
          class="flex min-h-16 w-full items-center gap-4 px-4 py-3 text-left outline-none hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:ring-inset"
          onclick={() => openDetail('primary')}
        >
          <span class="min-w-0 flex-1">
            <span class="block text-sm leading-5 font-medium">
              {i18n.t('wallet.settings.recovery.primarySection')}
            </span>
            <span class="mt-0.5 block text-xs leading-5 text-settings-muted-foreground">
              {recoverySecretKindLabel(secrets.primarySecretKind)}
            </span>
          </span>
          <ChevronRightIcon class="size-4 shrink-0 text-settings-muted-foreground" />
        </button>

        {#if secrets.dlightSecret}
          <button
            type="button"
            class="flex min-h-16 w-full items-center gap-4 px-4 py-3 text-left outline-none hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:ring-inset"
            onclick={() => openDetail('dlight')}
          >
            <span class="min-w-0 flex-1">
              <span class="block text-sm leading-5 font-medium">
                {i18n.t('wallet.settings.recovery.dlightSection')}
              </span>
              <span class="mt-0.5 block text-xs leading-5 text-settings-muted-foreground">
                {dlightSecretKindLabel(secrets.dlightSecretKind)}
              </span>
            </span>
            <ChevronRightIcon class="size-4 shrink-0 text-settings-muted-foreground" />
          </button>
        {/if}

        <button
          type="button"
          class="flex min-h-16 w-full items-center gap-4 px-4 py-3 text-left outline-none hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:ring-inset"
          onclick={() => openDetail('keys')}
        >
          <span class="min-w-0 flex-1">
            <span class="block text-sm leading-5 font-medium">
              {i18n.t('wallet.settings.recovery.derivedKeysSection')}
            </span>
            <span class="mt-0.5 block text-xs leading-5 text-settings-muted-foreground">
              {i18n.t('wallet.settings.recovery.keysDescription')}
            </span>
          </span>
          <ChevronRightIcon class="size-4 shrink-0 text-settings-muted-foreground" />
        </button>

        <button
          type="button"
          class="flex min-h-16 w-full items-center gap-4 px-4 py-3 text-left outline-none hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:ring-inset"
          onclick={() => openDetail('addresses')}
        >
          <span class="min-w-0 flex-1">
            <span class="block text-sm leading-5 font-medium">
              {i18n.t('wallet.settings.recovery.addressesSection')}
            </span>
            <span class="mt-0.5 block text-xs leading-5 text-settings-muted-foreground">
              {i18n.t('wallet.settings.recovery.addressesDescription')}
            </span>
          </span>
          <ChevronRightIcon class="size-4 shrink-0 text-settings-muted-foreground" />
        </button>
      </div>

      <Button class="mt-4 w-fit" size="sm" variant="secondary" onclick={leaveRecovery}>
        {i18n.t('wallet.settings.recovery.closeRecovery')}
      </Button>
    {/if}
  </section>
</div>

<PasswordConfirmOverlay
  bind:isOpen={passwordDialogOpen}
  bind:password={passwordInput}
  loading={isLoading}
  errorMessage={passwordError}
  placeholder={i18n.t('wallet.settings.recovery.passwordPlaceholder')}
  confirmLabel={i18n.t('wallet.settings.recovery.revealConfirm')}
  loadingLabel={i18n.t('wallet.settings.recovery.revealLoading')}
  onConfirm={() => void revealSecrets()}
  onCancel={leaveRecovery}
/>

<StandardRightSheet
  isOpen={activeDetail !== null}
  title={detailTitle}
  closeLabel={i18n.t('common.close')}
  onOpenChange={handleDetailOpenChange}
>
  {#if qrEntry}
    <div class="flex h-full min-h-0 flex-col items-center">
      <button
        type="button"
        class="inline-flex w-fit items-center gap-1 self-start text-[13px] leading-5 text-settings-muted-foreground outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
        onclick={closeQr}
      >
        <ChevronLeftIcon class="size-4" />
        {i18n.t('wallet.settings.recovery.qr.back')}
      </button>
      <p class="mt-4 self-start text-sm leading-5 font-medium">{qrEntry.label}</p>
      <p class="mt-1 self-start text-xs leading-5 text-settings-muted-foreground">
        {walletName} · {networkLabel}
      </p>

      <div class="mt-5 flex size-[260px] shrink-0 items-center justify-center bg-white">
        {#if qrState === 'loading'}
          <p class="text-[13px] text-neutral-600">
            {i18n.t('wallet.settings.recovery.qr.loading')}
          </p>
        {:else if qrState === 'error'}
          <div class="px-6 text-center">
            <p class="text-[13px] leading-5 text-neutral-700">
              {i18n.t('wallet.settings.recovery.qr.error')}
            </p>
            <Button
              class="mt-3"
              size="sm"
              variant="secondary"
              onclick={() => void generateQr(qrEntry!)}
            >
              {i18n.t('common.retry')}
            </Button>
          </div>
        {:else if qrCode}
          <svg
            class="size-full"
            viewBox={`0 0 ${qrCode.viewBoxSize} ${qrCode.viewBoxSize}`}
            role="img"
            aria-label={i18n.t('wallet.settings.recovery.qr.ariaLabel', { label: qrEntry.label })}
            shape-rendering="crispEdges"
          >
            <rect width={qrCode.viewBoxSize} height={qrCode.viewBoxSize} class="fill-white" />
            <path d={qrCode.path} class="fill-black" />
          </svg>
        {/if}
      </div>

      <div class="mt-5 w-full rounded-lg bg-amber-50 p-4 dark:bg-amber-500/10">
        <p class="text-[13px] leading-5 text-amber-950 dark:text-amber-100">
          {i18n.t('wallet.settings.recovery.qr.warning')}
        </p>
      </div>
    </div>
  {:else}
    <div class="flex min-h-0 flex-1 flex-col">
      <p class="shrink-0 text-[13px] leading-5 text-settings-muted-foreground">
        {activeDetail === 'addresses'
          ? i18n.t('wallet.settings.recovery.addressesSheetDescription')
          : i18n.t('wallet.settings.recovery.keepOffline')}
      </p>

      <div class="mt-4 min-h-0 flex-1 overflow-auto">
        <div class="space-y-3 pb-1">
          {#each activeEntries as entry (entry.id)}
            <div class="rounded-lg bg-settings-surface p-4">
              <p class="text-[13px] leading-5 font-medium">{entry.label}</p>
              <p class="mt-2 font-mono text-xs leading-5 break-all text-settings-muted-foreground">
                {renderedValue(entry)}
              </p>

              {#if entry.value.trim()}
                <div class="mt-3 flex items-center gap-1">
                  {#if entry.isSecret}
                    <button
                      type="button"
                      class="flex size-8 items-center justify-center rounded-md text-settings-muted-foreground outline-none hover:bg-settings-control-surface hover:text-foreground focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
                      aria-label={i18n.t(
                        visibleSecretById[entry.id]
                          ? 'wallet.settings.recovery.hideValue'
                          : 'wallet.settings.recovery.revealValue',
                        { label: entry.label }
                      )}
                      onclick={() => toggleSecretVisibility(entry.id)}
                    >
                      {#if visibleSecretById[entry.id]}
                        <EyeOffIcon class="size-4" />
                      {:else}
                        <EyeIcon class="size-4" />
                      {/if}
                    </button>
                  {/if}

                  <CopyButton
                    size="default"
                    copied={copyStatusById[entry.id] === 'copied'}
                    aria-label={i18n.t('wallet.settings.recovery.copyValue', {
                      label: entry.label,
                    })}
                    onclick={() => void copyValue(entry)}
                  />

                  {#if entry.supportsQr}
                    <button
                      type="button"
                      class="flex size-8 items-center justify-center rounded-md text-settings-muted-foreground outline-none hover:bg-settings-control-surface hover:text-foreground focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
                      aria-label={i18n.t('wallet.settings.recovery.qr.show', {
                        label: entry.label,
                      })}
                      onclick={() => void generateQr(entry)}
                    >
                      <QrCodeIcon class="size-4" />
                    </button>
                  {/if}
                </div>

                {#if copyStatusById[entry.id] === 'failed'}
                  <p class="mt-1 text-xs text-destructive" role="status">
                    {i18n.t('wallet.settings.recovery.copyFailed')}
                  </p>
                {/if}
              {/if}
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</StandardRightSheet>
