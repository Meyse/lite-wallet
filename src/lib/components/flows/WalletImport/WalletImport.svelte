<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { goto } from '$app/navigation';
  import { Button } from '$lib/components/ui/button';
  import StepperLayout from '$lib/components/shared/StepperLayout.svelte';
  import NameStep from '$lib/components/flows/WalletCreation/NameStep.svelte';
  import PasswordStep from '$lib/components/flows/WalletCreation/PasswordStep.svelte';
  import { i18nStore } from '$lib/i18n';
  import ImportCompleteStep from './ImportCompleteStep.svelte';
  import SeedPhraseStep from './SeedPhraseStep.svelte';
  import TextImportStep from './TextImportStep.svelte';
  import type { ImportMethod, WalletData, WalletUpdate } from './types';

  type CreateWalletResult = { wallet_id: string; success: boolean };

  type WalletImportProps = {
    initialMethod?: ImportMethod;
    onGoHome?: () => void;
  };

  const TOTAL_STEPS = 4;

   
  let { initialMethod = 'seed24', onGoHome = () => {} }: WalletImportProps = $props();
   
  const selectedMethod = $derived(initialMethod === 'text' ? 'text' : 'seed24');

  let currentStep = $state(1);
  let walletData = $state<WalletData>({
    name: '',
    emoji: '💰',
    color: 'blue',
    password: '',
    network: 'mainnet'
  });
  let seedPhraseInput = $state('');
  let seedPhraseNormalized = $state('');
  let seedPhraseValid = $state(false);
  let textImportInput = $state('');
  let textImportValid = $state(false);
  let canImportWallet = $state(false);
  let isSubmitting = $state(false);
  let submitError = $state('');
  let createdWalletId = $state('');
  let openWalletLoading = $state(false);
  let openWalletError = $state('');
  let openWalletAttempted = $state(false);

  const i18n = $derived($i18nStore);

  function extractWalletErrorType(error: unknown): string | null {
    if (typeof error === 'string') {
      if (error.includes('invalid args')) return 'InvalidArgs';
      return null;
    }
    if (!error || typeof error !== 'object') return null;
    const obj = error as Record<string, unknown>;
    if (typeof obj.type === 'string') return obj.type;
    if (typeof obj.message === 'string' && obj.message.includes('invalid args')) {
      return 'InvalidArgs';
    }
    if (obj.data && typeof obj.data === 'object') {
      const data = obj.data as Record<string, unknown>;
      if (typeof data.type === 'string') return data.type;
    }
    return null;
  }

  function nextStep() {
    currentStep = Math.min(currentStep + 1, TOTAL_STEPS);
  }

  function clearSensitiveData() {
    seedPhraseInput = '';
    seedPhraseNormalized = '';
    seedPhraseValid = false;
    textImportInput = '';
    textImportValid = false;
    walletData = { name: '', emoji: '💰', color: 'blue', password: '', network: 'mainnet' };
    canImportWallet = false;
    submitError = '';
    isSubmitting = false;
    createdWalletId = '';
    openWalletError = '';
    openWalletLoading = false;
    openWalletAttempted = false;
  }

  function handleGoHome() {
    clearSensitiveData();
    onGoHome();
  }

  function handleBack() {
    if (currentStep === TOTAL_STEPS) {
      handleGoHome();
      return;
    }
    if (currentStep > 1) {
      currentStep -= 1;
      return;
    }
    handleGoHome();
  }

  async function handleImportWallet() {
    if (!canImportWallet) return;
    if (selectedMethod === 'seed24' && (!seedPhraseValid || !seedPhraseNormalized)) return;
    if (selectedMethod === 'text' && !textImportValid) return;

    isSubmitting = true;
    submitError = '';
    openWalletError = '';

    try {
      const result =
        selectedMethod === 'seed24'
          ? await invoke<CreateWalletResult>('create_wallet', {
              request: {
                wallet_name: walletData.name,
                seed_phrase: seedPhraseNormalized,
                network: walletData.network,
                emoji: walletData.emoji,
                color: walletData.color
              },
              password: walletData.password
            })
          : await invoke<CreateWalletResult>('import_wallet_text', {
              request: {
                wallet_name: walletData.name,
                import_text: textImportInput,
                network: walletData.network,
                emoji: walletData.emoji,
                color: walletData.color
              },
              password: walletData.password
            });

      if (!result?.wallet_id) {
        throw new Error('Missing wallet_id from import command');
      }

      createdWalletId = result.wallet_id;
      nextStep();
      openWalletAttempted = false;
    } catch (error) {
      const errorType = extractWalletErrorType(error);
      if (errorType === 'WalletExists') {
        submitError = i18n.t('walletImport.error.walletExists');
      } else if (errorType === 'InvalidSeedPhrase') {
        submitError = i18n.t('walletImport.error.invalidSeed');
      } else if (errorType === 'InvalidImportText') {
        submitError = i18n.t('walletImport.error.invalidImportText');
      } else if (errorType === 'InvalidWalletName') {
        submitError = i18n.t('walletImport.error.invalidName');
      } else if (errorType === 'InvalidPassword') {
        submitError = i18n.t('walletImport.error.invalidPassword');
      } else if (errorType === 'InvalidArgs') {
        submitError = i18n.t('walletImport.error.invalidArgs');
      } else {
        submitError = i18n.t('walletImport.error.importFailed');
      }
    } finally {
      isSubmitting = false;
    }
  }

  async function handleOpenWallet() {
    if (!createdWalletId || !walletData.password) {
      openWalletError = i18n.t('walletImport.error.openMissingData');
      return;
    }

    openWalletLoading = true;
    openWalletError = '';

    try {
      await invoke('unlock_wallet', {
        account_id: createdWalletId,
        password: walletData.password
      });
      clearSensitiveData();
      await goto('/wallet');
    } catch (error) {
      const errorType = extractWalletErrorType(error);
      if (errorType === 'SecureStorageUnavailable') {
        openWalletError = i18n.t('common.error.secureStorageUnavailable');
      } else if (errorType === 'InvalidPassword' || errorType === 'OperationFailed') {
        openWalletError = i18n.t('walletImport.error.openFailed');
      } else if (errorType === 'InvalidArgs') {
        openWalletError = i18n.t('walletImport.error.openInvalidArgs');
      } else {
        openWalletError = i18n.t('walletImport.error.openGeneric');
      }
    } finally {
      openWalletLoading = false;
    }
  }

  $effect(() => {
    if (currentStep !== TOTAL_STEPS) return;
    if (!createdWalletId || !walletData.password) return;
    if (openWalletAttempted || openWalletLoading || openWalletError) return;
    openWalletAttempted = true;
    handleOpenWallet();
  });

  $effect(() => {
    return () => {
      clearSensitiveData();
    };
  });
