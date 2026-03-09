<!-- 
  Component: WalletLayout
  Purpose: Main layout component that assembles sidebar and content sections
  Last Updated: Initial creation
  Security: No sensitive operations - layout and navigation only
-->

<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
  import GenericRequestImportSheet from '$lib/components/flows/GenericRequest/GenericRequestImportSheet.svelte';
  import AppSidebar from './AppSidebar.svelte';
  import Overview from './sections/Overview.svelte';
  import AssetDetails from './sections/AssetDetails.svelte';
  import Send from './sections/Send.svelte';
  import Receive from './sections/Receive.svelte';
  import Conversions from './sections/Conversions.svelte';
  import Identity from './sections/Identity.svelte';
  import Apps from './sections/Apps.svelte';
  import Activity from './sections/Activity.svelte';
  import AddressBook from './sections/AddressBook.svelte';
  import Settings from './sections/Settings.svelte';
  import { dismissWalletError, pushWalletError, walletErrorsStore } from '$lib/stores/walletErrors.js';
  import {
    consumeQueuedGenericRequest,
    genericRequestQueueStore
  } from '$lib/stores/genericRequest.js';
  import { i18nStore } from '$lib/i18n';
  import * as genericRequestService from '$lib/services/genericRequestService.js';
  import type { TransferEntryContext } from './sections/transfer-wizard/types';
  import type { WalletEntrySelection } from '$lib/types/wallet';
  import { extractWalletErrorMessage, extractWalletErrorType } from '$lib/utils/walletErrors.js';
  import type { GenericRequestFlowSession } from '$lib/genericRequest/session';

  interface WalletData {
    name: string;
    emoji: string;
    color: string;
    network?: 'mainnet' | 'testnet';
  }

  type SectionId =
    | 'overview'
    | 'send'
    | 'receive'
    | 'conversions'
    | 'identity'
    | 'apps'
    | 'activity'
    | 'address-book'
    | 'settings';

  const { walletData }: { walletData: WalletData } = $props();
  let activeSection = $state<SectionId>('overview');
  let settingsResetSignal = $state(0);
  let activeAssetDetailsEntry = $state<WalletEntrySelection | null>(null);
  let transferEntryContext = $state<TransferEntryContext | null>(null);
  let genericRequestImportOpen = $state(false);
  let genericRequestImportValue = $state('');
  let genericRequestImportBusy = $state(false);
  let genericRequestImportError = $state('');
  let genericRequestFlowOpen = $state(false);
  let genericRequestSession = $state<GenericRequestFlowSession | null>(null);
  let GenericRequestFlowHostComponent = $state<null | (typeof import('$lib/components/flows/GenericRequest/GenericRequestFlowHost.svelte').default)>(null);
  const walletErrors = $derived($walletErrorsStore);
  const latestError = $derived(walletErrors.latest);
  const i18n = $derived($i18nStore);
  const isTransferFocusMode = $derived(activeSection === 'send' || activeSection === 'conversions');
  const queuedGenericRequest = $derived($genericRequestQueueStore);

  $effect(() => {
    if (activeSection === 'send' || activeSection === 'conversions') return;
    transferEntryContext = null;
  });

  $effect(() => {
    if (!queuedGenericRequest || genericRequestImportBusy || genericRequestFlowOpen) return;

    const queued = consumeQueuedGenericRequest();
    if (!queued) return;

    void openGenericRequestFlow(queued.input, queued.passthroughAutoLinkFqn, true);
  });

  function resolveGenericRequestErrorMessage(errorValue: unknown): string {
    const errorType = extractWalletErrorType(errorValue);

    switch (errorType) {
      case 'WalletLocked':
        return i18n.t('genericRequest.error.walletLocked');
      case 'GenericRequestInvalidEnvelope':
        return i18n.t('genericRequest.import.error.invalid');
      case 'GenericRequestUnsupportedSignature':
        return i18n.t('genericRequest.error.invalidSignature');
      default:
        break;
    }

    if (errorValue instanceof Error && errorValue.message.startsWith('genericRequest.')) {
      return i18n.t(errorValue.message);
    }

    return extractWalletErrorMessage(errorValue) || i18n.t('genericRequest.error.generic');
  }

  async function openGenericRequestFlow(
    input: string,
    passthroughAutoLinkFqn: string | null = null,
    surfaceAsWalletError = false
  ): Promise<void> {
    genericRequestImportBusy = true;
    if (!surfaceAsWalletError) {
      genericRequestImportError = '';
    }

    try {
      const [{ parseGenericRequestSession }, flowHostModule] = await Promise.all([
        import('$lib/genericRequest/session'),
        GenericRequestFlowHostComponent
          ? Promise.resolve({ default: GenericRequestFlowHostComponent })
          : import('$lib/components/flows/GenericRequest/GenericRequestFlowHost.svelte')
      ]);
      GenericRequestFlowHostComponent = flowHostModule.default;

      const nextSession = parseGenericRequestSession(input, passthroughAutoLinkFqn);
      const requestNetwork = nextSession.testnet ? 'testnet' : 'mainnet';
      if ((walletData.network ?? 'mainnet') !== requestNetwork) {
        throw new Error('genericRequest.import.error.networkMismatch');
      }

      const verification = await genericRequestService.verifyGenericRequestSignature(nextSession.requestHex);
      if (!verification.valid) {
        throw new Error('genericRequest.error.invalidSignature');
      }

      genericRequestSession = nextSession;
      genericRequestFlowOpen = true;
      genericRequestImportOpen = false;
      genericRequestImportError = '';
      genericRequestImportValue = input.trim();
    } catch (errorValue) {
      const message = resolveGenericRequestErrorMessage(errorValue);
      if (surfaceAsWalletError) {
        pushWalletError(message);
      } else {
        genericRequestImportError = message;
      }
    } finally {
      genericRequestImportBusy = false;
    }
  }

  function closeGenericRequestFlow(): void {
    genericRequestFlowOpen = false;
    genericRequestSession = null;
  }
