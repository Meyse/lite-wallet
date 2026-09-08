<!--
  UnlockScreen: Login with password for existing wallet(s).
  Shows wallet list and password field; invokes unlock_wallet then navigates to /wallet.
  Hero panel: Verus logo (white) with light/dark seedling images matching first-launch layout.
  Security: No password logging; generic error messages only.
-->

<script lang="ts">
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import CirclePlusIcon from '@lucide/svelte/icons/circle-plus';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import { onDestroy, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Spinner } from '$lib/components/ui/spinner';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { i18nStore, networkLocaleKey } from '$lib/i18n';
  import HelpDrawerLink from '$lib/components/common/HelpDrawerLink.svelte';
  import WalletCreation from '$lib/components/flows/WalletCreation/WalletCreation.svelte';
  import WalletImport from '$lib/components/flows/WalletImport/WalletImport.svelte';
  import ImportMethodList from '$lib/components/flows/WalletImport/ImportMethodList.svelte';
  import VerusIdGuardDock from '$lib/components/wallet/VerusIdGuardDock.svelte';
  import WalletHeroBackground from '$lib/components/wallet/WalletHeroBackground.svelte';
  import type { ImportMethod } from '$lib/components/flows/WalletImport/types';
  import { getWalletColorHex } from '$lib/constants/walletColors';
  import { buildNeedHelpContent } from '$lib/utils/helpContent';
  import { cn } from '$lib/utils.js';
  import * as walletService from '$lib/services/walletService.js';

  export type WalletListItem = {
    account_id: string;
    wallet_name: string;
    network?: 'mainnet' | 'testnet';
    emoji?: string;
    color?: string;
  };

  const { wallets }: { wallets: WalletListItem[] } = $props();

  let selectedAccountId = $state('');
  let password = $state('');
  let errorMessage = $state('');
  let isLoading = $state(false);
  let shakePasswordField = $state(false);
  let showCreateOptionsDrawer = $state(false);
  let showWalletSwitcherDrawer = $state(false);
  let showCreateWallet = $state(false);
  let showWalletImport = $state(false);
  let selectedImportMethod = $state<ImportMethod>('seed24');
  let createDrawerView = $state<'root' | 'importMethods'>('root');
  let shakeResetTimer: ReturnType<typeof setTimeout> | null = null;
  const appWindow = getCurrentWindow();
  const i18n = $derived($i18nStore);

  const effectiveAccountId = $derived(
    wallets.length === 1 ? wallets[0].account_id : selectedAccountId
  );
  const selectedWallet = $derived(
    wallets.length === 0
      ? null
      : wallets.find((wallet) => wallet.account_id === effectiveAccountId) ?? wallets[0]
  );

  $effect(() => {
    if (wallets.length <= 1) {
      selectedAccountId = '';
      return;
    }
    const selectedExists = wallets.some((wallet) => wallet.account_id === selectedAccountId);
    if (!selectedExists) {
      selectedAccountId = wallets[0].account_id;
    }
  });

  function networkLabel(network?: 'mainnet' | 'testnet'): string {
    return i18n.t(networkLocaleKey(network));
  }

  function walletColorHex(color?: string): string {
    return getWalletColorHex(color);
  }

  function walletEmoji(emoji?: string): string {
    return emoji?.trim() || '💰';
  }

  function handleCreateWallet() {
    createDrawerView = 'root';
    showCreateOptionsDrawer = true;
  }

  function handleStartNewWalletFlow() {
    showCreateOptionsDrawer = false;
    createDrawerView = 'root';
    showCreateWallet = true;
  }

  function handleShowImportMethods() {
    createDrawerView = 'importMethods';
  }

  function handleStartImportWalletFlow(method: ImportMethod) {
    selectedImportMethod = method;
    showCreateOptionsDrawer = false;
    createDrawerView = 'root';
    showWalletImport = true;
  }

  const unlockHelpContent = $derived(buildNeedHelpContent(i18n.t, { includeLostAccess: true }));

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

  async function triggerWrongPasswordShake() {
    shakePasswordField = false;
    await tick();
    shakePasswordField = true;

    if (shakeResetTimer) {
      clearTimeout(shakeResetTimer);
    }
    shakeResetTimer = setTimeout(() => {
      shakePasswordField = false;
    }, 320);
  }

  onDestroy(() => {
    if (shakeResetTimer) {
      clearTimeout(shakeResetTimer);
    }
  });

  $effect(() => {
    if (!showCreateOptionsDrawer) {
      createDrawerView = 'root';
    }
  });

  $effect(() => {
    if (password.trim().length > 0) return;
    if (!errorMessage && !shakePasswordField) return;
    errorMessage = '';
    shakePasswordField = false;
    if (shakeResetTimer) {
      clearTimeout(shakeResetTimer);
      shakeResetTimer = null;
    }
  });

  async function handleUnlock() {
    if (!effectiveAccountId || !password.trim()) return;
    isLoading = true;
    errorMessage = '';
    try {
      await walletService.unlockWallet({
        account_id: effectiveAccountId,
        password
      });

      try {
        await goto('/wallet');
        password = '';
      } catch (navigationError) {
        console.error('[UNLOCK] Wallet unlocked but dashboard navigation failed', navigationError);
        await walletService.lockWallet().catch(() => {});
        errorMessage = i18n.t('unlock.error.openFailed');
      }
    } catch (error) {
      const errorType = extractWalletErrorType(error);
      if (errorType === 'InvalidPassword') {
        errorMessage = i18n.t('unlock.error.invalidPassword');
        await triggerWrongPasswordShake();
      } else if (errorType === 'SecureStorageUnavailable') {
        errorMessage = i18n.t('common.error.secureStorageUnavailable');
      } else if (errorType === 'OperationFailed') {
        errorMessage = i18n.t('unlock.error.operationFailed');
      } else if (errorType === 'InvalidArgs') {
        errorMessage = i18n.t('unlock.error.invalidArgs');
      } else {
        errorMessage = i18n.t('unlock.error.generic');
      }
    } finally {
      isLoading = false;
    }
  }

  async function handleDragMouseDown(event: MouseEvent) {
    if (event.button !== 0) return;
    event.preventDefault();
    try {
      await appWindow.startDragging();
    } catch {
      // Keep the data-tauri-drag-region fallback.
    }
  }