</script>

<StepperLayout
  currentStep={currentStep}
  totalSteps={TOTAL_STEPS}
  onClose={handleGoHome}
  showNetworkToggle={currentStep === 1}
  network={walletData.network}
  networkLabel={i18n.t('walletCreation.name.network')}
  onNetworkChange={(value) => {
    walletData = { ...walletData, network: value };
  }}
  contentClass={`flex-1 overflow-y-auto px-6 sm:px-8 ${currentStep === 2 ? 'py-6' : 'py-10'}`}
  contentInnerClass={`mx-auto w-full max-w-[620px] ${currentStep === 2 ? 'space-y-4' : 'space-y-6'}`}
>
  {#snippet children()}
    {#if currentStep === 1}
      <div class="text-center">
        <h1 class="text-foreground text-2xl leading-tight font-semibold tracking-tight">
          {i18n.t('walletImport.step1.title')}
        </h1>
      </div>
      <NameStep
        walletData={walletData}
        onUpdate={(data: WalletUpdate) => {
          walletData = { ...walletData, ...data };
        }}
        errorMessage=""
      />
    {:else if currentStep === 2}
      <div class="space-y-2 text-center">
        <h1 class="text-foreground text-2xl leading-tight font-semibold tracking-tight">
          {selectedMethod === 'seed24'
            ? i18n.t('walletImport.step2.seedTitle')
            : i18n.t('walletImport.step2.textTitle')}
        </h1>
        {#if selectedMethod !== 'seed24'}
          <p class="text-muted-foreground text-sm">{i18n.t('walletImport.step2.textDescription')}</p>
        {/if}
      </div>
      {#if selectedMethod === 'seed24'}
        <SeedPhraseStep
          seedPhraseInput={seedPhraseInput}
          onInputChanged={(value: string) => {
            seedPhraseInput = value;
          }}
          onNormalizedChanged={(value: string) => {
            seedPhraseNormalized = value;
          }}
          onValidityChanged={(valid: boolean) => {
            seedPhraseValid = valid;
          }}
        />
      {:else}
        <TextImportStep
          importTextInput={textImportInput}
          onInputChanged={(value: string) => {
            textImportInput = value;
          }}
          onValidityChanged={(valid: boolean) => {
            textImportValid = valid;
          }}
        />
      {/if}
    {:else if currentStep === 3}
      <div class="space-y-3 text-center">
        <h1 class="text-foreground text-2xl leading-tight font-semibold tracking-tight">
          {i18n.t('walletImport.step3.title')}
        </h1>
        <p class="text-muted-foreground text-sm">{i18n.t('walletImport.step3.description')}</p>
      </div>
      <PasswordStep
        walletData={walletData}
        onUpdate={(data: WalletUpdate) => {
          walletData = { ...walletData, ...data };
        }}
        onCanCreateChanged={(canCreate: boolean) => {
          canImportWallet = canCreate;
        }}
      />
    {:else if currentStep === 4}
      <div class="space-y-3 text-center">
        <h1 class="text-foreground text-2xl leading-tight font-semibold tracking-tight">
          {i18n.t('walletImport.step4.title')}
        </h1>
        <p class="text-muted-foreground text-sm">{i18n.t('walletImport.step4.description')}</p>
      </div>
      <ImportCompleteStep method={selectedMethod} isOpening={openWalletLoading} openError={openWalletError} />
    {/if}
  {/snippet}

  {#snippet footer()}
    <div class="flex w-full items-center justify-between gap-4">
      <Button variant="secondary" onclick={handleBack} class="min-w-48 px-6">
        {currentStep === 1 ? i18n.t('common.cancel') : i18n.t('common.back')}
      </Button>

      {#if currentStep === 1}
        <Button
          onclick={nextStep}
          disabled={!walletData.name.trim() || /[/\\:*?"<>|]/.test(walletData.name)}
          class="min-w-48 px-6"
        >
          {i18n.t('common.continue')}
        </Button>
      {:else if currentStep === 2}
        <Button
          onclick={nextStep}
          disabled={selectedMethod === 'seed24' ? !seedPhraseValid : !textImportValid}
          class="min-w-48 px-6"
        >
          {i18n.t('common.continue')}
        </Button>
      {:else if currentStep === 3}
        <div class="flex flex-col items-end gap-2">
          {#if submitError}
            <p class="text-destructive text-sm" aria-live="polite">{submitError}</p>
          {/if}
          {#if isSubmitting}
            <p class="text-muted-foreground text-xs">{i18n.t('walletImport.step3.loadingHint')}</p>
          {/if}
          <Button
            onclick={handleImportWallet}
            disabled={selectedMethod === 'seed24'
              ? !canImportWallet || !seedPhraseValid || !seedPhraseNormalized || isSubmitting
              : !canImportWallet || !textImportValid || isSubmitting}
            class="min-w-48 px-6"
          >
            {isSubmitting ? i18n.t('walletImport.button.importing') : i18n.t('walletImport.button.import')}
          </Button>
        </div>
      {:else if currentStep === 4}
        <div class="flex flex-col items-end gap-2">
          <div class="min-h-5">
            {#if openWalletError}
              <p class="text-destructive text-sm" aria-live="polite">{openWalletError}</p>
            {/if}
          </div>
          {#if openWalletError}
            <Button
              onclick={() => {
                openWalletError = '';
                openWalletAttempted = true;
                handleOpenWallet();
              }}
              disabled={openWalletLoading}
              class="min-w-48 px-6"
            >
              {i18n.t('walletImport.button.retryOpen')}
            </Button>
          {:else}
            <Button disabled={true} class="min-w-48 px-6">
              {i18n.t('walletImport.button.opening')}
            </Button>
          {/if}
        </div>
      {/if}
    </div>
  {/snippet}
</StepperLayout>
