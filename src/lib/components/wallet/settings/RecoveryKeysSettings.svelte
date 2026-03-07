<!--
  Component: RecoveryKeysSettings
  Purpose: Password-gated recovery page for seed and derived key material.
-->

<script lang="ts">
  import { onDestroy } from 'svelte';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import EyeIcon from '@lucide/svelte/icons/eye';
  import EyeOffIcon from '@lucide/svelte/icons/eye-off';
  import PasswordConfirmOverlay from '$lib/components/common/PasswordConfirmOverlay.svelte';
  import { Button } from '$lib/components/ui/button';
  import { CopyActionButton } from '$lib/components/ui/copy-action-button';
  import { i18nStore } from '$lib/i18n';
  import { TimedRecordState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import * as walletService from '$lib/services/walletService';
  import type {
    DlightRecoverySecretKind,
    RecoverySecretKind,
    WalletRecoverySecretsResult
  } from '$lib/types/wallet';

  type RecoveryKeysSettingsProps = {
    onBack: () => void;
  };

  type SecretEntry = {
    id: string;
    label: string;
    value: string;
    isSecret: boolean;
  };

  const { onBack }: RecoveryKeysSettingsProps = $props();
  const i18n = $derived($i18nStore);

  let secrets = $state<WalletRecoverySecretsResult | null>(null);
  let isLoading = $state(false);
  let passwordDialogOpen = $state(false);
  let passwordInput = $state('');
  let passwordError = $state('');
  let visibleSecretById = $state<Record<string, boolean>>({});
  const copyFeedbackState = new TimedRecordState<'copied' | 'failed'>();
  const copyStatusById = $derived(copyFeedbackState.values);

  const primaryEntries = $derived<SecretEntry[]>(
    secrets
      ? [
          {
            id: 'primarySecret',
            label: i18n.t('wallet.settings.recovery.field.primarySecret'),
            value: secrets.primarySecret,
            isSecret: true
          }
        ]
      : []
  );

  const derivedKeyEntries = $derived<SecretEntry[]>(
    secrets
      ? [
          {
            id: 'verusWif',
            label: i18n.t('wallet.settings.recovery.field.verusWif'),
            value: secrets.verusWif,
            isSecret: true
          },
          {
            id: 'btcWif',
            label: i18n.t('wallet.settings.recovery.field.btcWif'),
            value: secrets.btcWif,
            isSecret: true
          },
          {
            id: 'ethPrivateKey',
            label: i18n.t('wallet.settings.recovery.field.ethPrivateKey'),
            value: secrets.ethPrivateKey,
            isSecret: true
          }
        ]
      : []
  );

  const addressEntries = $derived<SecretEntry[]>(
    secrets
      ? [
          {
            id: 'verusAddress',
            label: i18n.t('wallet.settings.recovery.field.verusAddress'),
            value: secrets.verusAddress,
            isSecret: false
          },
          {
            id: 'btcAddress',
            label: i18n.t('wallet.settings.recovery.field.btcAddress'),
            value: secrets.btcAddress,
            isSecret: false
          },
          {
            id: 'ethAddress',
            label: i18n.t('wallet.settings.recovery.field.ethAddress'),
            value: secrets.ethAddress,
            isSecret: false
          }
        ]
      : []
  );

  const dlightEntries = $derived<SecretEntry[]>(
    secrets && secrets.dlightSecret
      ? [
          {
            id: 'dlightSecret',
            label: i18n.t('wallet.settings.recovery.field.dlightSecret'),
            value: secrets.dlightSecret,
            isSecret: true
          },
          {
            id: 'dlightShieldedAddress',
            label: i18n.t('wallet.settings.recovery.field.dlightShieldedAddress'),
            value: secrets.dlightShieldedAddress ?? '',
            isSecret: false
          },
          {
            id: 'dlightDerivedSpendingKey',
            label: i18n.t('wallet.settings.recovery.field.dlightDerivedSpendingKey'),
            value: secrets.dlightDerivedSpendingKey ?? '',
            isSecret: true
          }
        ]
      : []
  );

  const shouldShowDlightSection = $derived(Boolean(secrets?.dlightSecret));

  function clearSecrets(): void {
    secrets = null;
    passwordInput = '';
    passwordError = '';
    visibleSecretById = {};
    copyFeedbackState.clearAll();
  }

  function handleBack(): void {
    clearSecrets();
    onBack();
  }

  function openRevealFlow(): void {
    passwordDialogOpen = true;
    passwordInput = '';
    passwordError = '';
  }

  function closePasswordDialog(): void {
    passwordDialogOpen = false;
    passwordInput = '';
    passwordError = '';
  }

  function recoverySecretKindLabel(kind: RecoverySecretKind): string {
    if (kind === 'wif') return i18n.t('wallet.settings.recovery.kind.wif');
    if (kind === 'private_key_hex') return i18n.t('wallet.settings.recovery.kind.privateKeyHex');
    return i18n.t('wallet.settings.recovery.kind.seedText');
  }

  function dlightSecretKindLabel(kind: DlightRecoverySecretKind | null | undefined): string {
    if (kind === 'spending_key') return i18n.t('wallet.settings.recovery.kind.dlightSpendingKey');
    if (kind === 'mnemonic') return i18n.t('wallet.settings.recovery.kind.dlightMnemonic');
    return i18n.t('wallet.settings.recovery.kind.unknown');
  }

  function toggleSecretVisibility(id: string): void {
    visibleSecretById = {
      ...visibleSecretById,
      [id]: !visibleSecretById[id]
    };
  }

  function renderedEntryValue(entry: SecretEntry): string {
    const trimmed = entry.value.trim();
    if (!trimmed) return i18n.t('wallet.settings.recovery.valueUnavailable');
    if (!entry.isSecret || visibleSecretById[entry.id]) return trimmed;
    return '••••••••••••••••••••';
  }

  async function copyValue(entry: SecretEntry): Promise<void> {
    const value = entry.value.trim();
    if (!value) {
      copyFeedbackState.set(entry.id, 'failed', 2000);
      return;
    }

    const feedbackState = (await writeClipboardText(value)) ? 'copied' : 'failed';
    copyFeedbackState.set(entry.id, feedbackState, 2000);
  }

  async function revealSecrets(): Promise<void> {
    if (isLoading || !passwordInput.trim()) return;

    isLoading = true;
    passwordError = '';

    try {
      const result = await walletService.getWalletRecoverySecrets(passwordInput.trim());
      secrets = result;
      visibleSecretById = {};
      copyFeedbackState.clearAll();
      closePasswordDialog();
    } catch {
      passwordError = i18n.t('wallet.settings.recovery.passwordInvalid');
    } finally {
      isLoading = false;
    }
  }

  onDestroy(() => {
    clearSecrets();
  });
</script>

<div class="mx-auto flex h-full min-h-0 w-full max-w-5xl flex-col px-6 pb-6 pt-0 sm:px-8">
  <section class="flex min-h-0 flex-1 flex-col overflow-auto pt-3">
    <div class="space-y-4">
      <button
        type="button"
        class="text-muted-foreground hover:text-foreground inline-flex w-fit items-center gap-1.5 text-sm"
        onclick={handleBack}
      >
        <ArrowLeftIcon class="size-4" />
        {i18n.t('common.back')}
      </button>

      <section class="space-y-4">
        <div>
          <h2 class="text-xl font-semibold">{i18n.t('wallet.settings.recovery.title')}</h2>
        </div>

        {#if !secrets}
          <div>
            <Button onclick={openRevealFlow} class="w-fit">
              {i18n.t('wallet.settings.recovery.revealCta')}
            </Button>
          </div>
        {:else}
          <div class="space-y-4">
            <div class="rounded-lg bg-amber-50 px-3 py-2 text-sm text-amber-900 dark:bg-amber-500/12 dark:text-amber-100">
              {i18n.t('wallet.settings.recovery.warningInline')}
            </div>

            <div class="rounded-lg p-1">
              <p class="text-sm font-medium">{i18n.t('wallet.settings.recovery.primaryKindLabel')}</p>
              <p class="text-muted-foreground mt-1 text-sm">
                {recoverySecretKindLabel(secrets.primarySecretKind)}
              </p>
            </div>

            <div class="rounded-lg p-1">
              <p class="text-sm font-medium">{i18n.t('wallet.settings.recovery.primarySection')}</p>
              <div class="mt-3 space-y-3">
                {#each primaryEntries as entry (entry.id)}
                  <div class="space-y-1 rounded-md p-3">
                    <p class="text-xs font-medium">{entry.label}</p>
                    <p class="rounded px-2 py-1.5 font-mono text-xs break-all">{renderedEntryValue(entry)}</p>
                    <div class="flex flex-wrap items-center gap-2">
                      <Button
                        size="sm"
                        variant="secondary"
                        onclick={() => toggleSecretVisibility(entry.id)}
                      >
                        {#if visibleSecretById[entry.id]}
                          <EyeOffIcon class="mr-1 size-3.5" />
                          {i18n.t('wallet.settings.recovery.hide')}
                        {:else}
                          <EyeIcon class="mr-1 size-3.5" />
                          {i18n.t('wallet.settings.recovery.reveal')}
                        {/if}
                      </Button>
                      <CopyActionButton
                        state={copyStatusById[entry.id] ?? 'idle'}
                        variant="secondary"
                        size="sm"
                        onclick={() => copyValue(entry)}
                        label={i18n.t('wallet.settings.recovery.copy')}
                        copiedLabel={i18n.t('wallet.settings.recovery.copySuccess')}
                        failedLabel={i18n.t('wallet.settings.recovery.copyFailed')}
                      />
                    </div>
                  </div>
                {/each}
              </div>
            </div>

            <div class="rounded-lg p-1">
              <p class="text-sm font-medium">{i18n.t('wallet.settings.recovery.derivedKeysSection')}</p>
              <div class="mt-3 space-y-3">
                {#each derivedKeyEntries as entry (entry.id)}
                  <div class="space-y-1 rounded-md p-3">
                    <p class="text-xs font-medium">{entry.label}</p>
                    <p class="rounded px-2 py-1.5 font-mono text-xs break-all">{renderedEntryValue(entry)}</p>
                    <div class="flex flex-wrap items-center gap-2">
                      <Button
                        size="sm"
                        variant="secondary"
                        onclick={() => toggleSecretVisibility(entry.id)}
                      >
                        {#if visibleSecretById[entry.id]}
                          <EyeOffIcon class="mr-1 size-3.5" />
                          {i18n.t('wallet.settings.recovery.hide')}
                        {:else}
                          <EyeIcon class="mr-1 size-3.5" />
                          {i18n.t('wallet.settings.recovery.reveal')}
                        {/if}
                      </Button>
                      <CopyActionButton
                        state={copyStatusById[entry.id] ?? 'idle'}
                        variant="secondary"
                        size="sm"
                        onclick={() => copyValue(entry)}
                        label={i18n.t('wallet.settings.recovery.copy')}
                        copiedLabel={i18n.t('wallet.settings.recovery.copySuccess')}
                        failedLabel={i18n.t('wallet.settings.recovery.copyFailed')}
                      />
                    </div>
                  </div>
                {/each}
              </div>
            </div>

            <div class="rounded-lg p-1">
              <p class="text-sm font-medium">{i18n.t('wallet.settings.recovery.addressesSection')}</p>
              <div class="mt-3 space-y-3">
                {#each addressEntries as entry (entry.id)}
                  <div class="space-y-1 rounded-md p-3">
                    <p class="text-xs font-medium">{entry.label}</p>
                    <p class="rounded px-2 py-1.5 font-mono text-xs break-all">{renderedEntryValue(entry)}</p>
                    <div class="flex items-center gap-2">
                      <CopyActionButton
                        state={copyStatusById[entry.id] ?? 'idle'}
                        variant="secondary"
                        size="sm"
                        onclick={() => copyValue(entry)}
                        label={i18n.t('wallet.settings.recovery.copy')}
                        copiedLabel={i18n.t('wallet.settings.recovery.copySuccess')}
                        failedLabel={i18n.t('wallet.settings.recovery.copyFailed')}
                      />
                    </div>
                  </div>
                {/each}
              </div>
            </div>

            {#if shouldShowDlightSection}
              <div class="rounded-lg p-1">
                <p class="text-sm font-medium">{i18n.t('wallet.settings.recovery.dlightSection')}</p>
                <p class="text-muted-foreground mt-1 text-xs">
                  {i18n.t('wallet.settings.recovery.dlightKindLabel')}:
                  {dlightSecretKindLabel(secrets.dlightSecretKind)}
                </p>
                <div class="mt-3 space-y-3">
                  {#each dlightEntries as entry (entry.id)}
                    <div class="space-y-1 rounded-md p-3">
                      <p class="text-xs font-medium">{entry.label}</p>
                      <p class="rounded px-2 py-1.5 font-mono text-xs break-all">{renderedEntryValue(entry)}</p>
                      <div class="flex flex-wrap items-center gap-2">
                        {#if entry.isSecret && entry.value.trim()}
                          <Button
                            size="sm"
                            variant="secondary"
                            onclick={() => toggleSecretVisibility(entry.id)}
                          >
                            {#if visibleSecretById[entry.id]}
                              <EyeOffIcon class="mr-1 size-3.5" />
                              {i18n.t('wallet.settings.recovery.hide')}
                            {:else}
                              <EyeIcon class="mr-1 size-3.5" />
                              {i18n.t('wallet.settings.recovery.reveal')}
                            {/if}
                          </Button>
                        {/if}
                        {#if entry.value.trim()}
                          <CopyActionButton
                            state={copyStatusById[entry.id] ?? 'idle'}
                            variant="secondary"
                            size="sm"
                            onclick={() => copyValue(entry)}
                            label={i18n.t('wallet.settings.recovery.copy')}
                            copiedLabel={i18n.t('wallet.settings.recovery.copySuccess')}
                            failedLabel={i18n.t('wallet.settings.recovery.copyFailed')}
                          />
                        {/if}
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {/if}
      </section>
    </div>
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
  onCancel={closePasswordDialog}
  onConfirm={() => {
    void revealSecrets();
  }}
/>
