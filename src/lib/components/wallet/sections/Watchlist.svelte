<script lang="ts">
  import { onMount, tick, untrack } from 'svelte';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import CheckIcon from '@lucide/svelte/icons/check';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';
  import InfoIcon from '@lucide/svelte/icons/info';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import { Spinner } from '$lib/components/ui/spinner';
  import InlineTextActionButton from '$lib/components/common/InlineTextActionButton.svelte';
  import NavigationBackButton from '$lib/components/common/NavigationBackButton.svelte';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import WalletEmptyState from '$lib/components/wallet/WalletEmptyState.svelte';
  import WatchlistAvatar from './WatchlistAvatar.svelte';
  import { Button } from '$lib/components/ui/button';
  import { CopyButton } from '$lib/components/ui/copy-button';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { i18nStore } from '$lib/i18n';
  import { contactChainId, matchingContacts } from '$lib/contacts/identity';
  import { addIdentityContact, loadContacts } from '$lib/contacts/service';
  import { contactSession } from '$lib/contacts/session';
  import { validateDestinationAddress } from '$lib/services/addressBookService';
  import type { ContactIdentity } from '$lib/types/addressBook';
  import { ratesStore } from '$lib/stores/rates.js';
  import { settingsStore } from '$lib/stores/settings.js';
  import * as watchlistService from '$lib/services/watchlistService.js';
  import type { WalletNetwork } from '$lib/types/wallet.js';
  import type {
    WatchlistEntry,
    WatchlistEntrySnapshot,
    WatchlistResolvedTarget,
  } from '$lib/types/watchlist.js';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import { extractWalletErrorType } from '$lib/utils/walletErrors.js';
  import {
    buildWatchlistEntryViewModel,
    mergeWatchlistSnapshot,
    type WatchlistRecord,
  } from '$lib/utils/watchlist.js';
  import {
    addWatchlistEntry,
    loadWatchlistEntries,
    refreshWatchlist,
    removeWatchlistEntry,
  } from '$lib/watchlist/service.js';
  import {
    watchlistEntriesStore,
    watchlistLoadState,
    watchlistSession,
  } from '$lib/watchlist/session.js';

  const {
    walletNetwork,
    onRetryWalletSession,
    initialSelectedEntryId = null,
    onCreateAddressContact,
    onViewIdentityProfile,
    onBackToList,
  }: {
    walletNetwork: WalletNetwork;
    onRetryWalletSession?: () => Promise<void>;
    initialSelectedEntryId?: string | null;
    onCreateAddressContact?: (entry: WatchlistEntry) => void;
    onViewIdentityProfile?: (entry: WatchlistEntry) => void;
    onBackToList?: () => void;
  } = $props();
  const i18n = $derived($i18nStore);
  const rates = $derived($ratesStore);
  const settings = $derived($settingsStore);

  const BALANCE_SKELETON_DELAY_MS = 240;

  let snapshots = $state<Record<string, WatchlistEntrySnapshot>>({});
  let loadError = $state('');
  let refreshError = $state('');
  let refreshing = $state(false);
  let showBalanceSkeleton = $state(false);
  let selectedEntryId = $state<string | null>(null);
  let appliedInitialEntryId: string | null = null;
  let contactState = $state<'checking' | 'ready' | 'duplicate' | 'saving' | 'saved' | 'error'>(
    'checking'
  );
  let contactError = $state<'load' | 'save'>('load');
  let contactGeneration = 0;
  let addViewOpen = $state(false);
  let addQuery = $state('');
  let addName = $state('');
  let resolvedTarget = $state<WatchlistResolvedTarget | null>(null);
  let resolveError = $state('');
  let resolving = $state(false);
  let adding = $state(false);
  let removeDialogOpen = $state(false);
  let removeError = $state('');
  let removing = $state(false);
  let mounted = false;
  let addGeneration = 0;
  /** Balance attempts already made per entry id, scoped to this component. */
  const attemptedEntryIds = new Set<string>();
  let recoveringWalletSession = $state(false);
  const copiedAddressState = new TimedValueState<string>();

  const records = $derived.by<WatchlistRecord[]>(() => {
    const entries = $watchlistEntriesStore;
    if (!entries) return [];
    return entries.map((entry) => {
      const snapshot = snapshots[entry.id];
      return snapshot
        ? { snapshot, stale: snapshot.availability !== 'available' }
        : { snapshot: placeholderSnapshot(entry), stale: true };
    });
  });
  const rows = $derived(
    records.map((record) => ({
      record,
      view: buildWatchlistEntryViewModel(record, rates, i18n.intlLocale, settings.displayCurrency),
    }))
  );
  const selectedRow = $derived(
    selectedEntryId ? (rows.find((row) => row.view.id === selectedEntryId) ?? null) : null
  );
  const selectedView = $derived(selectedRow?.view ?? null);
  const hasStaleData = $derived(records.some((record) => record.stale));
  const hasKnownBalances = $derived(records.some((record) => record.snapshot.holdings.length > 0));
  const hasPendingBalances = $derived(records.some((record) => isBalancePending(record.snapshot)));
  const hasUnsuccessfulBalances = $derived(
    records.some((record) => !isBalancePending(record.snapshot) && record.stale)
  );
  // Provider failures often resolve as unavailable/partial snapshots instead of
  // rejections, so Retry must follow the settled state, not only errors.
  const canRetryBalances = $derived(
    !refreshing && (Boolean(refreshError) || hasUnsuccessfulBalances)
  );
  const membershipError = $derived(
    Boolean(loadError) || ($watchlistLoadState === 'error' && $watchlistEntriesStore === null)
  );
  const membershipPending = $derived($watchlistEntriesStore === null && !membershipError);
  const listStatusKey = $derived(
    refreshError
      ? hasKnownBalances
        ? 'wallet.watchlist.updateUnavailable'
        : 'wallet.watchlist.updateUnavailableEmpty'
      : hasStaleData && !refreshing && !hasPendingBalances
        ? hasKnownBalances
          ? 'wallet.watchlist.updateUnavailable'
          : 'wallet.watchlist.updateUnavailableEmpty'
        : null
  );
  const detailStatusKey = $derived.by<string | null>(() => {
    if (!selectedView) return null;
    if (selectedRow && isBalancePending(selectedRow.record.snapshot)) {
      return refreshError
        ? 'wallet.watchlist.updateUnavailableEmpty'
        : 'wallet.watchlist.loadingBalances';
    }
    if (refreshError) {
      return selectedView.holdings.length > 0
        ? 'wallet.watchlist.updateUnavailable'
        : 'wallet.watchlist.updateUnavailableEmpty';
    }
    if (!selectedView.stale) return null;
    return selectedView.holdings.length > 0
      ? 'wallet.watchlist.updateUnavailable'
      : 'wallet.watchlist.updateUnavailableEmpty';
  });
  const detailBalancePending = $derived(
    Boolean(selectedRow && isBalancePending(selectedRow.record.snapshot) && !refreshError)
  );

  $effect(() => {
    if (initialSelectedEntryId && initialSelectedEntryId !== appliedInitialEntryId) {
      selectedEntryId = initialSelectedEntryId;
      appliedInitialEntryId = initialSelectedEntryId;
    } else if (!initialSelectedEntryId) {
      appliedInitialEntryId = null;
    }
  });

  $effect(() => {
    const entryId = selectedEntryId;
    const session = $contactSession;
    const entry = $watchlistEntriesStore?.find((candidate) => candidate.id === entryId);
    if (!entry) return;
    untrack(() => void checkContact(entry, session));
  });

  function entryIdentity(entry: WatchlistEntry): ContactIdentity {
    return {
      identityAddress: entry.address,
      fullyQualifiedName: entry.displayName,
      network: walletNetwork,
      chainId: contactChainId(walletNetwork),
    };
  }

  async function checkContact(
    entry: WatchlistEntry,
    session: typeof $contactSession = $contactSession
  ): Promise<void> {
    const generation = ++contactGeneration;
    contactState = 'checking';
    if (!session || session.network !== walletNetwork) {
      contactError = 'load';
      contactState = 'error';
      return;
    }
    try {
      // The backend supplies the canonical normalized VRPC address. Base58
      // addresses are case-sensitive, so compare that value without case folding.
      const normalized = await validateDestinationAddress({ kind: 'vrpc', address: entry.address });
      if (!normalized.valid || !normalized.normalizedAddress)
        throw new Error('Invalid watchlist contact address');
      const contacts = await loadContacts(true);
      if (!mounted || generation !== contactGeneration || $contactSession !== session) return;
      const identityMatches =
        entry.targetKind === 'identity' ? matchingContacts(contacts, entryIdentity(entry)) : [];
      const matches = contacts.filter(
        (contact) =>
          identityMatches.includes(contact) ||
          contact.endpoints.some(
            (endpoint) =>
              endpoint.kind === 'vrpc' &&
              endpoint.normalizedAddress === normalized.normalizedAddress
          )
      );
      contactState = matches.length ? 'duplicate' : 'ready';
    } catch {
      if (!mounted || generation !== contactGeneration) return;
      contactError = 'load';
      contactState = 'error';
    }
  }

  async function activateContact(): Promise<void> {
    const entry = selectedRow?.record.snapshot.entry;
    if (!entry || contactState === 'checking' || contactState === 'saving') return;
    if (contactState === 'duplicate' || contactState === 'saved') return;
    if (contactState === 'error') {
      if (contactError === 'load') await checkContact(entry);
      else await saveIdentityContact(entry);
      return;
    }
    if (entry.targetKind === 'address') onCreateAddressContact?.(entry);
    else await saveIdentityContact(entry);
  }

  async function saveIdentityContact(entry: WatchlistEntry): Promise<void> {
    if (entry.targetKind !== 'identity') return;
    const session = $contactSession;
    if (!session || session.network !== walletNetwork) {
      contactError = 'load';
      contactState = 'error';
      return;
    }
    const generation = ++contactGeneration;
    contactState = 'saving';
    try {
      await addIdentityContact(entryIdentity(entry));
      if (!mounted || generation !== contactGeneration || $contactSession !== session) return;
      contactState = 'saved';
    } catch {
      if (!mounted || generation !== contactGeneration) return;
      contactError = 'save';
      contactState = 'error';
    }
  }

  $effect(() => {
    let timer: ReturnType<typeof setTimeout> | null = null;
    showBalanceSkeleton = false;
    if (!refreshError && hasPendingBalances) {
      timer = setTimeout(() => (showBalanceSkeleton = true), BALANCE_SKELETON_DELAY_MS);
    }
    return () => {
      if (timer) clearTimeout(timer);
    };
  });

  // Membership can grow after the initial hydrate (for example an add that
  // finished after the user left and re-entered). Refresh entries that no
  // covering request has attempted yet instead of leaving their balance fields
  // loading forever. Attempts are tracked against the shared request that
  // actually covered them, so joining an older request cannot suppress the
  // follow-up refresh for a newer entry.
  $effect(() => {
    if (membershipPending || membershipError || records.length === 0) return;
    if (!hasPendingBalances || refreshing || refreshError) return;
    const hasUnattemptedPending = records.some(
      (record) =>
        isBalancePending(record.snapshot) && !attemptedEntryIds.has(record.snapshot.entry.id)
    );
    if (!hasUnattemptedPending) return;
    untrack(() => void refreshAll());
  });

  function placeholderSnapshot(entry: WatchlistEntry): WatchlistEntrySnapshot {
    return { entry, holdings: [], sources: [], availability: 'unavailable', refreshedAt: 0 };
  }

  function isBalancePending(snapshot: WatchlistEntrySnapshot): boolean {
    return snapshot.refreshedAt === 0 && snapshot.holdings.length === 0;
  }

  // A confirmed empty response may show zero currencies; unavailable or partial
  // responses must not claim zero before a refresh verifies them.
  function showsCurrencyCount(snapshot: WatchlistEntrySnapshot): boolean {
    return snapshot.availability === 'available' || snapshot.holdings.length > 0;
  }

  function currencyCountLabel(count: number): string {
    return i18n.t(
      count === 1 ? 'wallet.watchlist.currencyCountOne' : 'wallet.watchlist.currencyCount',
      {
        count,
      }
    );
  }

  function translateError(error: unknown, fallbackKey: string): string {
    switch (extractWalletErrorType(error)) {
      case 'WatchlistInvalidInput':
        return i18n.t('wallet.watchlist.error.invalidInput');
      case 'IdentityNotFound':
        return i18n.t('wallet.watchlist.error.notFound');
      case 'WatchlistDuplicate':
        return i18n.t('wallet.watchlist.error.duplicate');
      default:
        return i18n.t(fallbackKey);
    }
  }

  // An unbound Watchlist cannot load membership on its own. Ask the route to
  // recover the verified wallet session first, then retry the membership read.
  async function retryMembership(): Promise<void> {
    if (recoveringWalletSession) return;
    if (!$watchlistSession && onRetryWalletSession) {
      recoveringWalletSession = true;
      try {
        await onRetryWalletSession();
      } catch {
        // The route owns metadata recovery errors; keep the guarded failure.
      } finally {
        recoveringWalletSession = false;
      }
    }
    if ($watchlistSession) await hydrate(true);
  }

  async function hydrate(force = false): Promise<void> {
    loadError = '';
    try {
      const entries = await loadWatchlistEntries(force);
      if (!mounted) return;
      loadError = '';
      if (entries.length > 0) await refreshAll();
    } catch (error) {
      if (!mounted) return;
      loadError = translateError(error, 'wallet.watchlist.error.loadFailed');
    }
  }

  function applyRefreshSnapshots(
    entries: WatchlistEntrySnapshot[],
    refreshedAt: number,
    requestedEntryIds: Set<string>
  ): void {
    const next = { ...snapshots };
    for (const snapshot of entries) {
      next[snapshot.entry.id] = mergeWatchlistSnapshot(next[snapshot.entry.id], snapshot);
    }
    // A completed refresh that did not report one of its own requested entries
    // must not leave that row loading forever. Entries added after the request
    // started stay pending and are fetched by the follow-up refresh.
    for (const record of records) {
      const id = record.snapshot.entry.id;
      if (next[id] || !isBalancePending(record.snapshot)) continue;
      if (!requestedEntryIds.has(id)) continue;
      next[id] = { ...record.snapshot, refreshedAt };
    }
    snapshots = next;
  }

  async function refreshAll(): Promise<void> {
    if (refreshing) return;
    refreshing = true;
    refreshError = '';
    try {
      const { result, requestedEntryIds } = await refreshWatchlist();
      // Only entries covered by this shared request count as attempted; entries
      // added after it started stay eligible for the follow-up refresh.
      for (const id of requestedEntryIds) attemptedEntryIds.add(id);
      if (!mounted) return;
      if (result.network !== walletNetwork) {
        refreshError = i18n.t('wallet.watchlist.error.refreshFailed');
        return;
      }
      applyRefreshSnapshots(result.entries, result.refreshedAt, new Set(requestedEntryIds));
    } catch (error) {
      if (!mounted) return;
      refreshError = translateError(error, 'wallet.watchlist.error.refreshFailed');
    } finally {
      if (mounted) refreshing = false;
    }
  }

  function openAddView(): void {
    addGeneration++;
    addQuery = '';
    addName = '';
    resolvedTarget = null;
    resolveError = '';
    resolving = false;
    adding = false;
    addViewOpen = true;
    void tick().then(() => document.getElementById('watchlist-target')?.focus());
  }

  function closeAddView(): void {
    if (adding) return;
    addGeneration++;
    addViewOpen = false;
    resolving = false;
  }

  function updateAddQuery(value: string): void {
    addGeneration++;
    addQuery = value;
    resolvedTarget = null;
    resolveError = '';
    resolving = false;
  }

  async function resolveTarget(): Promise<void> {
    const query = addQuery.trim();
    if (!query) {
      resolveError = i18n.t('wallet.watchlist.error.inputRequired');
      return;
    }
    const generation = ++addGeneration;
    resolving = true;
    resolveError = '';
    try {
      const result = await watchlistService.resolveWatchlistTarget(query);
      if (!mounted || !addViewOpen || generation !== addGeneration) return;
      resolvedTarget = result;
    } catch (error) {
      if (!mounted || !addViewOpen || generation !== addGeneration) return;
      resolvedTarget = null;
      resolveError = translateError(error, 'wallet.watchlist.error.resolveFailed');
    } finally {
      if (mounted && generation === addGeneration) resolving = false;
    }
  }

  async function addResolvedTarget(): Promise<void> {
    if (!resolvedTarget || adding) return;
    const generation = addGeneration;
    const query = addQuery.trim();
    const name = resolvedTarget.targetKind === 'address' ? addName.trim() : undefined;
    adding = true;
    resolveError = '';
    try {
      const snapshot = await addWatchlistEntry(query, name || undefined);
      if (!mounted || !addViewOpen || generation !== addGeneration) return;
      snapshots = { ...snapshots, [snapshot.entry.id]: snapshot };
      addGeneration++;
      addViewOpen = false;
      addQuery = '';
      resolvedTarget = null;
      loadError = '';
    } catch (error) {
      if (!mounted || !addViewOpen || generation !== addGeneration) return;
      resolveError = translateError(error, 'wallet.watchlist.error.addFailed');
    } finally {
      if (mounted) adding = false;
    }
  }

  async function backToList(): Promise<void> {
    contactGeneration++;
    const rowId = selectedEntryId;
    selectedEntryId = null;
    onBackToList?.();
    await tick();
    const row = [
      ...document.querySelectorAll<HTMLButtonElement>('[data-testid="watchlist-entry"]'),
    ].find((element) => element.dataset.entryId === rowId);
    row?.focus();
  }

  async function copyAddress(address: string): Promise<void> {
    if (await writeClipboardText(address)) copiedAddressState.set(address);
  }

  function requestRemove(): void {
    removeError = '';
    removeDialogOpen = true;
  }

  async function confirmRemove(): Promise<void> {
    if (!selectedEntryId || removing) return;
    removing = true;
    removeError = '';
    try {
      await removeWatchlistEntry(selectedEntryId);
      if (!mounted) return;
      selectedEntryId = null;
      removeDialogOpen = false;
    } catch (error) {
      if (mounted) removeError = translateError(error, 'wallet.watchlist.error.removeFailed');
    } finally {
      if (mounted) removing = false;
    }
  }

  onMount(() => {
    mounted = true;
    void hydrate();
    return () => {
      mounted = false;
      addGeneration++;
      contactGeneration++;
    };
  });
