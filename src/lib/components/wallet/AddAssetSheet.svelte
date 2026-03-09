<script lang="ts">
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';
  import CheckIcon from '@lucide/svelte/icons/check';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import SearchInput from '$lib/components/common/SearchInput.svelte';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import InlineTextActionButton from '$lib/components/common/InlineTextActionButton.svelte';
  import AddAssetRow from '$lib/components/wallet/AddAssetRow.svelte';
  import { Input } from '$lib/components/ui/input';
  import { Button } from '$lib/components/ui/button';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import { i18nStore } from '$lib/i18n';
  import { isWalletSupportedAsset } from '$lib/coins/supportedAssets.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { buildWalletChannels, walletChannelsStore } from '$lib/stores/walletChannels.js';
  import * as coinsService from '$lib/services/coinsService.js';
  import * as walletService from '$lib/services/walletService.js';
  import {
    type AddAssetEntry,
    applyCatalogMetadataToCoinDefinition,
    buildAddAssetCatalogView,
    catalogEntryToCoinDefinition,
    erc20ContractValue,
    pbaasLookupValue
  } from '$lib/stores/addAssetCatalog.js';
  import type { ActiveAssetsState, CoinDefinition, PbaasCandidate, WalletNetwork } from '$lib/types/wallet.js';

   
  let { isOpen = $bindable(false), network }: { isOpen?: boolean; network: WalletNetwork } = $props();
   

  const i18n = $derived($i18nStore);
  const walletChannels = $derived($walletChannelsStore);
  let registryCoins = $state<CoinDefinition[]>([]);
  let activeAssetIds = $state<string[]>([]);

  let searchInput = $state('');
  let debouncedSearch = $state('');
  let view = $state<'catalog' | 'manual'>('catalog');

  let actionError = $state('');
  let actionSuccess = $state('');
  let actionSuccessTone = $state<'success' | 'destructive'>('success');
  let actionSuccessTimer = $state<ReturnType<typeof setTimeout> | null>(null);
  let activeRowKey = $state<string | null>(null);

  let manualInput = $state('');
  let manualResolving = $state(false);
  let manualAdding = $state(false);
  let manualAdded = $state(false);
  let manualError = $state('');
  let manualResolvedCoin = $state<CoinDefinition | null>(null);
  let manualCandidates = $state<PbaasCandidate[]>([]);

  const catalogView = $derived(
    buildAddAssetCatalogView({
      coins: registryCoins,
      network,
      query: debouncedSearch,
      activeCoinIds: activeAssetIds
    })
  );

  $effect(() => {
    const query = searchInput;
    const timer = setTimeout(() => {
      debouncedSearch = query;
    }, 150);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    if (isOpen) return;
    resetSheetState();
  });

  $effect(() => {
    return () => {
      if (actionSuccessTimer) {
        clearTimeout(actionSuccessTimer);
      }
    };
  });

  $effect(() => {
    if (!isOpen) return;
    void (async () => {
      try {
        await hydrateCatalogState();
        actionError = '';
      } catch (error) {
        actionError = translateAssetError(error, 'wallet.addAsset.error.addFailed');
      }
    })();
  });

  function normalizeCoinId(value: string): string {
    return value.trim().toLowerCase();
  }

  function filterCoinsByActiveIds(coins: CoinDefinition[], coinIds: string[]): CoinDefinition[] {
    const activeSet = new Set(
      coinIds
        .map((coinId) => normalizeCoinId(coinId))
        .filter((coinId) => coinId.length > 0)
    );
    if (activeSet.size === 0) return [];

    return coins.filter((coin) => activeSet.has(normalizeCoinId(coin.id)));
  }

  function applyActiveAssetsState(state: ActiveAssetsState, sourceCoins: CoinDefinition[] = registryCoins): void {
    activeAssetIds = state.coinIds;
    const activeCoins = filterCoinsByActiveIds(sourceCoins, state.coinIds);
    coinsStore.set(activeCoins);
    walletChannelsStore.set(buildWalletChannels(activeCoins, walletChannels.vrpcAddress));
  }

  async function hydrateCatalogState(): Promise<void> {
    const [allCoins, activeAssets] = await Promise.all([
      coinsService.getCoinRegistry(),
      walletService.getActiveAssets()
    ]);
    const networkCoins = allCoins.filter((coin) => isWalletSupportedAsset(coin, network));
    registryCoins = networkCoins;
    applyActiveAssetsState(activeAssets, networkCoins);
  }

  function handleOpenAutoFocus(event: Event) {
    event.preventDefault();
  }

  function clearActionSuccessTimer() {
    if (!actionSuccessTimer) return;
    clearTimeout(actionSuccessTimer);
    actionSuccessTimer = null;
  }

  function clearActionSuccess() {
    clearActionSuccessTimer();
    actionSuccess = '';
    actionSuccessTone = 'success';
  }

  function setActionSuccess(
    message: string,
    tone: 'success' | 'destructive' = 'success',
    autoClearMs?: number
  ) {
    clearActionSuccessTimer();
    actionSuccess = message;
    actionSuccessTone = tone;
    if (!autoClearMs || autoClearMs <= 0 || !message) return;
    actionSuccessTimer = setTimeout(() => {
      actionSuccess = '';
      actionSuccessTone = 'success';
      actionSuccessTimer = null;
    }, autoClearMs);
  }

  function resetSheetState() {
    view = 'catalog';
    searchInput = '';
    debouncedSearch = '';
    actionError = '';
    clearActionSuccess();
    activeRowKey = null;

    manualInput = '';
    manualResolving = false;
    manualAdding = false;
    manualAdded = false;
    manualError = '';
    manualResolvedCoin = null;
    manualCandidates = [];
  }

  function extractWalletErrorType(error: unknown): string | null {
    if (typeof error === 'string') {
      try {
        const parsed = JSON.parse(error) as unknown;
        if (parsed && typeof parsed === 'object' && 'type' in parsed) {
          const typed = (parsed as { type?: unknown }).type;
          if (typeof typed === 'string') return typed;
        }
      } catch {
        return null;
      }
      return null;
    }

    if (!error || typeof error !== 'object') return null;

    const obj = error as { type?: unknown; data?: unknown; message?: unknown };
    if (typeof obj.type === 'string') return obj.type;
    if (obj.data && typeof obj.data === 'object') {
      const data = obj.data as { type?: unknown };
      if (typeof data.type === 'string') return data.type;
    }
    if (typeof obj.message === 'string') {
      try {
        const parsed = JSON.parse(obj.message) as { type?: unknown };
        if (typeof parsed.type === 'string') return parsed.type;
      } catch {
        return null;
      }
    }

    return null;
  }

  function translateAssetError(error: unknown, fallbackKey: string): string {
    const errorType = extractWalletErrorType(error);

    switch (errorType) {
      case 'AssetAlreadyExists':
      case 'DuplicatePbaasCurrency':
        return i18n.t('wallet.addAsset.error.assetExists');
      case 'PbaasNotFound':
        return i18n.t('wallet.addAsset.error.pbaasNotFound');
      case 'PbaasAmbiguous':
        return i18n.t('wallet.addAsset.error.pbaasAmbiguous');
      case 'InvalidContract':
        return i18n.t('wallet.addAsset.error.invalidContract');
      case 'UnsupportedNetwork':
        return i18n.t('wallet.addAsset.error.unsupportedNetwork');
      case 'EthNotConfigured':
        return i18n.t('wallet.addAsset.error.ethNotConfigured');
      case 'WalletLocked':
        return i18n.t('wallet.addAsset.error.walletLocked');
      default:
        break;
    }

    if (error instanceof Error && error.message.trim().length > 0) {
      return error.message;
    }

    return i18n.t(fallbackKey);
  }

  async function refreshRegistryState() {
    const allCoins = await coinsService.getCoinRegistry();
    const networkCoins = allCoins.filter((coin) => isWalletSupportedAsset(coin, network));
    registryCoins = networkCoins;

    const activeCoins = filterCoinsByActiveIds(networkCoins, activeAssetIds);
    coinsStore.set(activeCoins);
    walletChannelsStore.set(buildWalletChannels(activeCoins, walletChannels.vrpcAddress));
  }

  async function persistActiveAssets(coinIds: string[]): Promise<void> {
    const nextState = await walletService.setActiveAssets(coinIds);
    applyActiveAssetsState(nextState);
  }

  async function activateAsset(coinId: string): Promise<void> {
    await persistActiveAssets([...activeAssetIds, coinId]);
  }

  async function deactivateAsset(coinId: string): Promise<void> {
    const nextIds = activeAssetIds.filter((existingId) => normalizeCoinId(existingId) !== normalizeCoinId(coinId));
    await persistActiveAssets(nextIds);
  }

  function findMatchingRegistryCoin(definition: CoinDefinition): CoinDefinition | null {
    const normalizedId = normalizeCoinId(definition.id);
    const normalizedCurrencyId = normalizeCoinId(definition.currencyId ?? '');
    const byId = registryCoins.find((coin) => normalizeCoinId(coin.id) === normalizedId);
    if (byId) return byId;

    if (normalizedCurrencyId.length === 0) return null;
    return (
      registryCoins.find(
        (coin) =>
          coin.proto === definition.proto &&
          normalizeCoinId(coin.currencyId ?? '') === normalizedCurrencyId
      ) ?? null
    );
  }

  async function addCoinToRegistry(
    definition: CoinDefinition,
    successMessageKey: string,
    options?: { showSuccessNotice?: boolean; successTone?: 'success' | 'destructive'; autoClearMs?: number }
  ) {
    const showSuccessNotice = options?.showSuccessNotice ?? true;
    const successTone = options?.successTone ?? 'success';
    const autoClearMs = options?.autoClearMs;
    try {
      const addedCoin = await coinsService.addCoinDefinition(definition);
      await refreshRegistryState();
      await activateAsset(addedCoin.id);
      actionError = '';
      if (showSuccessNotice) {
        setActionSuccess(i18n.t(successMessageKey, { ticker: definition.displayTicker }), successTone, autoClearMs);
      } else {
        clearActionSuccess();
      }
    } catch (error) {
      const errorType = extractWalletErrorType(error);
      if (errorType === 'AssetAlreadyExists' || errorType === 'DuplicatePbaasCurrency') {
        await refreshRegistryState();
        const existingCoin = findMatchingRegistryCoin(definition);
        if (!existingCoin) {
          throw error;
        }
        await activateAsset(existingCoin.id);
        actionError = '';
        if (showSuccessNotice) {
          setActionSuccess(i18n.t('wallet.addAsset.toast.enabled', { ticker: definition.displayTicker }), successTone, autoClearMs);
        } else {
          clearActionSuccess();
        }
        return;
      }

      throw error;
    }
  }

  async function handleCatalogAction(entry: AddAssetEntry) {
    clearActionSuccess();
    actionError = '';
    activeRowKey = entry.key;

    try {
      if (entry.status === 'added') {
        await deactivateAsset(entry.id);
        setActionSuccess(i18n.t('wallet.addAsset.toast.disabled', { ticker: entry.displayTicker }), 'destructive', 2000);
        return;
      }

      if (entry.addStrategy === 'activate') {
        await activateAsset(entry.id);
        setActionSuccess(i18n.t('wallet.addAsset.toast.enabled', { ticker: entry.displayTicker }));
        return;
      }

      if (entry.addStrategy === 'direct') {
        const definition = catalogEntryToCoinDefinition(entry, network);
        if (!definition) {
          throw new Error(i18n.t('wallet.addAsset.error.addFailed'));
        }
        await addCoinToRegistry(definition, 'wallet.addAsset.toast.added');
      } else if (entry.addStrategy === 'resolve_pbaas') {
        const result = await coinsService.resolvePbaasCurrency(pbaasLookupValue(entry));
        if (result.status === 'ambiguous') {
          throw new Error(i18n.t('wallet.addAsset.error.pbaasAmbiguous'));
        }
        const hydratedCoin = applyCatalogMetadataToCoinDefinition(result.coin);
        await addCoinToRegistry(hydratedCoin, 'wallet.addAsset.toast.added');
      } else if (entry.addStrategy === 'resolve_erc20') {
        const result = await coinsService.resolveErc20Contract(erc20ContractValue(entry));
        const hydratedCoin = applyCatalogMetadataToCoinDefinition(result.coin);
        await addCoinToRegistry(hydratedCoin, 'wallet.addAsset.toast.added');
      }
    } catch (error) {
      actionError = translateAssetError(error, 'wallet.addAsset.error.addFailed');
    } finally {
      activeRowKey = null;
    }
  }

  function normalizedErc20ContractCandidate(value: string): string | null {
    const trimmed = value.trim();
    if (/^0x[a-fA-F0-9]{40}$/.test(trimmed)) return trimmed;
    if (/^[a-fA-F0-9]{40}$/.test(trimmed)) return `0x${trimmed}`;
    return null;
  }

  async function resolveManualAsset() {
    manualError = '';
    manualAdded = false;
    manualCandidates = [];
    manualResolvedCoin = null;
    clearActionSuccess();

    const input = manualInput.trim();
    if (!input) {
      manualError = i18n.t('wallet.addAsset.error.manualInputRequired');
      return;
    }

    const contractCandidate = normalizedErc20ContractCandidate(input);
    manualResolving = true;
    try {
      if (contractCandidate) {
        const result = await coinsService.resolveErc20Contract(contractCandidate);
        manualResolvedCoin = result.coin;
        return;
      }

      const result = await coinsService.resolvePbaasCurrency(input);
      if (result.status === 'ambiguous') {
        manualCandidates = result.candidates;
        manualError = i18n.t('wallet.addAsset.error.pbaasAmbiguous');
        return;
      }

      manualResolvedCoin = result.coin;
    } catch (error) {
      manualError = translateAssetError(
        error,
        contractCandidate ? 'wallet.addAsset.error.erc20ResolveFailed' : 'wallet.addAsset.error.pbaasResolveFailed'
      );
    } finally {
      manualResolving = false;
    }
  }

  async function addResolvedManualAsset() {
    if (!manualResolvedCoin || manualAdded) return;
    manualAdding = true;
    manualError = '';

    try {
      const hydratedCoin = applyCatalogMetadataToCoinDefinition(manualResolvedCoin);
      await addCoinToRegistry(hydratedCoin, 'wallet.addAsset.toast.added', { showSuccessNotice: false });
      manualAdded = true;
    } catch (error) {
      manualError = translateAssetError(error, 'wallet.addAsset.error.addFailed');
    } finally {
      manualAdding = false;
    }
  }

  function shouldShowResolvedTicker(coin: CoinDefinition): boolean {
    const ticker = coin.displayTicker.trim();
    const name = coin.displayName.trim();
    if (!ticker || !name) return Boolean(ticker);
    return ticker.toLowerCase() !== name.toLowerCase();
  }
