<script lang="ts">
  import { onMount, tick } from 'svelte';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';
  import InfoIcon from '@lucide/svelte/icons/info';
  import MoreHorizontalIcon from '@lucide/svelte/icons/ellipsis';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import { Spinner } from '$lib/components/ui/spinner';
  import NavigationBackButton from '$lib/components/common/NavigationBackButton.svelte';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import WalletEmptyState from '$lib/components/wallet/WalletEmptyState.svelte';
  import WatchlistAvatar from './WatchlistAvatar.svelte';
  import { Button } from '$lib/components/ui/button';
  import { CopyButton } from '$lib/components/ui/copy-button';
  import * as Dialog from '$lib/components/ui/dialog';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { i18nStore } from '$lib/i18n';
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
    mergeWatchlistSnapshots,
    type WatchlistRecord,
  } from '$lib/utils/watchlist.js';

  const { walletNetwork }: { walletNetwork: WalletNetwork } = $props();
  const i18n = $derived($i18nStore);
  const rates = $derived($ratesStore);
  const settings = $derived($settingsStore);

  let records = $state<WatchlistRecord[]>([]);
  let initialLoading = $state(true);
  let loadError = $state('');
  let refreshError = $state('');
  let refreshing = $state(false);
  let selectedEntryId = $state<string | null>(null);
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
  const copiedAddressState = new TimedValueState<string>();

  const viewModels = $derived(
    records.map((record) =>
      buildWatchlistEntryViewModel(record, rates, i18n.intlLocale, settings.displayCurrency)
    )
  );
  const selectedView = $derived(
    selectedEntryId ? (viewModels.find((entry) => entry.id === selectedEntryId) ?? null) : null
  );
  const hasStaleData = $derived(records.some((record) => record.stale));
  const hasKnownBalances = $derived(records.some((record) => record.snapshot.holdings.length > 0));
  const detailStatusKey = $derived(
    selectedView?.stale
      ? refreshing && selectedView.refreshedAt === 0
        ? 'wallet.watchlist.loadingBalances'
        : selectedView.holdings.length > 0
          ? 'wallet.watchlist.updateUnavailable'
          : 'wallet.watchlist.updateUnavailableEmpty'
      : null
  );

  function placeholderSnapshot(entry: WatchlistEntry): WatchlistEntrySnapshot {
    return { entry, holdings: [], sources: [], availability: 'unavailable', refreshedAt: 0 };
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

  async function hydrate(): Promise<void> {
    initialLoading = true;
    loadError = '';
    try {
      const entries = await watchlistService.getWatchlistEntries();
      if (!mounted) return;
      records = entries.map((entry) => ({ snapshot: placeholderSnapshot(entry), stale: true }));
      initialLoading = false;
      if (entries.length > 0) await refreshAll();
    } catch (error) {
      if (!mounted) return;
      loadError = translateError(error, 'wallet.watchlist.error.loadFailed');
      initialLoading = false;
    }
  }

  async function refreshAll(): Promise<void> {
    if (refreshing) return;
    refreshing = true;
    refreshError = '';
    try {
      const result = await watchlistService.refreshWatchlist();
      if (!mounted) return;
      records = mergeWatchlistSnapshots(records, result.entries);
    } catch (error) {
      if (!mounted) return;
      records = records.map((record) => ({ ...record, stale: true }));
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
      const snapshot = await watchlistService.addWatchlistEntry(query, name || undefined);
      if (!mounted || !addViewOpen || generation !== addGeneration) return;
      records = mergeWatchlistSnapshots(records, [
        snapshot,
        ...records.map((record) => record.snapshot),
      ]);
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
    const rowId = selectedEntryId;
    selectedEntryId = null;
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
      await watchlistService.removeWatchlistEntry(selectedEntryId);
      if (!mounted) return;
      records = records.filter((record) => record.snapshot.entry.id !== selectedEntryId);
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
    };
  });
</script>