</script>

<div
  class="flex min-h-0 w-full flex-1 flex-col"
  data-testid="watchlist-section"
  aria-busy={membershipPending ? 'true' : undefined}
>
  {#if addViewOpen}
    <form
      class="flex min-h-0 flex-1 flex-col px-5 pt-5 pb-5"
      onsubmit={(event) => {
        event.preventDefault();
        if (resolvedTarget) void addResolvedTarget();
        else void resolveTarget();
      }}
    >
      <NavigationBackButton
        label={i18n.t('wallet.watchlist.back')}
        tone="settings"
        class="mb-5 self-start"
        disabled={adding}
        onclick={closeAddView}
      />
      <h2 class="mb-5 text-xl font-semibold tracking-tight">
        {i18n.t('wallet.watchlist.addAddress')}
      </h2>
      <div class="space-y-2">
        <Label
          for="watchlist-target"
          class="text-[13px] font-normal text-settings-muted-foreground"
        >
          {i18n.t('wallet.watchlist.addSheet.inputLabel')}
        </Label>
        <Input
          id="watchlist-target"
          value={addQuery}
          oninput={(event) => updateAddQuery(event.currentTarget.value)}
          placeholder={i18n.t('wallet.watchlist.addSheet.inputPlaceholder')}
          autocomplete="off"
          spellcheck="false"
          disabled={adding}
          aria-invalid={Boolean(resolveError)}
          aria-describedby={resolveError ? 'watchlist-add-error' : undefined}
        />
        {#if resolveError}
          <p id="watchlist-add-error" class="text-xs text-destructive" role="alert">
            {resolveError}
          </p>
        {/if}
      </div>

      {#if resolving}
        <div
          class="mt-5 flex items-center gap-2 border-t border-border/60 pt-5 text-sm text-settings-muted-foreground"
          role="status"
        >
          <Spinner class="size-4" />
          {i18n.t('wallet.watchlist.addSheet.resolving')}
        </div>
      {:else if resolvedTarget}
        <div
          class="mt-5 rounded-xl border border-border/60 bg-settings-surface/50 px-4 py-4"
          data-testid="watchlist-resolved-preview"
        >
          <p class="mb-3 text-[13px] font-medium text-primary">
            {i18n.t(
              resolvedTarget.targetKind === 'identity'
                ? 'wallet.watchlist.addSheet.identityFound'
                : 'wallet.watchlist.addSheet.addressFound'
            )}
          </p>
          <div class="flex min-w-0 items-center gap-3">
            <WatchlistAvatar
              address={resolvedTarget.address}
              displayName={resolvedTarget.displayName}
              targetKind={resolvedTarget.targetKind}
              network={walletNetwork}
              class="size-10"
            />
            <div class="min-w-0">
              {#if resolvedTarget.targetKind === 'identity'}
                <p class="truncate text-sm font-semibold">{resolvedTarget.displayName}</p>
                <IdentifierText
                  value={resolvedTarget.address}
                  mode="review"
                  class="block truncate text-xs text-settings-muted-foreground"
                />
              {:else}
                <IdentifierText
                  value={resolvedTarget.address}
                  mode="review"
                  class="block truncate text-sm font-medium"
                />
              {/if}
            </div>
          </div>
        </div>
        {#if resolvedTarget.targetKind === 'address'}
          <div class="mt-5 space-y-2">
            <Label
              for="watchlist-name"
              class="text-[13px] font-normal text-settings-muted-foreground"
            >
              {i18n.t('wallet.watchlist.addSheet.nameOptional')}
            </Label>
            <Input
              id="watchlist-name"
              bind:value={addName}
              maxlength={80}
              placeholder={i18n.t('wallet.watchlist.addSheet.namePlaceholder')}
              autocomplete="off"
              disabled={adding}
            />
          </div>
        {/if}
        <p class="mt-5 flex items-start gap-2 text-[13px] text-settings-muted-foreground">
          <InfoIcon class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
          {i18n.t('wallet.watchlist.addSheet.publicInfo')}
        </p>
      {/if}

      <footer class="mt-auto flex shrink-0 justify-end gap-2 pt-6">
        <Button type="button" variant="secondary" disabled={adding} onclick={closeAddView}>
          {i18n.t('common.cancel')}
        </Button>
        <Button type="submit" disabled={adding || resolving || !addQuery.trim()}>
          {#if adding}<Spinner class="size-3.5" />{/if}
          {i18n.t(
            adding
              ? 'wallet.watchlist.addSheet.adding'
              : resolvedTarget
                ? 'wallet.watchlist.addSheet.add'
                : 'common.continue'
          )}
        </Button>
      </footer>
    </form>
  {:else if selectedView}
    <div class="flex min-h-0 flex-1 flex-col px-5 pt-5">
      <div class="mb-4 flex h-9 shrink-0 items-start justify-between">
        <NavigationBackButton
          label={i18n.t('wallet.watchlist.back')}
          tone="settings"
          onclick={() => void backToList()}
        />
        <InlineTextActionButton
          tone="destructive"
          class="h-8 px-1 text-[13px]"
          onclick={requestRemove}
        >
          {i18n.t('wallet.watchlist.remove')}
        </InlineTextActionButton>
      </div>
      <header class="flex shrink-0 items-center gap-4 pb-5">
        <WatchlistAvatar
          address={selectedView.address}
          displayName={selectedView.displayName}
          targetKind={selectedView.targetKind}
          network={walletNetwork}
        />
        <div class="min-w-0 flex-1">
          <h2 class="truncate text-xl leading-7 font-semibold tracking-tight">
            {selectedView.displayName}
          </h2>
          {#if selectedView.displayName !== selectedView.address}
            <div class="mt-1 flex min-w-0 items-center gap-1">
              <IdentifierText
                value={selectedView.address}
                mode="review"
                class="min-w-0 text-xs text-settings-muted-foreground"
              />
              <CopyButton
                size="xs"
                copied={copiedAddressState.current === selectedView.address}
                onclick={() => void copyAddress(selectedView.address)}
                aria-label={i18n.t('common.copy')}
                title={i18n.t('common.copy')}
              />
            </div>
          {/if}
        </div>
      </header>
      <div
        class="mb-5 flex shrink-0 flex-col items-start gap-1.5"
        data-testid="watchlist-contact-action"
      >
        <div class="flex flex-wrap items-center gap-2">
          <Button
            size="sm"
            variant="secondary"
            class="gap-1.5 {contactState === 'duplicate' || contactState === 'saved'
              ? 'bg-contact-saved text-contact-saved-foreground hover:bg-contact-saved disabled:cursor-default disabled:opacity-100 disabled:hover:bg-contact-saved'
              : ''}"
            disabled={contactState === 'checking' ||
              contactState === 'saving' ||
              contactState === 'duplicate' ||
              contactState === 'saved'}
            aria-busy={contactState === 'checking' || contactState === 'saving'}
            onclick={() => void activateContact()}
          >
            {#if contactState === 'checking' || contactState === 'saving'}
              <Spinner class="size-3.5" />
            {:else if contactState === 'duplicate' || contactState === 'saved'}
              <CheckIcon class="size-3.5" aria-hidden="true" />
            {/if}
            {i18n.t(
              contactState === 'checking'
                ? 'wallet.identity.publicProfile.checkingContacts'
                : contactState === 'saving'
                  ? 'wallet.addressBook.form.saving'
                  : contactState === 'duplicate' || contactState === 'saved'
                    ? 'wallet.identity.publicProfile.inContacts'
                    : contactState === 'error'
                      ? 'wallet.contacts.retry'
                      : 'wallet.contacts.add'
            )}
          </Button>
          {#if selectedRow?.record.snapshot.entry.targetKind === 'identity'}
            <Button
              size="sm"
              variant="secondary"
              onclick={() => onViewIdentityProfile?.(selectedRow.record.snapshot.entry)}
            >
              {i18n.t('wallet.identity.lookup.viewProfile')}
            </Button>
          {/if}
        </div>
        {#if contactState === 'error'}
          <p class="text-xs text-destructive" role="alert">
            {i18n.t(
              contactError === 'save' ? 'wallet.contacts.saveFailed' : 'wallet.contacts.loadFailed'
            )}
          </p>
        {/if}
      </div>
      <ScrollArea.Root class="min-h-0 flex-1">
        <ScrollArea.Viewport>
          <div class="pb-8" aria-busy={detailBalancePending ? 'true' : undefined}>
            {#if detailStatusKey}
              <div class="mb-3 flex items-center gap-2 text-xs text-settings-muted-foreground">
                <p role="status" data-testid="watchlist-detail-status">
                  {i18n.t(detailStatusKey)}
                </p>
                {#if canRetryBalances}
                  <InlineTextActionButton tone="muted" onclick={() => void refreshAll()}>
                    {i18n.t('wallet.watchlist.retry')}
                  </InlineTextActionButton>
                {/if}
              </div>
            {/if}
            <section
              class="rounded-xl bg-settings-surface px-5 py-5"
              data-testid="public-value-card"
              aria-busy={detailBalancePending ? 'true' : undefined}
            >
              <p class="text-xs font-medium text-settings-muted-foreground">
                {i18n.t('wallet.watchlist.publicValue')}
              </p>
              {#if detailBalancePending && showBalanceSkeleton}
                <div
                  class="mt-1 flex h-9 items-center"
                  aria-label={i18n.t('wallet.loading.balancePending')}
                >
                  <Skeleton class="h-8 w-40 rounded-md motion-reduce:animate-none" />
                </div>
              {:else if detailBalancePending}
                <div class="mt-1 h-9" aria-hidden="true"></div>
              {:else}
                <p class="mt-1 text-[30px] leading-9 font-semibold tracking-tight">
                  {selectedView.publicValueDisplay}
                </p>
              {/if}
            </section>
            {#if detailBalancePending}
              <div
                class="mt-5 overflow-hidden rounded-xl border border-border/60"
                data-testid="watchlist-detail-skeleton"
                aria-hidden="true"
              >
                {#if showBalanceSkeleton}
                  {#each [0, 1, 2] as row (row)}
                    <div
                      class="flex min-h-[70px] items-center gap-3 px-4 py-3 {row > 0
                        ? 'border-t border-border/60'
                        : ''}"
                    >
                      <Skeleton class="size-8 shrink-0 rounded-full motion-reduce:animate-none" />
                      <div class="min-w-0 flex-1">
                        <Skeleton class="h-3.5 w-24 rounded-sm motion-reduce:animate-none" />
                        <Skeleton class="mt-2 h-3 w-12 rounded-sm motion-reduce:animate-none" />
                      </div>
                      <Skeleton class="h-4 w-16 rounded-sm motion-reduce:animate-none" />
                    </div>
                  {/each}
                {:else}
                  <div class="h-[212px]"></div>
                {/if}
              </div>
            {:else if selectedView.holdings.length === 0}
              {#if !selectedView.stale}
                <div
                  class="mt-5 rounded-xl border border-border/60 px-5 py-8 text-center text-sm text-settings-muted-foreground"
                >
                  {i18n.t('wallet.watchlist.noCurrencies')}
                </div>
              {/if}
            {:else}
              <div class="mt-5 overflow-hidden rounded-xl border border-border/60">
                {#each selectedView.holdings as holding, index (holding.key)}
                  <div
                    class="flex min-h-[70px] items-center gap-3 px-4 py-3 {index > 0
                      ? 'border-t border-border/60'
                      : ''}"
                  >
                    <CoinIcon
                      coinId={holding.coinId}
                      coinName={holding.name}
                      proto="vrsc"
                      size={32}
                      decorative
                    />
                    <div class="min-w-0 flex-1">
                      <p class="truncate text-sm font-medium">{holding.name}</p>
                      <p class="truncate text-xs text-settings-muted-foreground">
                        {holding.ticker}
                      </p>
                    </div>
                    <p class="shrink-0 text-sm font-medium tabular-nums">
                      {holding.balanceDisplay}
                      {holding.ticker}
                    </p>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>
    </div>
  {:else if membershipError}
    <div class="flex flex-1 flex-col items-center justify-center px-8 text-center">
      <CircleAlertIcon class="size-7 text-settings-muted-foreground" aria-hidden="true" />
      <p class="mt-3 text-sm font-medium">
        {loadError || i18n.t('wallet.watchlist.error.loadFailed')}
      </p>
      <Button
        variant="secondary"
        size="sm"
        class="mt-4"
        disabled={recoveringWalletSession}
        onclick={() => void retryMembership()}
      >
        {i18n.t('wallet.watchlist.retry')}
      </Button>
    </div>
  {:else if membershipPending}
    <div class="min-h-0 flex-1" data-testid="watchlist-membership-pending" aria-hidden="true"></div>
  {:else if records.length === 0}
    <WalletEmptyState
      illustration="watch-list"
      eyebrow={i18n.t('wallet.empty.encrypted')}
      title={i18n.t('wallet.watchlist.emptyTitle')}
      actionLabel={i18n.t('wallet.watchlist.addAddress')}
      onAction={openAddView}
      testId="watchlist-empty"
    />
  {:else}
    <div class="flex min-h-0 flex-1 flex-col px-5">
      <div class="flex h-[72px] shrink-0 items-center justify-between gap-3">
        <div class="flex min-w-0 items-center gap-2">
          <span class="truncate text-xs text-settings-muted-foreground" aria-live="polite">
            {#if listStatusKey}{i18n.t(listStatusKey)}{/if}
          </span>
          {#if canRetryBalances}
            <InlineTextActionButton class="shrink-0" tone="muted" onclick={() => void refreshAll()}>
              {i18n.t('wallet.watchlist.retry')}
            </InlineTextActionButton>
          {/if}
        </div>
        <Button size="sm" class="shrink-0" onclick={openAddView}>
          <PlusIcon class="size-3.5" aria-hidden="true" />
          {i18n.t('wallet.watchlist.addAddress')}
        </Button>
      </div>
      <ScrollArea.Root class="min-h-0 flex-1">
        <ScrollArea.Viewport>
          <div class="pb-8" aria-busy={hasPendingBalances && !refreshError ? 'true' : undefined}>
            {#each rows as row, index (row.view.id)}
              <button
                type="button"
                class="row-hover-fade group/watch-row flex min-h-[88px] w-full items-center gap-3.5 px-2 text-left outline-none focus-visible:rounded-sm focus-visible:ring-2 focus-visible:ring-settings-focus-ring {index <
                rows.length - 1
                  ? 'border-b border-border/60'
                  : ''}"
                data-testid="watchlist-entry"
                data-entry-id={row.view.id}
                onclick={() => (selectedEntryId = row.view.id)}
              >
                <WatchlistAvatar
                  address={row.view.address}
                  displayName={row.view.displayName}
                  targetKind={row.view.targetKind}
                  network={walletNetwork}
                />
                <div class="min-w-0 flex-1">
                  <p class="truncate text-[15px] font-semibold">{row.view.displayName}</p>
                  {#if row.view.targetKind === 'address' && row.view.displayName !== row.view.address}
                    <IdentifierText
                      value={row.view.address}
                      mode="review"
                      class="mt-0.5 block truncate text-xs text-settings-muted-foreground"
                    />
                  {/if}
                </div>
                <div class="shrink-0 text-right">
                  {#if isBalancePending(row.record.snapshot) && !refreshError}
                    {#if showBalanceSkeleton}
                      <div
                        class="flex h-5 items-center justify-end"
                        aria-label={i18n.t('wallet.loading.balancePending')}
                      >
                        <Skeleton class="h-3.5 w-16 rounded-sm motion-reduce:animate-none" />
                      </div>
                      <div class="mt-0.5 flex h-4 items-center justify-end" aria-hidden="true">
                        <Skeleton class="h-3 w-20 rounded-sm motion-reduce:animate-none" />
                      </div>
                    {:else}
                      <div class="h-5" aria-hidden="true"></div>
                      <div class="mt-0.5 h-4" aria-hidden="true"></div>
                    {/if}
                  {:else}
                    <p class="text-sm font-semibold tabular-nums">
                      {row.view.publicValueDisplay}
                    </p>
                    {#if showsCurrencyCount(row.record.snapshot)}
                      <p class="mt-0.5 text-xs text-settings-muted-foreground">
                        {currencyCountLabel(row.view.currencyCount)}
                      </p>
                    {/if}
                  {/if}
                </div>
                <ChevronRightIcon
                  class="size-4 shrink-0 text-muted-foreground/70 transition-colors group-hover/watch-row:text-foreground group-focus-visible/watch-row:text-foreground"
                  aria-hidden="true"
                />
              </button>
            {/each}
          </div>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>
    </div>
  {/if}
</div>

<Dialog.Root
  open={removeDialogOpen}
  onOpenChange={(open) => {
    if (!open && !removing) removeDialogOpen = false;
  }}
>
  <Dialog.Content class="max-w-md" showCloseButton={!removing}>
    <Dialog.Header>
      <Dialog.Title>{i18n.t('wallet.watchlist.removeTitle')}</Dialog.Title>
      <Dialog.Description>{i18n.t('wallet.watchlist.removeDescription')}</Dialog.Description>
    </Dialog.Header>
    {#if removeError}<p class="text-sm text-destructive" role="alert">{removeError}</p>{/if}
    <Dialog.Footer class="flex justify-end gap-3">
      <Button variant="secondary" disabled={removing} onclick={() => (removeDialogOpen = false)}>
        {i18n.t('common.cancel')}
      </Button>
      <Button variant="destructive" disabled={removing} onclick={confirmRemove}>
        {i18n.t(removing ? 'wallet.watchlist.removing' : 'wallet.watchlist.remove')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
