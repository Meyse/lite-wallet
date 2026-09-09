<!--
  Component: AssetDetails
  Purpose: Coin detail view with scope selectors (address × network), scoped actions, and scoped transaction history.
-->

<script lang="ts">
  import { onDestroy } from 'svelte';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import SendIcon from '@lucide/svelte/icons/send';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import ArrowLeftRightIcon from '@lucide/svelte/icons/arrow-left-right';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import CheckIcon from '@lucide/svelte/icons/check';
  import SearchInput from '$lib/components/common/SearchInput.svelte';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { Button } from '$lib/components/ui/button';
  import { CopyButton } from '$lib/components/ui/copy-button';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';
  import { Spinner } from '$lib/components/ui/spinner';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import PrivateVerusWordmark from '$lib/components/wallet/PrivateVerusWordmark.svelte';
  import { get } from 'svelte/store';
  import { i18nStore } from '$lib/i18n';
  import { resolveCoinPresentation } from '$lib/coins/presentation.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { balanceStore, getBalance } from '$lib/stores/balances.js';
  import { networkStore } from '$lib/stores/network.js';
  import { ratesStore } from '$lib/stores/rates.js';
  import { settingsStore } from '$lib/stores/settings.js';
  import {
    scopesByCoinId,
    selectedAddressByCoinId,
    selectedSystemByCoinId,
    setCoinScopes,
    setSelectedScopeAddress,
    setSelectedScopeSystem,
  } from '$lib/stores/coinScopes.js';
  import {
    createEmptyTransactionPageState,
    loadHistoryPageWithInvalidationRetry,
    transactionHistoryPagesStore,
    updateTransactionHistoryPage,
  } from '$lib/stores/transactionHistoryPages.js';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import { formatFiatAmount, getRateForCurrency } from '$lib/utils/fiatDisplay.js';
  import { extractWalletErrorMessage, extractWalletErrorType } from '$lib/utils/walletErrors.js';
  import * as walletService from '$lib/services/walletService.js';
  import * as walletDisplayService from '$lib/services/walletDisplayService.js';
  import type {
    CoinScope,
    DlightRuntimeStatusResult,
    ScopeKind,
    Transaction,
    WalletEntryKind,
  } from '$lib/types/wallet.js';
  import type { TransferEntryContext } from './transfer-wizard/types';

  const TRANSACTION_PAGE_SIZE = 50;
  const DLIGHT_STATUS_POLL_MS = 4000;
  const SPEND_RATE_SMOOTHING = 0.25;
  const initialTransactionSkeletonRows = [0, 1, 2, 3, 4, 5];
  const loadMoreTransactionSkeletonRows = [0, 1];

  type AssetDetailsProps = {
    coinId: string;
    walletEntryKind?: WalletEntryKind;
    scopeFilterMode?: ScopeKind;
    entryDisplayName?: string;
    onBack?: () => void;
    onNavigateToReceive?: () => void;

    onNavigateToSend?: (_context: TransferEntryContext) => void;

    onNavigateToConvert?: (_context: TransferEntryContext) => void;
  };

  const noop = () => {};

  let {
    coinId,
    walletEntryKind = 'coin',
    scopeFilterMode = 'transparent',
    entryDisplayName,
    onBack = noop,
    onNavigateToReceive = noop,
    onNavigateToSend = noop,
    onNavigateToConvert = noop,
  }: AssetDetailsProps = $props();

  const i18n = $derived($i18nStore);
  const coins = $derived($coinsStore);
  const balances = $derived($balanceStore);
  const chainInfo = $derived($networkStore);
  const rates = $derived($ratesStore);
  const settings = $derived($settingsStore);
  const displayCurrency = $derived(settings.displayCurrency);
  const allScopesByCoinId = $derived($scopesByCoinId);
  const selectedAddressMap = $derived($selectedAddressByCoinId);
  const selectedSystemMap = $derived($selectedSystemByCoinId);
  const txPagesByScopeKey = $derived($transactionHistoryPagesStore);

  let scopesLoading = $state(false);
  let scopesError = $state('');
  let loadingSelectedBalance = $state(false);
  let showScopeSheet = $state(false);
  let addressSearchTerm = $state('');
  const copiedAddressState = new TimedValueState<string>();
  const copiedAddressKey = $derived(copiedAddressState.current);
  let dlightRuntimeStatus = $state<DlightRuntimeStatusResult | null>(null);
  let spendRateBlocksPerSec = $state<number | null>(null);
  let spendRateSample = $state<{ scannedHeight: number; updatedAt: number } | null>(null);

  let loadedBalanceByScopeKey = $state<Record<string, boolean>>({});
  let txScrollElement = $state<HTMLElement | null>(null);
  let canScrollTxDown = $state(false);

  let scopeRequestSequence = 0;
  let selectedBalanceRequestSequence = 0;
  let componentActive = true;
  const inFlightBalanceChannels = new Set<string>();
  const inFlightTransactionScopeKeys = new Set<string>();

  onDestroy(() => {
    componentActive = false;
  });

  const coin = $derived(coins.find((item) => item.id === coinId) ?? null);
  const coinPresentation = $derived(coin ? resolveCoinPresentation(coin) : null);
  const coinScopes = $derived(allScopesByCoinId[coinId] ?? []);
  const allScopes = $derived(coinScopes.filter((scope) => scope.scopeKind === scopeFilterMode));

  const selectedAddress = $derived(selectedAddressMap[coinId] ?? '');
  const selectedSystem = $derived(selectedSystemMap[coinId] ?? '');

  const filteredScopeOptions = $derived(
    (() => {
      const query = addressSearchTerm.trim().toLowerCase();
      if (!query) return allScopes;
      return allScopes.filter((scope) => {
        return (
          scope.address.toLowerCase().includes(query) ||
          scope.addressLabel.toLowerCase().includes(query) ||
          networkLabelForScope(scope).toLowerCase().includes(query)
        );
      });
    })()
  );

  const selectedScope = $derived(
    allScopes.find(
      (scope) => scope.address === selectedAddress && scope.systemId === selectedSystem
    ) ?? null
  );
  const selectedScopeDisplayAddress = $derived(
    selectedScope ? preferredScopeDisplayValue(selectedScope) : selectedAddress
  );
  const isDlightShieldedScope = $derived(
    selectedScope?.scopeKind === 'shielded' &&
      selectedScope.channelId.toLowerCase().startsWith('dlight_private.')
  );

  const selectedScopeBalance = $derived(
    selectedScope && coin ? getBalance(selectedScope.channelId, coin.id, balances) : undefined
  );

  const selectedScopePageKey = $derived(
    selectedScope && coin ? getScopePageKey(selectedScope.channelId, coin.id) : ''
  );

  const selectedScopeTxPage = $derived(
    selectedScopePageKey ? (txPagesByScopeKey[selectedScopePageKey] ?? null) : null
  );

  const selectedScopeTransactions = $derived(selectedScopeTxPage?.items ?? []);

  const sortedSelectedTransactions = $derived(
    [...selectedScopeTransactions].sort((left, right) => {
      const pendingOrder = Number(right.pending) - Number(left.pending);
      if (pendingOrder !== 0) return pendingOrder;
      return (right.timestamp ?? 0) - (left.timestamp ?? 0);
    })
  );

  const loadingSelectedTransactions = $derived(selectedScopeTxPage?.loadingInitial ?? false);
  const loadingMoreTransactions = $derived(selectedScopeTxPage?.loadingMore ?? false);
  const selectedScopeTransactionsError = $derived(selectedScopeTxPage?.error ?? '');
  const selectedScopeLoadMoreError = $derived(selectedScopeTxPage?.loadMoreError ?? '');
  const selectedScopeHasMoreTransactions = $derived(selectedScopeTxPage?.hasMore ?? false);

  const selectedAmountValue = $derived(toFiniteNumber(selectedScopeBalance?.total));

  const selectedRateMetrics = $derived(
    (() => {
      if (!coin) return null;
      const candidates = [coin.id, coin.currencyId, coin.mappedTo].filter(
        (value): value is string => typeof value === 'string' && value.trim().length > 0
      );
      for (const candidate of candidates) {
        const snapshot = rates[candidate];
        const fiatRate = getRateForCurrency(snapshot?.rates, displayCurrency);
        const rawChange = snapshot?.usdChange24hPct;
        const change24hPct =
          typeof rawChange === 'number' && Number.isFinite(rawChange) ? rawChange : null;
        if (fiatRate !== null || change24hPct !== null) {
          return {
            fiatRate,
            change24hPct,
          };
        }
      }
      return null;
    })()
  );

  const selectedFiatRate = $derived(selectedRateMetrics?.fiatRate ?? null);

  const selectedChange24hPct = $derived(selectedRateMetrics?.change24hPct ?? null);

  const selectedChange24hDirection = $derived(getChangeDirection(selectedChange24hPct));

  const selectedChange24hDisplay = $derived(
    selectedChange24hDirection === 'none' || selectedChange24hPct === null
      ? '—'
      : formatPercentChange(selectedChange24hPct)
  );

  const selectedUnitRateDisplay = $derived(
    selectedFiatRate === null
      ? '—'
      : formatFiatAmount(selectedFiatRate, i18n.intlLocale, displayCurrency)
  );

  const selectedFiatDisplay = $derived(
    selectedFiatRate === null || selectedAmountValue === null
      ? '—'
      : formatFiatAmount(selectedAmountValue * selectedFiatRate, i18n.intlLocale, displayCurrency)
  );

  const selectedCryptoAmountDisplay = $derived(
    selectedAmountValue === null ? '—' : formatCryptoValue(selectedAmountValue, coin?.decimals ?? 8)
  );
  const totalAmountValue = $derived(
    (() => {
      if (!coin || allScopes.length === 0) return null;

      let total = 0;
      const seenScopeChannels = new Set<string>();
      for (const scope of allScopes) {
        if (seenScopeChannels.has(scope.channelId)) continue;
        seenScopeChannels.add(scope.channelId);

        const amount = toFiniteNumber(getBalance(scope.channelId, coin.id, balances)?.total);
        if (amount === null) return null;
        total += amount;
      }

      return total;
    })()
  );
  const totalFiatDisplay = $derived(
    selectedFiatRate === null || totalAmountValue === null
      ? '—'
      : formatFiatAmount(totalAmountValue * selectedFiatRate, i18n.intlLocale, displayCurrency)
  );
  const totalCryptoAmountDisplay = $derived(
    totalAmountValue === null ? '—' : formatCryptoValue(totalAmountValue, coin?.decimals ?? 8)
  );

  const selectedSyncPercent = $derived(
    selectedScope ? toFiniteNumber(chainInfo[selectedScope.channelId]?.percent) : null
  );
  const selectedBalanceSyncPercent = $derived(
    isDlightShieldedScope
      ? (toFiniteNumber(dlightRuntimeStatus?.percent) ?? selectedSyncPercent)
      : selectedSyncPercent
  );
  const selectedSendSyncPercent = $derived(
    isDlightShieldedScope
      ? (toFiniteNumber(dlightRuntimeStatus?.spendCachePercent) ?? selectedBalanceSyncPercent)
      : selectedBalanceSyncPercent
  );
  const isShieldedSyncBlocked = $derived(
    (() => {
      if (selectedScope?.scopeKind !== 'shielded') return false;
      if (isDlightShieldedScope && dlightRuntimeStatus) {
        const runtimeReady = dlightRuntimeStatus.statusKind.trim().toLowerCase() === 'synced';
        const spendReady = dlightRuntimeStatus.spendCacheReady === true;
        return !(runtimeReady && spendReady);
      }
      return (
        selectedSyncPercent !== null && selectedSyncPercent !== 100 && selectedSyncPercent !== -1
      );
    })()
  );
  const balanceSyncReady = $derived(
    !isDlightShieldedScope
      ? true
      : dlightRuntimeStatus?.statusKind?.trim().toLowerCase() === 'synced' ||
          (selectedBalanceSyncPercent !== null && selectedBalanceSyncPercent >= 100)
  );
  const sendSyncReady = $derived(
    !isDlightShieldedScope
      ? true
      : dlightRuntimeStatus?.spendCacheReady === true ||
          (selectedSendSyncPercent !== null && selectedSendSyncPercent >= 100)
  );
  const showBalanceSyncProgress = $derived(isDlightShieldedScope && !balanceSyncReady);
  const balanceSyncPercentDisplay = $derived(formatSyncPercentLabel(selectedBalanceSyncPercent));
  const sendSyncPercentDisplay = $derived(formatSyncPercentLabel(selectedSendSyncPercent));
  const showSendSyncProgressHelper = $derived(
    isDlightShieldedScope && !selectedScope?.isReadOnly && !sendSyncReady
  );
  const privateSendEtaSeconds = $derived(
    (() => {
      const lagBlocks = toFiniteNumber(dlightRuntimeStatus?.spendCacheLagBlocks);
      if (lagBlocks === null || lagBlocks <= 0) return null;
      if (spendRateBlocksPerSec === null || spendRateBlocksPerSec <= 0) return null;
      return lagBlocks / spendRateBlocksPerSec;
    })()
  );
  const privateSendEtaDisplay = $derived(
    privateSendEtaSeconds === null
      ? null
      : i18n.t('wallet.assetDetails.privateSendEtaMinutes', {
          minutes: formatEtaMinutes(privateSendEtaSeconds),
        })
  );
  const privateSendEtaOrPlaceholder = $derived(
    privateSendEtaDisplay ?? i18n.t('wallet.assetDetails.privateSendEtaPending')
  );
  const canSendOrConvert = $derived(
    !!selectedScope && !selectedScope.isReadOnly && !isShieldedSyncBlocked
  );
  const selectedNetworkDisplay = $derived(
    (() => {
      if (selectedScope) return networkLabelForScope(selectedScope);
      if (allScopes.length > 0) return networkLabelForScope(allScopes[0]);
      return '—';
    })()
  );
  const isSingleAddressExternalAsset = $derived(
    (() => {
      if (!coin) return false;
      if (coin.proto !== 'eth' && coin.proto !== 'erc20' && coin.proto !== 'btc') return false;
      const uniqueAddresses = new Set(allScopes.map((scope) => scope.address));
      return uniqueAddresses.size <= 1;
    })()
  );
  const useStaticAddressRow = $derived(
    isSingleAddressExternalAsset || scopeFilterMode === 'shielded'
  );
  const selectedFqnDisplay = $derived(
    (() => {
      const runtimeCoin = coin as (typeof coin & { fullyQualifiedName?: string | null }) | null;
      const candidates = [
        runtimeCoin?.fullyQualifiedName ?? null,
        coinPresentation?.displayTicker ?? null,
        coin?.displayTicker ?? null,
      ]
        .map((value) => (typeof value === 'string' ? value.trim() : ''))
        .filter((value) => value.length > 0);
      const dotted = candidates.find((value) => value.includes('.'));
      return dotted ?? candidates[0] ?? '';
    })()
  );
  const headerDisplayName = $derived(
    entryDisplayName?.trim() || coinPresentation?.displayName || coin?.displayName || ''
  );
  const headerFqnDisplay = $derived(entryDisplayName?.trim() ? '' : selectedFqnDisplay);
  const usePrivateMutedIcon = $derived(scopeFilterMode === 'shielded');
  const usePrivateWordmark = $derived(walletEntryKind === 'private_verus');

  $effect(() => {
    coinId;
    scopeFilterMode;
    addressSearchTerm = '';
    showScopeSheet = false;
    scopesError = '';
    void loadScopes();
  });

  $effect(() => {
    if (!useStaticAddressRow) return;
    showScopeSheet = false;
    addressSearchTerm = '';
  });

  $effect(() => {
    const scopes = allScopes;
    if (scopes.length === 0) return;

    const preferredAddress = selectedAddressMap[coinId];
    let nextAddress =
      preferredAddress && scopes.some((scope) => scope.address === preferredAddress)
        ? preferredAddress
        : '';
    if (!nextAddress) {
      nextAddress = scopes.find((scope) => scope.isPrimaryAddress)?.address ?? scopes[0].address;
      setSelectedScopeAddress(coinId, nextAddress);
    }

    const systemsForAddress = scopes
      .filter((scope) => scope.address === nextAddress)
      .map((scope) => scope.systemId);
    const preferredSystem = selectedSystemMap[coinId];
    let nextSystem =
      preferredSystem && systemsForAddress.includes(preferredSystem) ? preferredSystem : '';
    if (!nextSystem) {
      const rootSystemId = coin?.systemId ?? '';
      nextSystem = systemsForAddress.includes(rootSystemId)
        ? rootSystemId
        : (systemsForAddress[0] ?? '');
      if (nextSystem) {
        setSelectedScopeSystem(coinId, nextSystem);
      }
    }
  });

  $effect(() => {
    const activeScope = selectedScope;
    const currentCoin = coin;
    if (!activeScope || !currentCoin) return;

    const cachedBalance = getBalance(activeScope.channelId, currentCoin.id, get(balanceStore));
    const requestKey = `${activeScope.channelId}::${currentCoin.id}`;
    selectedBalanceRequestSequence += 1;
    const requestSequence = selectedBalanceRequestSequence;
    loadingSelectedBalance = !cachedBalance;

    void (async () => {
      await fetchBalanceForScope(activeScope, currentCoin.id);
      if (
        selectedBalanceRequestSequence === requestSequence &&
        requestKey === `${activeScope.channelId}::${currentCoin.id}`
      ) {
        loadingSelectedBalance = false;
      }
    })();
  });

  $effect(() => {
    const scope = selectedScope;
    const currentCoin = coin;
    let intervalId: ReturnType<typeof setInterval> | null = null;
    let cancelled = false;

    if (
      !scope ||
      !currentCoin ||
      scope.scopeKind !== 'shielded' ||
      !scope.channelId.toLowerCase().startsWith('dlight_private.')
    ) {
      dlightRuntimeStatus = null;
      spendRateBlocksPerSec = null;
      spendRateSample = null;
    } else {
      const channelId = scope.channelId;
      const currentCoinId = currentCoin.id;

      const refreshStatus = async () => {
        try {
          const status = await walletService.getDlightRuntimeStatus(channelId, currentCoinId);
          if (cancelled) return;
          dlightRuntimeStatus = status;
          updateSpendRate(status);
        } catch {
          if (cancelled) return;
          dlightRuntimeStatus = null;
        }
      };

      void refreshStatus();
      intervalId = globalThis.setInterval(() => {
        void refreshStatus();
      }, DLIGHT_STATUS_POLL_MS);
    }

    return () => {
      cancelled = true;
      if (intervalId !== null) globalThis.clearInterval(intervalId);
    };
  });

  $effect(() => {
    const currentCoin = coin;
    const activeScope = selectedScope;
    const address = selectedAddress;
    if (!currentCoin || !activeScope || !address) return;

    const siblingScopes = allScopes.filter(
      (scope) => scope.address === address && scope.channelId !== activeScope.channelId
    );
    if (siblingScopes.length === 0) return;
    void fetchSiblingBalances(siblingScopes, currentCoin.id);
  });

  $effect(() => {
    const currentCoin = coin;
    if (!currentCoin || allScopes.length === 0) return;
    void fetchSiblingBalances(allScopes, currentCoin.id);
  });

  $effect(() => {
    const currentCoin = coin;
    const activeScope = selectedScope;
    selectedScopeTxPage?.initialLoaded;
    selectedScopeTxPage?.loadingInitial;
    if (!currentCoin || !activeScope) return;
    void ensureInitialTransactionsForScope(activeScope, currentCoin.id);
  });

  $effect(() => {
    sortedSelectedTransactions.length;
    loadingSelectedTransactions;
    loadingMoreTransactions;
    selectedScopeHasMoreTransactions;
    const element = txScrollElement;
    if (!element) return () => {};

    const resizeObserver = new ResizeObserver(() => {
      updateTxScrollAffordance();
    });
    resizeObserver.observe(element);
    const viewportContent = element.querySelector('[data-scroll-area-content]');
    if (viewportContent instanceof HTMLElement) {
      resizeObserver.observe(viewportContent);
    } else {
      const fallbackContent = element.lastElementChild;
      if (fallbackContent instanceof HTMLElement) {
        resizeObserver.observe(fallbackContent);
      }
    }
    const frame = window.requestAnimationFrame(() => {
      updateTxScrollAffordance();
    });

    return () => {
      window.cancelAnimationFrame(frame);
      resizeObserver.disconnect();
    };
  });

  async function loadScopes(): Promise<void> {
    const currentCoinId = coinId;
    scopeRequestSequence += 1;
    const requestSequence = scopeRequestSequence;

    scopesLoading = true;
    scopesError = '';
    try {
      const result = await walletDisplayService.getDisplayCoinScopes(currentCoinId);
      if (requestSequence !== scopeRequestSequence) return;
      setCoinScopes(currentCoinId, result.scopes);
    } catch (error) {
      if (requestSequence !== scopeRequestSequence) return;
      scopesError = mapWalletError(error);
    } finally {
      if (requestSequence === scopeRequestSequence) {
        scopesLoading = false;
      }
    }
  }

  async function fetchBalanceForScope(scope: CoinScope, currentCoinId: string): Promise<void> {
    if (inFlightBalanceChannels.has(scope.channelId)) return;
    inFlightBalanceChannels.add(scope.channelId);
    const loadKey = getScopeBalanceLoadKey(scope.channelId, currentCoinId);

    try {
      const balance = await walletDisplayService.getDisplayBalance(scope.channelId, currentCoinId);
      balanceStore.update((state) => ({
        ...state,
        [scope.channelId]: {
          ...(state[scope.channelId] ?? {}),
          [currentCoinId]: balance,
        },
      }));
      loadedBalanceByScopeKey = {
        ...loadedBalanceByScopeKey,
        [loadKey]: true,
      };
    } catch {
      // Balance refresh is best effort for sibling scopes.
    } finally {
      inFlightBalanceChannels.delete(scope.channelId);
    }
  }

  async function fetchSiblingBalances(scopes: CoinScope[], currentCoinId: string): Promise<void> {
    const pendingScopes = scopes.filter(
      (scope) =>
        !loadedBalanceByScopeKey[getScopeBalanceLoadKey(scope.channelId, currentCoinId)] &&
        !inFlightBalanceChannels.has(scope.channelId)
    );
    if (pendingScopes.length === 0) return;

    const concurrency = Math.min(2, pendingScopes.length);
    let cursor = 0;

    const workers = Array.from({ length: concurrency }, async () => {
      while (cursor < pendingScopes.length) {
        const index = cursor;
        cursor += 1;
        const scope = pendingScopes[index];
        if (!scope) return;
        await fetchBalanceForScope(scope, currentCoinId);
      }
    });

    await Promise.all(workers);
  }

  function getScopePageKey(channelId: string, currentCoinId: string): string {
    return `${channelId}::${currentCoinId}`;
  }

  function getScopeBalanceLoadKey(channelId: string, currentCoinId: string): string {
    return `${channelId}::${currentCoinId}`;
  }

  function dedupeTransactions(items: Transaction[]): Transaction[] {
    const seen = new Set<string>();
    const out: Transaction[] = [];
    for (const item of items) {
      if (seen.has(item.txid)) continue;
      seen.add(item.txid);
      out.push(item);
    }
    return out;
  }

  function isCurrentHistorySelection(scope: CoinScope, currentCoinId: string): boolean {
    return (
      componentActive && selectedScope?.channelId === scope.channelId && coin?.id === currentCoinId
    );
  }

  async function ensureInitialTransactionsForScope(
    scope: CoinScope,
    currentCoinId: string
  ): Promise<void> {
    const scopePageKey = getScopePageKey(scope.channelId, currentCoinId);
    const existing = txPagesByScopeKey[scopePageKey];
    if (existing?.initialLoaded || existing?.loadingInitial) return;
    if (inFlightTransactionScopeKeys.has(scopePageKey)) return;

    inFlightTransactionScopeKeys.add(scopePageKey);
    updateTransactionHistoryPage(scopePageKey, (state) => ({
      ...state,
      loadingInitial: true,
      loadingMore: false,
      error: '',
      loadMoreError: '',
    }));

    try {
      const page = await loadHistoryPageWithInvalidationRetry({
        load: () =>
          walletDisplayService.getDisplayTransactionHistoryPage(
            scope.channelId,
            currentCoinId,
            undefined,
            TRANSACTION_PAGE_SIZE
          ),
        isInvalidated: walletDisplayService.isWalletDisplayRequestInvalidated,
        shouldRetry: () => isCurrentHistorySelection(scope, currentCoinId),
      });
      if (!page) return;
      updateTransactionHistoryPage(scopePageKey, (state) => ({
        ...state,
        items: dedupeTransactions(page.transactions),
        nextCursor: page.nextCursor ?? null,
        hasMore: page.hasMore,
        initialLoaded: true,
        loadingInitial: false,
        loadingMore: false,
        error: '',
        loadMoreError: '',
      }));
    } catch (error) {
      if (walletDisplayService.isWalletDisplayRequestInvalidated(error)) return;
      updateTransactionHistoryPage(scopePageKey, (state) => ({
        ...state,
        loadingInitial: false,
        initialLoaded: false,
        error: mapWalletError(error),
      }));
    } finally {
      inFlightTransactionScopeKeys.delete(scopePageKey);
      updateTxScrollAffordance();
    }
  }

  async function loadMoreTransactionsForScope(
    scope: CoinScope,
    currentCoinId: string
  ): Promise<void> {
    const scopePageKey = getScopePageKey(scope.channelId, currentCoinId);
    const state = txPagesByScopeKey[scopePageKey];
    if (!state?.initialLoaded) return;
    if (state.loadingInitial || state.loadingMore) return;
    if (!state.hasMore || !state.nextCursor) return;
    if (inFlightTransactionScopeKeys.has(scopePageKey)) return;

    inFlightTransactionScopeKeys.add(scopePageKey);
    updateTransactionHistoryPage(scopePageKey, (previous) => ({
      ...previous,
      loadingMore: true,
      loadMoreError: '',
    }));

    let reloadInitialAfterInvalidation = false;
    try {
      const page = await walletDisplayService.getDisplayTransactionHistoryPage(
        scope.channelId,
        currentCoinId,
        state.nextCursor,
        TRANSACTION_PAGE_SIZE
      );
      updateTransactionHistoryPage(scopePageKey, (previous) => ({
        ...previous,
        items: dedupeTransactions([...previous.items, ...page.transactions]),
        nextCursor: page.nextCursor ?? null,
        hasMore: page.hasMore,
        loadingMore: false,
        loadMoreError: '',
      }));
    } catch (error) {
      if (walletDisplayService.isWalletDisplayRequestInvalidated(error)) {
        reloadInitialAfterInvalidation = isCurrentHistorySelection(scope, currentCoinId);
        return;
      }
      updateTransactionHistoryPage(scopePageKey, (previous) => ({
        ...previous,
        loadingMore: false,
        loadMoreError: mapWalletError(error),
      }));
    } finally {
      inFlightTransactionScopeKeys.delete(scopePageKey);
      if (reloadInitialAfterInvalidation && isCurrentHistorySelection(scope, currentCoinId)) {
        void ensureInitialTransactionsForScope(scope, currentCoinId);
      }
      updateTxScrollAffordance();
    }
  }

  async function retryTransactions(): Promise<void> {
    const activeScope = selectedScope;
    const currentCoin = coin;
    if (!activeScope || !currentCoin) return;
    const scopePageKey = getScopePageKey(activeScope.channelId, currentCoin.id);
    updateTransactionHistoryPage(scopePageKey, () => createEmptyTransactionPageState());
    await ensureInitialTransactionsForScope(activeScope, currentCoin.id);
  }

  async function retryLoadMoreTransactions(): Promise<void> {
    const activeScope = selectedScope;
    const currentCoin = coin;
    if (!activeScope || !currentCoin) return;
    await loadMoreTransactionsForScope(activeScope, currentCoin.id);
  }

  function selectScope(scope: CoinScope): void {
    setSelectedScopeAddress(coinId, scope.address);
    setSelectedScopeSystem(coinId, scope.systemId);
    showScopeSheet = false;
    addressSearchTerm = '';
  }

  async function copyAddress(address: string, key: string): Promise<void> {
    if (!address) return;
    if (await writeClipboardText(address)) {
      copiedAddressState.set(key, 1800);
      return;
    }
    copiedAddressState.clear();
  }

  function updateTxScrollAffordance(): void {
    if (!txScrollElement) {
      canScrollTxDown = false;
      return;
    }
    const maxScrollTop = Math.max(0, txScrollElement.scrollHeight - txScrollElement.clientHeight);
    canScrollTxDown = maxScrollTop > 1 && txScrollElement.scrollTop < maxScrollTop - 1;
  }

  function onTxScroll(event: Event): void {
    const target = event.currentTarget;
    if (!(target instanceof HTMLElement)) return;
    const maxScrollTop = Math.max(0, target.scrollHeight - target.clientHeight);
    canScrollTxDown = maxScrollTop > 1 && target.scrollTop < maxScrollTop - 1;
  }

  function toTransferContext(scope: CoinScope): TransferEntryContext {
    return {
      coinId,
      channelId: scope.channelId,
      readOnly: scope.isReadOnly,
      scopeKind: scope.scopeKind,
    };
  }

  function formatCryptoAmount(value: number, ticker: string, decimals: number): string {
    const maxFractionDigits = Math.max(2, Math.min(8, decimals));
    const minimumFractionDigits = value === 0 ? 0 : Math.min(2, maxFractionDigits);
    const formatted = i18n.formatNumber(value, {
      minimumFractionDigits,
      maximumFractionDigits: maxFractionDigits,
    });
    return `${formatted} ${ticker}`;
  }

  function formatCryptoValue(value: number, decimals: number): string {
    const maxFractionDigits = Math.max(2, Math.min(8, decimals));
    const minimumFractionDigits = value === 0 ? 0 : Math.min(2, maxFractionDigits);
    return i18n.formatNumber(value, {
      minimumFractionDigits,
      maximumFractionDigits: maxFractionDigits,
    });
  }

  function getChangeDirection(changePct: number | null): 'up' | 'down' | 'flat' | 'none' {
    if (changePct === null) return 'none';
    if (Math.abs(changePct) < 0.01) return 'flat';
    if (changePct > 0) return 'up';
    return 'down';
  }

  function formatPercentChange(changePct: number): string {
    const formatted = i18n.formatNumber(Math.abs(changePct), {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    });
    if (changePct > 0) return `+${formatted}%`;
    if (changePct < 0) return `-${formatted}%`;
    return `${formatted}%`;
  }

  function formatSyncPercentLabel(percent: number | null): string {
    const value = percent === null ? 0 : Math.max(0, Math.min(percent, 100));
    const floored = Math.floor(value * 10) / 10;
    return i18n.formatNumber(floored, {
      minimumFractionDigits: 1,
      maximumFractionDigits: 1,
    });
  }

  function formatEtaMinutes(seconds: number): string {
    const minutes = Math.max(1, Math.ceil(seconds / 60));
    return i18n.formatNumber(minutes, {
      maximumFractionDigits: 0,
    });
  }

  function updateSpendRate(status: DlightRuntimeStatusResult): void {
    const scannedHeight = toFiniteNumber(status.spendCacheScannedHeight);
    const updatedAt = toFiniteNumber(status.spendCacheLastUpdated);
    if (scannedHeight === null || updatedAt === null) return;

    const previous = spendRateSample;
    if (previous && updatedAt > previous.updatedAt) {
      const deltaHeight = scannedHeight - previous.scannedHeight;
      const deltaSeconds = updatedAt - previous.updatedAt;
      if (deltaHeight > 0 && deltaSeconds > 0) {
        const instantaneousRate = deltaHeight / deltaSeconds;
        if (Number.isFinite(instantaneousRate) && instantaneousRate > 0) {
          spendRateBlocksPerSec =
            spendRateBlocksPerSec === null
              ? instantaneousRate
              : spendRateBlocksPerSec * (1 - SPEND_RATE_SMOOTHING) +
                instantaneousRate * SPEND_RATE_SMOOTHING;
        }
      }
    }

    spendRateSample = {
      scannedHeight,
      updatedAt,
    };
  }

  function toFiniteNumber(value: unknown): number | null {
    if (typeof value === 'number') {
      return Number.isFinite(value) ? value : null;
    }

    if (typeof value === 'string') {
      const parsed = Number(value.trim());
      return Number.isFinite(parsed) ? parsed : null;
    }

    return null;
  }

  function transactionDirection(transaction: Transaction): 'in' | 'out' {
    const amount = toFiniteNumber(transaction.amount) ?? 0;
    return amount >= 0 ? 'in' : 'out';
  }

  function transactionAmountDisplay(transaction: Transaction): string {
    const amount = Math.abs(toFiniteNumber(transaction.amount) ?? 0);
    const ticker = coinPresentation?.displayTicker ?? coin?.displayTicker ?? coinId;
    return `${transactionDirection(transaction) === 'in' ? '+' : '-'}${formatCryptoAmount(amount, ticker, coin?.decimals ?? 8)}`;
  }

  function transactionCounterpartyRaw(transaction: Transaction): string {
    const direction = transactionDirection(transaction);
    return direction === 'in' ? transaction.fromAddress.trim() : transaction.toAddress.trim();
  }

  function transactionCounterparty(transaction: Transaction): string {
    const direction = transactionDirection(transaction);
    const counterparty = transactionCounterpartyRaw(transaction);
    if (!counterparty) {
      return direction === 'in'
        ? i18n.t('wallet.assetDetails.receivedFallback')
        : i18n.t('wallet.assetDetails.sentFallback');
    }

    return truncateMiddle(counterparty, 10, 10);
  }

  function transactionCounterpartyIsAddress(transaction: Transaction): boolean {
    return transactionCounterpartyRaw(transaction).length > 0;
  }

  function truncateMiddle(value: string, start = 8, end = 8): string {
    if (value.length <= start + end + 3) return value;
    return `${value.slice(0, start)}...${value.slice(-end)}`;
  }

  function formatTimestamp(transaction: Transaction): string {
    if (transaction.pending) return i18n.t('wallet.assetDetails.pendingLabel');

    const timestamp = transaction.timestamp;
    if (!timestamp || timestamp <= 0) return '—';

    return i18n.formatDate(timestamp * 1000, {
      day: '2-digit',
      month: 'short',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function networkLabelForScope(scope: CoinScope): string {
    const ticker = scope.systemTicker.trim();
    if (ticker) return ticker;
    const displayName = scope.systemDisplayName.trim();
    return displayName || '—';
  }

  function preferredScopeDisplayValue(scope: CoinScope): string {
    const label = scope.addressLabel.trim();
    if (label.endsWith('@')) return label;
    return scope.address;
  }

  function scopeAmountValue(scope: CoinScope): number | null {
    if (!coin) return null;
    return toFiniteNumber(getBalance(scope.channelId, coin.id, balances)?.total);
  }

  function scopeCryptoAmountDisplay(scope: CoinScope): string {
    const value = scopeAmountValue(scope);
    if (value === null) return '—';
    const ticker = coinPresentation?.displayTicker ?? coin?.displayTicker ?? coinId;
    return formatCryptoAmount(value, ticker, coin?.decimals ?? 8);
  }

  function scopeFiatAmountDisplay(scope: CoinScope): string {
    const value = scopeAmountValue(scope);
    if (value === null || selectedFiatRate === null) return '—';
    return formatFiatAmount(value * selectedFiatRate, i18n.intlLocale, displayCurrency);
  }

  function mapWalletError(error: unknown): string {
    const errorType = extractWalletErrorType(error);
    const rawMessage = extractWalletErrorMessage(error);

    if (errorType === 'OperationFailed') {
      if (rawMessage && rawMessage.toLowerCase() !== 'operation failed') return rawMessage;
    }

    if (rawMessage) return rawMessage;

    return i18n.t('common.unknownError');
  }
</script>

<div class="mx-auto flex h-full min-h-0 w-full max-w-6xl flex-col px-6 pt-3 pb-6 sm:px-8">
  <button
    type="button"
    class="mb-2 inline-flex w-fit items-center gap-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
    onclick={onBack}
  >
    <ArrowLeftIcon class="size-4" aria-hidden="true" />
    {i18n.t('common.back')}
  </button>

  <section class="flex min-h-0 flex-1 flex-col overflow-hidden">
    {#if scopesLoading}
      <div class="space-y-4 pt-3">
        <Skeleton class="h-12 w-56 rounded-lg" />
        <Skeleton class="h-9 w-32 rounded-lg" />
        <Skeleton class="h-10 w-full rounded-lg" />
        <Skeleton class="h-10 w-full rounded-lg" />
        <Skeleton class="h-48 w-full rounded-xl" />
      </div>
    {:else if scopesError}
      <div class="mt-4 rounded-lg bg-destructive/10 px-4 py-3 text-sm text-destructive">
        <p>{i18n.t('wallet.assetDetails.errorLoadScopes')}</p>
        <p class="mt-1 text-xs">{scopesError}</p>
        <Button variant="secondary" size="sm" class="mt-3" onclick={loadScopes}>
          {i18n.t('common.retry')}
        </Button>
      </div>
    {:else if !coin || !selectedScope}
      <div class="mt-8 rounded-lg bg-muted/45 px-4 py-5 text-sm text-muted-foreground">
        {i18n.t('wallet.assetDetails.scopeUnavailable')}
      </div>
    {:else}
      <div class="flex min-h-0 flex-1 flex-col gap-4 pt-0">
        <div class="px-0 py-1">
          <div class="flex items-start justify-between gap-4">
            <div class="flex min-w-0 items-center gap-3">
              <CoinIcon
                coinId={coin.id}
                coinName={coinPresentation?.displayName}
                proto={coin.proto}
                size={44}
                showBadge
                privateMuted={usePrivateMutedIcon}
                decorative
              />
              <div class="min-w-0">
                <p class="truncate text-lg font-semibold">
                  {#if usePrivateWordmark}
                    <PrivateVerusWordmark label={headerDisplayName} />
                  {:else}
                    {headerDisplayName}
                  {/if}
                  {#if headerFqnDisplay}
                    <span class="ml-2 text-sm font-medium text-muted-foreground"
                      >{headerFqnDisplay}</span
                    >
                  {/if}
                </p>
                <div class="mt-0.5 flex items-center gap-2">
                  <p class="text-sm text-muted-foreground">{selectedUnitRateDisplay}</p>
                  <p
                    class={`text-sm font-semibold ${
                      selectedChange24hDirection === 'up'
                        ? 'text-emerald-700 dark:text-emerald-300'
                        : selectedChange24hDirection === 'down'
                          ? 'text-destructive'
                          : 'text-muted-foreground'
                    }`}
                  >
                    {selectedChange24hDisplay}
                  </p>
                </div>
              </div>
            </div>

            <div class="relative shrink-0 text-right">
              {#if showBalanceSyncProgress}
                <p class="text-2xl leading-tight font-semibold tracking-tight text-foreground">
                  <span class="inline-flex items-center justify-end gap-2">
                    <span class="font-mono text-sm font-semibold tracking-tight tabular-nums">
                      {balanceSyncPercentDisplay}%
                    </span>
                    <Spinner class="size-4" />
                  </span>
                </p>
                <p class="mt-1 text-[11px] text-muted-foreground">
                  {i18n.t('wallet.assetDetails.privateSyncInlineHelper')}
                </p>
              {:else}
                <p class="text-2xl leading-tight font-semibold tracking-tight text-foreground">
                  {totalCryptoAmountDisplay}
                </p>
                <p class="mt-1 text-sm leading-tight text-muted-foreground">{totalFiatDisplay}</p>
              {/if}
              {#if loadingSelectedBalance}
                <div
                  class="pointer-events-none absolute top-full right-0 mt-1 flex items-center justify-end"
                >
                  <Skeleton class="h-[11px] w-14 rounded-sm" />
                  <span class="sr-only">{i18n.t('common.loading')}</span>
                </div>
              {/if}
            </div>
          </div>

          <div class="mt-8 flex items-center gap-2">
            {#if useStaticAddressRow}
              <div
                class="flex h-[52px] min-w-0 flex-1 items-center justify-between gap-2 rounded-md bg-muted/55 pr-1.5 pl-3"
              >
                <p class="identifier-text truncate text-sm font-medium text-foreground">
                  {truncateMiddle(selectedScopeDisplayAddress || '—', 10, 10)}
                </p>
                <CopyButton
                  copied={copiedAddressKey === 'selected-static'}
                  size="sm"
                  class="-mr-0.5"
                  onclick={() => copyAddress(selectedScopeDisplayAddress, 'selected-static')}
                  title={i18n.t('wallet.receive.copy')}
                  aria-label={i18n.t('wallet.receive.copy')}
                />
              </div>
            {:else}
              <div
                class="flex h-[52px] min-w-0 flex-1 items-center gap-1 rounded-md bg-primary pr-1 pl-1.5"
              >
                <button
                  type="button"
                  class="flex min-w-0 flex-1 items-center justify-between gap-2 rounded-md px-2 py-1 text-left transition-colors hover:bg-primary/90 focus-visible:ring-2 focus-visible:ring-primary-foreground/60 focus-visible:outline-none"
                  aria-label={i18n.t('wallet.assetDetails.scopePicker')}
                  title={i18n.t('wallet.assetDetails.scopePicker')}
                  onclick={() => (showScopeSheet = true)}
                >
                  <div class="min-w-0 flex-1 text-left">
                    <p class="identifier-text truncate text-sm font-medium text-primary-foreground">
                      {truncateMiddle(selectedScopeDisplayAddress || '—', 10, 10)}
                      <span class="ml-1.5 font-normal text-primary-foreground/80"
                        >• {selectedNetworkDisplay}</span
                      >
                    </p>
                    <p class="mt-0.5 truncate text-xs text-primary-foreground/80">
                      {selectedCryptoAmountDisplay}
                      <span class="mx-1.5">•</span>
                      {selectedFiatDisplay}
                    </p>
                  </div>
                  <ChevronDownIcon class="h-4 w-4 shrink-0 text-primary-foreground/80" />
                </button>
                <CopyButton
                  copied={copiedAddressKey === 'selected-interactive'}
                  variant="inverse"
                  size="sm"
                  class="-mr-0.5"
                  onclick={() => copyAddress(selectedScopeDisplayAddress, 'selected-interactive')}
                  title={i18n.t('wallet.receive.copy')}
                  aria-label={i18n.t('wallet.receive.copy')}
                  copiedIconClass="size-4 text-emerald-300 dark:text-emerald-200"
                />
              </div>
            {/if}

            <div class="flex items-center gap-2">
              <Button
                variant="secondary"
                size="icon-lg"
                class="size-[52px] rounded-md"
                aria-label={i18n.t('wallet.overview.receive')}
                title={i18n.t('wallet.overview.receive')}
                onclick={onNavigateToReceive}
              >
                <DownloadIcon class="h-[18px] w-[18px]" />
              </Button>
              <Button
                variant="secondary"
                size="icon-lg"
                class="size-[52px] rounded-md"
                disabled={!canSendOrConvert}
                aria-label={i18n.t('wallet.overview.send')}
                title={i18n.t('wallet.overview.send')}
                onclick={() => {
                  if (!selectedScope || !canSendOrConvert) return;
                  onNavigateToSend(toTransferContext(selectedScope));
                }}
              >
                <SendIcon class="h-[18px] w-[18px]" />
              </Button>
              <Button
                variant="secondary"
                size="icon-lg"
                class="size-[52px] rounded-md"
                disabled={!canSendOrConvert}
                aria-label={i18n.t('wallet.overview.convert')}
                title={i18n.t('wallet.overview.convert')}
                onclick={() => {
                  if (!selectedScope || !canSendOrConvert) return;
                  onNavigateToConvert(toTransferContext(selectedScope));
                }}
              >
                <ArrowLeftRightIcon class="h-[18px] w-[18px]" />
              </Button>
            </div>
          </div>

          {#if showSendSyncProgressHelper}
            <p class="mt-2 text-right text-[11px] leading-snug text-muted-foreground">
              <span class="inline-flex items-center justify-end gap-1.5 tabular-nums">
                <span>
                  {i18n.t('wallet.assetDetails.sendCapabilityInline', {
                    percent: sendSyncPercentDisplay,
                  })}
                </span>
                <span aria-hidden="true" class="text-muted-foreground/70">·</span>
                <span>
                  {i18n.t('wallet.assetDetails.privateSendEtaLine', {
                    eta: privateSendEtaOrPlaceholder,
                  })}
                </span>
                <Spinner class="size-3" />
              </span>
            </p>
          {:else if !canSendOrConvert && !isShieldedSyncBlocked}
            <p class="mt-2 text-right text-[11px] leading-snug text-muted-foreground">
              {i18n.t('wallet.assetDetails.readOnlyHelper')}
            </p>
          {/if}
        </div>

        <div class="relative flex min-h-0 flex-1 flex-col">
          <div class="px-0 pt-2 pb-2">
            <p class="text-sm font-medium">{i18n.t('wallet.assetDetails.transactions')}</p>
          </div>

          <ScrollArea.Root class="h-full min-h-0 flex-1" type="scroll">
            <ScrollArea.Viewport
              class="asset-tx-scroll h-full overscroll-contain pr-5"
              bind:ref={txScrollElement}
              onscroll={onTxScroll}
            >
              {#if loadingSelectedTransactions && sortedSelectedTransactions.length === 0}
                <ul class="space-y-2 py-2 pr-1">
                  {#each initialTransactionSkeletonRows as skeletonRow (skeletonRow)}
                    <li class="flex items-center justify-between rounded-md py-2">
                      <div class="min-w-0 flex-1">
                        <Skeleton class="h-4 w-40 rounded-sm" />
                        <Skeleton class="mt-2 h-3 w-28 rounded-sm" />
                      </div>
                      <div class="ml-4 min-w-[9rem] text-right">
                        <Skeleton class="ml-auto h-4 w-24 rounded-sm" />
                        <Skeleton class="mt-2 ml-auto h-3 w-20 rounded-sm" />
                      </div>
                    </li>
                  {/each}
                </ul>
              {:else if selectedScopeTransactionsError}
                <div class="py-5">
                  <p class="text-sm text-destructive">
                    {i18n.t('wallet.assetDetails.errorLoadTransactions')}
                  </p>
                  <p class="mt-1 text-xs text-muted-foreground">{selectedScopeTransactionsError}</p>
                  <Button variant="secondary" size="sm" class="mt-3" onclick={retryTransactions}>
                    {i18n.t('common.retry')}
                  </Button>
                </div>
              {:else if sortedSelectedTransactions.length === 0}
                <p class="py-6 text-sm text-muted-foreground">
                  {selectedScopeHasMoreTransactions
                    ? i18n.t('wallet.assetDetails.noTransactionsInRecentRange')
                    : i18n.t('wallet.assetDetails.noTransactionsForScope')}
                </p>
              {:else}
                <ul class="space-y-1.5 py-2 pr-1">
                  {#each sortedSelectedTransactions as transaction (transaction.txid)}
                    <li
                      class="flex items-center justify-between rounded-md px-0 py-2 hover:bg-muted/45"
                    >
                      <div class="min-w-0">
                        <p
                          class={`truncate text-sm font-medium ${transactionCounterpartyIsAddress(transaction) ? 'identifier-text' : ''}`}
                        >
                          {transactionCounterparty(transaction)}
                        </p>
                        <p class="mt-0.5 text-xs text-muted-foreground">
                          {formatTimestamp(transaction)}
                        </p>
                      </div>
                      <div class="text-right">
                        <p
                          class={`text-sm font-semibold ${transactionDirection(transaction) === 'in' ? 'text-emerald-700 dark:text-emerald-300' : 'text-foreground'}`}
                        >
                          {transactionAmountDisplay(transaction)}
                        </p>
                        <p class="identifier-text mt-0.5 text-[11px] text-muted-foreground">
                          {truncateMiddle(transaction.txid, 8, 8)}
                        </p>
                      </div>
                    </li>
                  {/each}
                </ul>
              {/if}

              {#if loadingMoreTransactions}
                <ul class="space-y-2 pr-1 pb-2">
                  {#each loadMoreTransactionSkeletonRows as skeletonRow (skeletonRow)}
                    <li class="flex items-center justify-between rounded-md py-2">
                      <div class="min-w-0 flex-1">
                        <Skeleton class="h-4 w-36 rounded-sm" />
                        <Skeleton class="mt-2 h-3 w-24 rounded-sm" />
                      </div>
                      <div class="ml-4 min-w-[8.5rem] text-right">
                        <Skeleton class="ml-auto h-4 w-20 rounded-sm" />
                        <Skeleton class="mt-2 ml-auto h-3 w-16 rounded-sm" />
                      </div>
                    </li>
                  {/each}
                </ul>
              {/if}

              {#if selectedScopeLoadMoreError}
                <div class="pr-1 pb-3">
                  <p class="text-xs text-destructive">
                    {i18n.t('wallet.assetDetails.errorLoadMoreTransactions')}
                  </p>
                  <p class="mt-1 text-xs text-muted-foreground">{selectedScopeLoadMoreError}</p>
                  <Button
                    variant="secondary"
                    size="sm"
                    class="mt-2"
                    onclick={retryLoadMoreTransactions}
                  >
                    {i18n.t('common.retry')}
                  </Button>
                </div>
              {:else if selectedScopeHasMoreTransactions && !loadingMoreTransactions}
                <div class="pt-1 pr-1 pb-3">
                  <Button variant="secondary" size="sm" onclick={retryLoadMoreTransactions}>
                    {i18n.t('wallet.assetDetails.loadOlderTransactions')}
                  </Button>
                </div>
              {/if}
            </ScrollArea.Viewport>
            <ScrollArea.Scrollbar orientation="vertical" />
          </ScrollArea.Root>

          {#if canScrollTxDown}
            <div
              class="pointer-events-none absolute inset-x-0 bottom-0 h-14 bg-gradient-to-t from-background to-transparent dark:from-app-canvas"
            ></div>
          {/if}
        </div>
      </div>
    {/if}
  </section>
</div>

<StandardRightSheet
  bind:isOpen={showScopeSheet}
  title={i18n.t('wallet.assetDetails.scopeSheetTitle')}
>
  <div class="flex h-full min-h-0 flex-col gap-3">
    <SearchInput
      bind:value={addressSearchTerm}
      placeholder={i18n.t('wallet.assetDetails.scopeSearchPlaceholder')}
    />
    <ScrollArea.Root class="min-h-0 flex-1" type="scroll">
      <ScrollArea.Viewport class="h-full pr-1">
        {#if filteredScopeOptions.length === 0}
          <p class="px-1 py-4 text-sm text-muted-foreground">
            {i18n.t('wallet.assetDetails.noScopeMatches')}
          </p>
        {:else}
          <ul class="space-y-1.5 pb-2">
            {#each filteredScopeOptions as scopeOption (scopeOption.channelId)}
              <li>
                <div class="flex items-center gap-1 rounded-md pr-0.5 hover:bg-muted/50">
                  <button
                    type="button"
                    class="flex min-w-0 flex-1 items-center justify-between rounded-md px-3 py-2.5 text-left outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
                    onclick={() => selectScope(scopeOption)}
                  >
                    <div class="min-w-0">
                      <p class="identifier-text truncate text-sm">
                        {truncateMiddle(scopeOption.addressLabel, 10, 10)}
                      </p>
                      <p class="mt-0.5 text-xs text-muted-foreground">
                        {networkLabelForScope(scopeOption)}
                        <span class="mx-1.5">•</span>
                        {scopeCryptoAmountDisplay(scopeOption)}
                        <span class="mx-1.5">•</span>
                        {scopeFiatAmountDisplay(scopeOption)}
                      </p>
                    </div>

                    <div class="flex items-center gap-1.5">
                      {#if scopeOption.address === selectedAddress && scopeOption.systemId === selectedSystem}
                        <CheckIcon class="h-4 w-4 text-primary" />
                      {/if}
                    </div>
                  </button>
                  <CopyButton
                    copied={copiedAddressKey === `scope:${scopeOption.channelId}`}
                    size="sm"
                    class="-mr-0.5"
                    onclick={() =>
                      copyAddress(
                        preferredScopeDisplayValue(scopeOption),
                        `scope:${scopeOption.channelId}`
                      )}
                    title={i18n.t('wallet.receive.copy')}
                    aria-label={i18n.t('wallet.receive.copy')}
                  />
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </ScrollArea.Viewport>
      <ScrollArea.Scrollbar orientation="vertical" />
    </ScrollArea.Root>
  </div>
</StandardRightSheet>

<style>
  .asset-tx-scroll {
    scrollbar-gutter: stable;
  }
</style>