<div class="flex min-h-0 w-full flex-1 flex-col" data-testid="watchlist-section">
  {#if addViewOpen}
    <form
      class="flex min-h-0 flex-1 flex-col px-5 pt-5 pb-7"
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
      <NavigationBackButton
        label={i18n.t('wallet.watchlist.back')}
        tone="settings"
        class="mb-5 self-start"
        onclick={() => void backToList()}
      />
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
        <DropdownMenu.Root>
          <DropdownMenu.Trigger
            class="flex size-8 shrink-0 items-center justify-center rounded-md text-settings-muted-foreground transition-colors outline-none hover:bg-accent hover:text-foreground focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
            aria-label={i18n.t('wallet.watchlist.moreActions')}
          >
            <MoreHorizontalIcon class="size-4" />
          </DropdownMenu.Trigger>
          <DropdownMenu.Content align="end" class="w-52">
            <DropdownMenu.Item variant="destructive" onclick={requestRemove}>
              {i18n.t('wallet.watchlist.remove')}
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </header>
      <ScrollArea.Root class="min-h-0 flex-1">
        <ScrollArea.Viewport>
          <div class="pb-8">
            {#if detailStatusKey}
              <p
                class="mb-3 text-xs text-settings-muted-foreground"
                role="status"
                data-testid="watchlist-detail-status"
              >
                {i18n.t(detailStatusKey)}
              </p>
            {/if}
            <section
              class="rounded-xl bg-settings-surface px-5 py-5"
              data-testid="public-value-card"
            >
              <p class="text-xs font-medium text-settings-muted-foreground">
                {i18n.t('wallet.watchlist.publicValue')}
              </p>
              <p class="mt-1 text-[30px] leading-9 font-semibold tracking-tight">
                {selectedView.publicValueDisplay}
              </p>
            </section>
            {#if selectedView.holdings.length === 0}
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
  {:else if initialLoading}
    <div
      class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 px-8 text-center"
      data-testid="watchlist-loading"
      role="status"
    >
      <Spinner class="size-5 text-settings-muted-foreground" />
      <p class="text-sm text-settings-muted-foreground">{i18n.t('wallet.watchlist.loading')}</p>
    </div>
  {:else if loadError && records.length === 0}
    <div class="flex flex-1 flex-col items-center justify-center px-8 text-center">
      <CircleAlertIcon class="size-7 text-settings-muted-foreground" aria-hidden="true" />
      <p class="mt-3 text-sm font-medium">{loadError}</p>
      <Button variant="secondary" size="sm" class="mt-4" onclick={() => void hydrate()}>
        {i18n.t('wallet.watchlist.retry')}
      </Button>
    </div>
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
      <div class="flex h-[74px] shrink-0 items-center justify-between gap-3">
        <span class="text-xs text-settings-muted-foreground" aria-live="polite">
          {#if refreshError || (hasStaleData && !refreshing)}
            {i18n.t(
              hasKnownBalances
                ? 'wallet.watchlist.updateUnavailable'
                : 'wallet.watchlist.updateUnavailableEmpty'
            )}
          {/if}
        </span>
        <Button size="sm" onclick={openAddView}>
          <PlusIcon class="size-3.5" aria-hidden="true" />
          {i18n.t('wallet.watchlist.addAddress')}
        </Button>
      </div>
      <ScrollArea.Root class="min-h-0 flex-1">
        <ScrollArea.Viewport>
          <div class="pb-8">
            {#each viewModels as entry, index (entry.id)}
              <button
                type="button"
                class="group/watch-row relative isolate flex min-h-[88px] w-full items-center gap-3.5 px-2 text-left outline-none before:pointer-events-none before:absolute before:inset-0 before:-z-10 before:bg-linear-to-r before:from-transparent before:via-muted/25 before:to-transparent before:opacity-0 before:transition-opacity hover:before:opacity-100 focus-visible:rounded-sm focus-visible:ring-2 focus-visible:ring-settings-focus-ring dark:before:via-muted/15 {index <
                viewModels.length - 1
                  ? 'border-b border-border/60'
                  : ''}"
                data-testid="watchlist-entry"
                data-entry-id={entry.id}
                onclick={() => (selectedEntryId = entry.id)}
              >
                <WatchlistAvatar
                  address={entry.address}
                  displayName={entry.displayName}
                  targetKind={entry.targetKind}
                  network={walletNetwork}
                />
                <div class="min-w-0 flex-1">
                  <p class="truncate text-[15px] font-semibold">{entry.displayName}</p>
                  {#if entry.targetKind === 'address' && entry.displayName !== entry.address}
                    <IdentifierText
                      value={entry.address}
                      mode="review"
                      class="mt-0.5 block truncate text-xs text-settings-muted-foreground"
                    />
                  {/if}
                </div>
                <div class="shrink-0 text-right">
                  <p class="text-sm font-semibold tabular-nums">{entry.publicValueDisplay}</p>
                  <p class="mt-0.5 text-xs text-settings-muted-foreground">
                    {currencyCountLabel(entry.currencyCount)}
                  </p>
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
