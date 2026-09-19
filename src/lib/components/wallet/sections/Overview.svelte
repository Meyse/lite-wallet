<!--
  Component: Overview
  Purpose: Wallet overview with hero balances, quick actions, and currency list
  Last Updated: Wallet overview redesign live-data only
  Security: No sensitive operations - display only
-->

<script lang="ts">
  import { tick } from 'svelte';
  import { Button } from '$lib/components/ui/button';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
  import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
  import ArrowLeftRightIcon from '@lucide/svelte/icons/arrow-left-right';
  import SlidersHorizontalIcon from '@lucide/svelte/icons/sliders-horizontal';
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
    aggregateOverviewBalances,
    buildWalletOverviewViewModel,
    filterWalletOverviewRows,
    formatCryptoAmount,
    OVERVIEW_UNAVAILABLE_DISPLAY,
    overviewNetworkMetadata,
    sortWalletOverviewRows,
    type WalletOverviewRowViewModel,
  } from '$lib/utils/walletOverview.js';
  import {
    formatFiatAmount,
    formatFiatAmountParts,
    getRateForCurrency,
  } from '$lib/utils/fiatDisplay.js';
  import OverviewAssetTools from './OverviewAssetTools.svelte';
  import {
    readWalletOverviewPreferences,
    type WalletOverviewPreferences,
    writeWalletOverviewPreferences,
  } from '$lib/stores/walletOverviewPreferences.js';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import PrivateVerusWordmark from '$lib/components/wallet/PrivateVerusWordmark.svelte';
  import AddAssetSheet from '$lib/components/wallet/AddAssetSheet.svelte';
  import * as walletDisplayService from '$lib/services/walletDisplayService.js';
  import type {
    CoinDefinition,
    CoinScope,
    ScopeKind,
    WalletEntryKind,
    WalletEntrySelection,
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
    onNavigateToConvert = () => {},
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
  let assetsButtonElement = $state<HTMLButtonElement | null>(null);
  let listScrollElement = $state<HTMLElement | null>(null);
  let hasOverviewScroll = $state(false);
  let canScrollDown = $state(false);
  let hasSeenScrollHint = $state(false);
  let hideHoldings = $state(false);
  let searchQuery = $state('');
  let assetTools = $state<OverviewAssetTools | null>(null);
  let preferences = $derived(readWalletOverviewPreferences(walletData.name, walletNetwork));

  function updatePreferences(value: WalletOverviewPreferences): void {
    writeWalletOverviewPreferences(walletData.name, walletNetwork, value);
    preferences = value;
  }

  $effect(() => {
    walletData.name;
    walletNetwork;
    searchQuery = '';
  });
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
  const privateBaseCoin = $derived(coins.find((coin) => coin.id === privateBaseCoinId) ?? null);
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

    const maxScrollTop = Math.max(
      0,
      listScrollElement.scrollHeight - listScrollElement.clientHeight
    );
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
      network: walletData.network,
    })
  );
  const overview = $derived(liveOverview);
  const baseRows = $derived<WalletEntryRow[]>(
    overview.rows.map((row) => ({
      ...row,
      walletEntryKind: 'coin',
      scopeFilterMode: 'transparent',
      isConfirmedZero:
        row.isConfirmedZero &&
        !isBootstrapping &&
        !isChannelSyncing(row.coinId) &&
        // VRPC scope discovery must finish before a primary-address zero can be hidden.
        (row.proto !== 'vrsc' || row.coinId in transparentScopeChannelIdsByCoinId) &&
        !(
          transparentScopeChannelIdsByCoinId[row.coinId] ?? [walletChannels.byCoinId[row.coinId]]
        ).some((channelId) => isChannelSyncing(channelId)),
    }))
  );
  const privateRow = $derived<WalletEntryRow | null>(
    (() => {
      const baseCoin = privateBaseCoin;
      if (!privateConfigured || !baseCoin) return null;

      const {
        amountValue: totalAmount,
        hasSnapshot,
        isConfirmedZero,
      } = aggregateOverviewBalances(
        privateScopes.map((scope) => balances[scope.channelId]?.[baseCoin.id])
      );

      const hasBalance = hasSnapshot && totalAmount > 0;
      const rateMetrics = resolveRateMetrics(baseCoin, rates);
      const fiatRate = rateMetrics?.fiatRate ?? null;
      const change24hPct = rateMetrics?.change24hPct ?? null;
      const fiatValue = hasSnapshot && fiatRate !== null ? totalAmount * fiatRate : null;
      const rowFractionDigits = Math.max(0, Math.min(4, baseCoin.decimals));
      const syncSnapshot = getPrivateSyncSnapshot(privateScopes, baseCoin.systemId, chainInfo);
      const syncPercent = syncSnapshot.percent;
      const syncLabel =
        syncPercent !== null && syncPercent !== 100 && syncPercent !== -1
          ? i18n.t('wallet.private.syncingPercent', {
              percent: formatPrivateSyncPercent(syncPercent),
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
        fiatSortValue: fiatValue ?? Number.NEGATIVE_INFINITY,
        amountSortValue: hasSnapshot ? totalAmount : null,
        isConfirmedZero:
          isConfirmedZero &&
          !isBootstrapping &&
          !syncLabel &&
          !privateScopes.some((scope) => isChannelSyncing(scope.channelId)),
        ...overviewNetworkMetadata(baseCoin),
        defaultSortGroup: 1,
        walletEntryKind: 'private_verus',
        baseCoinId: baseCoin.id,
        scopeFilterMode: 'shielded',
        syncLabel,
      };
    })()
  );
  // The full enabled wallet is the balance authority. View controls only transform visibleRows.
  const allRows = $derived<WalletEntryRow[]>(privateRow ? [...baseRows, privateRow] : baseRows);
  const visibleRows = $derived(
    sortWalletOverviewRows(
      filterWalletOverviewRows(allRows, searchQuery, preferences.withBalance),
      preferences.sort,
      i18n.intlLocale,
      preferences.reversed
    )
  );
  const heroSummary = $derived(
    (() => {
      const rows = allRows;
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
          hasPartialRates: false,
          hasPartialBalances: rows.some((row) => !row.hasSnapshot),
        };
      }

      if (!rows.some((row) => row.hasSnapshot)) {
        return {
          symbol: '',
          value: OVERVIEW_UNAVAILABLE_DISPLAY,
          hasPartialRates: false,
          hasPartialBalances: false,
        };
      }

      const parts = formatFiatAmountParts(totalFiat, i18n.intlLocale, displayCurrency);
      return {
        symbol: parts.symbol,
        value: parts.value,
        hasPartialRates: hasHoldings && hasAnyFiatForHoldings && hasMissingFiatForHoldings,
        hasPartialBalances: rows.some((row) => !row.hasSnapshot),
      };
    })()
  );
  const heroValueIsLoading = $derived(isBootstrapping && allRows.some((row) => !row.hasSnapshot));
  const heroTotalIsPartial = $derived(
    heroSummary.value !== OVERVIEW_UNAVAILABLE_DISPLAY &&
      (heroSummary.hasPartialRates || heroSummary.hasPartialBalances)
  );
  const rowIconSize = 34;
  const partialTotalTooltipId = 'wallet-overview-partial-total-description';

  function isChannelSyncing(channelId: string): boolean {
    const info = chainInfo[channelId];
    return (
      info?.syncing === true ||
      (info?.percent !== undefined && info.percent >= 0 && info.percent < 100)
    );
  }

  function isBalanceValueLoading(row: WalletEntryRow): boolean {
    return isBootstrapping && !row.hasSnapshot;
  }

  function isRateValueLoading(row: WalletEntryRow): boolean {
    return walletNetwork === 'mainnet' && isBootstrapping && row.unitRateDisplay === null;
  }

  function isFiatValueLoading(row: WalletEntryRow): boolean {
    return (
      walletNetwork === 'mainnet' &&
      isBootstrapping &&
      (!row.hasSnapshot || row.unitRateDisplay === null)
    );
  }

  function getChangeDirection(
    changePct: number | null
  ): WalletOverviewRowViewModel['change24hDirection'] {
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
      maximumFractionDigits: 1,
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
      percent: toFiniteNumber(snapshot?.percent),
    };
  }

  async function loadDlightStatus(): Promise<void> {
    dlightStatusRequestSequence += 1;
    const requestSequence = dlightStatusRequestSequence;
    try {
      const status = await walletDisplayService.getDisplayDlightSeedStatus();
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
      const result = await walletDisplayService.getDisplayCoinScopes(coinId);
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
      const balance = await walletDisplayService.getDisplayBalance(scope.channelId, coinId);
      balanceStore.update((state) => ({
        ...state,
        [scope.channelId]: {
          ...(state[scope.channelId] ?? {}),
          [coinId]: balance,
        },
      }));
      loadedTransparentAggregateByChannel = {
        ...loadedTransparentAggregateByChannel,
        [channelKey]: true,
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
      const scopeResult = await walletDisplayService.getDisplayCoinScopes(coin.id);
      const scopes = scopeResult.scopes.filter((scope) => scope.scopeKind === 'transparent');
      transparentScopeChannelIdsByCoinId = {
        ...transparentScopeChannelIdsByCoinId,
        [coin.id]: Array.from(new Set(scopes.map((scope) => scope.channelId))),
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

  async function closeManageAssets(): Promise<void> {
    showAddAssetSheet = false;
    await tick();
    assetsButtonElement?.focus();
  }
</script>

<div class="mx-auto flex h-full min-h-0 w-full max-w-6xl flex-col px-6 pb-6 sm:px-8">
  <section
    class="min-h-0 flex-1 flex-col overflow-hidden"
    class:flex={!showAddAssetSheet}
    class:hidden={showAddAssetSheet}
  >
    <div
      class={`z-10 shrink-0 bg-background pb-3 dark:bg-app-canvas ${hasOverviewScroll ? 'overview-scroll-shadow' : ''}`}
    >
      <div
        class="balance-banner flex min-h-[92px] items-center justify-between gap-4 rounded-md py-4 pr-3.5 pl-[22px]"
      >
        <div class="relative z-20 min-w-0">
          <div class="flex flex-wrap items-center gap-2">
            {#if heroValueIsLoading}
              <div class="flex h-[49px] items-center" aria-label={i18n.t('common.loading')}>
                <Skeleton class="h-9 w-40 rounded-md bg-white/20 sm:h-10 sm:w-48" />
              </div>
            {:else}
              <div
                class={`flex min-h-[49px] min-w-0 items-center ${hideHoldings ? 'holdings-obscured' : ''}`}
              >
                {#if heroSummary.symbol}
                  <span
                    class="balance-banner-muted mr-1.5 shrink-0 text-xl font-semibold sm:text-2xl"
                  >
                    {heroSummary.symbol}
                  </span>
                {/if}
                <p
                  class="font-google-sans-17pt min-w-0 text-4xl leading-[1.02] font-semibold tracking-tight break-all sm:text-5xl"
                >
                  {heroSummary.value}
                </p>
              </div>
              {#if heroTotalIsPartial}
                <Tooltip.Root>
                  <Tooltip.Trigger>
                    {#snippet child({ props })}
                      <button
                        {...props}
                        type="button"
                        aria-describedby={partialTotalTooltipId}
                        class="inline-flex h-5 items-center rounded-sm border border-white/30 bg-white/10 px-1.5 text-[11px] leading-none font-medium text-white transition-colors hover:bg-white/20 focus-visible:ring-2 focus-visible:ring-white focus-visible:outline-none"
                      >
                        {i18n.t('wallet.overview.partialTotalLabel')}
                      </button>
                    {/snippet}
                  </Tooltip.Trigger>
                  <Tooltip.Content side="bottom" align="start" class="max-w-72 leading-5">
                    <span id={partialTotalTooltipId} role="tooltip">
                      {i18n.t('wallet.overview.partialTotalDescription')}
                    </span>
                  </Tooltip.Content>
                </Tooltip.Root>
              {/if}
            {/if}
          </div>
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          class="rounded-full text-(--wallet-balance-muted) hover:bg-transparent hover:text-white focus-visible:ring-white dark:hover:bg-transparent"
          aria-label={hideHoldings
            ? i18n.t('wallet.overview.showHoldings')
            : i18n.t('wallet.overview.hideHoldings')}
          title={hideHoldings
            ? i18n.t('wallet.overview.showHoldings')
            : i18n.t('wallet.overview.hideHoldings')}
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
      </div>
      <div class="mt-2 w-full">
        <div class="grid w-full grid-cols-4 gap-2">
          <Button
            variant="secondary"
            size="lg"
            class="h-10 w-full gap-1.5 rounded-md px-3"
            onclick={onNavigateToReceive}
          >
            <ArrowDownIcon class="h-4 w-4" />
            <span>{i18n.t('wallet.overview.receive')}</span>
          </Button>
          <Button
            variant="secondary"
            size="lg"
            class="h-10 w-full gap-1.5 rounded-md px-3"
            onclick={onNavigateToSend}
          >
            <ArrowUpIcon class="h-4 w-4" />
            <span>{i18n.t('wallet.overview.send')}</span>
          </Button>
          <Button
            variant="secondary"
            size="lg"
            class="h-10 w-full gap-1.5 rounded-md px-3"
            onclick={onNavigateToConvert}
          >
            <ArrowLeftRightIcon class="h-4 w-4" />
            <span>{i18n.t('wallet.overview.convert')}</span>
          </Button>
          <Button
            bind:ref={assetsButtonElement}
            variant="secondary"
            size="lg"
            class="h-10 w-full gap-1.5 rounded-md px-3"
            onclick={() => {
              searchQuery = '';
              showAddAssetSheet = true;
            }}
          >
            <SlidersHorizontalIcon class="h-4 w-4" />
            <span>{i18n.t('wallet.manageAssets.action')}</span>
          </Button>
        </div>
      </div>
      <OverviewAssetTools
        bind:this={assetTools}
        bind:query={searchQuery}
        {preferences}
        onPreferencesChange={updatePreferences}
      />
    </div>

    <div class="relative min-h-0 flex-1">
      <ScrollArea.Root class="h-full" type="scroll">
        <ScrollArea.Viewport
          class="h-full overscroll-contain"
          bind:ref={listScrollElement}
          onscroll={onOverviewScroll}
        >
          {#if visibleRows.length === 0}
            {#if searchQuery.trim() || preferences.withBalance}
              <div class="flex h-60 flex-col items-center justify-center gap-3">
                <p class="text-base leading-6 font-medium" role="status">
                  {i18n.t('wallet.overview.noAssetsFound')}
                </p>
                <Button
                  variant="secondary"
                  class="h-8 rounded-md px-3 text-[13px] font-normal"
                  onclick={() => {
                    if (searchQuery.trim()) assetTools?.clearSearch();
                    else {
                      updatePreferences({ ...preferences, withBalance: false });
                      assetTools?.focusSearch();
                    }
                  }}
                >
                  {searchQuery.trim()
                    ? i18n.t('wallet.overview.clearSearch')
                    : i18n.t('wallet.overview.clearBalanceFilter')}
                </Button>
              </div>
            {:else}
              <p class="px-1 py-8 text-sm text-muted-foreground">
                {i18n.t('wallet.overview.noChannel')}
              </p>
            {/if}
          {:else}
            <ul class="space-y-1">
              {#each visibleRows as row (row.key)}
                <li>
                  <button
                    type="button"
                    class="grid w-full grid-cols-[minmax(0,1fr)_11rem_10.25rem_auto] items-center gap-3.5 rounded-md px-3.5 py-3 text-left transition-colors hover:bg-muted/40 focus-visible:ring-2 focus-visible:ring-ring/55 focus-visible:outline-none focus-visible:ring-inset"
                    onclick={() =>
                      onOpenAssetDetails({
                        walletEntryKind: row.walletEntryKind,
                        coinId: row.coinId,
                        baseCoinId: row.baseCoinId,
                        scopeFilterMode: row.scopeFilterMode,
                        displayName: row.walletEntryKind === 'private_verus' ? row.name : undefined,
                      })}
                  >
                    <div class="flex w-full min-w-0 items-center gap-3.5">
                      <CoinIcon
                        coinId={row.coinId}
                        coinName={row.name}
                        proto={row.proto}
                        size={rowIconSize}
                        showBadge
                        privateMuted={row.walletEntryKind === 'private_verus'}
                        decorative
                      />
                      <div class="flex min-h-8 min-w-0 flex-1 items-center">
                        {#if row.walletEntryKind === 'private_verus'}
                          <p class="truncate text-base leading-tight font-medium text-foreground">
                            <PrivateVerusWordmark label={row.name} />
                          </p>
                        {:else}
                          <p class="truncate text-base leading-tight font-medium text-foreground">
                            {row.name}
                          </p>
                        {/if}
                      </div>
                    </div>

                    <div class="justify-self-end pr-4 text-right tabular-nums">
                      {#if isRateValueLoading(row)}
                        <div class="flex h-4 items-center justify-end">
                          <Skeleton class="h-3 w-20 rounded-sm" />
                        </div>
                      {:else}
                        <p class="h-4 text-xs font-medium text-foreground/75">
                          {row.marketPriceDisplay}
                        </p>
                      {/if}
                      <div
                        class={`mt-0.5 flex h-4 items-center justify-end text-xs ${
                          row.change24hDirection === 'up'
                            ? 'text-emerald-700 dark:text-emerald-300'
                            : row.change24hDirection === 'down'
                              ? 'text-destructive'
                              : 'text-muted-foreground'
                        }`}
                      >
                        {#if isRateValueLoading(row)}
                          <Skeleton class="h-3 w-14 rounded-sm" />
                        {:else}
                          <span>{row.change24hDisplay}</span>
                        {/if}
                      </div>
                    </div>

                    <div
                      class={`text-right tabular-nums ${row.syncLabel ? 'flex items-center justify-end self-stretch' : ''}`}
                    >
                      {#if row.syncLabel}
                        <p
                          class={`text-[13px] text-muted-foreground ${hideHoldings ? 'holdings-obscured' : ''}`}
                        >
                          {row.syncLabel}
                        </p>
                      {:else}
                        {#if isFiatValueLoading(row)}
                          <div class="flex h-6 items-center justify-end">
                            <Skeleton class="h-4 w-20 rounded-sm" />
                          </div>
                        {:else}
                          <p
                            class={`h-6 text-base leading-6 font-semibold text-foreground ${hideHoldings ? 'holdings-obscured' : ''}`}
                          >
                            {row.fiatValueDisplay}
                          </p>
                        {/if}
                        {#if isBalanceValueLoading(row)}
                          <div class="mt-0.5 flex h-5 items-center justify-end">
                            <Skeleton class="h-3 w-24 rounded-sm" />
                          </div>
                        {:else}
                          <p
                            class={`mt-0.5 h-5 text-[13px] leading-5 text-muted-foreground ${hideHoldings ? 'holdings-obscured' : ''}`}
                          >
                            {row.cryptoAmountDisplay}
                          </p>
                        {/if}
                      {/if}
                    </div>

                    <ChevronRightIcon
                      class="h-[18px] w-[18px] justify-self-end text-muted-foreground/70"
                      aria-hidden="true"
                    />
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>

      {#if canScrollDown}
        <div
          class="pointer-events-none absolute inset-x-0 bottom-0 h-14 bg-gradient-to-t from-background to-transparent dark:from-app-canvas"
        ></div>
      {/if}

      {#if canScrollDown && !hasOverviewScroll && !hasSeenScrollHint}
        <div class="pointer-events-none absolute inset-x-0 bottom-3 flex justify-center">
          <div
            class="scroll-hint inline-flex items-center gap-1 text-[11px] text-muted-foreground/85"
          >
            <ChevronDownIcon class="scroll-hint-icon h-3.5 w-3.5" aria-hidden="true" />
            <span>{i18n.t('wallet.overview.scrollHintMoreAssets')}</span>
          </div>
        </div>
      {/if}
    </div>
  </section>
  {#if showAddAssetSheet}
    <AddAssetSheet
      bind:isOpen={showAddAssetSheet}
      network={walletNetwork}
      onClose={closeManageAssets}
    />
  {/if}
</div>

<style>
  .balance-banner {
    color: var(--wallet-balance-foreground);
    background: linear-gradient(
      180deg in oklab,
      var(--wallet-balance-gradient-start) 0.01%,
      var(--wallet-balance-gradient-end) 99.99%
    );
    border-bottom: 1px solid var(--wallet-balance-edge);
  }

  :global(.dark) .balance-banner {
    border-top: 1px solid var(--wallet-balance-edge);
    border-bottom: 0;
  }

  .balance-banner-muted {
    color: var(--wallet-balance-muted);
  }

  /* Fast requests should not flash a notice; persistent partial data stays visible. */
  .overview-notice {
    animation: reveal-notice 0s 400ms both;
  }

  @keyframes reveal-notice {
    from {
      visibility: hidden;
    }
    to {
      visibility: visible;
    }
  }

  .holdings-obscured {
    /* Keep WebKit's blur composited at rest, not only during row hover. */
    transform: translateZ(0);
    filter: blur(12px);
    user-select: none;
    pointer-events: none;
    transition: filter 120ms ease;
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