</script>

<StandardRightSheet
  bind:isOpen
  title={i18n.t('wallet.addAsset.title')}
  hideTitle
  bodyClass="mt-0"
  onOpenAutoFocus={handleOpenAutoFocus}
>
  <div class="flex h-full min-h-0 flex-col">
    {#if view === 'catalog'}
      <div class="pr-8 pt-4">
        <div class="flex items-center justify-between gap-3">
          <h2 class="text-base font-semibold text-foreground">{i18n.t('wallet.addAsset.title')}</h2>
          <InlineTextActionButton
            onclick={() => {
              view = 'manual';
            }}
          >
            {i18n.t('wallet.addAsset.cantFindTitle')}
          </InlineTextActionButton>
        </div>
      </div>
    {/if}

    {#if view === 'catalog'}
      <div class="mt-5 min-h-0 flex-1 overflow-y-auto pr-1">
        <div class="sticky top-0 z-10 bg-background pb-3">
          <SearchInput
            bind:value={searchInput}
            placeholder={i18n.t('wallet.addAsset.searchPlaceholder')}
            autocomplete="off"
            autocorrect="off"
            autocapitalize="off"
            spellcheck={false}
            inputClass="focus-visible:ring-0 focus-visible:ring-transparent"
          />

          {#if actionError}
            <div class="mt-2 flex items-start gap-2 rounded-md bg-destructive/12 px-2.5 py-2 text-xs text-destructive">
              <AlertCircleIcon class="mt-0.5 h-4 w-4 shrink-0" />
              <p>{actionError}</p>
            </div>
          {/if}

          {#if actionSuccess}
            <div
              class={`mt-2 rounded-md px-2.5 py-2 text-xs ${
                actionSuccessTone === 'destructive'
                  ? 'bg-destructive/12 text-destructive'
                  : 'bg-emerald-500/12 text-emerald-700 dark:text-emerald-300'
              }`}
            >
              {actionSuccess}
            </div>
          {/if}
        </div>

        <section class="mt-1 space-y-2">
          <h3 class="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
            {i18n.t('wallet.addAsset.sectionAdded')}
          </h3>
          {#if catalogView.addedEntries.length === 0}
            <p class="rounded-lg bg-muted/55 px-3 py-2.5 text-xs text-muted-foreground dark:bg-muted/50">
              {i18n.t('wallet.addAsset.emptySearch')}
            </p>
          {:else}
            <ul class="space-y-2">
              {#each catalogView.addedEntries as entry (entry.key)}
                <AddAssetRow
                  {entry}
                  busy={activeRowKey === entry.key}
                  onAction={handleCatalogAction}
                />
              {/each}
            </ul>
          {/if}
        </section>

        <section class="mt-4 space-y-2 pb-1">
          <h3 class="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
            {i18n.t('wallet.addAsset.sectionAvailable')}
          </h3>
          {#if catalogView.availableEntries.length === 0}
            <p class="rounded-lg bg-muted/55 px-3 py-2.5 text-xs text-muted-foreground dark:bg-muted/50">
              {i18n.t('wallet.addAsset.emptySearch')}
            </p>
          {:else}
            <ul class="space-y-2">
              {#each catalogView.availableEntries as entry (entry.key)}
                <AddAssetRow
                  {entry}
                  busy={activeRowKey === entry.key}
                  onAction={handleCatalogAction}
                />
              {/each}
            </ul>
          {/if}
        </section>
      </div>
    {:else}
      <div class="mt-2 flex min-h-0 flex-1 flex-col">
        <button
          type="button"
          class="text-muted-foreground hover:text-foreground inline-flex items-center gap-1.5 text-sm transition-colors"
          onclick={() => {
            view = 'catalog';
          }}
        >
          <ArrowLeftIcon class="size-4" />
          {i18n.t('common.back')}
        </button>

        <div class="min-h-0 flex-1 overflow-y-auto pr-1">
          <section class="mt-3 space-y-4 pb-1">
            {#if actionSuccess}
              <div
                class={`rounded-md px-2.5 py-2 text-xs ${
                  actionSuccessTone === 'destructive'
                    ? 'bg-destructive/12 text-destructive'
                    : 'bg-emerald-500/12 text-emerald-700 dark:text-emerald-300'
                }`}
              >
                {actionSuccess}
              </div>
            {/if}

            <form
              class="space-y-4"
              onsubmit={(event) => {
                event.preventDefault();
                if (manualResolving || manualAdding) return;
                resolveManualAsset();
              }}
            >
              <Input
                type="text"
                bind:value={manualInput}
                placeholder={i18n.t('wallet.addAsset.manualPlaceholder')}
                autocomplete="off"
                autocorrect="off"
                autocapitalize="off"
                spellcheck={false}
                class="h-10 focus-visible:ring-0 focus-visible:ring-transparent focus-visible:border-transparent"
              />

              <div class="flex gap-2">
                <Button variant="secondary" type="submit" class="h-8" disabled={manualResolving || manualAdding}>
                  {manualResolving ? i18n.t('wallet.addAsset.resolving') : i18n.t('wallet.addAsset.resolve')}
                </Button>
              </div>
            </form>

            {#if manualResolvedCoin}
              <div class="flex items-center gap-3 rounded-lg bg-muted/65 px-3.5 py-3 dark:bg-muted/55">
                <CoinIcon coinId={manualResolvedCoin.id} coinName={manualResolvedCoin.displayName} size={20} decorative />

                <div class="min-w-0 flex-1">
                  <p class="truncate text-sm font-semibold text-foreground">
                    {manualResolvedCoin.displayName}
                  </p>
                  {#if shouldShowResolvedTicker(manualResolvedCoin)}
                    <p class="truncate text-xs text-muted-foreground">{manualResolvedCoin.displayTicker}</p>
                  {/if}
                </div>

                <div class="flex shrink-0 items-center gap-2">
                  <span
                    class="bg-background/60 text-muted-foreground inline-flex rounded-full px-2.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide dark:bg-background/45"
                  >
                    {manualResolvedCoin.proto.toUpperCase()}
                  </span>
                  <button
                    type="button"
                    class={`inline-flex h-8 w-8 items-center justify-center rounded-md transition-colors focus-visible:ring-ring focus-visible:ring-[2px] focus-visible:outline-none disabled:opacity-45 ${
                      manualAdded
                        ? 'text-emerald-700 bg-emerald-500/15 dark:text-emerald-300 dark:bg-emerald-500/20'
                        : 'text-primary bg-primary/12 hover:bg-primary/20 dark:bg-primary/20 dark:hover:bg-primary/30'
                    }`}
                    onclick={addResolvedManualAsset}
                    disabled={manualAdding || manualResolving || manualAdded}
                    aria-label={
                      manualAdded
                        ? i18n.t('wallet.addAsset.stateAdded')
                        : manualAdding
                          ? i18n.t('wallet.addAsset.adding')
                          : i18n.t('wallet.addAsset.add')
                    }
                    title={
                      manualAdded
                        ? i18n.t('wallet.addAsset.stateAdded')
                        : manualAdding
                          ? i18n.t('wallet.addAsset.adding')
                          : i18n.t('wallet.addAsset.add')
                    }
                  >
                    {#if manualAdded}
                      <CheckIcon class="h-4 w-4" absoluteStrokeWidth />
                    {:else}
                      <PlusIcon class="h-4 w-4" absoluteStrokeWidth />
                    {/if}
                  </button>
                </div>
              </div>
            {/if}

            {#if manualCandidates.length > 1}
              <div class="space-y-1">
                <p class="text-[11px] text-muted-foreground">{i18n.t('wallet.addAsset.pbaasMatches')}</p>
                {#each manualCandidates as candidate}
                  <button
                    type="button"
                    class="text-left text-xs text-foreground underline-offset-2 hover:underline"
                    onclick={() => {
                      manualInput = candidate.currencyId;
                      resolveManualAsset();
                    }}
                  >
                    {candidate.displayTicker} ({candidate.currencyId})
                  </button>
                {/each}
              </div>
            {/if}

            {#if manualError}
              <p class="text-xs text-destructive">{manualError}</p>
            {/if}
          </section>
        </div>
      </div>
    {/if}
  </div>
</StandardRightSheet>