</script>

<main class="bg-background relative flex min-h-screen overflow-hidden">
  <div class="bg-app-canvas absolute inset-0"></div>
  <div
    class="absolute top-0 right-0 left-0 z-20 h-11"
    data-tauri-drag-region
    aria-hidden="true"
    onmousedown={handleDragMouseDown}
  ></div>

  <div class="relative z-10 flex min-h-screen w-full">
    <section class="relative hidden w-[clamp(320px,38vw,500px)] shrink-0 overflow-hidden md:block">
      <WalletHeroBackground suspended={showCreateWallet || showWalletImport} />
    </section>

    <section class="flex min-w-0 flex-1 items-center justify-center px-6 py-10 sm:px-8">
      <div class="w-full max-w-[360px] space-y-5">
        {#if selectedWallet}
          {#snippet walletIdentity()}
            <span
              class="flex h-12 w-12 shrink-0 select-none items-center justify-center rounded-xl text-2xl leading-none text-white"
              style={`background-color: ${walletColorHex(selectedWallet.color)};`}
              aria-hidden="true"
            >
              {walletEmoji(selectedWallet.emoji)}
            </span>
            <span class="text-foreground min-w-0 truncate text-xl font-semibold leading-tight">
              {selectedWallet.wallet_name}
            </span>
          {/snippet}

          <div>
            {#if wallets.length > 1}
              <button
                type="button"
                class="hover:bg-muted/60 focus-visible:ring-ring -m-2 flex max-w-[calc(100%+1rem)] cursor-pointer items-center gap-3 rounded-xl p-2 text-left transition-colors focus-visible:outline-none focus-visible:ring-2"
                aria-label={i18n.t('unlock.switchLabel', { name: selectedWallet.wallet_name })}
                aria-haspopup="dialog"
                aria-expanded={showWalletSwitcherDrawer}
                onclick={() => {
                  showWalletSwitcherDrawer = true;
                }}
              >
                {@render walletIdentity()}
                <ChevronRightIcon class="text-muted-foreground -ml-1 size-4 shrink-0" aria-hidden="true" />
              </button>
            {:else}
              <div class="flex min-w-0 items-center gap-3">
                {@render walletIdentity()}
              </div>
            {/if}
          </div>
        {/if}

        <div class="space-y-2">
          <div class="space-y-2">
            <Label for="unlock-password" class="sr-only">{i18n.t('unlock.password')}</Label>
            <div class={shakePasswordField ? 'unlock-error-shake' : ''}>
              <Input
                id="unlock-password"
                type="password"
                bind:value={password}
                placeholder={i18n.t('unlock.password')}
                autocomplete="current-password"
                class={errorMessage ? 'border-destructive' : ''}
                onkeydown={(e) => e.key === 'Enter' && handleUnlock()}
              />
            </div>
            <p class="text-destructive min-h-8 text-sm leading-5" aria-live="polite">
              {errorMessage || ' '}
            </p>
          </div>

          <div class="space-y-2">
            <Button
              class="w-full"
              onclick={handleUnlock}
              disabled={!effectiveAccountId || !password.trim() || isLoading}
            >
              <span class="inline-flex items-center gap-2">
                {#if isLoading}
                  <Spinner class="h-4 w-4" />
                {/if}
                <span>{isLoading ? i18n.t('unlock.button.unlocking') : i18n.t('unlock.button.unlock')}</span>
              </span>
            </Button>

            <Button
              variant="secondary"
              class="w-full"
              onclick={handleCreateWallet}
            >
              {i18n.t('unlock.button.createWallet')}
            </Button>
          </div>

          <div class="pt-2 text-muted-foreground text-xs">
            <HelpDrawerLink
              linkText={i18n.t('help.link.needHelp')}
              title={i18n.t('help.sheet.title')}
              content={unlockHelpContent}
            />
          </div>
        </div>
      </div>
    </section>
  </div>
</main>

<VerusIdGuardDock
  context="unlock"
  defaultNetwork={selectedWallet?.network === 'testnet' ? 'testnet' : 'mainnet'}
/>

<StandardRightSheet bind:isOpen={showWalletSwitcherDrawer} title={i18n.t('unlock.switcher.title')}>
  <div class="flex-1 space-y-2 overflow-y-auto pr-1">
    {#each wallets as wallet}
      {@const isSelected = effectiveAccountId === wallet.account_id}
      <button
        type="button"
        class={cn(
          'group w-full rounded-lg p-3 text-left transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
          isSelected
            ? 'bg-primary/14 hover:bg-primary/20 dark:bg-primary/28 dark:hover:bg-primary/36'
            : 'bg-muted/65 hover:bg-muted/70 dark:bg-muted/55 dark:hover:bg-muted/65'
        )}
        onclick={() => {
          selectedAccountId = wallet.account_id;
          errorMessage = '';
          password = '';
          showWalletSwitcherDrawer = false;
        }}
      >
        <div class="flex items-center gap-3">
          <div
            class="flex h-9 w-9 shrink-0 cursor-default select-none items-center justify-center rounded-lg text-base text-white"
            style={`background-color: ${walletColorHex(wallet.color)};`}
          >
            {walletEmoji(wallet.emoji)}
          </div>
          <div class="min-w-0 text-left">
            <p class="text-foreground truncate text-sm font-semibold">{wallet.wallet_name}</p>
            <p class="text-muted-foreground text-xs">{networkLabel(wallet.network)}</p>
          </div>
        </div>
      </button>
    {/each}
  </div>
</StandardRightSheet>

<StandardRightSheet
  bind:isOpen={showCreateOptionsDrawer}
  title={i18n.t(createDrawerView === 'root' ? 'unlock.create.title' : 'unlock.importMethods.title')}
>
  {#if createDrawerView === 'root'}
    <div class="space-y-3">
      <button
        type="button"
        class="group selection-card-button selection-card-button--neutral"
        onclick={handleStartNewWalletFlow}
      >
        <div class="flex items-start gap-3">
          <CirclePlusIcon
            class="selection-card-icon"
            absoluteStrokeWidth
            stroke-linecap="butt"
            aria-hidden="true"
          />
          <div class="min-w-0">
            <p class="text-foreground text-sm font-semibold">{i18n.t('unlock.create.newTitle')}</p>
            <p class="text-muted-foreground mt-1 text-xs">
              {i18n.t('unlock.create.newDescription')}
            </p>
          </div>
        </div>
      </button>

      <button
        type="button"
        class="group selection-card-button selection-card-button--neutral"
        onclick={handleShowImportMethods}
      >
        <div class="flex items-start gap-3">
          <DownloadIcon
            class="selection-card-icon"
            absoluteStrokeWidth
            stroke-linecap="butt"
            aria-hidden="true"
          />
          <div class="min-w-0">
            <p class="text-foreground text-sm font-semibold">{i18n.t('unlock.create.importTitle')}</p>
            <p class="text-muted-foreground mt-1 text-xs">
              {i18n.t('unlock.create.importDescription')}
            </p>
          </div>
        </div>
      </button>
    </div>
  {:else}
    <div class="space-y-3">
      <button
        type="button"
        class="text-muted-foreground hover:text-foreground inline-flex items-center gap-1.5 text-sm transition-colors"
        onclick={() => {
          createDrawerView = 'root';
        }}
      >
        <ArrowLeftIcon class="size-4" />
        {i18n.t('unlock.importMethods.back')}
      </button>

      <ImportMethodList
        showHeader={false}
        onSelect={(method) => {
          handleStartImportWalletFlow(method);
        }}
      />
    </div>
  {/if}
</StandardRightSheet>

{#if showCreateWallet}
  <div class="fixed inset-0 z-50">
    <WalletCreation
      onGoHome={() => {
        showCreateWallet = false;
      }}
    />
  </div>
{/if}

{#if showWalletImport}
  <div class="fixed inset-0 z-50">
    <WalletImport
      initialMethod={selectedImportMethod}
      onGoHome={() => {
        showWalletImport = false;
        selectedImportMethod = 'seed24';
      }}
    />
  </div>
{/if}

<style>
  @keyframes unlock-error-shake {
    0%,
    100% {
      transform: translateX(0);
    }
    20% {
      transform: translateX(-5px);
    }
    40% {
      transform: translateX(5px);
    }
    60% {
      transform: translateX(-4px);
    }
    80% {
      transform: translateX(4px);
    }
  }

  .unlock-error-shake {
    animation: unlock-error-shake 320ms cubic-bezier(0.36, 0.07, 0.19, 0.97);
    will-change: transform;
  }

  @media (prefers-reduced-motion: reduce) {
    .unlock-error-shake {
      animation: none;
    }
  }
</style>
