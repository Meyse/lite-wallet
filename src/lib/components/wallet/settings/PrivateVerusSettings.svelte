<!--
  Component: PrivateVerusSettings
  Purpose: Focused settings detail page for private Verus setup and status.
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import { Button } from '$lib/components/ui/button';
  import { Label } from '$lib/components/ui/label';
  import { Textarea } from '$lib/components/ui/textarea';
  import { i18nStore } from '$lib/i18n';
  import * as walletService from '$lib/services/walletService';
  import type { WalletNetwork } from '$lib/types/wallet';

  type PrivateVerusSettingsProps = {
    walletNetwork: WalletNetwork;
    onBack: () => void;
  };

  const { walletNetwork, onBack }: PrivateVerusSettingsProps = $props();

  const i18n = $derived($i18nStore);
  const privateLabel = $derived(
    walletNetwork === 'testnet'
      ? i18n.t('wallet.private.label.testnet')
      : i18n.t('wallet.private.label.mainnet')
  );

  let loadingStatus = $state(true);
  let configured = $state(false);
  let shieldedAddress = $state('');
  let errorMessage = $state('');
  let successMessage = $state('');
  let generatedSeedPhrase = $state('');
  let importText = $state('');
  let showAdvanced = $state(false);
  let submittingMode = $state<'reuse_primary' | 'create_new' | 'import_text' | null>(null);

  async function loadStatus(): Promise<void> {
    loadingStatus = true;
    errorMessage = '';
    try {
      const status = await walletService.getDlightSeedStatus();
      configured = status.configured;
      shieldedAddress = status.shieldedAddress ?? '';
    } catch {
      errorMessage = i18n.t('wallet.settings.privateVerus.statusLoadError');
      configured = false;
      shieldedAddress = '';
    } finally {
      loadingStatus = false;
    }
  }

  async function setup(mode: 'reuse_primary' | 'create_new' | 'import_text'): Promise<void> {
    if (submittingMode) return;

    successMessage = '';
    errorMessage = '';
    generatedSeedPhrase = '';
    submittingMode = mode;

    try {
      const result = await walletService.setupDlightSeed({
        mode,
        importText: mode === 'import_text' ? importText : undefined
      });
      configured = result.configured;
      if (result.generatedSeedPhrase) {
        generatedSeedPhrase = result.generatedSeedPhrase;
      }
      successMessage = result.requiresRelogin
        ? i18n.t('wallet.settings.privateVerus.setupSuccessRelogin')
        : i18n.t('wallet.settings.privateVerus.setupSuccess');
      await loadStatus();
    } catch {
      errorMessage = i18n.t('wallet.settings.privateVerus.setupError');
    } finally {
      submittingMode = null;
    }
  }

  const showSetupActions = $derived(!configured || showAdvanced);

  onMount(() => {
    void loadStatus();
  });
</script>

<div class="mx-auto flex h-full min-h-0 w-full max-w-5xl flex-col px-6 pb-6 pt-0 sm:px-8">
  <section class="flex min-h-0 flex-1 flex-col overflow-auto pt-3">
    <div class="space-y-4">
      <button
        type="button"
        class="text-muted-foreground hover:text-foreground inline-flex w-fit items-center gap-1.5 text-sm"
        onclick={onBack}
      >
        <ArrowLeftIcon class="size-4" />
        {i18n.t('common.back')}
      </button>

      <section class="space-y-4">
        <div class="space-y-1">
          <h2 class="text-xl font-semibold">{i18n.t('wallet.settings.privateVerus.title')}</h2>
          <p class="text-muted-foreground text-sm">
            {i18n.t('wallet.settings.privateVerus.description', { label: privateLabel })}
          </p>
        </div>

        {#if loadingStatus}
          <p class="text-muted-foreground text-sm">{i18n.t('common.loading')}</p>
        {:else}
          <div class="rounded-lg p-4">
            <p class="text-sm font-medium">
              {configured
                ? i18n.t('wallet.settings.privateVerus.statusConfigured')
                : i18n.t('wallet.settings.privateVerus.statusNotConfigured')}
            </p>
            {#if configured && shieldedAddress}
              <p class="text-muted-foreground mt-1 break-all text-xs">
                {i18n.t('wallet.settings.privateVerus.statusAddress', {
                  address: shieldedAddress
                })}
              </p>
            {/if}
          </div>
        {/if}

        {#if configured}
          <div class="space-y-2">
            <Button
              variant="secondary"
              size="sm"
              class="w-fit"
              onclick={() => {
                showAdvanced = !showAdvanced;
              }}
            >
              {showAdvanced
                ? i18n.t('wallet.settings.privateVerus.advancedToggleHide')
                : i18n.t('wallet.settings.privateVerus.advancedToggleShow')}
            </Button>
            <p class="text-muted-foreground text-xs">{i18n.t('wallet.settings.privateVerus.advancedWarning')}</p>
          </div>
        {/if}

        {#if errorMessage}
          <div class="rounded-md bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {errorMessage}
          </div>
        {/if}

        {#if successMessage}
          <div class="rounded-md bg-emerald-50 px-3 py-2 text-sm text-emerald-800 dark:bg-emerald-500/12 dark:text-emerald-200">
            {successMessage}
          </div>
        {/if}

        {#if generatedSeedPhrase}
          <div class="rounded-lg p-4">
            <p class="text-sm font-medium">{i18n.t('wallet.settings.privateVerus.generatedSeedTitle')}</p>
            <p class="text-muted-foreground mt-2 break-all text-sm leading-relaxed">{generatedSeedPhrase}</p>
          </div>
        {/if}

        {#if showSetupActions}
          <div class="space-y-3">
            <Button
              variant="secondary"
              class="justify-start"
              disabled={loadingStatus || submittingMode !== null}
              onclick={() => {
                void setup('reuse_primary');
              }}
            >
              {submittingMode === 'reuse_primary'
                ? i18n.t('wallet.settings.privateVerus.settingUp')
                : i18n.t('wallet.settings.privateVerus.reusePrimary')}
            </Button>

            <Button
              variant="secondary"
              class="justify-start"
              disabled={loadingStatus || submittingMode !== null}
              onclick={() => {
                void setup('create_new');
              }}
            >
              {submittingMode === 'create_new'
                ? i18n.t('wallet.settings.privateVerus.settingUp')
                : i18n.t('wallet.settings.privateVerus.createNew')}
            </Button>

            <div class="space-y-2 rounded-lg p-3">
              <Label for="private-seed-import">
                {i18n.t('wallet.settings.privateVerus.importLabel')}
              </Label>
              <Textarea
                id="private-seed-import"
                variant="surface"
                placeholder={i18n.t('wallet.settings.privateVerus.importPlaceholder')}
                bind:value={importText}
              ></Textarea>
              <Button
                variant="secondary"
                class="justify-start"
                disabled={loadingStatus || submittingMode !== null || !importText.trim()}
                onclick={() => {
                  void setup('import_text');
                }}
              >
                {submittingMode === 'import_text'
                  ? i18n.t('wallet.settings.privateVerus.settingUp')
                  : i18n.t('wallet.settings.privateVerus.importAction')}
              </Button>
            </div>
          </div>
        {/if}
      </section>
    </div>
  </section>
</div>
