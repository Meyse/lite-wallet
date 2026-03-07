<!-- 
  Component: WalletLayout
  Purpose: Main layout component that assembles sidebar and content sections
  Last Updated: Initial creation
  Security: No sensitive operations - layout and navigation only
-->

<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
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
  import { dismissWalletError, walletErrorsStore } from '$lib/stores/walletErrors.js';
  import { i18nStore } from '$lib/i18n';
  import type { TransferEntryContext } from './sections/transfer-wizard/types';
  import type { WalletEntrySelection } from '$lib/types/wallet';

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
  const walletErrors = $derived($walletErrorsStore);
  const latestError = $derived(walletErrors.latest);
  const i18n = $derived($i18nStore);
  const isTransferFocusMode = $derived(activeSection === 'send' || activeSection === 'conversions');

  $effect(() => {
    if (activeSection === 'send' || activeSection === 'conversions') return;
    transferEntryContext = null;
  });
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
</div>
