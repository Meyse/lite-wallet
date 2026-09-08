<!--
  Component: Overview
  Purpose: Wallet overview with hero balances, quick actions, and currency list
  Last Updated: Wallet overview redesign live-data only
  Security: No sensitive operations - display only
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import { Button } from '$lib/components/ui/button';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';
  import SendIcon from '@lucide/svelte/icons/send';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import ArrowLeftRightIcon from '@lucide/svelte/icons/arrow-left-right';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import EyeIcon from '@lucide/svelte/icons/eye';
  import EyeOffIcon from '@lucide/svelte/icons/eye-off';
  import { balanceStore } from '$lib/stores/balances.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { networkStore } from '$lib/stores/network.js';
  import { ratesStore } from '$lib/stores/rates.js';
  import { walletBootstrapStore } from '$lib/stores/walletBootstrap.js';
  import { walletChannelsStore } from '$lib/stores/walletChannels.js';
  import { settingsStore } from '$lib/stores/settings.js';
  import { i18nStore } from '$lib/i18n';
  import {
    buildWalletOverviewViewModel,
    formatCryptoAmount,
    OVERVIEW_UNAVAILABLE_DISPLAY,
    type WalletOverviewRowViewModel
  } from '$lib/utils/walletOverview.js';
  import { formatFiatAmount, formatFiatAmountParts, getRateForCurrency } from '$lib/utils/fiatDisplay.js';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import PrivateVerusWordmark from '$lib/components/wallet/PrivateVerusWordmark.svelte';
  import AddAssetSheet from '$lib/components/wallet/AddAssetSheet.svelte';
  import * as walletService from '$lib/services/walletService';
  import type {
    CoinDefinition,
    CoinScope,
    ScopeKind,
    WalletEntryKind,
    WalletEntrySelection
  } from '$lib/types/wallet';

  interface WalletData {
    name: string;
    emoji: string;
    color: string;
    network?: 'mainnet' | 'testnet';
  }

  const {
    walletData,
    onOpenAssetDetails = () => {},
    onNavigateToSend = () => {},
    onNavigateToReceive = () => {},
    onNavigateToConvert = () => {}
  }: {
    walletData: WalletData;
     
    onOpenAssetDetails?: (_entry: WalletEntrySelection) => void;
    onNavigateToSend?: () => void;
    onNavigateToReceive?: () => void;
    onNavigateToConvert?: () => void;
  } = $props();

  const coins = $derived($coinsStore);
  const i18n = $derived($i18nStore);
  const walletNetwork = $derived(walletData.network ?? 'mainnet');
  const walletChannels = $derived($walletChannelsStore);
  const balances = $derived($balanceStore);
  const chainInfo = $derived($networkStore);
  const rates = $derived($ratesStore);
  const settings = $derived($settingsStore);
  const displayCurrency = $derived(settings.displayCurrency);
  const isBootstrapping = $derived($walletBootstrapStore);
  let showAddAssetSheet = $state(false);
  let listScrollElement = $state<HTMLElement | null>(null);
  let hasOverviewScroll = $state(false);
  let canScrollDown = $state(false);
  let hasSeenScrollHint = $state(false);
  let hideHoldings = $state(false);
  let privateConfigured = $state(false);
  let privateScopes = $state<CoinScope[]>([]);
  let dlightStatusRequestSequence = 0;
  let privateScopesRequestSequence = 0;
  let transparentScopeChannelIdsByCoinId = $state<Record<string, string[]>>({});
  let loadedTransparentAggregateByChannel = $state<Record<string, true>>({});
  const inFlightTransparentAggregateByChannel = new Set<string>();
  const inFlightTransparentScopeCoins = new Set<string>();

  type WalletEntryRow = WalletOverviewRowViewModel & {
    walletEntryKind: WalletEntryKind;
    baseCoinId?: string;
    scopeFilterMode: ScopeKind;
    syncLabel?: string | null;
  };

  const privateBaseCoinId = $derived(walletNetwork === 'testnet' ? 'VRSCTEST' : 'VRSC');
  const privateBaseCoin = $derived(
    coins.find((coin) => coin.id === privateBaseCoinId) ?? null
  );
  const privateLabel = $derived(
    walletNetwork === 'testnet'
      ? i18n.t('wallet.private.label.testnet')
      : i18n.t('wallet.private.label.mainnet')
  );

  function updateScrollAffordance(): void {
    if (!listScrollElement) {
      hasOverviewScroll = false;
      canScrollDown = false;
      return;
    }

    const maxScrollTop = Math.max(0, listScrollElement.scrollHeight - listScrollElement.clientHeight);
    hasOverviewScroll = listScrollElement.scrollTop > 0;
    canScrollDown = maxScrollTop > 1 && listScrollElement.scrollTop < maxScrollTop - 1;
  }

  function onOverviewScroll(event: Event): void {
    const target = event.currentTarget;
    if (!(target instanceof HTMLElement)) return;
    const maxScrollTop = Math.max(0, target.scrollHeight - target.clientHeight);
    hasOverviewScroll = target.scrollTop > 0;
    canScrollDown = maxScrollTop > 1 && target.scrollTop < maxScrollTop - 1;
    if (target.scrollTop > 0) {
      hasSeenScrollHint = true;
    }
  }

  $effect(() => {
    visibleRows.length;
    const element = listScrollElement;
    if (!element) return () => {};

    const resizeObserver = new ResizeObserver(() => {
      updateScrollAffordance();
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
      updateScrollAffordance();
      if (!canScrollDown) {
        hasSeenScrollHint = false;
      }
    });

    return () => {
      window.cancelAnimationFrame(frame);
      resizeObserver.disconnect();
    };
  });

  onMount(() => {
    void loadDlightStatus();
  });

  $effect(() => {
    walletNetwork;
    void loadDlightStatus();
  });

  $effect(() => {
    const baseCoin = privateBaseCoin;
    if (!privateConfigured || !baseCoin) {
      privateScopes = [];
      return;
    }
    void loadPrivateScopes(baseCoin.id);
  });

  $effect(() => {
    const vrpcCoins = coins.filter((coin) => coin.compatibleChannels.includes('vrpc'));
    for (const coin of vrpcCoins) {
      void loadTransparentAggregateBalances(coin);
    }
  });

  const liveOverview = $derived(
    buildWalletOverviewViewModel({
      coins,
      walletChannels,
      balances,
      scopeChannelIdsByCoinId: transparentScopeChannelIdsByCoinId,
      rates,
      intlLocale: i18n.intlLocale,
      displayCurrency,
      network: walletData.network
    })
  );
  const overview = $derived(liveOverview);
  const baseRows = $derived<WalletEntryRow[]>(
    overview.rows.map((row) => ({
      ...row,
      walletEntryKind: 'coin',
      scopeFilterMode: 'transparent'
    }))
  );
  const privateRow = $derived<WalletEntryRow | null>(
    (() => {
      const baseCoin = privateBaseCoin;
      if (!privateConfigured || !baseCoin) return null;

      let totalAmount = 0;
      let hasSnapshot = false;
      for (const scope of privateScopes) {
        const snapshot = balances[scope.channelId]?.[baseCoin.id];
        if (!snapshot) continue;
        const amount = toFiniteNumber(snapshot.total);
        if (amount === null) continue;
        totalAmount += amount;
        hasSnapshot = true;
      }

      const hasBalance = hasSnapshot && totalAmount > 0;
      const rateMetrics = resolveRateMetrics(baseCoin, rates);
      const fiatRate = rateMetrics?.fiatRate ?? null;
      const change24hPct = rateMetrics?.change24hPct ?? null;
      const fiatValue = hasSnapshot && fiatRate !== null ? totalAmount * fiatRate : null;
      const rowFractionDigits = Math.max(0, Math.min(4, baseCoin.decimals));
      const syncSnapshot = getPrivateSyncSnapshot(
        privateScopes,
        baseCoin.systemId,
        chainInfo
      );
      const syncPercent = syncSnapshot.percent;
      const syncLabel =
        syncPercent !== null && syncPercent !== 100 && syncPercent !== -1
          ? i18n.t('wallet.private.syncingPercent', {
              percent: formatPrivateSyncPercent(syncPercent)
            })
          : null;

      return {
        key: `private-${baseCoin.id}`,
        coinId: baseCoin.id,
        proto: baseCoin.proto,
        ticker: baseCoin.displayTicker,
        name: privateLabel,
        hasBalance,
        hasSnapshot,
        cryptoAmountDisplay: hasSnapshot
          ? formatCryptoAmount(
              totalAmount,
              baseCoin.displayTicker,
              i18n.intlLocale,
              rowFractionDigits,
              rowFractionDigits
            )
          : `${OVERVIEW_UNAVAILABLE_DISPLAY} ${baseCoin.displayTicker}`,
        fiatValueDisplay:
          fiatValue === null
            ? OVERVIEW_UNAVAILABLE_DISPLAY
            : formatFiatAmount(fiatValue, i18n.intlLocale, displayCurrency),
        marketPriceDisplay:
          fiatRate === null
            ? OVERVIEW_UNAVAILABLE_DISPLAY
            : formatFiatAmount(fiatRate, i18n.intlLocale, displayCurrency),
        change24hDisplay:
          change24hPct === null ? OVERVIEW_UNAVAILABLE_DISPLAY : formatPercentChange(change24hPct),
        change24hDirection: getChangeDirection(change24hPct),
        unitRateDisplay:
          fiatRate === null ? null : formatFiatAmount(fiatRate, i18n.intlLocale, displayCurrency),
        fiatSortValue: hasBalance && fiatValue !== null ? fiatValue : Number.NEGATIVE_INFINITY,
        walletEntryKind: 'private_verus',
        baseCoinId: baseCoin.id,
        scopeFilterMode: 'shielded',
        syncLabel
      };
    })()
  );
  const visibleRows = $derived<WalletEntryRow[]>(
    (() => {
      const rows = [...baseRows];
      if (!privateRow) return rows;
      const insertAfterIndex = rows.findIndex((row) => row.coinId === privateBaseCoinId);
      if (insertAfterIndex >= 0) {
        rows.splice(insertAfterIndex + 1, 0, privateRow);
      } else {
        rows.push(privateRow);
      }
      return rows;
    })()
  );
  const heroSummary = $derived(
    (() => {
      const rows = visibleRows;
      const hasHoldings = rows.some((row) => row.hasBalance);
      const hasAnyFiatForHoldings = rows.some(
        (row) => row.hasBalance && row.fiatSortValue !== Number.NEGATIVE_INFINITY
      );
      const hasMissingFiatForHoldings = rows.some(
        (row) => row.hasBalance && row.fiatSortValue === Number.NEGATIVE_INFINITY
      );
      const totalFiat = rows
        .filter((row) => row.hasBalance && row.fiatSortValue !== Number.NEGATIVE_INFINITY)
        .reduce((sum, row) => sum + row.fiatSortValue, 0);

      if (hasHoldings && !hasAnyFiatForHoldings) {
        return {
          symbol: '',
          value: OVERVIEW_UNAVAILABLE_DISPLAY,
          hasPartialRates: false
        };
      }

      const parts = formatFiatAmountParts(totalFiat, i18n.intlLocale, displayCurrency);
      return {
        symbol: parts.symbol,
        value: parts.value,
        hasPartialRates: hasHoldings && hasAnyFiatForHoldings && hasMissingFiatForHoldings
      };
    })()
  );
  const rowIconSize = 34;
  const overviewSkeletonRows = [0, 1, 2, 3, 4, 5];

  function getChangeDirection(changePct: number | null): WalletOverviewRowViewModel['change24hDirection'] {
    if (changePct === null) return 'none';
    if (Math.abs(changePct) < 0.01) return 'flat';
    if (changePct > 0) return 'up';
    return 'down';
  }

  function formatPercentChange(changePct: number): string {
    const formatted = i18n.formatNumber(Math.abs(changePct), {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    });
    if (changePct > 0) return `+${formatted}%`;
    if (changePct < 0) return `-${formatted}%`;
    return `${formatted}%`;
  }

  function toFiniteNumber(value: unknown): number | null {
    if (typeof value === 'number') return Number.isFinite(value) ? value : null;
    if (typeof value === 'string') {
      const parsed = Number(value.trim());
      return Number.isFinite(parsed) ? parsed : null;
    }
    return null;
  }

  function resolveRateMetrics(
    coin: CoinDefinition,
    allRates: typeof rates
  ): { fiatRate: number | null; change24hPct: number | null } | null {
    const candidates = [coin.id, coin.currencyId, coin.mappedTo].filter(
      (value): value is string => typeof value === 'string' && value.trim().length > 0
    );
    for (const candidate of candidates) {
      const snapshot = allRates[candidate];
      if (!snapshot) continue;
      const fiatRate = getRateForCurrency(snapshot.rates, displayCurrency);
      const rawChange = snapshot.usdChange24hPct;
      const change24hPct =
        typeof rawChange === 'number' && Number.isFinite(rawChange) ? rawChange : null;
      if (fiatRate !== null || change24hPct !== null) {
        return { fiatRate, change24hPct };
      }
    }
    return null;
  }

  function formatPrivateSyncPercent(percent: number): string {
    const clamped = Math.max(0, Math.min(percent, 100));
    if (clamped > 0 && clamped < 1) {
      return '<1';
    }
    const floored = Math.floor(clamped * 10) / 10;
    return i18n.formatNumber(floored, {
      minimumFractionDigits: 0,
      maximumFractionDigits: 1
    });
  }

  function getPrivateSyncSnapshot(
    scopes: CoinScope[],
    rootSystemId: string,
    infoByChannel: Record<string, { percent?: number }>
  ): { percent: number | null } {
    const rootScope = scopes.find((scope) => scope.systemId === rootSystemId) ?? scopes[0];
    if (!rootScope) {
      return { percent: null };
    }

    const snapshot = infoByChannel[rootScope.channelId];
    return {
      percent: toFiniteNumber(snapshot?.percent)
    };
  }

  async function loadDlightStatus(): Promise<void> {
    dlightStatusRequestSequence += 1;
    const requestSequence = dlightStatusRequestSequence;
    try {
      const status = await walletService.getDlightSeedStatus();
      if (requestSequence !== dlightStatusRequestSequence) return;
      privateConfigured = status.configured;
      if (!status.configured) {
        privateScopes = [];
      }
    } catch {
      if (requestSequence !== dlightStatusRequestSequence) return;
      privateConfigured = false;
      privateScopes = [];
    }
  }

  async function loadPrivateScopes(coinId: string): Promise<void> {
    privateScopesRequestSequence += 1;
    const requestSequence = privateScopesRequestSequence;
    try {
      const result = await walletService.getCoinScopes(coinId);
      if (requestSequence !== privateScopesRequestSequence) return;
      privateScopes = result.scopes.filter((scope) => scope.scopeKind === 'shielded');
    } catch {
      if (requestSequence !== privateScopesRequestSequence) return;
      privateScopes = [];
    }
  }

  async function fetchTransparentAggregateBalance(scope: CoinScope, coinId: string): Promise<void> {
    const channelKey = `${scope.channelId}::${coinId}`;
    if (inFlightTransparentAggregateByChannel.has(channelKey)) return;
    inFlightTransparentAggregateByChannel.add(channelKey);

    try {
      const balance = await walletService.getBalances(scope.channelId, coinId);
      balanceStore.update((state) => ({
        ...state,
        [scope.channelId]: {
          ...(state[scope.channelId] ?? {}),
          [coinId]: balance
        }
      }));
      loadedTransparentAggregateByChannel = {
        ...loadedTransparentAggregateByChannel,
        [channelKey]: true
      };
    } catch {
      // Best effort preload for overview totals.
    } finally {
      inFlightTransparentAggregateByChannel.delete(channelKey);
    }
  }

  async function loadTransparentAggregateBalances(coin: CoinDefinition): Promise<void> {
    if (!coin.compatibleChannels.includes('vrpc')) return;
    if (inFlightTransparentScopeCoins.has(coin.id)) return;
    inFlightTransparentScopeCoins.add(coin.id);

    try {
      const scopeResult = await walletService.getCoinScopes(coin.id);
      const scopes = scopeResult.scopes.filter((scope) => scope.scopeKind === 'transparent');
      transparentScopeChannelIdsByCoinId = {
        ...transparentScopeChannelIdsByCoinId,
        [coin.id]: Array.from(new Set(scopes.map((scope) => scope.channelId)))
      };
      const pendingScopes = scopes.filter((scope) => {
        const channelKey = `${scope.channelId}::${coin.id}`;
        return (
          !loadedTransparentAggregateByChannel[channelKey] &&
          !inFlightTransparentAggregateByChannel.has(channelKey)
        );
      });

      if (pendingScopes.length === 0) return;

      const concurrency = Math.min(3, pendingScopes.length);
      let cursor = 0;
      const workers = Array.from({ length: concurrency }, async () => {
        while (cursor < pendingScopes.length) {
          const next = cursor;
          cursor += 1;
          const scope = pendingScopes[next];
          if (!scope) return;
          await fetchTransparentAggregateBalance(scope, coin.id);
        }
      });

      await Promise.all(workers);
    } catch {
      // Best effort preload for overview totals.
    } finally {
      inFlightTransparentScopeCoins.delete(coin.id);
    }
  }
