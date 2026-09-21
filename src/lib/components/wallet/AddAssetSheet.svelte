<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';
  import CheckIcon from '@lucide/svelte/icons/check';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import EyeOffIcon from '@lucide/svelte/icons/eye-off';
  import InfoIcon from '@lucide/svelte/icons/info';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import SearchIcon from '@lucide/svelte/icons/search';
  import NavigationBackButton from '$lib/components/common/NavigationBackButton.svelte';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { Button } from '$lib/components/ui/button';
  import { CopyButton } from '$lib/components/ui/copy-button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import { isWalletSupportedAsset } from '$lib/coins/supportedAssets.js';
  import { i18nStore } from '$lib/i18n';
  import * as coinsService from '$lib/services/coinsService.js';
  import { isForcedWalletLockError } from '$lib/services/walletLockCoordinator.js';
  import * as walletService from '$lib/services/walletService.js';
  import {
    type AddAssetEntry,
    applyCatalogMetadataToCoinDefinition,
    buildAddAssetCatalogView,
    catalogEntryToCoinDefinition,
    erc20ContractValue,
    pbaasLookupValue,
  } from '$lib/stores/addAssetCatalog.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import {
    type AggregatedDiscoveryHolding,
    aggregateDiscoveryHoldings,
    ASSET_DISCOVERY_TIMEOUT_MS,
    assetKeyForCoin,
    finiteAssetBalance,
    formatAssetBalance,
    type KnownAssetBalance,
    scanKnownNonVrpcAssets,
    withAssetLookupTimeout,
  } from '$lib/stores/manageAssets.js';
  import { buildWalletChannels, walletChannelsStore } from '$lib/stores/walletChannels.js';
  import type {
    AssetDiscoveryResult,
    CoinDefinition,
    PbaasCandidate,
    WalletNetwork,
  } from '$lib/types/wallet.js';
  import { extractWalletErrorType } from '$lib/utils/walletErrors.js';

  type ManageTab = 'yours' | 'browse' | 'hidden';
  type BalanceStatus = 'loading' | 'available' | 'partial' | 'unavailable';
  type VisitGroup = 'shown' | 'found' | 'browse' | 'hidden' | 'other';

  interface ManagedAssetRow {
    key: string;
    coinId: string;
    currencyId: string;
    displayTicker: string;
    displayName: string;
    proto: CoinDefinition['proto'];
    coin: CoinDefinition | null;
    catalogEntry: AddAssetEntry | null;
    cataloged: boolean;
    inPortfolio: boolean;
    hidden: boolean;
    discovered: boolean;
    balance: string | null;
    balanceStatus: BalanceStatus;
    includesReadOnly: boolean;
    networks: Array<{
      systemId: string;
      systemTicker: string;
      systemDisplayName: string;
      balance: string | null;
      status: BalanceStatus;
    }>;
  }

  let {
    isOpen = $bindable(false),
    network,
    onClose = () => {},
  }: {
    isOpen?: boolean;
    network: WalletNetwork;
    onClose?: () => void;
  } = $props();

  const i18n = $derived($i18nStore);
  const walletChannels = $derived($walletChannelsStore);

  let headingElement = $state<HTMLElement | null>(null);
  let listScrollElement = $state<HTMLElement | null>(null);
  let canScrollUp = $state(false);
  let canScrollDown = $state(false);
  let registryCoins = $state<CoinDefinition[]>([]);
  let portfolioCoinIds = $state<string[]>([]);
  let hiddenAssetKeys = $state<string[]>([]);
  let discovery = $state<AssetDiscoveryResult | null>(null);
  let knownBalances = $state<KnownAssetBalance[]>([]);
  let pendingKnownAssetKeys = $state<string[]>([]);
  let loading = $state(true);
  let refreshing = $state(false);
  let discoveryStale = $state(false);
  let discoveryError = $state('');
  let refreshError = $state('');
  let tab = $state<ManageTab>('yours');
  let query = $state('');
  let networkFilter = $state('all');
  let otherAssetsExpanded = $state(false);
  let expandedAssetKeys = $state<string[]>([]);
  let pendingCounts = $state<Record<string, number>>({});
  let rowErrors = $state<Record<string, string>>({});
  let retryDesired = $state<Record<string, boolean>>({});
  let rowRevisions = $state<Record<string, number>>({});
  let visitGroupByKey = $state<Record<string, VisitGroup>>({});
  let expectedSessionId = $state('');
  let savingPreference = $state(false);
  let closing = $state(false);
  let networkMenuOpen = $state(false);
  let lifecycleGeneration = 0;
  let componentMounted = true;
  let activePreferenceSave: Promise<boolean> | null = null;

  let view = $state<'manage' | 'manual'>('manage');
  let manualInputElement = $state<HTMLInputElement | null>(null);
  let manualReturnFocus: HTMLElement | null = null;
  let manualReturnFocusKey = '';
  let manualInput = $state('');
  let manualResolving = $state(false);
  let manualAdding = $state(false);
  let manualAdded = $state(false);
  let manualError = $state('');
  let manualResolvedCoin = $state<CoinDefinition | null>(null);
  let manualCandidates = $state<PbaasCandidate[]>([]);
  let identifierCopyState = $state<'idle' | 'copied' | 'failed'>('idle');
  let manualLookupRevision = 0;

  const activeSet = $derived(
    new Set(portfolioCoinIds.map((coinId) => coinId.trim().toLowerCase()))
  );
  const hiddenSet = $derived(new Set(hiddenAssetKeys.map((key) => key.trim().toLowerCase())));
  const aggregatedDiscovery = $derived(aggregateDiscoveryHoldings(discovery?.holdings ?? []));
  const catalogView = $derived(
    buildAddAssetCatalogView({
      coins: registryCoins,
      network,
      query: '',
      activeCoinIds: portfolioCoinIds,
    })
  );
  const catalogEntries = $derived([...catalogView.addedEntries, ...catalogView.availableEntries]);

  function normalize(value: string): string {
    return value.trim().toLowerCase();
  }

  function entryAssetKey(entry: AddAssetEntry): string {
    return `${entry.proto}:${normalize(entry.currencyId || entry.id)}`;
  }

  function matchingRegistryCoin(entry: AddAssetEntry): CoinDefinition | null {
    return (
      registryCoins.find((coin) => normalize(coin.id) === normalize(entry.id)) ??
      registryCoins.find(
        (coin) =>
          coin.proto === entry.proto &&
          Boolean(entry.currencyId) &&
          normalize(coin.currencyId) === normalize(entry.currencyId)
      ) ??
      null
    );
  }

  function discoveryForKey(key: string): AggregatedDiscoveryHolding | null {
    return aggregatedDiscovery.find((holding) => holding.assetKey === key) ?? null;
  }

  function knownBalanceForKey(key: string): KnownAssetBalance | null {
    return knownBalances.find((holding) => holding.assetKey === key) ?? null;
  }

  function vrpcCoverageStatus(): BalanceStatus {
    if (!discovery) return refreshing && !discoveryError ? 'loading' : 'unavailable';
    if (discovery.sources.length === 0) return 'unavailable';
    const statuses = discovery.sources.map((source) => source.status);
    if (statuses.every((status) => status === 'unavailable')) return 'unavailable';
    if (statuses.some((status) => status !== 'available') || !discovery.scopeMetadataComplete) {
      return 'partial';
    }
    return 'available';
  }

  function mergeBalanceStatus(
    holdingStatus: BalanceStatus,
    coverageStatus: BalanceStatus
  ): BalanceStatus {
    if (coverageStatus === 'unavailable') {
      return holdingStatus === 'available' || holdingStatus === 'partial'
        ? 'partial'
        : 'unavailable';
    }
    if (coverageStatus === 'partial' && holdingStatus === 'available') return 'partial';
    return holdingStatus;
  }

  function fallbackNetwork(entry: AddAssetEntry): {
    systemId: string;
    systemTicker: string;
    systemDisplayName: string;
  } {
    if (entry.proto === 'eth' || entry.proto === 'erc20') {
      return {
        systemId: network === 'testnet' ? 'GETH' : 'ETH',
        systemTicker: network === 'testnet' ? 'Sepolia' : 'ETH',
        systemDisplayName: network === 'testnet' ? 'Sepolia' : 'Ethereum',
      };
    }
    if (entry.proto === 'btc') {
      return {
        systemId: network === 'testnet' ? 'BTCTEST' : 'BTC',
        systemTicker: network === 'testnet' ? 'Bitcoin Testnet' : 'BTC',
        systemDisplayName: network === 'testnet' ? 'Bitcoin Testnet' : 'Bitcoin',
      };
    }
    return {
      systemId: entry.systemId,
      systemTicker: network === 'testnet' ? 'VRSCTEST' : 'VRSC',
      systemDisplayName: network === 'testnet' ? 'Verus Testnet' : 'Verus',
    };
  }

  function rowFromEntry(entry: AddAssetEntry): ManagedAssetRow {
    const key = entryAssetKey(entry);
    const runtimeCoin = matchingRegistryCoin(entry);
    const discovered = discoveryForKey(key);
    const known = knownBalanceForKey(key);
    const fallback = fallbackNetwork(entry);
    const coverageStatus = entry.proto === 'vrsc' ? vrpcCoverageStatus() : null;
    const balanceStatus: BalanceStatus = discovered
      ? mergeBalanceStatus(discovered.status, coverageStatus ?? 'available')
      : (known?.status ??
        coverageStatus ??
        (pendingKnownAssetKeys.includes(key) ? 'loading' : 'unavailable'));
    const networks = discovered
      ? discovered.networks.map((holding) => ({
          systemId: holding.systemId,
          systemTicker: holding.systemTicker,
          systemDisplayName: holding.systemDisplayName,
          balance: holding.balance,
          status: holding.balanceStatus as BalanceStatus,
        }))
      : known
        ? [
            {
              systemId: known.systemId,
              systemTicker: known.systemTicker,
              systemDisplayName: known.systemDisplayName,
              balance: known.balance,
              status: known.status as BalanceStatus,
            },
          ]
        : [
            {
              ...fallback,
              balance: balanceStatus === 'available' ? '0' : null,
              status: balanceStatus,
            },
          ];

    return {
      key,
      coinId: runtimeCoin?.id ?? entry.id,
      currencyId: entry.currencyId,
      displayTicker: runtimeCoin?.displayTicker ?? entry.displayTicker,
      displayName: runtimeCoin?.displayName ?? entry.displayName,
      proto: runtimeCoin?.proto ?? (entry.proto as CoinDefinition['proto']),
      coin: runtimeCoin ?? discovered?.coin ?? null,
      catalogEntry: entry,
      cataloged: true,
      inPortfolio: activeSet.has(normalize(runtimeCoin?.id ?? entry.id)),
      hidden: hiddenSet.has(key),
      discovered: Boolean(
        (discovered && discovered.total > 0) ||
        (known?.status === 'available' && finiteAssetBalance(known.balance) > 0)
      ),
      balance:
        discovered?.totalDisplay ?? known?.balance ?? (balanceStatus === 'available' ? '0' : null),
      balanceStatus,
      includesReadOnly: discovered?.includesReadOnly ?? false,
      networks,
    };
  }

  const catalogRows = $derived(catalogEntries.map(rowFromEntry));
  const catalogKeys = $derived(new Set(catalogRows.map((row) => row.key)));
  const otherRows = $derived(
    aggregatedDiscovery
      .filter((holding) => !catalogKeys.has(holding.assetKey))
      .map((holding): ManagedAssetRow => ({
        key: holding.assetKey,
        coinId: holding.coin?.id ?? holding.currencyId,
        currencyId: holding.currencyId,
        displayTicker: holding.coin?.displayTicker ?? holding.currencyId,
        displayName: holding.coin?.displayName ?? holding.currencyId,
        proto: 'vrsc',
        coin: holding.coin,
        catalogEntry: null,
        cataloged: false,
        inPortfolio: holding.coin ? activeSet.has(normalize(holding.coin.id)) : false,
        hidden: hiddenSet.has(holding.assetKey),
        discovered: holding.total > 0,
        balance: holding.totalDisplay,
        balanceStatus: mergeBalanceStatus(holding.status, vrpcCoverageStatus()),
        includesReadOnly: holding.includesReadOnly,
        networks: holding.networks.map((networkHolding) => ({
          systemId: networkHolding.systemId,
          systemTicker: networkHolding.systemTicker,
          systemDisplayName: networkHolding.systemDisplayName,
          balance: networkHolding.balance,
          status: networkHolding.balanceStatus,
        })),
      }))
  );

  const allRows = $derived([...catalogRows, ...otherRows]);
  const networkOptions = $derived.by(() => {
    const options = new Map<string, string>();
    for (const source of discovery?.sources ?? []) {
      options.set(source.systemId, source.systemDisplayName || source.systemTicker);
    }
    for (const holding of knownBalances) {
      options.set(holding.systemId, holding.systemDisplayName || holding.systemTicker);
    }
    return Array.from(options, ([id, label]) => ({ id, label })).sort((a, b) =>
      a.label.localeCompare(b.label, undefined, { sensitivity: 'base' })
    );
  });

  function rowMatchesNetwork(row: ManagedAssetRow): boolean {
    return (
      networkFilter === 'all' ||
      row.networks.some(
        (holdingNetwork) => normalize(holdingNetwork.systemId) === normalize(networkFilter)
      )
    );
  }

  function rowMatchesQuery(row: ManagedAssetRow): boolean {
    const normalized = normalize(query);
    if (!normalized) return true;
    return [row.displayName, row.displayTicker, row.coinId, row.currencyId].some((value) =>
      normalize(value).includes(normalized)
    );
  }

  function compactBalanceLabel(row: ManagedAssetRow, balance: string): string {
    const displayBalance = displayBalanceAmount(balance);
    const tickerIsLongFallbackIdentifier =
      !row.cataloged &&
      normalize(row.displayTicker) === normalize(row.currencyId) &&
      row.displayTicker.length > 20;
    return tickerIsLongFallbackIdentifier
      ? displayBalance
      : `${displayBalance} ${row.displayTicker}`;
  }

  function displayBalanceAmount(balance: string | null): string {
    if (balance === null) return '';
    return /^-?0+(?:\.0+)?$/.test(balance) ? '0' : balance;
  }

  const scopedRows = $derived(
    allRows.filter((row) => rowMatchesNetwork(row) && rowMatchesQuery(row))
  );
  const hasQuery = $derived(normalize(query).length > 0);
  const searchCatalogRows = $derived(hasQuery ? scopedRows.filter((row) => row.cataloged) : []);
  const shownRows = $derived(
    hasQuery ? [] : scopedRows.filter((row) => visitGroup(row) === 'shown')
  );
  const foundRows = $derived(
    hasQuery ? [] : scopedRows.filter((row) => visitGroup(row) === 'found')
  );
  const browseRows = $derived(
    hasQuery ? [] : scopedRows.filter((row) => row.cataloged && visitGroup(row) !== 'hidden')
  );
  const hiddenRows = $derived(
    hasQuery ? [] : scopedRows.filter((row) => row.cataloged && visitGroup(row) === 'hidden')
  );
  const visibleOtherRows = $derived(
    scopedRows.filter(
      (row) =>
        !row.cataloged &&
        (hasQuery ||
          (tab === 'hidden' ? visitGroup(row) === 'hidden' : visitGroup(row) === 'other'))
    )
  );
  const currentNetworkLabel = $derived(
    networkFilter === 'all'
      ? i18n.t('wallet.manageAssets.allNetworks')
      : (networkOptions.find((option) => option.id === networkFilter)?.label ?? networkFilter)
  );
  const failedDiscoverySystemIds = $derived(
    discovery?.sources
      .filter((source) => source.status !== 'available')
      .map((source) => source.systemId) ?? []
  );
  const hasPartialDiscoveryCoverage = $derived(
    failedDiscoverySystemIds.length > 0 || discovery?.scopeMetadataComplete === false
  );
  const hasUnavailableKnownBalances = $derived(
    knownBalances.some((balance) => balance.status === 'unavailable')
  );
  const manualResolvedRow = $derived(
    manualResolvedCoin
      ? (allRows.find((row) => row.key === assetKeyForCoin(manualResolvedCoin!)) ?? null)
      : null
  );
  const manualResolvedBalance = $derived(
    manualResolvedRow ? displayedBalance(manualResolvedRow) : null
  );
  const manualAlreadyShown = $derived(
    manualResolvedCoin ? activeSet.has(normalize(manualResolvedCoin.id)) : false
  );
  const manualLookupLabel = $derived(
    manualResolvedCoin?.proto === 'erc20' || normalizedErc20ContractCandidate(manualInput)
      ? i18n.t('wallet.manageAssets.contractAddress')
      : manualResolvedCoin
        ? i18n.t('wallet.manageAssets.currencyNameOrId')
        : i18n.t('wallet.manageAssets.lookupLabel')
  );

  function translateAssetError(error: unknown, fallbackKey: string): string {
    if (isForcedWalletLockError(error)) return '';
    switch (extractWalletErrorType(error)) {
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
      case 'EthNetworkMismatch':
        return i18n.t('wallet.addAsset.error.ethNetworkMismatch');
      default:
        return error instanceof Error && error.message.trim() ? error.message : i18n.t(fallbackKey);
    }
  }

  function applyPortfolioStores(): void {
    const visible = new Set(portfolioCoinIds.map(normalize));
    const activeCoins = registryCoins.filter((coin) => visible.has(normalize(coin.id)));
    coinsStore.set(activeCoins);
    walletChannelsStore.set(buildWalletChannels(activeCoins, walletChannels.vrpcAddress));
  }

  function isOperationCurrent(generation: number, sessionId = expectedSessionId): boolean {
    return (
      componentMounted &&
      isOpen &&
      lifecycleGeneration === generation &&
      expectedSessionId === sessionId
    );
  }

  function defaultVisitGroup(row: ManagedAssetRow): VisitGroup {
    if (row.hidden) return 'hidden';
    if (!row.cataloged) return 'other';
    if (row.inPortfolio) return 'shown';
    if (row.discovered) return 'found';
    return 'browse';
  }

  function visitGroup(row: ManagedAssetRow): VisitGroup {
    return visitGroupByKey[row.key] ?? defaultVisitGroup(row);
  }

  function freezeVisitGroup(row: ManagedAssetRow): void {
    if (visitGroupByKey[row.key]) return;
    visitGroupByKey = { ...visitGroupByKey, [row.key]: defaultVisitGroup(row) };
  }

  async function refreshRegistry(generation = lifecycleGeneration): Promise<void> {
    const allCoins = await coinsService.getCoinRegistry();
    if (!isOperationCurrent(generation)) return;
    registryCoins = allCoins.filter((coin) => isWalletSupportedAsset(coin, network));
  }

  async function hydrate(generation: number): Promise<void> {
    loading = true;
    discoveryError = '';
    try {
      const preferences = await walletService.getAssetPreferences();
      if (!componentMounted || lifecycleGeneration !== generation || !isOpen) return;
      expectedSessionId = preferences.sessionId;
      const [allCoins, confirmedPreferences] = await Promise.all([
        coinsService.getCoinRegistry(),
        walletService.getAssetPreferences(),
      ]);
      if (
        !isOperationCurrent(generation, preferences.sessionId) ||
        confirmedPreferences.sessionId !== preferences.sessionId
      ) {
        return;
      }
      registryCoins = allCoins.filter((coin) => isWalletSupportedAsset(coin, network));
      portfolioCoinIds = preferences.portfolioCoinIds;
      hiddenAssetKeys = preferences.hiddenAssetKeys;
      applyPortfolioStores();
      // Local rows are ready. Network discovery must not hold the entire list
      // behind skeletons, even if one provider never responds.
      void refreshDiscovery(generation, preferences.sessionId);
    } catch (error) {
      if (componentMounted && lifecycleGeneration === generation && isOpen) {
        discoveryError = translateAssetError(error, 'wallet.manageAssets.loadError');
      }
    } finally {
      if (componentMounted && lifecycleGeneration === generation && isOpen) loading = false;
    }
  }

  function mergeDiscoveryUpdate(
    current: AssetDiscoveryResult | null,
    update: AssetDiscoveryResult,
    systemIds: string[]
  ): AssetDiscoveryResult {
    if (!current || systemIds.length === 0) return update;
    const replaced = new Set(systemIds.map(normalize));
    return {
      ...update,
      sources: [
        ...current.sources.filter((source) => !replaced.has(normalize(source.systemId))),
        ...update.sources,
      ],
      holdings: [
        ...current.holdings.filter((holding) => !replaced.has(normalize(holding.systemId))),
        ...update.holdings,
      ],
    };
  }

  async function refreshDiscovery(
    generation = lifecycleGeneration,
    sessionId = expectedSessionId,
    systemIds: string[] = []
  ): Promise<void> {
    if (refreshing) return;
    refreshing = true;
    discoveryError = '';
    const previousDiscovery = discovery;
    const retryingSources = systemIds.length > 0;
    if (!retryingSources) {
      pendingKnownAssetKeys = registryCoins
        .filter((coin) => !coin.compatibleChannels.includes('vrpc'))
        .map(assetKeyForCoin);
    }

    async function canPublish(): Promise<boolean> {
      if (!isOperationCurrent(generation, sessionId)) return false;
      try {
        const preferences = await withAssetLookupTimeout(walletService.getAssetPreferences());
        return isOperationCurrent(generation, sessionId) && preferences.sessionId === sessionId;
      } catch {
        // A lock or replacement session invalidates this refresh.
        return false;
      }
    }

    async function refreshVrpc(): Promise<void> {
      try {
        const result = await withAssetLookupTimeout(
          walletService.discoverVrpcAssets(retryingSources ? systemIds : undefined),
          ASSET_DISCOVERY_TIMEOUT_MS
        );
        if (!(await canPublish())) return;
        discovery = mergeDiscoveryUpdate(previousDiscovery, result, systemIds);
        discoveryStale = false;
      } catch {
        if (!(await canPublish())) return;
        discoveryStale = Boolean(previousDiscovery);
        discoveryError = i18n.t('wallet.manageAssets.discoveryPartial');
      }
    }

    await Promise.allSettled([
      refreshVrpc(),
      retryingSources
        ? Promise.resolve()
        : scanKnownNonVrpcAssets(
            registryCoins,
            walletService.getCoinScopes,
            walletService.getBalances,
            async (balance) => {
              if (!(await canPublish())) return;
              knownBalances = [
                ...knownBalances.filter((known) => known.assetKey !== balance.assetKey),
                balance,
              ];
              pendingKnownAssetKeys = pendingKnownAssetKeys.filter(
                (key) => key !== balance.assetKey
              );
            }
          ),
    ]);
    if (isOperationCurrent(generation, sessionId)) {
      pendingKnownAssetKeys = [];
      refreshing = false;
    }
  }

  function retryIncompleteDiscovery(): void {
    void refreshDiscovery(
      lifecycleGeneration,
      expectedSessionId,
      hasUnavailableKnownBalances ? [] : failedDiscoverySystemIds
    );
  }

  async function ensureRegisteredCoin(
    coin: CoinDefinition,
    generation: number,
    sessionId: string
  ): Promise<CoinDefinition> {
    if (!isOperationCurrent(generation, sessionId)) throw new Error('Stale wallet session');
    const existing = registryCoins.find(
      (candidate) =>
        normalize(candidate.id) === normalize(coin.id) ||
        (candidate.proto === coin.proto &&
          Boolean(coin.currencyId) &&
          normalize(candidate.currencyId) === normalize(coin.currencyId))
    );
    if (existing) return existing;

    try {
      const added = await coinsService.addCoinDefinition(coin, sessionId);
      if (!isOperationCurrent(generation, sessionId)) throw new Error('Stale wallet session');
      await refreshRegistry(generation);
      return added;
    } catch (error) {
      const errorType = extractWalletErrorType(error);
      if (errorType !== 'AssetAlreadyExists' && errorType !== 'DuplicatePbaasCurrency') throw error;
      if (!isOperationCurrent(generation, sessionId)) throw error;
      await refreshRegistry(generation);
      const recovered = registryCoins.find(
        (candidate) =>
          normalize(candidate.id) === normalize(coin.id) ||
          (candidate.proto === coin.proto &&
            Boolean(coin.currencyId) &&
            normalize(candidate.currencyId) === normalize(coin.currencyId))
      );
      if (!recovered) throw error;
      return recovered;
    }
  }

  async function resolveRowCoin(
    row: ManagedAssetRow,
    generation: number,
    sessionId: string
  ): Promise<CoinDefinition> {
    if (row.coin) return ensureRegisteredCoin(row.coin, generation, sessionId);
    const entry = row.catalogEntry;
    if (!entry) throw new Error(i18n.t('wallet.manageAssets.reviewRequired'));

    if (entry.addStrategy === 'direct') {
      const definition = catalogEntryToCoinDefinition(entry, network);
      if (!definition) throw new Error(i18n.t('wallet.addAsset.error.addFailed'));
      return ensureRegisteredCoin(definition, generation, sessionId);
    }
    if (entry.addStrategy === 'resolve_pbaas') {
      const result = await coinsService.resolvePbaasCurrency(pbaasLookupValue(entry));
      if (result.status === 'ambiguous') {
        throw new Error(i18n.t('wallet.addAsset.error.pbaasAmbiguous'));
      }
      if (!isOperationCurrent(generation, sessionId)) throw new Error('Stale wallet session');
      return ensureRegisteredCoin(
        applyCatalogMetadataToCoinDefinition(result.coin),
        generation,
        sessionId
      );
    }
    if (entry.addStrategy === 'resolve_erc20') {
      const result = await coinsService.resolveErc20Contract(erc20ContractValue(entry));
      if (!isOperationCurrent(generation, sessionId)) throw new Error('Stale wallet session');
      return ensureRegisteredCoin(
        applyCatalogMetadataToCoinDefinition(result.coin),
        generation,
        sessionId
      );
    }
    const runtime = matchingRegistryCoin(entry);
    if (!runtime) throw new Error(i18n.t('wallet.addAsset.error.addFailed'));
    return runtime;
  }

  function setPending(key: string, change: number): void {
    const next = Math.max(0, (pendingCounts[key] ?? 0) + change);
    pendingCounts = { ...pendingCounts, [key]: next };
  }

  async function runPortfolioVisibilityChange(
    row: ManagedAssetRow,
    desired: boolean,
    freezePlacement: boolean
  ): Promise<boolean> {
    if (savingPreference || !expectedSessionId) return false;
    const generation = lifecycleGeneration;
    const sessionId = expectedSessionId;
    const revision = (rowRevisions[row.key] ?? 0) + 1;
    rowRevisions = { ...rowRevisions, [row.key]: revision };
    rowErrors = { ...rowErrors, [row.key]: '' };
    retryDesired = { ...retryDesired, [row.key]: desired };
    if (freezePlacement) freezeVisitGroup(row);
    savingPreference = true;
    setPending(row.key, 1);

    try {
      const coin = desired ? await resolveRowCoin(row, generation, sessionId) : row.coin;
      if (!isOperationCurrent(generation, sessionId)) return false;
      const coinId = coin?.id ?? row.coinId;
      const nextPortfolioCoinIds = desired
        ? Array.from(new Set([...portfolioCoinIds, coinId]))
        : portfolioCoinIds.filter((id) => normalize(id) !== normalize(coinId));
      const nextHiddenAssetKeys = desired
        ? hiddenAssetKeys.filter((key) => key !== row.key)
        : Array.from(new Set([...hiddenAssetKeys, row.key]));

      const saved = await walletService.setAssetPreferences(
        sessionId,
        nextPortfolioCoinIds,
        nextHiddenAssetKeys
      );
      if (!isOperationCurrent(generation, sessionId)) return false;
      portfolioCoinIds = saved.portfolioCoinIds;
      hiddenAssetKeys = saved.hiddenAssetKeys;
      applyPortfolioStores();

      try {
        const activeCoins = registryCoins.filter((candidate) =>
          portfolioCoinIds.some((id) => normalize(id) === normalize(candidate.id))
        );
        const channels = buildWalletChannels(activeCoins, walletChannels.vrpcAddress);
        await walletService.startUpdateEngine({
          includeTransactions: false,
          priorityCoinIds: activeCoins.map((candidate) => candidate.id),
          priorityChannelIds: channels.channels,
        });
        if (!isOperationCurrent(generation, sessionId)) return true;
        refreshError = '';
      } catch {
        if (isOperationCurrent(generation, sessionId)) {
          refreshError = i18n.t('wallet.manageAssets.refreshAfterSaveFailed');
        }
      }
      return true;
    } catch (error) {
      if (isOperationCurrent(generation, sessionId) && rowRevisions[row.key] === revision) {
        rowErrors = {
          ...rowErrors,
          [row.key]: translateAssetError(error, 'wallet.manageAssets.saveFailed'),
        };
      }
      return false;
    } finally {
      if (componentMounted) {
        savingPreference = false;
        setPending(row.key, -1);
      }
    }
  }

  function setPortfolioVisibility(
    row: ManagedAssetRow,
    desired: boolean,
    freezePlacement = true
  ): Promise<boolean> {
    if (savingPreference) return Promise.resolve(false);
    const operation = runPortfolioVisibilityChange(row, desired, freezePlacement);
    activePreferenceSave = operation;
    void operation.finally(() => {
      if (activePreferenceSave === operation) activePreferenceSave = null;
    });
    return operation;
  }

  async function dismissDiscovery(row: ManagedAssetRow): Promise<void> {
    if (row.inPortfolio) return;
    await setPortfolioVisibility(row, false);
  }

  function normalizedErc20ContractCandidate(value: string): string | null {
    const trimmed = value.trim();
    if (/^0x[a-fA-F0-9]{40}$/.test(trimmed)) return trimmed;
    if (/^[a-fA-F0-9]{40}$/.test(trimmed)) return `0x${trimmed}`;
    return null;
  }

  function clearManualLookup(): void {
    manualLookupRevision += 1;
    manualResolving = false;
    manualResolvedCoin = null;
    manualCandidates = [];
    manualAdded = false;
    manualError = '';
    identifierCopyState = 'idle';
  }

  async function resolveManualAsset(inputOverride?: string): Promise<void> {
    const generation = lifecycleGeneration;
    const sessionId = expectedSessionId;
    const lookupRevision = ++manualLookupRevision;
    manualError = '';
    manualAdded = false;
    manualCandidates = [];
    manualResolvedCoin = null;
    if (inputOverride) manualInput = inputOverride;
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
        if (
          !isOperationCurrent(generation, sessionId) ||
          view !== 'manual' ||
          manualLookupRevision !== lookupRevision
        ) {
          return;
        }
        manualResolvedCoin = result.coin;
      } else {
        const result = await coinsService.resolvePbaasCurrency(input);
        if (
          !isOperationCurrent(generation, sessionId) ||
          view !== 'manual' ||
          manualLookupRevision !== lookupRevision
        ) {
          return;
        }
        if (result.status === 'ambiguous') {
          manualCandidates = result.candidates;
          manualError = i18n.t('wallet.addAsset.error.pbaasAmbiguous');
        } else {
          manualResolvedCoin = result.coin;
        }
      }
    } catch (error) {
      if (
        isOperationCurrent(generation, sessionId) &&
        view === 'manual' &&
        manualLookupRevision === lookupRevision
      ) {
        manualError = translateAssetError(
          error,
          contractCandidate
            ? 'wallet.addAsset.error.erc20ResolveFailed'
            : 'wallet.addAsset.error.pbaasResolveFailed'
        );
      }
    } finally {
      if (componentMounted && manualLookupRevision === lookupRevision) manualResolving = false;
    }
  }

  async function addResolvedManualAsset(): Promise<void> {
    if (!manualResolvedCoin || manualAdded || manualAdding || manualAlreadyShown) return;
    const generation = lifecycleGeneration;
    const sessionId = expectedSessionId;
    manualAdding = true;
    manualError = '';
    try {
      const registered = await ensureRegisteredCoin(
        applyCatalogMetadataToCoinDefinition(manualResolvedCoin),
        generation,
        sessionId
      );
      if (!isOperationCurrent(generation, sessionId) || view !== 'manual') return;
      const row = allRows.find((candidate) => candidate.key === assetKeyForCoin(registered)) ?? {
        key: assetKeyForCoin(registered),
        coinId: registered.id,
        currencyId: registered.currencyId,
        displayTicker: registered.displayTicker,
        displayName: registered.displayName,
        proto: registered.proto,
        coin: registered,
        catalogEntry: null,
        cataloged: false,
        inPortfolio: false,
        hidden: false,
        discovered: false,
        balance: null,
        balanceStatus: 'unavailable' as const,
        includesReadOnly: false,
        networks: [],
      };
      const saved = await setPortfolioVisibility(row, true, false);
      if (!isOperationCurrent(generation, sessionId) || view !== 'manual') return;
      if (saved) {
        manualAdded = true;
      } else {
        manualError = rowErrors[row.key] || i18n.t('wallet.manageAssets.saveFailed');
      }
    } catch (error) {
      manualError = translateAssetError(error, 'wallet.addAsset.error.addFailed');
    } finally {
      if (componentMounted) manualAdding = false;
    }
  }

  async function openManual(row?: ManagedAssetRow): Promise<void> {
    manualReturnFocus =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
    manualReturnFocusKey = row?.key ?? 'add-custom';
    manualLookupRevision += 1;
    view = 'manual';
    manualError = '';
    manualAdded = false;
    manualCandidates = [];
    manualInput = row?.currencyId ?? '';
    manualResolvedCoin = row?.coin ?? null;
    identifierCopyState = 'idle';
    await tick();
    manualInputElement?.focus();
  }

  async function closeManual(): Promise<void> {
    manualLookupRevision += 1;
    manualResolving = false;
    view = 'manage';
    await tick();
    const replacementFocus = Array.from(
      document.querySelectorAll<HTMLElement>('[data-manage-assets-return-focus]')
    ).find(
      (element) => element.getAttribute('data-manage-assets-return-focus') === manualReturnFocusKey
    );
    if (manualReturnFocus?.isConnected) manualReturnFocus.focus();
    else if (replacementFocus) replacementFocus.focus();
    else headingElement?.focus();
  }

  async function closeManageAssets(): Promise<void> {
    if (closing) return;
    closing = true;
    const saveSucceeded = activePreferenceSave ? await activePreferenceSave : true;
    if (!componentMounted) return;
    if (!saveSucceeded) {
      closing = false;
      return;
    }
    isOpen = false;
    onClose();
    closing = false;
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (!isOpen || event.key !== 'Escape') return;
    if (networkMenuOpen) return;
    event.preventDefault();
    if (view === 'manual') {
      void closeManual();
      return;
    }
    void closeManageAssets();
  }

  function handleTabKeydown(event: KeyboardEvent, current: ManageTab): void {
    const tabs: ManageTab[] = ['yours', 'browse', 'hidden'];
    const currentIndex = tabs.indexOf(current);
    let nextIndex: number | null = null;
    if (event.key === 'ArrowRight') nextIndex = (currentIndex + 1) % tabs.length;
    if (event.key === 'ArrowLeft') nextIndex = (currentIndex - 1 + tabs.length) % tabs.length;
    if (event.key === 'Home') nextIndex = 0;
    if (event.key === 'End') nextIndex = tabs.length - 1;
    if (nextIndex === null) return;
    event.preventDefault();
    tab = tabs[nextIndex];
    void tick().then(() =>
      document.querySelector<HTMLButtonElement>(`[data-manage-assets-tab="${tab}"]`)?.focus()
    );
  }

  async function copyResolvedIdentifier(): Promise<void> {
    if (!manualResolvedCoin) return;
    try {
      await navigator.clipboard.writeText(manualResolvedCoin.currencyId);
      identifierCopyState = 'copied';
    } catch {
      identifierCopyState = 'failed';
    }
  }

  function toggleExpanded(key: string): void {
    expandedAssetKeys = expandedAssetKeys.includes(key)
      ? expandedAssetKeys.filter((candidate) => candidate !== key)
      : [...expandedAssetKeys, key];
  }

  function displayedNetworks(row: ManagedAssetRow): ManagedAssetRow['networks'] {
    if (networkFilter === 'all') return row.networks;
    return row.networks.filter(
      (holdingNetwork) => normalize(holdingNetwork.systemId) === normalize(networkFilter)
    );
  }

  function displayedBalance(row: ManagedAssetRow): {
    balance: string | null;
    status: BalanceStatus;
  } {
    if (networkFilter === 'all') return { balance: row.balance, status: row.balanceStatus };
    const networks = displayedNetworks(row);
    if (networks.length === 0 || networks.every((holding) => holding.status === 'unavailable')) {
      return { balance: null, status: 'unavailable' };
    }
    const checked = networks.filter((holding) => holding.status !== 'unavailable');
    const total = checked.reduce((sum, holding) => sum + finiteAssetBalance(holding.balance), 0);
    return {
      balance: formatAssetBalance(total),
      status: networks.some((holding) => holding.status !== 'available') ? 'partial' : 'available',
    };
  }

  function updateScrollAffordance(element = listScrollElement): void {
    if (!element) {
      canScrollUp = false;
      canScrollDown = false;
      return;
    }
    const maxScrollTop = Math.max(0, element.scrollHeight - element.clientHeight);
    canScrollUp = maxScrollTop > 1 && element.scrollTop > 1;
    canScrollDown = maxScrollTop > 1 && element.scrollTop < maxScrollTop - 1;
  }

  function onManageAssetsScroll(event: Event): void {
    const target = event.currentTarget;
    if (target instanceof HTMLElement) updateScrollAffordance(target);
  }

  $effect(() => {
    shownRows.length;
    foundRows.length;
    browseRows.length;
    hiddenRows.length;
    visibleOtherRows.length;
    loading;
    tab;
    query;
    networkFilter;

    const element = listScrollElement;
    if (view !== 'manage' || !element) {
      canScrollUp = false;
      canScrollDown = false;
      return undefined;
    }

    const resizeObserver = new ResizeObserver(() => updateScrollAffordance(element));
    resizeObserver.observe(element);
    const viewportContent = element.querySelector('[data-scroll-area-content]');
    if (viewportContent instanceof HTMLElement) {
      resizeObserver.observe(viewportContent);
    } else if (element.lastElementChild instanceof HTMLElement) {
      resizeObserver.observe(element.lastElementChild);
    }
    void tick().then(() => {
      if (listScrollElement === element && view === 'manage') updateScrollAffordance(element);
    });

    return () => resizeObserver.disconnect();
  });

  $effect(() => {
    if (!isOpen) {
      lifecycleGeneration += 1;
      return undefined;
    }
    const generation = ++lifecycleGeneration;
    expectedSessionId = '';
    visitGroupByKey = {};
    discovery = null;
    knownBalances = [];
    pendingKnownAssetKeys = [];
    refreshing = false;
    void hydrate(generation);
    return () => {
      lifecycleGeneration += 1;
    };
  });

  onMount(async () => {
    await tick();
    headingElement?.focus();
  });

  onDestroy(() => {
    componentMounted = false;
    lifecycleGeneration += 1;
    manualLookupRevision += 1;
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<section
  class="flex h-full min-h-0 flex-col"
  aria-labelledby={view === 'manage' ? 'manage-assets-title' : 'add-custom-asset-title'}
>
  {#if view === 'manage'}
    <header class="flex h-8 shrink-0 items-center justify-between">
      <h1
        id="manage-assets-title"
        class="sr-only outline-none"
        tabindex="-1"
        bind:this={headingElement}
      >
        {i18n.t('wallet.manageAssets.title')}
      </h1>
      <NavigationBackButton
        label={i18n.t('wallet.assetDetails.back')}
        disabled={closing}
        onclick={closeManageAssets}
      />
      <Button
        type="button"
        variant="secondary"
        size="sm"
        class="h-8 gap-1.5 px-3 text-[13px]"
        data-manage-assets-return-focus="add-custom"
        onclick={() => openManual()}
      >
        <PlusIcon class="h-3.5 w-3.5" aria-hidden="true" />
        {i18n.t('wallet.manageAssets.addCustom')}
      </Button>
    </header>

    <div class="mt-4 flex h-10 shrink-0 gap-3">
      <Label class="relative min-w-0 flex-1">
        <span class="sr-only">{i18n.t('wallet.manageAssets.search')}</span>
        <SearchIcon
          class="pointer-events-none absolute top-3 left-3 h-4 w-4 text-muted-foreground"
        />
        <Input
          type="search"
          bind:value={query}
          placeholder={i18n.t('wallet.manageAssets.search')}
          autocomplete="off"
          spellcheck="false"
          class="h-10 w-full rounded-[7px] border-0 bg-muted/75 pr-3 pl-9 text-sm text-foreground outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring/55 dark:bg-muted/55"
        />
      </Label>

      <DropdownMenu.Root bind:open={networkMenuOpen}>
        <DropdownMenu.Trigger
          class="flex h-10 w-[180px] shrink-0 items-center justify-between rounded-[7px] bg-muted/75 px-3 text-[13px] text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring/55 dark:bg-muted/55"
          aria-label={i18n.t('wallet.manageAssets.filterNetwork')}
        >
          <span class="truncate">{currentNetworkLabel}</span>
          <ChevronDownIcon class="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="end" class="w-[244px]">
          <DropdownMenu.Label class="flex justify-between font-normal text-muted-foreground">
            <span>{i18n.t('wallet.manageAssets.showAssetsOn')}</span>
            <span>{i18n.t(`wallet.manageAssets.environment.${network}`)}</span>
          </DropdownMenu.Label>
          <DropdownMenu.RadioGroup value={networkFilter}>
            <DropdownMenu.RadioItem value="all" onclick={() => (networkFilter = 'all')}>
              {i18n.t('wallet.manageAssets.allNetworks')}
            </DropdownMenu.RadioItem>
            {#each networkOptions as option (option.id)}
              <DropdownMenu.RadioItem value={option.id} onclick={() => (networkFilter = option.id)}>
                {option.label}
              </DropdownMenu.RadioItem>
            {/each}
          </DropdownMenu.RadioGroup>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    </div>

    <div class="mt-4 grid h-9 shrink-0 grid-cols-[minmax(0,1fr)_190px_84px] items-center border-b">
      <div
        class="flex h-9 min-w-0 gap-5"
        role="tablist"
        aria-label={i18n.t('wallet.manageAssets.title')}
      >
        {#each ['yours', 'browse', 'hidden'] as item (item)}
          <button
            type="button"
            role="tab"
            aria-selected={tab === item}
            aria-controls="manage-assets-panel"
            id={`manage-assets-tab-${item}`}
            data-manage-assets-tab={item}
            tabindex={tab === item ? 0 : -1}
            class={`h-9 shrink-0 border-b-2 px-0 text-[13px] transition-colors focus-visible:ring-2 focus-visible:ring-ring/55 focus-visible:outline-none ${
              tab === item
                ? 'border-primary font-medium text-foreground'
                : 'border-transparent text-muted-foreground hover:text-foreground'
            }`}
            onclick={() => (tab = item as ManageTab)}
            onkeydown={(event) => handleTabKeydown(event, item as ManageTab)}
          >
            {i18n.t(`wallet.manageAssets.tab.${item}`)}
          </button>
        {/each}
      </div>
      <span class="text-right text-xs text-muted-foreground"
        >{i18n.t('wallet.manageAssets.balance')}</span
      >
      <span class="text-right text-xs text-muted-foreground"
        >{i18n.t('wallet.manageAssets.inPortfolio')}</span
      >
    </div>

    {#if discoveryError || discoveryStale || refreshError || hasPartialDiscoveryCoverage || hasUnavailableKnownBalances}
      <div class="mt-3 flex min-h-6 shrink-0 items-center justify-between gap-3 text-xs">
        <div class="flex min-w-0 items-center gap-1.5 text-muted-foreground">
          <InfoIcon class="h-3.5 w-3.5 shrink-0" />
          <span class="truncate">
            {discoveryStale
              ? i18n.t('wallet.manageAssets.stale')
              : discoveryError ||
                refreshError ||
                (discovery?.scopeMetadataComplete === false
                  ? i18n.t('wallet.manageAssets.scopeMetadataPartial')
                  : i18n.t('wallet.manageAssets.discoveryPartial'))}
          </span>
        </div>
        {#if discoveryError || discoveryStale || hasPartialDiscoveryCoverage || hasUnavailableKnownBalances}
          <button
            type="button"
            class="shrink-0 font-medium text-text-action hover:text-text-action hover:underline disabled:opacity-50"
            disabled={refreshing}
            onclick={retryIncompleteDiscovery}
          >
            {i18n.t('common.retry')}
          </button>
        {/if}
      </div>
    {/if}

    <div
      id="manage-assets-panel"
      class="relative mt-3 min-h-0 flex-1"
      role="tabpanel"
      aria-labelledby={`manage-assets-tab-${tab}`}
      aria-busy={loading}
    >
      <ScrollArea.Root class="h-full" type="scroll">
        <ScrollArea.Viewport
          class="h-full overscroll-contain pr-1"
          bind:ref={listScrollElement}
          onscroll={onManageAssetsScroll}
        >
          {#if loading}
            <p class="sr-only" role="status">{i18n.t('wallet.manageAssets.checking')}</p>
            <div aria-hidden="true">
              {#each [0, 1, 2] as index}
                <div
                  class={`grid min-h-[72px] grid-cols-[minmax(0,1fr)_190px_84px] items-center px-1 ${index < 2 ? 'border-b' : ''}`}
                >
                  <div class="flex items-center gap-3">
                    <div class="flex w-9 shrink-0 justify-center">
                      <Skeleton class="h-8 w-8 rounded-full" />
                    </div>
                    <Skeleton class={`h-4 rounded-sm ${index === 1 ? 'w-28' : 'w-36'}`} />
                  </div>
                  <Skeleton class="ml-auto h-4 w-24 rounded-sm" />
                  <Skeleton class="ml-auto h-5 w-[34px] rounded-full" />
                </div>
              {/each}
            </div>
          {:else}
            {#if tab === 'hidden' && !hasQuery}
              <p class="mb-3 text-xs text-muted-foreground">
                {i18n.t('wallet.manageAssets.hiddenHelp')}
              </p>
            {/if}

            {#if hasQuery}
              {@render AssetSection({ rows: searchCatalogRows })}
            {:else if tab === 'yours'}
              {#if foundRows.length > 0}
                {@render AssetSection({
                  title: i18n.t('wallet.manageAssets.found'),
                  rows: foundRows,
                  hideLastDivider: shownRows.length === 0,
                })}
              {/if}
              {#if shownRows.length > 0}
                {@render AssetSection({ rows: shownRows, hideLastDivider: true })}
              {/if}
            {:else if tab === 'browse'}
              {@render AssetSection({ rows: browseRows })}
            {:else}
              {@render AssetSection({ rows: hiddenRows })}
            {/if}

            {#if visibleOtherRows.length > 0 && (tab !== 'browse' || hasQuery)}
              <section class="mt-4">
                <button
                  type="button"
                  class="flex h-9 w-full items-center justify-between px-1 text-left text-[13px] font-medium text-foreground focus-visible:ring-2 focus-visible:ring-ring/55 focus-visible:outline-none"
                  aria-expanded={otherAssetsExpanded}
                  onclick={() => (otherAssetsExpanded = !otherAssetsExpanded)}
                >
                  <span>{i18n.t('wallet.manageAssets.otherFound')}</span>
                  {#if otherAssetsExpanded}
                    <ChevronDownIcon class="h-4 w-4 text-muted-foreground" />
                  {:else}
                    <ChevronRightIcon class="h-4 w-4 text-muted-foreground" />
                  {/if}
                </button>
                {#if otherAssetsExpanded || hasQuery}
                  {#each visibleOtherRows as row (row.key)}
                    {@const balance = displayedBalance(row)}
                    <div class="asset-row mt-2 rounded-xl bg-muted/65 px-3 dark:bg-muted/55">
                      <div class="flex min-h-[68px] items-center gap-3 py-2">
                        <div class="flex w-9 shrink-0 justify-center">
                          <CoinIcon
                            coinId={row.coinId}
                            coinName={row.displayName}
                            proto={row.proto}
                            size={32}
                            showBadge
                            decorative
                          />
                        </div>
                        <div class="min-w-0 flex-1">
                          <p class="truncate text-sm font-medium text-foreground">
                            {row.displayName}
                          </p>
                          <p class="truncate text-xs text-muted-foreground">
                            {row.displayTicker} · {displayedNetworks(row)[0]?.systemDisplayName}
                            {row.hidden ? ` · ${i18n.t('wallet.manageAssets.hiddenStatus')}` : ''}
                          </p>
                        </div>
                        <div
                          class="w-[150px] shrink-0 text-right text-sm font-medium tabular-nums"
                          data-manage-assets-balance={row.key}
                        >
                          {#if balance.status === 'loading'}
                            <span class="text-muted-foreground">{i18n.t('common.loading')}</span>
                          {:else if balance.status === 'unavailable'}
                            <span>{i18n.t('wallet.manageAssets.unavailable')}</span>
                          {:else if balance.status === 'partial' && balance.balance === null}
                            <span>{i18n.t('wallet.manageAssets.partial')}</span>
                          {:else}
                            <span
                              title={`${displayBalanceAmount(balance.balance)} ${row.displayTicker}`}
                            >
                              {compactBalanceLabel(row, balance.balance ?? '')}
                            </span>
                            {#if balance.status === 'partial'}
                              <span class="block text-[11px] font-normal text-muted-foreground">
                                {i18n.t('wallet.manageAssets.partial')}
                              </span>
                            {/if}
                          {/if}
                        </div>
                        <div class="flex w-[82px] shrink-0 justify-end gap-2">
                          <button
                            type="button"
                            class="text-xs font-medium text-text-action hover:text-text-action hover:underline disabled:opacity-50"
                            data-manage-assets-return-focus={row.key}
                            disabled={savingPreference}
                            onclick={() => openManual(row)}
                          >
                            {i18n.t('wallet.manageAssets.review')}
                          </button>
                          {#if !row.hidden}
                            <button
                              type="button"
                              class="text-muted-foreground disabled:opacity-50"
                              aria-label={i18n.t('wallet.manageAssets.dismissAsset', {
                                asset: row.displayName,
                              })}
                              title={i18n.t('wallet.manageAssets.dismiss')}
                              disabled={savingPreference}
                              onclick={() => dismissDiscovery(row)}
                            >
                              <EyeOffIcon class="h-4 w-4" />
                            </button>
                          {/if}
                        </div>
                      </div>
                      {#if rowErrors[row.key]}
                        <div
                          class="flex min-h-7 items-start justify-end gap-2 pb-1 text-xs"
                          role="alert"
                        >
                          <span class="text-destructive">{rowErrors[row.key]}</span>
                          <button
                            type="button"
                            class="font-medium text-text-action hover:text-text-action hover:underline"
                            disabled={savingPreference}
                            onclick={() =>
                              setPortfolioVisibility(
                                row,
                                retryDesired[row.key] ?? !row.inPortfolio
                              )}
                          >
                            {i18n.t('common.retry')}
                          </button>
                        </div>
                      {/if}
                    </div>
                  {/each}
                {/if}
              </section>
            {/if}

            {#if (hasQuery && searchCatalogRows.length === 0 && visibleOtherRows.length === 0) || (!hasQuery && tab === 'yours' && foundRows.length === 0 && shownRows.length === 0 && visibleOtherRows.length === 0) || (!hasQuery && tab === 'browse' && browseRows.length === 0) || (!hasQuery && tab === 'hidden' && hiddenRows.length === 0 && visibleOtherRows.length === 0)}
              <div class="flex h-40 flex-col items-center justify-center px-6 text-center">
                <p class="text-sm font-medium text-foreground">
                  {query
                    ? i18n.t('wallet.manageAssets.noTabMatches', {
                        tab: i18n.t(`wallet.manageAssets.tab.${tab}`),
                      })
                    : i18n.t(`wallet.manageAssets.empty.${tab}`)}
                </p>
                {#if !hasQuery && tab !== 'browse'}
                  <button
                    type="button"
                    class="mt-2 text-xs font-medium text-text-action hover:text-text-action hover:underline"
                    onclick={() => (tab = 'browse')}
                  >
                    {i18n.t('wallet.manageAssets.searchAll')}
                  </button>
                {/if}
              </div>
            {/if}
          {/if}
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>

      {#if canScrollUp}
        <div
          class="pointer-events-none absolute inset-x-0 top-0 h-14 bg-gradient-to-b from-background to-transparent dark:from-app-canvas"
          data-manage-assets-scroll-fade="top"
          aria-hidden="true"
        ></div>
      {/if}

      {#if canScrollDown}
        <div
          class="pointer-events-none absolute inset-x-0 bottom-0 h-14 bg-gradient-to-t from-background to-transparent dark:from-app-canvas"
          data-manage-assets-scroll-fade="bottom"
          aria-hidden="true"
        ></div>
      {/if}
    </div>
  {:else}
    <header class="shrink-0">
      <NavigationBackButton label={i18n.t('wallet.manageAssets.back')} onclick={closeManual} />
      <h1 id="add-custom-asset-title" class="mt-6 text-xl leading-7 font-semibold text-foreground">
        {i18n.t('wallet.manageAssets.addCustom')}
      </h1>
    </header>

    <ScrollArea.Root class="-mx-1 mt-7 min-h-0 flex-1" type="scroll">
      <ScrollArea.Viewport class="h-full overscroll-contain px-1">
        <div class="pb-1">
          <form
            onsubmit={(event) => {
              event.preventDefault();
              if (!manualResolving && !manualAdding) resolveManualAsset();
            }}
          >
            <Label class="text-[13px] text-foreground" for="custom-asset-input">
              {manualLookupLabel}
            </Label>
            <div class="mt-2 flex gap-3">
              <div class="min-w-0 flex-1">
                <Input
                  id="custom-asset-input"
                  bind:ref={manualInputElement}
                  bind:value={manualInput}
                  oninput={clearManualLookup}
                  autocomplete="off"
                  spellcheck={false}
                  disabled={manualAdding}
                  class="h-10 border-0 bg-muted/75 focus-visible:ring-2 dark:bg-muted/55"
                />
              </div>
              <Button
                type="submit"
                variant="secondary"
                class="h-10 w-[116px]"
                disabled={manualResolving || manualAdding}
              >
                {manualResolving
                  ? i18n.t('wallet.addAsset.resolving')
                  : i18n.t('wallet.manageAssets.findAsset')}
              </Button>
            </div>
          </form>

          {#if manualCandidates.length > 1}
            <div class="mt-6 space-y-1">
              <p class="text-xs text-muted-foreground">{i18n.t('wallet.addAsset.pbaasMatches')}</p>
              {#each manualCandidates as candidate (candidate.currencyId)}
                <button
                  type="button"
                  class="block max-w-full truncate text-left text-sm text-text-action hover:text-text-action hover:underline"
                  onclick={() => resolveManualAsset(candidate.currencyId)}
                >
                  {candidate.displayName} · {candidate.currencyId}
                </button>
              {/each}
            </div>
          {/if}

          {#if manualResolvedCoin}
            <section class="mt-6 rounded-xl bg-asset-result-surface p-4">
              <div class="flex items-center gap-3 pb-4">
                <div class="flex w-9 shrink-0 justify-center">
                  <CoinIcon
                    coinId={manualResolvedCoin.id}
                    coinName={manualResolvedCoin.displayName}
                    proto={manualResolvedCoin.proto}
                    size={36}
                    showBadge
                    decorative
                  />
                </div>
                <div class="min-w-0 flex-1">
                  <p class="truncate text-[15px] font-semibold text-foreground">
                    {manualResolvedCoin.displayName}
                  </p>
                  <p class="truncate text-[13px] text-muted-foreground">
                    {manualResolvedCoin.displayTicker} ·
                    {manualResolvedCoin.proto === 'erc20'
                      ? i18n.t('wallet.manageAssets.ethereumErc20')
                      : i18n.t('wallet.manageAssets.verusPbaas')}
                  </p>
                </div>
              </div>

              <dl class="space-y-3 border-t pt-3.5 text-[13px]">
                <div class="flex gap-3">
                  <dt class="w-[120px] shrink-0 text-muted-foreground">
                    {manualResolvedCoin.proto === 'erc20'
                      ? i18n.t('wallet.manageAssets.contractAddress')
                      : i18n.t('wallet.manageAssets.currencyId')}
                  </dt>
                  <dd class="flex min-w-0 flex-1 items-start gap-1.5 text-foreground">
                    <IdentifierText
                      value={manualResolvedCoin.currencyId}
                      mode="full"
                      class="min-w-0 flex-1 font-mono text-[13px] leading-5"
                    />
                    <CopyButton
                      size="xs"
                      copied={identifierCopyState === 'copied'}
                      aria-label={i18n.t(
                        identifierCopyState === 'copied' ? 'common.copied' : 'common.copy'
                      )}
                      onclick={copyResolvedIdentifier}
                    />
                  </dd>
                </div>
                <div class="flex gap-3">
                  <dt class="w-[120px] shrink-0 text-muted-foreground">
                    {i18n.t('wallet.manageAssets.balance')}
                  </dt>
                  <dd class="min-w-0 flex-1 text-foreground tabular-nums">
                    {#if !manualResolvedBalance || manualResolvedBalance.status === 'unavailable'}
                      <span class="text-muted-foreground"
                        >{i18n.t('wallet.manageAssets.unavailable')}</span
                      >
                    {:else if manualResolvedBalance.status === 'loading'}
                      <span class="text-muted-foreground">{i18n.t('common.loading')}</span>
                    {:else}
                      <span class="font-medium"
                        >{displayBalanceAmount(manualResolvedBalance.balance)}
                        {manualResolvedCoin.displayTicker}</span
                      >
                      {#if manualResolvedBalance.status === 'partial'}
                        <span class="block text-[11px] font-normal text-muted-foreground">
                          {i18n.t('wallet.manageAssets.partial')}
                        </span>
                      {/if}
                    {/if}
                  </dd>
                </div>
              </dl>

              {#if identifierCopyState === 'failed'}
                <p class="mt-2 text-xs text-destructive" role="status">
                  {i18n.t('common.copyFailed')}
                </p>
              {/if}

              <div class="flex justify-end pt-[18px]">
                <Button
                  type="button"
                  variant={manualAlreadyShown || manualAdded ? 'secondary' : 'default'}
                  class="h-10 min-w-[164px] gap-1.5 px-4 text-[13px] {manualAlreadyShown ||
                  manualAdded
                    ? 'bg-contact-saved text-contact-saved-foreground hover:bg-contact-saved aria-disabled:opacity-100'
                    : ''}"
                  aria-busy={manualAdding}
                  aria-disabled={manualAdding || manualAlreadyShown || manualAdded}
                  data-manage-assets-success={manualAlreadyShown || manualAdded ? '' : undefined}
                  onclick={addResolvedManualAsset}
                >
                  {#if manualAlreadyShown || manualAdded}<CheckIcon
                      class="h-3.5 w-3.5"
                      aria-hidden="true"
                    />{/if}
                  {manualAlreadyShown || manualAdded
                    ? i18n.t('wallet.manageAssets.shownSuccess')
                    : manualAdding
                      ? i18n.t('wallet.manageAssets.saving')
                      : i18n.t('wallet.manageAssets.showInPortfolio')}
                </Button>
              </div>
              <span class="sr-only" aria-live="polite">
                {manualAdded ? i18n.t('wallet.manageAssets.shownSuccess') : ''}
              </span>
            </section>
          {/if}

          {#if manualError}
            <p class="mt-5 flex items-start gap-1.5 text-xs text-destructive" role="alert">
              <AlertCircleIcon class="mt-0.5 h-3.5 w-3.5 shrink-0" />
              {manualError}
            </p>
          {/if}
        </div>
      </ScrollArea.Viewport>
      <ScrollArea.Scrollbar orientation="vertical" />
    </ScrollArea.Root>
  {/if}
</section>

{#snippet AssetSection({
  title = '',
  rows,
  hideLastDivider = false,
}: {
  title?: string;
  rows: ManagedAssetRow[];
  hideLastDivider?: boolean;
})}
  <section>
    {#if title}
      <h2 class="h-8 text-[13px] leading-8 font-medium text-foreground">{title}</h2>
    {/if}
    {#each rows as row, index (row.key)}
      {@const rowBalance = displayedBalance(row)}
      {@const networks = displayedNetworks(row)}
      <div class="asset-row">
        <div
          class={`grid min-h-[72px] grid-cols-[minmax(0,1fr)_190px_84px] items-center px-1 ${hideLastDivider && index === rows.length - 1 ? '' : 'border-b'}`}
        >
          <div class="flex min-w-0 items-center gap-3 pr-3">
            <div class="flex w-9 shrink-0 justify-center">
              <CoinIcon
                coinId={row.coinId}
                coinName={row.displayName}
                proto={row.proto}
                size={32}
                showBadge
                decorative
              />
            </div>
            <div class="min-w-0 flex-1">
              <div class="flex min-w-0 items-center gap-1.5">
                <p class="truncate text-sm font-medium text-foreground">{row.displayName}</p>
                {#if networks.length > 1}
                  <button
                    type="button"
                    class="shrink-0 text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring/55 focus-visible:outline-none"
                    aria-label={i18n.t('wallet.manageAssets.showNetworkBreakdown', {
                      asset: row.displayName,
                    })}
                    aria-expanded={expandedAssetKeys.includes(row.key)}
                    onclick={() => toggleExpanded(row.key)}
                  >
                    {#if expandedAssetKeys.includes(row.key)}
                      <ChevronDownIcon class="h-3.5 w-3.5" />
                    {:else}
                      <ChevronRightIcon class="h-3.5 w-3.5" />
                    {/if}
                  </button>
                {/if}
              </div>
              {#if networks.length > 1 || row.includesReadOnly || tab !== 'yours' || hasQuery}
                <p class="truncate text-xs text-muted-foreground">
                  {row.displayTicker} ·
                  {networks.length > 1
                    ? i18n.t('wallet.manageAssets.networkCount', { count: networks.length })
                    : networks[0]?.systemDisplayName}
                  {row.includesReadOnly ? ` · ${i18n.t('wallet.manageAssets.readOnly')}` : ''}
                </p>
              {/if}
            </div>
          </div>

          <div class="truncate text-right text-sm font-medium text-foreground tabular-nums">
            {#if rowBalance.status === 'loading'}
              <span class="text-muted-foreground">{i18n.t('common.loading')}</span>
            {:else if rowBalance.status === 'unavailable'}
              <span>{i18n.t('wallet.manageAssets.unavailable')}</span>
            {:else if rowBalance.status === 'partial' && rowBalance.balance === null}
              <span>{i18n.t('wallet.manageAssets.partial')}</span>
            {:else}
              <span>{displayBalanceAmount(rowBalance.balance)} {row.displayTicker}</span>
              {#if rowBalance.status === 'partial'}
                <span class="block text-[11px] font-normal text-muted-foreground">
                  {i18n.t('wallet.manageAssets.partial')}
                </span>
              {/if}
            {/if}
          </div>

          <div class="flex items-center justify-end gap-2">
            {#if row.discovered && !row.inPortfolio && !row.hidden}
              <button
                type="button"
                class="inline-flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring/55 focus-visible:outline-none"
                aria-label={i18n.t('wallet.manageAssets.dismissAsset', { asset: row.displayName })}
                title={i18n.t('wallet.manageAssets.dismiss')}
                disabled={savingPreference}
                onclick={() => dismissDiscovery(row)}
              >
                <EyeOffIcon class="h-3.5 w-3.5" />
              </button>
            {/if}
            <button
              type="button"
              role="switch"
              aria-checked={row.inPortfolio}
              aria-label={i18n.t('wallet.manageAssets.toggleAsset', { asset: row.displayName })}
              class={`relative h-5 w-[34px] shrink-0 rounded-full p-0.5 transition-colors focus-visible:ring-2 focus-visible:ring-ring/55 focus-visible:ring-offset-2 focus-visible:outline-none ${
                row.inPortfolio ? 'bg-primary' : 'bg-zinc-300 dark:bg-zinc-600'
              }`}
              disabled={savingPreference}
              onclick={() => setPortfolioVisibility(row, !row.inPortfolio)}
            >
              <span
                class={`block h-4 w-4 rounded-full bg-white transition-transform ${
                  row.inPortfolio ? 'translate-x-3.5' : 'translate-x-0'
                }`}
              ></span>
            </button>
          </div>
        </div>

        {#if expandedAssetKeys.includes(row.key) && networks.length > 1}
          <div class="pb-2 pl-12">
            {#each networks as holdingNetwork (holdingNetwork.systemId)}
              <div class="grid h-8 grid-cols-[minmax(0,1fr)_174px_94px] items-center text-xs">
                <span class="truncate text-muted-foreground">
                  {holdingNetwork.systemDisplayName}
                </span>
                <span class="text-right text-foreground tabular-nums">
                  {holdingNetwork.status === 'unavailable'
                    ? i18n.t('wallet.manageAssets.unavailable')
                    : `${displayBalanceAmount(holdingNetwork.balance)} ${row.displayTicker}`}
                </span>
              </div>
            {/each}
          </div>
        {/if}

        {#if rowErrors[row.key]}
          <div class="flex min-h-7 items-start justify-end gap-2 pb-1 text-xs" role="alert">
            <span class="text-destructive">{rowErrors[row.key]}</span>
            <button
              type="button"
              class="font-medium text-text-action hover:text-text-action hover:underline"
              disabled={savingPreference}
              onclick={() => setPortfolioVisibility(row, retryDesired[row.key] ?? !row.inPortfolio)}
            >
              {i18n.t('common.retry')}
            </button>
          </div>
        {/if}
      </div>
    {/each}
  </section>
{/snippet}
