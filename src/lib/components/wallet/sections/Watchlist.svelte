<script lang="ts">
  import { onMount } from 'svelte';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';
  import EyeIcon from '@lucide/svelte/icons/eye';
  import InfoIcon from '@lucide/svelte/icons/info';
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
  import MoreHorizontalIcon from '@lucide/svelte/icons/ellipsis';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
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
  let addSheetOpen = $state(false);
  let addQuery = $state('');
  let resolvedTarget = $state<WatchlistResolvedTarget | null>(null);
  let resolveError = $state('');
  let resolving = $state(false);
  let adding = $state(false);
  let removeDialogOpen = $state(false);
  let removeError = $state('');
  let removing = $state(false);
  let mounted = false;
  const copiedAddressState = new TimedValueState<string>();

  const networkLabel = $derived(
    `${i18n.t('wallet.watchlist.addSheet.network')} · ${i18n.t(`common.network.${walletNetwork}`)}`
  );
  const viewModels = $derived(
    records.map((record) =>
      buildWatchlistEntryViewModel(record, rates, i18n.intlLocale, settings.displayCurrency)
    )
  );
  const selectedView = $derived(
    selectedEntryId ? (viewModels.find((entry) => entry.id === selectedEntryId) ?? null) : null
  );
  const selectedRecord = $derived(
    selectedEntryId
      ? (records.find((record) => record.snapshot.entry.id === selectedEntryId) ?? null)
      : null
  );
  const hasStaleData = $derived(records.some((record) => record.stale));

  function placeholderSnapshot(entry: WatchlistEntry): WatchlistEntrySnapshot {
    return {
      entry,
      holdings: [],
      sources: [],
      availability: 'unavailable',
      refreshedAt: 0,
    };
  }

  function currencyCountLabel(count: number): string {
    return i18n.t(
      count === 1 ? 'wallet.watchlist.currencyCountOne' : 'wallet.watchlist.currencyCount',
      { count }
    );
  }

  function translateError(error: unknown, fallbackKey: string): string {
    switch (extractWalletErrorType(error)) {
      case 'WatchlistInvalidInput':
        return i18n.t('wallet.watchlist.error.invalidInput');
      case 'WatchlistEntryNotFound':
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

  function openAddSheet(): void {
    addQuery = '';
    resolvedTarget = null;
    resolveError = '';
    resolving = false;
    adding = false;
    addSheetOpen = true;
  }

  function updateAddQuery(value: string): void {
    addQuery = value;
    resolvedTarget = null;
    resolveError = '';
  }

  async function resolveTarget(): Promise<void> {
    const query = addQuery.trim();
    if (!query) {
      resolveError = i18n.t('wallet.watchlist.error.inputRequired');
      return;
    }

    resolving = true;
    resolveError = '';
    try {
      resolvedTarget = await watchlistService.resolveWatchlistTarget(query);
    } catch (error) {
      resolvedTarget = null;
      resolveError = translateError(error, 'wallet.watchlist.error.resolveFailed');
    } finally {
      resolving = false;
    }
  }

  async function addResolvedTarget(): Promise<void> {
    if (!resolvedTarget || adding) return;
    adding = true;
    resolveError = '';
    try {
      const snapshot = await watchlistService.addWatchlistEntry(addQuery.trim());
      records = mergeWatchlistSnapshots(records, [
        ...records.map((record) => record.snapshot),
        snapshot,
      ]);
      addSheetOpen = false;
      addQuery = '';
      resolvedTarget = null;
      loadError = '';
    } catch (error) {
      resolveError = translateError(error, 'wallet.watchlist.error.addFailed');
    } finally {
      adding = false;
    }
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
      records = records.filter((record) => record.snapshot.entry.id !== selectedEntryId);
      selectedEntryId = null;
      removeDialogOpen = false;
    } catch (error) {
      removeError = translateError(error, 'wallet.watchlist.error.removeFailed');
    } finally {
      removing = false;
    }
  }

  onMount(() => {
    mounted = true;
    void hydrate();
    return () => {
      mounted = false;
    };
  });
</script>

<div class="flex min-h-0 w-full flex-1 flex-col" data-testid="watchlist-section">
  {#if selectedView && selectedRecord}
    <header class="flex shrink-0 items-start justify-between gap-4 px-8 pt-5 pb-4">
      <div class="min-w-0">
        <button
          type="button"
          class="mb-4 flex items-center gap-1.5 text-[13px] text-settings-muted-foreground transition-colors outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
          onclick={() => (selectedEntryId = null)}
        >
          <ArrowLeftIcon class="size-3.5" aria-hidden="true" />
          {i18n.t('wallet.watchlist.back')}
        </button>
        <div class="flex min-w-0 items-center gap-2.5">
          <h2 class="truncate text-2xl leading-8 font-semibold tracking-tight">
            {selectedView.displayName}
          </h2>
          <span
            class="shrink-0 rounded-full bg-settings-surface px-2 py-1 text-[11px] font-medium text-settings-muted-foreground"
          >
            {i18n.t('wallet.watchlist.readOnly')}
          </span>
        </div>
        <div class="mt-1 flex min-w-0 items-center gap-1.5">
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
      </div>
      <div class="mt-9 flex shrink-0 items-center gap-1">
        <Button
          variant="ghost"
          size="icon-sm"
          disabled={refreshing}
          aria-label={i18n.t(
            refreshing ? 'wallet.watchlist.refreshing' : 'wallet.watchlist.refresh'
          )}
          title={i18n.t(refreshing ? 'wallet.watchlist.refreshing' : 'wallet.watchlist.refresh')}
          onclick={() => void refreshAll()}
        >
          <RefreshCwIcon
            class="size-4 {refreshing ? 'animate-spin motion-reduce:animate-none' : ''}"
          />
        </Button>
        <DropdownMenu.Root>
          <DropdownMenu.Trigger
            class="flex size-8 items-center justify-center rounded-md text-settings-muted-foreground transition-colors outline-none hover:bg-accent hover:text-foreground focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
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
      </div>
    </header>

    <ScrollArea.Root class="min-h-0 flex-1">
      <ScrollArea.Viewport>
        <div class="mx-auto w-full max-w-3xl px-8 pb-8">
          <section class="rounded-xl bg-settings-surface px-5 py-5" data-testid="public-value-card">
            <p class="text-xs font-medium text-settings-muted-foreground">
              {i18n.t('wallet.watchlist.publicValue')}
            </p>
            <p class="mt-1 text-[30px] leading-9 font-semibold tracking-tight">
              {selectedView.publicValueDisplay}
            </p>
            <p class="mt-2 text-xs text-settings-muted-foreground">
              {currencyCountLabel(selectedView.currencyCount)}
            </p>
          </section>

          <div class="mt-7">
            {#if selectedView.holdings.length === 0}
              <div
                class="rounded-xl border border-border/60 px-5 py-8 text-center text-sm text-settings-muted-foreground"
              >
                {i18n.t('wallet.watchlist.noCurrencies')}
              </div>
            {:else}
              <div class="overflow-hidden rounded-xl border border-border/60">
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
                        {holding.systemName}
                      </p>
                    </div>
                    <div class="min-w-0 text-right">
                      <p class="text-sm font-medium tabular-nums">
                        {holding.balanceDisplay}
                        {holding.ticker}
                      </p>
                      <p class="text-xs text-settings-muted-foreground tabular-nums">
                        {holding.fiatDisplay}
                      </p>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        </div>
      </ScrollArea.Viewport>
      <ScrollArea.Scrollbar orientation="vertical" />
    </ScrollArea.Root>
  {:else}
    <header class="flex h-[58px] shrink-0 items-center justify-between gap-4 px-8">
      <h2 class="text-2xl leading-8 font-semibold tracking-tight">
        {i18n.t('wallet.watchlist.title')}
      </h2>
      <Button size="sm" onclick={openAddSheet}>
        <PlusIcon class="size-3.5" aria-hidden="true" />
        {i18n.t('wallet.watchlist.add')}
      </Button>
    </header>

    {#if initialLoading}
      <div class="flex flex-1 items-center justify-center text-settings-muted-foreground">
        <LoaderCircleIcon
          class="size-5 animate-spin motion-reduce:animate-none"
          aria-hidden="true"
        />
        <span class="sr-only">{i18n.t('common.loading')}</span>
      </div>
    {:else if loadError && records.length === 0}
      <div class="flex flex-1 flex-col items-center justify-center px-8 text-center">
        <CircleAlertIcon class="size-7 text-settings-muted-foreground" aria-hidden="true" />
        <p class="mt-3 text-sm font-medium">{loadError}</p>
        <Button variant="secondary" size="sm" class="mt-4" onclick={() => void hydrate()}>
          {i18n.t('common.retry')}
        </Button>
      </div>
    {:else if records.length === 0}
      <div
        class="flex flex-1 flex-col items-center justify-center px-8 pb-10 text-center"
        data-testid="watchlist-empty"
      >
        <EyeIcon class="size-8 text-settings-muted-foreground" aria-hidden="true" />
        <h3 class="mt-4 text-lg font-semibold tracking-tight">
          {i18n.t('wallet.watchlist.emptyTitle')}
        </h3>
        <p class="mt-1.5 max-w-sm text-[13px] leading-5 text-settings-muted-foreground">
          {i18n.t('wallet.watchlist.emptyDescription')}
        </p>
        <Button size="sm" class="mt-5" onclick={openAddSheet}>
          <PlusIcon class="size-3.5" aria-hidden="true" />
          {i18n.t('wallet.watchlist.addAddress')}
        </Button>
      </div>
    {:else}
      <div class="flex min-h-0 flex-1 flex-col">
        <div class="flex h-8 shrink-0 items-center justify-between px-8 text-xs">
          <span class="text-settings-muted-foreground" aria-live="polite">
            {#if refreshing}
              {i18n.t('wallet.watchlist.refreshing')}
            {:else if refreshError || hasStaleData}
              {i18n.t('wallet.watchlist.updateUnavailable')}
            {:else}
              {i18n.t('wallet.watchlist.updatedJustNow')}
            {/if}
          </span>
          <Button
            variant="ghost"
            size="sm"
            class="h-7 px-2 text-xs text-settings-muted-foreground"
            disabled={refreshing}
            onclick={() => void refreshAll()}
          >
            <RefreshCwIcon
              class="size-3.5 {refreshing ? 'animate-spin motion-reduce:animate-none' : ''}"
            />
            {i18n.t('wallet.watchlist.refresh')}
          </Button>
        </div>
        <ScrollArea.Root class="min-h-0 flex-1">
          <ScrollArea.Viewport>
            <div class="mx-auto grid w-full max-w-3xl gap-3 px-8 pt-2 pb-8">
              {#each viewModels as entry (entry.id)}
                <button
                  type="button"
                  class="w-full rounded-xl border border-border/60 bg-settings-surface px-4 py-4 text-left transition-colors outline-none hover:bg-muted/60 focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
                  data-testid="watchlist-entry"
                  onclick={() => (selectedEntryId = entry.id)}
                >
                  <div class="flex min-w-0 items-start justify-between gap-5">
                    <div class="min-w-0 flex-1">
                      <p class="truncate text-[15px] font-semibold">{entry.displayName}</p>
                      <IdentifierText
                        value={entry.address}
                        mode="review"
                        class="mt-0.5 block truncate text-xs text-settings-muted-foreground"
                      />
                    </div>
                    <div class="shrink-0 text-right">
                      <p class="text-sm font-semibold tabular-nums">{entry.publicValueDisplay}</p>
                      <p class="mt-0.5 text-xs text-settings-muted-foreground">
                        {currencyCountLabel(entry.currencyCount)}
                      </p>
                    </div>
                  </div>
                  {#if entry.holdings.length > 0}
                    <div class="mt-3 flex flex-wrap gap-2">
                      {#each entry.holdings.slice(0, 4) as holding (holding.key)}
                        <span
                          class="inline-flex h-7 items-center gap-1.5 rounded-full bg-settings-surface px-2.5 text-xs"
                        >
                          <CoinIcon
                            coinId={holding.coinId}
                            coinName={holding.name}
                            proto="vrsc"
                            size={16}
                            decorative
                          />
                          <span class="font-medium">{holding.balanceDisplay}</span>
                          <span class="text-settings-muted-foreground">{holding.ticker}</span>
                        </span>
                      {/each}
                    </div>
                  {/if}
                </button>
              {/each}
            </div>
          </ScrollArea.Viewport>
          <ScrollArea.Scrollbar orientation="vertical" />
        </ScrollArea.Root>
      </div>
    {/if}
  {/if}
</div>

<StandardRightSheet
  bind:isOpen={addSheetOpen}
  title={i18n.t('wallet.watchlist.addSheet.title')}
  closeLabel={i18n.t('common.close')}
  onOpenChange={(open) => {
    if (!open && !resolving && !adding) addSheetOpen = false;
  }}
>
  <form
    class="flex min-h-0 flex-1 flex-col"
    onsubmit={(event) => {
      event.preventDefault();
      if (resolvedTarget) void addResolvedTarget();
      else void resolveTarget();
    }}
  >
    <div class="min-h-0 flex-1">
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
          disabled={resolving || adding}
          aria-invalid={Boolean(resolveError)}
          aria-describedby={resolveError ? 'watchlist-add-error' : 'watchlist-add-info'}
          class="identifier-text h-10 px-3 text-sm"
        />
      </div>

      <div class="mt-3 flex items-center justify-between text-xs text-settings-muted-foreground">
        <span>{i18n.t('wallet.watchlist.addSheet.networkLabel')}</span>
        <span>{networkLabel}</span>
      </div>

      <div
        id="watchlist-add-info"
        class="mt-5 flex gap-2 rounded-lg bg-settings-surface px-3 py-3 text-xs leading-5 text-settings-muted-foreground"
      >
        <InfoIcon class="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
        <span>{i18n.t('wallet.watchlist.addSheet.info')}</span>
      </div>

      {#if resolveError}
        <p id="watchlist-add-error" class="mt-4 text-xs text-destructive" role="alert">
          {resolveError}
        </p>
      {/if}

      {#if resolvedTarget}
        <div class="mt-6" data-testid="watchlist-resolved-preview">
          <p class="text-xs font-medium text-primary dark:text-settings-focus-ring">
            {i18n.t('wallet.watchlist.addSheet.found')}
          </p>
          <div class="mt-2 rounded-xl bg-settings-surface px-4 py-4">
            <p class="truncate text-sm font-semibold">{resolvedTarget.displayName}</p>
            <IdentifierText
              value={resolvedTarget.address}
              mode="review"
              class="mt-1 block truncate text-xs text-settings-muted-foreground"
            />
          </div>
          <dl class="mt-4 space-y-3 text-xs">
            <div class="flex items-center justify-between gap-4">
              <dt class="text-settings-muted-foreground">
                {i18n.t('wallet.watchlist.addSheet.networkLabel')}
              </dt>
              <dd>{networkLabel}</dd>
            </div>
            <div class="flex items-center justify-between gap-4">
              <dt class="text-settings-muted-foreground">
                {i18n.t('wallet.watchlist.addSheet.visibleCurrencies')}
              </dt>
              <dd>
                {resolvedTarget.visibleCurrencyCount ??
                  i18n.t('wallet.watchlist.addSheet.unavailable')}
              </dd>
            </div>
          </dl>
        </div>
      {/if}
    </div>

    <footer class="flex shrink-0 justify-end gap-2 pt-6">
      <Button
        variant="secondary"
        size="sm"
        disabled={resolving || adding}
        onclick={() => (addSheetOpen = false)}
      >
        {i18n.t('common.cancel')}
      </Button>
      <Button type="submit" size="sm" disabled={resolving || adding || !addQuery.trim()}>
        {#if resolving || adding}
          <LoaderCircleIcon
            class="size-3.5 animate-spin motion-reduce:animate-none"
            aria-hidden="true"
          />
        {/if}
        {i18n.t(
          adding
            ? 'wallet.watchlist.addSheet.adding'
            : resolving
              ? 'wallet.watchlist.addSheet.resolving'
              : resolvedTarget
                ? 'wallet.watchlist.addSheet.add'
                : 'common.continue'
        )}
      </Button>
    </footer>
  </form>
</StandardRightSheet>

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
    {#if removeError}
      <p class="text-sm text-destructive" role="alert">{removeError}</p>
    {/if}
    <Dialog.Footer class="flex justify-end gap-3">
      <Button
        variant="secondary"
        size="sm"
        disabled={removing}
        onclick={() => (removeDialogOpen = false)}
      >
        {i18n.t('common.cancel')}
      </Button>
      <Button variant="destructive" size="sm" disabled={removing} onclick={confirmRemove}>
        {i18n.t(removing ? 'wallet.watchlist.removing' : 'wallet.watchlist.remove')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