</script>

<div class="relative h-screen overflow-hidden">
  {#if !isTransferFocusMode}
    <div class="absolute top-0 left-0 z-40 h-11 w-[15.25rem]" data-tauri-drag-region aria-hidden="true"></div>
  {/if}
  <Sidebar.Provider class="h-full overflow-hidden">
    {#if !isTransferFocusMode}
      <AppSidebar
        bind:activeSection
        {walletData}
        onOpenRequest={() => {
          genericRequestImportError = '';
          genericRequestImportOpen = true;
        }}
        onSelectOverview={() => {
          activeAssetDetailsEntry = null;
          transferEntryContext = null;
          activeSection = 'overview';
        }}
        onSelectSettings={() => {
          settingsResetSignal += 1;
        }}
      />
    {/if}
    <Sidebar.Inset class="h-full min-h-0 dark:bg-app-canvas">
      {#if !isTransferFocusMode}
        <div class="h-6 shrink-0" data-tauri-drag-region aria-hidden="true"></div>
      {/if}
      {#if latestError}
        <div class="mx-6 mt-4 rounded-md border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-900">
          <div class="flex items-start justify-between gap-3">
            <p class="break-all">{latestError}</p>
            <button class="shrink-0 text-xs underline" onclick={dismissWalletError}>
              {i18n.t('wallet.layout.dismiss')}
            </button>
          </div>
        </div>
      {/if}
      <main
        class={isTransferFocusMode || activeSection === 'overview'
          ? 'flex flex-1 min-h-0 overflow-hidden'
          : 'flex-1 min-h-0 overflow-auto'}
      >
        {#if activeSection === 'overview'}
          {#if activeAssetDetailsEntry}
            <AssetDetails
              coinId={activeAssetDetailsEntry.coinId}
              walletEntryKind={activeAssetDetailsEntry.walletEntryKind}
              scopeFilterMode={activeAssetDetailsEntry.scopeFilterMode}
              entryDisplayName={activeAssetDetailsEntry.displayName}
              onNavigateToReceive={() => {
                activeSection = 'receive';
              }}
              onNavigateToSend={(context) => {
                transferEntryContext = context;
                activeSection = 'send';
              }}
              onNavigateToConvert={(context) => {
                transferEntryContext = context;
                activeSection = 'conversions';
              }}
            />
          {:else}
            <Overview
              {walletData}
              onOpenAssetDetails={(entry) => {
                activeAssetDetailsEntry = entry;
              }}
              onNavigateToSend={() => {
                transferEntryContext = null;
                activeSection = 'send';
              }}
              onNavigateToReceive={() => {
                activeSection = 'receive';
              }}
              onNavigateToConvert={() => {
                transferEntryContext = null;
                activeSection = 'conversions';
              }}
            />
          {/if}
        {:else if activeSection === 'send'}
          <Send
            entryContext={transferEntryContext}
            onClose={() => {
              activeSection = 'overview';
              transferEntryContext = null;
            }}
          />
        {:else if activeSection === 'receive'}
          <Receive />
        {:else if activeSection === 'conversions'}
          <Conversions
            entryContext={transferEntryContext}
            onClose={() => {
              activeSection = 'overview';
              transferEntryContext = null;
            }}
          />
        {:else if activeSection === 'identity'}
          <Identity walletNetwork={walletData.network ?? 'mainnet'} />
        {:else if activeSection === 'apps'}
          <Apps />
        {:else if activeSection === 'activity'}
          <Activity />
        {:else if activeSection === 'address-book'}
          <AddressBook />
        {:else if activeSection === 'settings'}
          <Settings walletNetwork={walletData.network ?? 'mainnet'} resetSignal={settingsResetSignal} />
        {/if}
      </main>
    </Sidebar.Inset>
  </Sidebar.Provider>

  <GenericRequestImportSheet
    bind:isOpen={genericRequestImportOpen}
    bind:value={genericRequestImportValue}
    submitting={genericRequestImportBusy}
    errorMessage={genericRequestImportError}
    onSubmit={(value) => void openGenericRequestFlow(value)}
  />

  {#if GenericRequestFlowHostComponent}
    <GenericRequestFlowHostComponent
      bind:isOpen={genericRequestFlowOpen}
      session={genericRequestSession}
      onClose={closeGenericRequestFlow}
    />
  {/if}
</div>
