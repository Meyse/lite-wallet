<!--
  Component: WalletCreation
  Purpose: Wallet creation flow with onboarding, backup, and setup
  Last Updated: Centered single-panel setup flow with setup-only steps (intro removed)
  Security: Manages shared state and sensitive data clearing, enforces security acknowledgment
-->

<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { goto } from '$app/navigation';
  import { Button } from '$lib/components/ui/button';
  import StepperLayout from '$lib/components/shared/StepperLayout.svelte';
  import NameStep from './NameStep.svelte';
  import SecurityStep from './SecurityStep.svelte';
  import BackupStep from './BackupStep.svelte';
  import VerifyStep from './VerifyStep.svelte';
  import PasswordStep from './PasswordStep.svelte';
  import CompleteStep from './CompleteStep.svelte';
  import { i18nStore } from '$lib/i18n';

  type WalletData = {
    name: string;
    emoji: string;
    color: string;
    password: string;
    network: 'mainnet' | 'testnet';
  };

  type WalletUpdate = Partial<WalletData>;
  type CreateWalletResult = { wallet_id: string; success: boolean };

  const TOTAL_STEPS = 6;

  // Props
  const { onGoHome = () => {} } = $props();

  // Shared state for all steps
  let currentStep = $state(1);
  let walletData = $state<WalletData>({
    name: '',
    emoji: '💰',
    color: 'blue',
    password: '',
    network: 'mainnet'
  });
  let seedPhrase = $state<string>(''); // Security: Cleared on unmount
  let verificationIndices = $state<number[]>([]);
  let securityAccepted = $state(false);
  let allVerificationFieldsFilled = $state(false);
  let verifyStepRef: any = $state(null);
  let backupCanContinue = $state(false);
  let setupDlightWithPrimary = $state(true);
  let canCreateWallet = $state(false);
  let createLoading = $state(false);
  let createError = $state('');
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

  // Create wallet via backend then advance to CompleteStep
  async function handleCreateWallet() {
    if (!canCreateWallet) return;
    createLoading = true;
    createError = '';
    openWalletError = '';
    try {
      const result = await invoke<CreateWalletResult>('create_wallet', {
        request: {
          wallet_name: walletData.name,
          seed_phrase: seedPhrase,
          network: walletData.network,
          emoji: walletData.emoji,
          color: walletData.color,
          setup_dlight_with_primary: setupDlightWithPrimary
        },
        password: walletData.password
      });

      if (!result?.wallet_id) {
        throw new Error('Missing wallet_id from create_wallet');
      }

      createdWalletId = result.wallet_id;
      seedPhrase = '';
      openWalletAttempted = false;
      nextStep();
    } catch (error) {
      const errorType = extractWalletErrorType(error);
      if (errorType === 'WalletExists') {
        createError = i18n.t('walletCreation.error.walletExists');
      } else {
        createError = i18n.t('walletCreation.error.createFailed');
      }
    } finally {
      createLoading = false;
    }
  }

  async function handleOpenWallet() {
    if (!createdWalletId || !walletData.password) {
      openWalletError = i18n.t('walletCreation.error.openMissingData');
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
      if (errorType === 'InvalidPassword' || errorType === 'OperationFailed') {
        openWalletError = i18n.t('walletCreation.error.openFailed');
      } else if (errorType === 'InvalidArgs') {
        openWalletError = i18n.t('walletCreation.error.openInvalidArgs');
      } else {
        openWalletError = i18n.t('walletCreation.error.openGeneric');
      }
    } finally {
      openWalletLoading = false;
    }
  }

  function nextStep() {
    currentStep = Math.min(currentStep + 1, TOTAL_STEPS);
  }

  function handleBack() {
    if (currentStep > 1) {
      currentStep -= 1;
      return;
    }
    handleGoHome();
  }

  // Handle verification and continue
  function handleVerifyAndContinue() {
    if (verifyStepRef && verifyStepRef.verifyWords) {
      const isValid = verifyStepRef.verifyWords();
      if (isValid) {
        nextStep();
      }
      // If invalid, errors will show inline - no need to do anything else
    }
  }

  // Clear sensitive data and go home
  function handleGoHome() {
    console.info('[WALLET_CREATION] Clearing sensitive data before navigation');
    clearSensitiveData();
    console.info('[WALLET_CREATION] Calling parent onGoHome');
    onGoHome();
  }

  function clearSensitiveData() {
    seedPhrase = '';
    walletData = { name: '', emoji: '💰', color: 'blue', password: '', network: 'mainnet' };
    verificationIndices = [];
    securityAccepted = false;
    backupCanContinue = false;
    setupDlightWithPrimary = true;
    createdWalletId = '';
    openWalletError = '';
    openWalletLoading = false;
    openWalletAttempted = false;
  }

  $effect(() => {
    if (currentStep !== TOTAL_STEPS) return;
    if (!createdWalletId || !walletData.password) return;
    if (openWalletAttempted || openWalletLoading || openWalletError) return;
    openWalletAttempted = true;
    handleOpenWallet();
  });

  // Security: Clear all sensitive data on component destroy
  $effect(() => {
    return () => {
      clearSensitiveData();
      console.info('[WALLET] Component destroyed, sensitive data cleared');
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
>
  {#snippet children()}
    {#if currentStep === 1}
      <div class="space-y-3 text-center">
        <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
          {i18n.t('walletCreation.step2.title')}
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
      <div class="space-y-3 text-center">
        <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
          {i18n.t('walletCreation.step3.title')}
        </h1>
      </div>
      <SecurityStep bind:securityAccepted bind:setupPrivateVerus={setupDlightWithPrimary} />
    {:else if currentStep === 3}
      <div class="space-y-3 text-center">
        <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
          {i18n.t('walletCreation.step4.title')}
        </h1>
        <p class="text-muted-foreground text-sm">{i18n.t('walletCreation.step4.description')}</p>
      </div>
      <BackupStep
        walletData={walletData}
        seedPhrase={seedPhrase}
        bind:canContinue={backupCanContinue}
        onSeedGenerated={(seed: string) => {
          seedPhrase = seed;
        }}
      />
    {:else if currentStep === 4}
      <div class="space-y-3 text-center">
        <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
          {i18n.t('walletCreation.step5.title')}
        </h1>
        <p class="text-muted-foreground text-sm">{i18n.t('walletCreation.step5.description')}</p>
      </div>
      <VerifyStep
        seedPhrase={seedPhrase}
        verificationIndices={verificationIndices}
        onSetupVerification={(indices: number[]) => {
          verificationIndices = indices;
        }}
        onFieldsChanged={(filled: boolean) => {
          allVerificationFieldsFilled = filled;
        }}
        bind:this={verifyStepRef}
      />
    {:else if currentStep === 5}
      <div class="space-y-3 text-center">
        <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
          {i18n.t('walletCreation.step6.title')}
        </h1>
        <p class="text-muted-foreground text-sm">{i18n.t('walletCreation.step6.description')}</p>
      </div>
      <PasswordStep
        walletData={walletData}
        onUpdate={(data: WalletUpdate) => {
          walletData = { ...walletData, ...data };
        }}
        onCanCreateChanged={(canCreate: boolean) => {
          canCreateWallet = canCreate;
        }}
      />
    {:else if currentStep === 6}
      <div class="space-y-3 text-center">
        <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
          {i18n.t('walletCreation.step7.title')}
        </h1>
        <p class="text-muted-foreground text-sm">{i18n.t('walletCreation.step7.description')}</p>
      </div>
      <CompleteStep isOpening={openWalletLoading} openError={openWalletError} />
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
        <Button onclick={nextStep} disabled={!securityAccepted} class="min-w-48 px-6">
          {i18n.t('walletCreation.step3.button')}
        </Button>
      {:else if currentStep === 3}
        <Button onclick={nextStep} disabled={!backupCanContinue} class="min-w-48 px-6">
          {i18n.t('walletCreation.step4.button')}
        </Button>
      {:else if currentStep === 4}
        <Button
          onclick={handleVerifyAndContinue}
          disabled={!allVerificationFieldsFilled}
          class="min-w-48 px-6"
        >
          {i18n.t('walletCreation.step5.button')}
        </Button>
      {:else if currentStep === 5}
        <div class="flex flex-col items-end gap-2">
          {#if createError}
            <p class="text-destructive text-sm">{createError}</p>
          {/if}
          {#if createLoading}
            <p class="text-muted-foreground text-xs">{i18n.t('walletCreation.step6.loadingHint')}</p>
          {/if}
          <Button
            onclick={handleCreateWallet}
            disabled={!canCreateWallet || createLoading}
            class="min-w-48 px-6"
          >
            {createLoading ? i18n.t('walletCreation.step6.buttonCreating') : i18n.t('walletCreation.step6.buttonCreate')}
          </Button>
        </div>
      {:else if currentStep === 6}
        <div class="flex flex-col items-end gap-2">
          <div class="min-h-5">
            {#if openWalletError}
              <p class="text-destructive text-sm">{openWalletError}</p>
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
              {i18n.t('walletCreation.step7.buttonRetry')}
            </Button>
          {:else}
            <Button disabled class="min-w-48 px-6">
              {i18n.t('walletCreation.step7.buttonOpening')}
            </Button>
          {/if}
        </div>
      {/if}
    </div>
  {/snippet}
</StepperLayout>
