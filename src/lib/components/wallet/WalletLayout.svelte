<!-- 
  Component: WalletLayout
  Purpose: Main layout component that assembles sidebar and content sections
  Last Updated: Initial creation
  Security: No sensitive operations - layout and navigation only
-->

<script lang="ts">
  import { tick } from 'svelte';
  import type { ContactIdentity } from '$lib/types/addressBook';
  import { provideContactNavigation } from '$lib/contacts/navigation';
  import type { ContactReturnState } from '$lib/contacts/navigation';
  import * as Dialog from '$lib/components/ui/dialog';
  import * as Sidebar from '$lib/components/ui/sidebar';
  import * as Alert from '$lib/components/ui/alert';
  import { Button } from '$lib/components/ui/button';
  import GenericRequestImportSheet from '$lib/components/flows/GenericRequest/GenericRequestImportSheet.svelte';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';
  import XIcon from '@lucide/svelte/icons/x';
  import AppSidebar from './AppSidebar.svelte';
  import Overview from './sections/Overview.svelte';
  import AssetDetails from './sections/AssetDetails.svelte';
  import TransferWizard from './sections/TransferWizard.svelte';
  import Receive from './sections/Receive.svelte';
  import Identity from './sections/Identity.svelte';
  import Apps from './sections/Apps.svelte';
  import Activity from './sections/Activity.svelte';
  import AddressBook from './sections/AddressBook.svelte';
  import Watchlist from './sections/Watchlist.svelte';
  import Settings from './sections/Settings.svelte';
  import {
    createIdentitySectionSessionState,
    type IdentitySectionSessionState,
  } from './sections/identity/identitySectionSessionState.js';
  import {
    dismissWalletError,
    pushWalletError,
    walletErrorsStore,
  } from '$lib/stores/walletErrors.js';
  import type { WalletErrorEntry } from '$lib/stores/walletErrors.js';
  import {
    consumeQueuedGenericRequest,
    genericRequestQueueStore,
  } from '$lib/stores/genericRequest.js';
  import { i18nStore } from '$lib/i18n';
  import * as genericRequestService from '$lib/services/genericRequestService.js';
  import { isForcedWalletLockError } from '$lib/services/walletLockCoordinator.js';
  import type {
    TransferEntryContext,
    TransferNavigationState,
    TransferRecipientIntent,
  } from './sections/transfer-wizard/types';
  import type { WalletEntrySelection } from '$lib/types/wallet';
  import { extractWalletErrorMessage, extractWalletErrorType } from '$lib/utils/walletErrors.js';
  import type { GenericRequestFlowSession } from '$lib/genericRequest/session';
  import { transferWalletSessionKey } from './sections/transfer-wizard/preflightRequest';
  import { clearDlightSetupSession } from '$lib/utils/dlightSetupCoordinator';

  interface WalletData {
    name: string;
    emoji: string;
    color: string;
    network?: 'mainnet' | 'testnet';
    sessionId: string;
  }

  type SectionId =
    | 'overview'
    | 'send'
    | 'receive'
    | 'conversions'
    | 'identity'
    | 'apps'
    | 'activity'
    | 'watchlist'
    | 'address-book'
    | 'settings';

  const { walletData }: { walletData: WalletData } = $props();
  let activeSection = $state<SectionId>('overview');
  let settingsResetSignal = $state(0);
  let activeAssetDetailsEntry = $state<WalletEntrySelection | null>(null);
  type TransferDraft = {
    id: number;
    walletKey: string;
    intent: 'send' | 'convert';
    context: TransferEntryContext | null;
    recipientIntent: TransferRecipientIntent | null;
  };
  let transferDraft = $state<TransferDraft | null>(null);
  let nextDraftId = 0;
  let requestedContactIdentity = $state<ContactIdentity | null>(null);
  let contactReturnState = $state<ContactReturnState | null>(null);
  let pendingTransfer = $state<{
    intent: 'send' | 'convert';
    context: TransferEntryContext | null;
    recipientIntent: TransferRecipientIntent | null;
  } | null>(null);
  let transferNavigation = $state<TransferNavigationState>({
    mode: 'send',
    dirty: false,
    locked: false,
    completed: false,
  });
  let transferFocus: HTMLElement | null = null;
  let transferHost = $state<HTMLDivElement>();
  let resumeTransferButton = $state<HTMLButtonElement | null>(null);
  const isTransferSection = $derived(activeSection === 'send' || activeSection === 'conversions');
  let genericRequestImportOpen = $state(false);
  let genericRequestImportValue = $state('');
  let genericRequestImportBusy = $state(false);
  let genericRequestImportError = $state('');
  let genericRequestFlowOpen = $state(false);
  let genericRequestSession = $state<GenericRequestFlowSession | null>(null);
  let identitySectionSession = $state<IdentitySectionSessionState>(
    createIdentitySectionSessionState()
  );
  let identitySectionWalletKey = $state('');
  let GenericRequestFlowHostComponent = $state<
    | null
    | typeof import('$lib/components/flows/GenericRequest/GenericRequestFlowHost.svelte').default
  >(null);
  const walletErrors = $derived($walletErrorsStore);
  const latestError = $derived(walletErrors.latest);
  const i18n = $derived($i18nStore);
  const latestErrorTitle = $derived(
    latestError?.presentation.kind === 'background-update'
      ? i18n.t('wallet.layout.backgroundError.title')
      : i18n.t('wallet.layout.noticeTitle')
  );
  const latestErrorMessage = $derived(resolveWalletErrorMessage(latestError));
  const transferWalletKey = $derived(
    transferWalletSessionKey(walletData.name, walletData.network ?? 'mainnet', walletData.sessionId)
  );
  const queuedGenericRequest = $derived($genericRequestQueueStore);
  const currentDraft = $derived(
    transferDraft?.walletKey === transferWalletKey ? transferDraft : null
  );
  const navigationLocked = $derived(!!currentDraft && transferNavigation.locked);

  provideContactNavigation((identity, focus) => {
    if (navigationLocked) return;
    if (currentDraft && isTransferSection) transferFocus = focus;
    requestedContactIdentity = identity;
    contactReturnState = null;
    activeSection = 'address-book';
    closeGenericRequestFlow();
  });

  $effect(() => {
    if (
      activeSection !== 'address-book' &&
      !(
        activeSection === 'identity' &&
        identitySectionSession.publicProfile?.origin.kind === 'contacts'
      )
    ) {
      requestedContactIdentity = null;
    }
  });

  function navigateToSection(section: SectionId): void {
    if (navigationLocked) return;
    if (section !== 'address-book') contactReturnState = null;
    if (
      isTransferSection &&
      document.activeElement instanceof HTMLElement &&
      transferHost?.contains(document.activeElement)
    ) {
      transferFocus = document.activeElement;
    }
    if (section === 'overview') activeAssetDetailsEntry = null;
    if (section === 'settings') settingsResetSignal += 1;
    activeSection = section;
  }

  function startTransfer(
    intent: 'send' | 'convert',
    context: TransferEntryContext | null,
    recipientIntent: TransferRecipientIntent | null = null
  ): void {
    transferNavigation = { mode: intent, dirty: false, locked: false, completed: false };
    transferDraft = {
      id: ++nextDraftId,
      walletKey: transferWalletKey,
      intent,
      context,
      recipientIntent,
    };
    transferFocus = null;
    pendingTransfer = null;
    activeSection = intent === 'send' ? 'send' : 'conversions';
  }

  function requestTransfer(
    intent: 'send' | 'convert',
    context: TransferEntryContext | null = null,
    recipientIntent: TransferRecipientIntent | null = null
  ): void {
    if (navigationLocked) return;
    if (currentDraft && transferNavigation.dirty && !transferNavigation.completed) {
      // A new entry point must never silently replace the retained payment.
      pendingTransfer = { intent, context, recipientIntent };
      return;
    }
    startTransfer(intent, context, recipientIntent);
  }

  function openOverviewTransfer(intent: 'send' | 'convert'): void {
    if (currentDraft && transferNavigation.mode === intent) {
      void resumeTransfer();
      return;
    }

    requestTransfer(intent);
  }

  async function resumeTransfer(): Promise<void> {
    if (!currentDraft || navigationLocked) return;
    pendingTransfer = null;
    activeSection = transferNavigation.mode === 'convert' ? 'conversions' : 'send';
    await tick();
    if (transferFocus?.isConnected && !transferFocus.closest('[inert]')) transferFocus.focus();
    else
      transferHost
        ?.querySelector<HTMLElement>('[data-transfer-heading], #transfer-amount')
        ?.focus();
  }

  function closeTransfer(): void {
    if (navigationLocked) return;
    transferDraft = null;
    transferFocus = null;
    activeSection = 'overview';
  }

  $effect(() => {
    const walletSessionKey = transferWalletKey;
    return () => clearDlightSetupSession(walletSessionKey);
  });

  $effect(() => {
    if (transferDraft && transferDraft.walletKey !== transferWalletKey) {
      transferDraft = null;
      pendingTransfer = null;
      transferFocus = null;
      if (isTransferSection) activeSection = 'overview';
    }
  });

  $effect(() => {
    if (
      !queuedGenericRequest ||
      genericRequestImportBusy ||
      genericRequestFlowOpen ||
      navigationLocked
    )
      return;

    const queued = consumeQueuedGenericRequest();
    if (!queued) return;

    void openGenericRequestFlow(queued.input, queued.passthroughAutoLinkFqn, true);
  });

  $effect(() => {
    const nextWalletKey = transferWalletSessionKey(
      walletData.name,
      walletData.network ?? 'mainnet',
      walletData.sessionId
    );
    if (identitySectionWalletKey === nextWalletKey) return;

    identitySectionWalletKey = nextWalletKey;
    identitySectionSession = createIdentitySectionSessionState();
    requestedContactIdentity = null;
    contactReturnState = null;
  });

  function resolveGenericRequestErrorMessage(errorValue: unknown): string {
    if (isForcedWalletLockError(errorValue)) {
      return '';
    }

    const errorType = extractWalletErrorType(errorValue);

    switch (errorType) {
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

  function resolveWalletErrorMessage(error: WalletErrorEntry | null): string {
    if (!error) return '';
    if (error.presentation.kind === 'message') {
      return error.presentation.message;
    }

    switch (error.presentation.dataType.toLowerCase()) {
      case 'balance':
        return i18n.t('wallet.layout.backgroundError.balance');
      case 'info':
        return i18n.t('wallet.layout.backgroundError.info');
      case 'transactions_warning':
        return i18n.t('wallet.layout.backgroundError.transactionsWarning');
      case 'transactions':
        return i18n.t('wallet.layout.backgroundError.transactions');
      default:
        return i18n.t('wallet.layout.backgroundError.generic');
    }
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
          : import('$lib/components/flows/GenericRequest/GenericRequestFlowHost.svelte'),
      ]);
      GenericRequestFlowHostComponent = flowHostModule.default;

      const nextSession = parseGenericRequestSession(input, passthroughAutoLinkFqn);
      const requestNetwork = nextSession.testnet ? 'testnet' : 'mainnet';
      if ((walletData.network ?? 'mainnet') !== requestNetwork) {
        throw new Error('genericRequest.import.error.networkMismatch');
      }

      const verification = await genericRequestService.verifyGenericRequestSignature(
        nextSession.requestHex
      );
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
      if (!message) return;

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
  <div
    class="absolute top-0 left-0 z-40 h-11 w-[15.25rem]"
    data-tauri-drag-region
    aria-hidden="true"
  ></div>
  <Sidebar.Provider class="h-full overflow-hidden">
    <AppSidebar
      {activeSection}
      {walletData}
      navigationDisabled={navigationLocked}
      onNavigate={navigateToSection}
      onOpenRequest={() => {
        if (navigationLocked) return;
        genericRequestImportError = '';
        genericRequestImportOpen = true;
      }}
    />
    <Sidebar.Inset class="h-full min-h-0 min-w-0 dark:bg-app-canvas">
      <div
        class={activeSection === 'address-book'
          ? 'absolute inset-x-0 top-0 z-40 h-6'
          : activeSection === 'identity' || activeSection === 'settings'
            ? 'absolute inset-x-0 top-0 z-40 h-5'
            : `${activeSection === 'overview' ? 'h-5' : 'h-6'} shrink-0`}
        data-tauri-drag-region
        aria-hidden="true"
      ></div>
      {#if latestError}
        <div class="pointer-events-none absolute right-6 bottom-6 left-6 z-50 flex justify-end">
          <Alert.Root
            variant={latestError.presentation.kind === 'background-update'
              ? 'default'
              : 'destructive'}
            class="pointer-events-auto max-w-lg bg-background/95 pr-12 shadow-lg backdrop-blur-sm dark:bg-popover/95"
          >
            <CircleAlertIcon aria-hidden="true" />
            <Alert.Title>{latestErrorTitle}</Alert.Title>
            <Alert.Description>{latestErrorMessage}</Alert.Description>
            <Button
              variant="ghost"
              size="icon-sm"
              class="absolute top-2 right-2 rounded-full text-muted-foreground hover:text-foreground"
              aria-label={i18n.t('wallet.layout.dismiss')}
              title={i18n.t('wallet.layout.dismiss')}
              onclick={dismissWalletError}
            >
              <XIcon aria-hidden="true" />
            </Button>
          </Alert.Root>
        </div>
      {/if}
      <main
        class={isTransferSection ||
        activeSection === 'overview' ||
        activeSection === 'identity' ||
        activeSection === 'watchlist' ||
        activeSection === 'address-book'
          ? 'flex min-h-0 flex-1 overflow-hidden'
          : 'min-h-0 flex-1 overflow-auto'}
      >
        {#if currentDraft}
          {#key currentDraft.id}
            <div
              bind:this={transferHost}
              onfocusin={(event) => {
                if (event.target instanceof HTMLElement) transferFocus = event.target;
              }}
              class="h-full min-h-0 min-w-0 flex-1"
              hidden={!isTransferSection}
              inert={!isTransferSection}
            >
              <TransferWizard
                active={isTransferSection && !genericRequestFlowOpen && !genericRequestImportOpen}
                entryIntent={currentDraft.intent}
                entryContext={currentDraft.context}
                recipientIntent={currentDraft.recipientIntent}
                walletNetwork={walletData.network ?? 'mainnet'}
                walletKey={transferWalletKey}
                onNavigationStateChange={(state) => {
                  transferNavigation = state;
                }}
                onClose={closeTransfer}
              />
            </div>
          {/key}
        {/if}
        {#if activeSection === 'overview'}
          {#if activeAssetDetailsEntry}
            <AssetDetails
              coinId={activeAssetDetailsEntry.coinId}
              walletEntryKind={activeAssetDetailsEntry.walletEntryKind}
              scopeFilterMode={activeAssetDetailsEntry.scopeFilterMode}
              entryDisplayName={activeAssetDetailsEntry.displayName}
              onBack={() => {
                activeAssetDetailsEntry = null;
              }}
              onNavigateToReceive={() => {
                activeSection = 'receive';
              }}
              onNavigateToSend={(context) => {
                requestTransfer('send', context);
              }}
              onNavigateToConvert={(context) => {
                requestTransfer('convert', context);
              }}
            />
          {:else}
            <Overview
              {walletData}
              onOpenAssetDetails={(entry) => {
                activeAssetDetailsEntry = entry;
              }}
              onNavigateToSend={() => {
                openOverviewTransfer('send');
              }}
              onNavigateToReceive={() => {
                activeSection = 'receive';
              }}
              onNavigateToConvert={() => {
                openOverviewTransfer('convert');
              }}
            />
          {/if}
        {:else if activeSection === 'receive'}
          <Receive />
        {:else if activeSection === 'identity'}
          {#key identitySectionWalletKey}
            <Identity
              walletNetwork={walletData.network ?? 'mainnet'}
              sessionState={identitySectionSession}
              onSessionStateChange={(nextState) => {
                identitySectionSession = nextState;
              }}
              navigationDisabled={navigationLocked}
              onReturnToContacts={(returnState) => {
                identitySectionSession = { ...identitySectionSession, publicProfile: null };
                contactReturnState = returnState;
                activeSection = 'address-book';
              }}
              onSend={(identity) => {
                requestTransfer('send', null, identity);
              }}
            />
          {/key}
        {:else if activeSection === 'apps'}
          <Apps />
        {:else if activeSection === 'activity'}
          <Activity />
        {:else if activeSection === 'watchlist'}
          {#key transferWalletKey}
            <Watchlist walletNetwork={walletData.network ?? 'mainnet'} />
          {/key}
        {:else if activeSection === 'address-book'}
          <AddressBook
            requestedIdentity={requestedContactIdentity}
            returnState={contactReturnState}
            profileNavigationDisabled={navigationLocked}
            restoreProfileFocus={Boolean(contactReturnState)}
            onViewProfile={(identity, returnState) => {
              if (navigationLocked) return;
              contactReturnState = returnState;
              identitySectionSession = {
                ...identitySectionSession,
                publicProfile: { identity, origin: { kind: 'contacts', returnState } },
              };
              activeSection = 'identity';
            }}
            onReturn={requestedContactIdentity && currentDraft ? resumeTransfer : undefined}
            returnLabel={requestedContactIdentity && currentDraft
              ? i18n.t(
                  transferNavigation.completed
                    ? 'wallet.transfer.returnToResult'
                    : transferNavigation.mode === 'convert'
                      ? 'wallet.transfer.resumeConvert'
                      : 'wallet.transfer.resumeSend'
                )
              : ''}
          />
        {:else if activeSection === 'settings'}
          {#key transferWalletKey}
            <Settings
              walletNetwork={walletData.network ?? 'mainnet'}
              walletName={walletData.name}
              walletSessionKey={transferWalletKey}
              resetSignal={settingsResetSignal}
            />
          {/key}
        {/if}
      </main>
    </Sidebar.Inset>
  </Sidebar.Provider>

  <Dialog.Root
    open={pendingTransfer !== null}
    onOpenChange={(open) => {
      if (!open) pendingTransfer = null;
    }}
  >
    <Dialog.Content
      class="max-w-md"
      onOpenAutoFocus={(event) => {
        event.preventDefault();
        resumeTransferButton?.focus();
      }}
      onCloseAutoFocus={(event) => {
        if (isTransferSection) event.preventDefault();
      }}
    >
      <Dialog.Header>
        <Dialog.Title>{i18n.t('wallet.transfer.existingDraftTitle')}</Dialog.Title>
        <Dialog.Description>{i18n.t('wallet.transfer.existingDraftDescription')}</Dialog.Description
        >
      </Dialog.Header>
      <Dialog.Footer>
        <Button
          variant="secondary"
          onclick={() => {
            if (pendingTransfer)
              startTransfer(
                pendingTransfer.intent,
                pendingTransfer.context,
                pendingTransfer.recipientIntent
              );
          }}
        >
          {i18n.t('wallet.transfer.replaceDraft')}
        </Button>
        <Button bind:ref={resumeTransferButton} onclick={resumeTransfer}
          >{i18n.t('wallet.transfer.resumeDraft')}</Button
        >
      </Dialog.Footer>
    </Dialog.Content>
  </Dialog.Root>

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