</script>

<div class="mx-auto flex h-full min-h-0 w-full max-w-6xl flex-col px-6 pb-6 pt-0 sm:px-8">
  <section class="flex min-h-0 flex-1 flex-col overflow-hidden">
    <div
      class={`z-10 bg-background pb-4 pt-3 sm:pt-4 dark:bg-app-canvas ${hasOverviewScroll ? 'overview-scroll-shadow' : ''}`}
    >
      <div class="flex items-start justify-between gap-4">
        <div class="relative z-20 min-w-0">
          {#if isBootstrapping}
            <div class="holdings-obscured-bleed">
              <Skeleton class="h-11 w-44 rounded-md sm:h-12 sm:w-56" />
            </div>
          {:else}
            <div class={`holdings-obscured-bleed flex items-start ${hideHoldings ? 'holdings-obscured' : ''}`}>
              {#if heroSummary.symbol}
                <span
                  class="text-muted-foreground mt-1 mr-1.5 text-xl font-semibold sm:text-2xl"
                >
                  {heroSummary.symbol}
                </span>
              {/if}
              <p class="font-google-sans-17pt text-4xl leading-[1.02] font-semibold tracking-tight sm:text-5xl">
                {heroSummary.value}
              </p>
            </div>
          {/if}
        </div>
        {#if isBootstrapping}
          <Skeleton class="mt-0.5 h-8 w-8 rounded-full" />
        {:else}
          <Button
            variant="ghost"
            size="icon-sm"
            class="text-muted-foreground/85 mt-0.5 rounded-full hover:text-foreground"
            aria-label={hideHoldings ? i18n.t('wallet.overview.showHoldings') : i18n.t('wallet.overview.hideHoldings')}
            title={hideHoldings ? i18n.t('wallet.overview.showHoldings') : i18n.t('wallet.overview.hideHoldings')}
            onclick={() => {
              hideHoldings = !hideHoldings;
            }}
          >
            {#if hideHoldings}
              <EyeIcon class="h-4 w-4" />
            {:else}
              <EyeOffIcon class="h-4 w-4" />
            {/if}
          </Button>
        {/if}
      </div>
      <div class="min-w-0">
        {#if !isBootstrapping && heroSummary.hasPartialRates}
          <p class="text-muted-foreground mt-0.5 text-xs">
            {i18n.t('wallet.overview.partialRatesNotice')}
          </p>
        {/if}
      </div>

      <div class="mt-5 w-full">
        <div class="flex w-full gap-2">
          <div class="grid w-full flex-1 grid-cols-3 gap-2">
            <Button variant="secondary" size="lg" class="h-10 w-full gap-1.5 rounded-md px-3" onclick={onNavigateToReceive}>
              <DownloadIcon class="h-4 w-4" />
              <span>{i18n.t('wallet.overview.receive')}</span>
            </Button>
            <Button variant="secondary" size="lg" class="h-10 w-full gap-1.5 rounded-md px-3" onclick={onNavigateToSend}>
              <SendIcon class="h-4 w-4" />
              <span>{i18n.t('wallet.overview.send')}</span>
            </Button>
            <Button variant="secondary" size="lg" class="h-10 w-full gap-1.5 rounded-md px-3" onclick={onNavigateToConvert}>
              <ArrowLeftRightIcon class="h-4 w-4" />
              <span>{i18n.t('wallet.overview.convert')}</span>
            </Button>
          </div>
          <Button
            variant="default"
            size="icon"
            class="h-10 w-10 shrink-0 rounded-md"
            aria-label={i18n.t('wallet.addAsset.open')}
            onclick={() => {
              showAddAssetSheet = true;
            }}
          >
            <PlusIcon class="h-4 w-4" />
          </Button>
        </div>
      </div>
    </div>

    <div class="relative min-h-0 flex-1">
      <ScrollArea.Root class="h-full" type="scroll">
        <ScrollArea.Viewport
          class="overview-list-scroll h-full overscroll-contain pr-4"
          bind:ref={listScrollElement}
          onscroll={onOverviewScroll}
        >
          {#if isBootstrapping}
            <ul class="space-y-1 pb-3">
              {#each overviewSkeletonRows as skeletonRow (skeletonRow)}
                <li class="grid grid-cols-[minmax(0,1fr)_11rem_10.25rem_auto] items-center gap-3.5 rounded-md px-3.5 py-3">
                  <div class="min-w-0 flex w-full items-center gap-3.5">
                    <Skeleton class="h-[34px] w-[34px] rounded-full" />
                    <div class="min-w-0 flex flex-1 min-h-8 items-center">
                      <Skeleton class="h-5 w-28 rounded-sm" />
                    </div>
                  </div>

                  <div class="justify-self-end pr-4 text-right tabular-nums">
                    <Skeleton class="ml-auto h-3 w-20 rounded-sm" />
                    <Skeleton class="mt-1.5 ml-auto h-3 w-14 rounded-sm" />
                  </div>

                  <div class="text-right tabular-nums">
                    <Skeleton class="ml-auto h-5 w-20 rounded-sm" />
                    <Skeleton class="mt-1.5 ml-auto h-4 w-24 rounded-sm" />
                  </div>

                  <Skeleton class="h-[18px] w-[18px] justify-self-end rounded-sm" />
                </li>
              {/each}
            </ul>
          {:else if visibleRows.length === 0}
            <p class="text-muted-foreground px-1 py-8 text-sm">{i18n.t('wallet.overview.noChannel')}</p>
          {:else}
            <ul class="space-y-1 pb-3">
              {#each visibleRows as row (row.key)}
                <li>
                  <button
                    type="button"
                    class="hover:bg-muted/40 focus-visible:ring-ring/55 grid w-full grid-cols-[minmax(0,1fr)_11rem_10.25rem_auto] items-center gap-3.5 rounded-md px-3.5 py-3 text-left transition-colors focus-visible:outline-none focus-visible:ring-2"
                    onclick={() =>
                      onOpenAssetDetails({
                        walletEntryKind: row.walletEntryKind,
                        coinId: row.coinId,
                        baseCoinId: row.baseCoinId,
                        scopeFilterMode: row.scopeFilterMode,
                        displayName: row.walletEntryKind === 'private_verus' ? row.name : undefined
                      })}
                  >
                    <div class="min-w-0 flex w-full items-center gap-3.5">
                      <CoinIcon
                        coinId={row.coinId}
                        coinName={row.name}
                        proto={row.proto}
                        size={rowIconSize}
                        showBadge
                        privateMuted={row.walletEntryKind === 'private_verus'}
                        decorative
                      />
                      <div class="min-w-0 flex flex-1 min-h-8 items-center">
                        {#if row.walletEntryKind === 'private_verus'}
                          <p class="text-foreground truncate text-base leading-tight font-medium">
                            <PrivateVerusWordmark label={row.name} />
                          </p>
                        {:else}
                          <p class="text-foreground truncate text-base leading-tight font-medium">{row.name}</p>
                        {/if}
                      </div>
                    </div>

                    <div class="justify-self-end pr-4 text-right tabular-nums">
                      <p class="text-foreground/75 text-xs font-medium">{row.marketPriceDisplay}</p>
                      <div
                        class={`mt-0.5 flex items-center justify-end text-xs ${
                          row.change24hDirection === 'up'
                            ? 'text-emerald-700 dark:text-emerald-300'
                            : row.change24hDirection === 'down'
                              ? 'text-destructive'
                              : 'text-muted-foreground'
                        }`}
                      >
                        <span>{row.change24hDisplay}</span>
                      </div>
                    </div>

                    <div class={`text-right tabular-nums ${row.syncLabel ? 'self-stretch flex items-center justify-end' : ''}`}>
                      {#if row.syncLabel}
                        <p class={`text-muted-foreground text-[13px] ${hideHoldings ? 'holdings-obscured' : ''}`}>
                          {row.syncLabel}
                        </p>
                      {:else}
                        <p class={`text-foreground text-base font-semibold ${hideHoldings ? 'holdings-obscured' : ''}`}>
                          {row.fiatValueDisplay}
                        </p>
                        <p class={`text-muted-foreground mt-0.5 text-[13px] ${hideHoldings ? 'holdings-obscured' : ''}`}>
                          {row.cryptoAmountDisplay}
                        </p>
                      {/if}
                    </div>

                    <ChevronRightIcon class="text-muted-foreground/70 h-[18px] w-[18px] justify-self-end" aria-hidden="true" />
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>

      {#if !isBootstrapping && canScrollDown}
        <div
          class="pointer-events-none absolute inset-x-0 bottom-0 h-14 bg-gradient-to-t from-background to-transparent dark:from-app-canvas"
        ></div>
      {/if}

      {#if !isBootstrapping && canScrollDown && !hasOverviewScroll && !hasSeenScrollHint}
        <div class="pointer-events-none absolute inset-x-0 bottom-3 flex justify-center">
          <div class="scroll-hint text-muted-foreground/85 inline-flex items-center gap-1 text-[11px]">
            <ChevronDownIcon class="scroll-hint-icon h-3.5 w-3.5" aria-hidden="true" />
            <span>{i18n.t('wallet.overview.scrollHintMoreAssets')}</span>
          </div>
        </div>
      {/if}
    </div>
  </section>
</div>

<AddAssetSheet bind:isOpen={showAddAssetSheet} network={walletNetwork} />

<style>
  .overview-list-scroll {
    scrollbar-gutter: stable;
  }

  .holdings-obscured {
    filter: blur(12px);
    user-select: none;
    pointer-events: none;
    transition: filter 120ms ease;
  }

  .holdings-obscured-bleed {
    padding: 0.3rem 0.45rem;
  }

  .scroll-hint {
    animation: scroll-hint-nudge 1.8s ease-in-out infinite;
  }

  .scroll-hint-icon {
    animation: scroll-hint-icon-nudge 1.8s ease-in-out infinite;
  }

  @keyframes scroll-hint-nudge {
    0%,
    100% {
      transform: translateY(0);
      opacity: 0.82;
    }
    50% {
      transform: translateY(2px);
      opacity: 1;
    }
  }

  @keyframes scroll-hint-icon-nudge {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(1px);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .scroll-hint,
    .scroll-hint-icon {
      animation: none;
    }
  }
</style>
