<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
  import PencilIcon from '@lucide/svelte/icons/pencil';
  import XIcon from '@lucide/svelte/icons/x';
  import { Button } from '$lib/components/ui/button';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import * as RadioGroup from '$lib/components/ui/radio-group';
  import * as Card from '$lib/components/ui/card';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import * as Tabs from '$lib/components/ui/tabs';
  import InlineTextActionButton from '$lib/components/common/InlineTextActionButton.svelte';
  import SearchInput from '$lib/components/common/SearchInput.svelte';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import WalletTransferStepperShell from '$lib/components/shared/WalletTransferStepperShell.svelte';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import TransferSummaryRail from './transfer-wizard/TransferSummaryRail.svelte';
  import EthRecoveryReviewCard from './transfer-wizard/EthRecoveryReviewCard.svelte';
  import { i18nStore } from '$lib/i18n';
  import { resolveCoinPresentation, resolveCoinPresentationById } from '$lib/coins/presentation.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { balanceStore } from '$lib/stores/balances.js';
  import { networkStore } from '$lib/stores/network.js';
  import { ratesStore } from '$lib/stores/rates.js';
  import { settingsStore } from '$lib/stores/settings.js';
  import { transactionStore } from '$lib/stores/transactions.js';
  import { addressBookStore } from '$lib/stores/addressBook.js';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import { formatFiatAmount, getRateForCurrency } from '$lib/utils/fiatDisplay.js';
  import * as addressBookService from '$lib/services/addressBookService.js';
  import {
    findMatchingSavedEndpoint,
    isEndpointCompatibleWithDestinationKind,
    normalizeAddressByDestinationKind,
    sharesSuspiciousPrefixSuffix,
  } from '$lib/address-book/utils';
  import {
    classifyDlightDestinationAddress,
    validateDestinationAddressForKind,
  } from '$lib/transfer/recipientAddressValidation';
  import { openTrustedExternalUrl } from '$lib/utils/externalLinks.js';
  import * as walletService from '$lib/services/walletService.js';
  import {
    getDisplayCoinScopes,
    isWalletDisplayRequestInvalidated,
  } from '$lib/services/walletDisplayService.js';
  import {
    acknowledgePendingEthSubmission,
    getPendingEthSubmission,
    preflightSend,
    resumePendingEthSubmission,
    sendTransaction,
  } from '$lib/services/txService.js';
  import { isForcedWalletLockError } from '$lib/services/walletLockCoordinator.js';
  import {
    estimateBridgeConversion,
    estimateBridgeExportFee,
    getBridgeCapabilities,
    getBridgeConversionPaths,
    preflightBridgeTransfer,
  } from '$lib/services/bridgeTransferService.js';
  import { getRecipientInputCopy, getTransferStepLabels } from '$lib/transfer/transferWizardCopy';
  import { EthRecoveryLifetime, runEthRecovery } from '$lib/transfer/ethRecovery.js';
  import {
    buildReceiveAssetSections,
    type ExportRouteOption,
    filterReceiveAssetSectionsByQuery,
    type ReceiveAssetOption,
    type ReceiveAssetSections,
    type ViaRouteOption,
  } from '$lib/transfer/convertTargetOptions';
  import type {
    BridgeCapabilitiesResult,
    BridgeConversionPathQuote,
    BridgeExportFeeEstimateResult,
    BridgeTransferPreflightResult,
    CoinScope,
    DlightRuntimeStatusResult,
    EthPendingSubmissionReview,
    PreflightResult,
    PreflightWarning,
    SendResult,
    TxSendProgressEventPayload,
    TxSendProgressStage,
    WalletNetwork,
  } from '$lib/types/wallet.js';
  import type { AddressBookContact, AddressEndpointKind } from '$lib/types/addressBook';
  import type {
    DestinationAddressKind,
    TransferEntryContext,
    TransferStepId,
    TransferStepperStep,
    WizardOperationalStepId,
  } from './transfer-wizard/types';
  import { extractWalletErrorMessage, extractWalletErrorType } from '$lib/utils/walletErrors.js';
  import {
    DEFAULT_DIRECT_SEND_FEE_MODE,
    directSendFeeDraftForOpen,
    type DirectSendFeeMode,
    directSendFeeSelectionNeedsRefresh,
    type DirectSendRouteState,
    getDirectSendFeeOptions,
    isDirectSendFeeEligible,
    withDirectSendFeeMode,
  } from './transfer-wizard/directSendFee';
  import { PreflightRequestGuard } from './transfer-wizard/preflightRequestGuard';
  import {
    preflightRequestSignature,
    type ResolvedPreflightRequest,
    runGuardedPreflight,
  } from './transfer-wizard/preflightRequest';
  import {
    classifySubmittedTransferRoute,
    finalizeSubmittedTransferSnapshot,
    getSubmittedReceiptCopy,
    type SubmittedTransferSnapshot,
    transactionExplorerUrl,
  } from './transfer-wizard/transferReceipt';
  import {
    buildSpendableTransferSources,
    transferSourceSupportsConversion,
  } from './transfer-wizard/transferSources';
  import { EntryContextGuard } from './transfer-wizard/entryContextGuard';

  type EntryIntent = 'send' | 'convert';

  type AddressBookEndpointOption = {
    contactId: string;
    contactName: string;
    endpointId: string;
    endpointKind: AddressEndpointKind;
    endpointLabel: string;
    endpointAddress: string;
    normalizedAddress: string;
    lastUsedAt: number | null;
  };

  type SameAssetOption = {
    id: string;
    label: string;
    destinationId: string;
    receiveLabel: string;
    ethDestination?: boolean;
    convertTo?: string | null;
    exportTo?: string | null;
    via?: string | null;
    mapTo?: string | null;
    price?: string | null;
  };

  type SummaryRow = {
    label: string;
    primary: string;
    secondary?: string;
    breakAll?: boolean;
    primaryIdentifier?: boolean;
    secondaryIdentifier?: boolean;
    iconCoinId?: string;
    iconCoinName?: string;
  };

  type ConversionFeeInfo = {
    amount: string;
    currency: string;
    percentage: string;
  };

  type BridgeFeeInfo = {
    loading: boolean;
    feeCoins: string | null;
    feeSats: string | null;
    balanceCoins: string | null;
    currencyTicker: string;
    systemId: string | null;
    error: string | null;
  };

  type TransferWizardProps = {
    entryIntent: EntryIntent;
    entryContext?: TransferEntryContext | null;
    onClose?: () => void;
    walletNetwork?: WalletNetwork;
  };

  const defaultClose = () => {};
  const OPERATIONAL_STEPS: WizardOperationalStepId[] = ['details', 'review'];
  const VRSC_SYSTEM_ID = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
  const VRSCTEST_SYSTEM_ID = 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq';
  const VETH_SYSTEM_ID = 'i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X';
  const MAX_TRANSFER_AMOUNT_FRACTION_DIGITS = 8;

  let {
    entryIntent,
    entryContext = null,
    onClose = defaultClose,
    walletNetwork = 'mainnet',
  }: TransferWizardProps = $props();

  const i18n = $derived($i18nStore);
  const coins = $derived($coinsStore);
  const balances = $derived($balanceStore);
  const chainInfoByChannel = $derived($networkStore);
  const rates = $derived($ratesStore);
  const settings = $derived($settingsStore);
  const displayCurrency = $derived(settings.displayCurrency);
  const addressBookContacts = $derived($addressBookStore);
  const stepLabels = $derived(getTransferStepLabels(i18n.t));

  const sendableCoins = $derived(
    coins.filter(
      (coin) =>
        coin.compatibleChannels.includes('vrpc') ||
        coin.compatibleChannels.includes('btc') ||
        coin.compatibleChannels.includes('eth') ||
        coin.compatibleChannels.includes('erc20')
    )
  );

  let coinScopesByCoinId = $state<Record<string, CoinScope[]>>({});
  let loadedScopeCoinIds = $state<Record<string, true>>({});

  const sendableCoinOptions = $derived(
    buildSpendableTransferSources(sendableCoins, coinScopesByCoinId, balances).map((option) => {
      const presentation = resolveCoinPresentation(option.coin);
      return {
        ...option,
        displayName: presentation.displayName,
        displayTicker: presentation.displayTicker,
      };
    })
  );

  const positiveSendableCoinOptions = $derived(sendableCoinOptions);

  const selectedCoinOption = $derived(
    positiveSendableCoinOptions.find(
      (option) => option.coin.id === selectedCoinId && option.channelId === selectedSourceChannelId
    ) ?? null
  );

  let selectedCoinId = $state('');
  let selectedSourceChannelId = $state('');
  let currentStep = $state<TransferStepId>('details');
  let amount = $state('');
  let amountInputEl = $state<HTMLInputElement | null>(null);
  let destinationAddress = $state('');
  let memo = $state('');
  let conversionEnabled = $state(false);
  let conversionInitialized = $state(false);
  let selectedReceiveAssetId = $state('');
  let selectedExportSystemId = $state<string | null>(null);
  let selectedSendExportSystemId = $state<string | null>(null);
  let selectedViaOptionId = $state('');
  let manualViaLocked = $state(false);
  let sourceCoinManuallyChosen = $state(false);
  const entryContextGuard = new EntryContextGuard();
  let discoveredPathQuotes = $state<Record<string, BridgeConversionPathQuote[]>>({});
  let receiveSearchTerm = $state('');
  let pendingGroupedReceiveOption = $state<ReceiveAssetOption | null>(null);
  let pendingTargetOption = $state<ReceiveAssetOption | null>(null);
  let routeEstimateOutputs = $state<Record<string, string>>({});
  let bridgeCapabilities = $state<BridgeCapabilitiesResult | null>(null);
  let bridgeFeeInfo = $state<BridgeFeeInfo>({
    loading: false,
    feeCoins: null,
    feeSats: null,
    balanceCoins: null,
    currencyTicker: resolveCoinPresentationById('VRSC')?.displayTicker?.trim() || 'VRSC',
    systemId: null,
    error: null,
  });

  let loadingTargets = $state(false);
  let preflighting = $state(false);
  let sending = $state(false);
  let sendStage = $state<TxSendProgressStage | null>(null);
  let sendStageStartedAt = $state<number | null>(null);
  let sendStageTick = $state(0);
  let targetsError = $state('');
  let transferError = $state('');
  let pendingEthSubmission = $state<EthPendingSubmissionReview | null>(null);
  let recoveringEthSubmission = $state(false);
  const ethRecoveryLifetime = new EthRecoveryLifetime();

  let simplePreflightResult = $state<PreflightResult | null>(null);
  let bridgePreflightResult = $state<BridgeTransferPreflightResult | null>(null);
  let sendResult = $state<SendResult | null>(null);
  let submittedTransferSnapshot = $state<SubmittedTransferSnapshot | null>(null);
  let addresses = $state<{ vrsc_address: string; eth_address: string; btc_address: string } | null>(
    null
  );

  let showSourceAssetSheet = $state(false);
  let showReceiveAssetSheet = $state(false);
  let showViaSheet = $state(false);
  let showNetworkSheet = $state(false);
  let showExportSheet = $state(false);
  let exportSheetMode = $state<'convert' | 'send'>('convert');
  let showAddressBookSheet = $state(false);
  let showFeeSheet = $state(false);
  let directSendFeeMode = $state<DirectSendFeeMode>(DEFAULT_DIRECT_SEND_FEE_MODE);
  let pendingDirectSendFeeMode = $state<DirectSendFeeMode>(DEFAULT_DIRECT_SEND_FEE_MODE);
  const preflightRequestGuard = new PreflightRequestGuard();
  let addressBookSearchTerm = $state('');
  let unsavedRecipientConfirmed = $state(false);
  const copiedSuccessFieldState = new TimedValueState<'recipient' | 'txid'>();
  const copiedSuccessField = $derived(copiedSuccessFieldState.current);

  const selectedCoin = $derived(selectedCoinOption?.coin ?? null);

  const selectedCoinPresentation = $derived(
    selectedCoin ? resolveCoinPresentation(selectedCoin) : null
  );

  const selectedChannelId = $derived(selectedCoinOption?.channelId ?? null);
  const selectedChannelInfo = $derived(
    selectedChannelId ? (chainInfoByChannel[selectedChannelId] ?? null) : null
  );
  const selectedShieldedSyncPercent = $derived(
    selectedChannelId?.startsWith('dlight_private.')
      ? toOptionalFiniteNumber(selectedChannelInfo?.percent)
      : null
  );
  const selectedShieldedStatusKind = $derived(
    selectedChannelId?.startsWith('dlight_private.')
      ? (selectedChannelInfo?.statusKind?.toLowerCase() ?? null)
      : null
  );
  const isShieldedSyncBlocked = $derived(
    selectedShieldedStatusKind === 'initializing' ||
      selectedShieldedStatusKind === 'syncing' ||
      selectedShieldedStatusKind === 'error' ||
      (selectedShieldedStatusKind === null &&
        selectedShieldedSyncPercent !== null &&
        selectedShieldedSyncPercent !== 100 &&
        selectedShieldedSyncPercent !== -1)
  );
  const shieldedSyncBlockedHelper = $derived(
    isShieldedSyncBlocked && selectedShieldedStatusKind === 'error'
      ? i18n.t('wallet.transfer.error.network')
      : isShieldedSyncBlocked && selectedShieldedSyncPercent !== null
        ? i18n.t('wallet.transfer.privateSyncBlocked', {
            percent: formatSyncPercent(selectedShieldedSyncPercent),
          })
        : ''
  );

  const selectedChannelPrefix = $derived(selectedChannelId?.split('.')[0] ?? '');
  const selectedSourceSystemId = $derived(
    selectedCoinOption?.scope.systemId ?? selectedCoin?.systemId ?? selectedCoin?.id ?? ''
  );
  const sourceNetworkDisplayName = $derived(
    selectedCoinOption?.scope.systemDisplayName?.trim() ||
      resolveSourceNetworkDisplayName(selectedSourceSystemId, selectedChannelPrefix)
  );
  const sourceSupportsConversion = $derived(
    (() => {
      if (
        !selectedCoin ||
        !selectedChannelId ||
        !transferSourceSupportsConversion(selectedCoinOption)
      ) {
        return false;
      }
      if (bridgeCapabilities) return bridgeCapabilities.conversionSupported;
      // Preserve existing behavior while capabilities are loading.
      return selectedChannelPrefix === 'vrpc';
    })()
  );

  const selectedBalance = $derived(selectedCoinOption?.balanceTotal ?? '0');
  const selectedBalanceValue = $derived(toFiniteNumber(selectedBalance));

  const selectedDlightScopeAddress = $derived(
    selectedCoinOption?.sourceKind === 'private' ? selectedCoinOption.scope.address : ''
  );

  const selectedSourceAddress = $derived(selectedCoinOption?.scope.address ?? '');

  const showChooseCurrencyCallToAction = $derived(
    (entryIntent === 'send' || entryIntent === 'convert') && !sourceCoinManuallyChosen
  );

  const receiveAssetSelectionEnabled = $derived(sourceCoinManuallyChosen && !!selectedCoin);

  const rawReceiveAssetSections = $derived<ReceiveAssetSections>(
    buildReceiveAssetSections({
      paths: discoveredPathQuotes,
      sourceCurrencyId: selectedCoin?.currencyId || selectedCoin?.id || '',
      sourceCurrencyAliases: [
        selectedCoin?.currencyId,
        selectedCoin?.id,
        selectedCoinPresentation?.currencyId,
      ].filter((value): value is string => typeof value === 'string' && value.trim().length > 0),
    })
  );

  const receiveAssetSections = $derived<ReceiveAssetSections>(
    filterReceiveAssetSectionsByQuery(rawReceiveAssetSections, receiveSearchTerm)
  );

  const receiveAssetOptions = $derived(receiveAssetSections.allOptions);
  const popularReceiveAssetOptions = $derived(receiveAssetSections.popularOptions);
  const otherReceiveAssetOptions = $derived(receiveAssetSections.otherOptions);

  const selectableReceiveAssetOptions = $derived<ReceiveAssetOption[]>(
    rawReceiveAssetSections.allOptions.flatMap((option) =>
      option.isGrouped && option.networkOptions?.length ? option.networkOptions : [option]
    )
  );

  const selectedReceiveAssetOption = $derived(
    selectableReceiveAssetOptions.find((option) => option.id === selectedReceiveAssetId) ?? null
  );

  const selectedReceiveAssetViaOptions = $derived(
    selectedReceiveAssetOption
      ? filterViaOptionsByExport(selectedReceiveAssetOption.viaOptions, selectedExportSystemId)
      : []
  );

  const rankedViaOptions = $derived(
    sortViaOptionsByScore(selectedReceiveAssetViaOptions, amount, routeEstimateOutputs)
  );

  const bestViaOption = $derived(rankedViaOptions[0] ?? null);

  const selectedViaOption = $derived(
    selectedReceiveAssetViaOptions.find((option) => option.id === selectedViaOptionId) ?? null
  );

  const activeConvertRoute = $derived(
    isPositiveAmount(amount) ? (selectedViaOption ?? bestViaOption) : null
  );

  const sourceRouteAliasKeys = $derived(
    new Set(
      [selectedCoin?.currencyId, selectedCoin?.id, selectedCoinPresentation?.currencyId]
        .filter((value): value is string => typeof value === 'string' && value.trim().length > 0)
        .map((value) => value.trim().toLowerCase())
    )
  );
  const sourceCanonicalKey = $derived(
    (() => {
      const sourceTicker =
        selectedCoinPresentation?.displayTicker?.trim() ||
        selectedCoinPresentation?.displayName?.trim() ||
        selectedCoin?.displayTicker?.trim() ||
        selectedCoin?.displayName?.trim() ||
        selectedCoin?.id?.trim() ||
        '';
      const canonical = canonicalizeBridgeTicker(sourceTicker);
      return canonical;
    })()
  );
  const matchedSendSameAssetOption = $derived<ReceiveAssetOption | null>(
    (() => {
      if (!selectedCoin) return null;
      for (const option of selectableReceiveAssetOptions) {
        const destinationKey = option.destinationId.trim().toLowerCase();
        if (sourceRouteAliasKeys.has(destinationKey)) {
          return option;
        }
        const matchesViaOption = option.viaOptions.some((viaOption) => {
          const convertToKey = viaOption.convertTo?.trim().toLowerCase() ?? '';
          return !!convertToKey && sourceRouteAliasKeys.has(convertToKey);
        });
        if (matchesViaOption) {
          return option;
        }
        if (sourceCanonicalKey && option.canonicalKey.toUpperCase() === sourceCanonicalKey) {
          return option;
        }
        if (sourceCanonicalKey) {
          const optionCanonicalCandidates = [option.ticker, option.label, option.fullyqualifiedname]
            .map((value) => canonicalizeBridgeTicker(value ?? ''))
            .filter((value) => value.length > 0);
          if (optionCanonicalCandidates.includes(sourceCanonicalKey)) {
            return option;
          }
        }
      }
      return null;
    })()
  );
  const sendRawExportOptions = $derived<ExportRouteOption[]>(
    (() => {
      const dedupe = new Map<string, ExportRouteOption>();

      for (const pathList of Object.values(discoveredPathQuotes)) {
        if (!Array.isArray(pathList) || pathList.length === 0) continue;

        for (const quote of pathList) {
          if (quote.prelaunch) continue;
          const exportTo = quote.exportTo?.trim() ?? '';
          if (!exportTo) continue;

          const destinationKey = quote.destinationId.trim().toLowerCase();
          const convertToKey = quote.convertTo?.trim().toLowerCase() ?? '';
          const isSameCurrencyRoute =
            sourceRouteAliasKeys.has(destinationKey) ||
            (!!convertToKey && sourceRouteAliasKeys.has(convertToKey));

          if (isSameCurrencyRoute) {
            upsertSendExportOption(dedupe, {
              exportTo,
              exportToName: quote.exportToDisplayName?.trim() || exportTo,
              gateway: quote.gateway,
              via: null,
              price: quote.price ?? null,
              mappingDestination: undefined,
            });
            continue;
          }

          if (quote.mapping) {
            upsertSendExportOption(dedupe, {
              exportTo,
              exportToName: quote.exportToDisplayName?.trim() || exportTo,
              gateway: quote.gateway,
              via: null,
              price: quote.price ?? null,
              mappingDestination: resolveSendMappingDestination(quote),
            });
          }
        }
      }

      return Array.from(dedupe.values());
    })()
  );
  const sendRawMapToByExportKey = $derived(
    (() => {
      const map = new Map<string, string>();

      for (const pathList of Object.values(discoveredPathQuotes)) {
        if (!Array.isArray(pathList) || pathList.length === 0) continue;

        for (const quote of pathList) {
          if (quote.prelaunch || !quote.mapping) continue;
          const exportTo = quote.exportTo?.trim() ?? '';
          const mapTo = quote.mapTo?.trim() ?? '';
          if (!exportTo || !mapTo) continue;

          const exportKey = normalizeSendExportKey(exportTo);
          if (!exportKey || map.has(exportKey)) continue;
          map.set(exportKey, mapTo);
        }
      }

      return map;
    })()
  );
  const sendSameAssetOption = $derived<ReceiveAssetOption | null>(
    (() => {
      if (!selectedCoin) return null;

      const mergedExportOptions = mergeSendExportOptions(
        sendRawExportOptions,
        matchedSendSameAssetOption?.exportOptions ?? []
      );

      if (matchedSendSameAssetOption) {
        return {
          ...matchedSendSameAssetOption,
          isCrossChain: mergedExportOptions.length > 0,
          exportOptions: mergedExportOptions,
        };
      }

      const sourceLabel =
        selectedCoinPresentation?.displayName?.trim() ||
        selectedCoin.displayName?.trim() ||
        selectedCoin.id;
      const sourceTicker =
        selectedCoinPresentation?.displayTicker?.trim() ||
        selectedCoin.displayTicker?.trim() ||
        sourceLabel;

      return {
        id: `send-${selectedCoin.id}`,
        key: `send-${selectedCoin.id}`.toLowerCase(),
        label: sourceLabel,
        ticker: sourceTicker,
        subtitle: undefined,
        fullyqualifiedname: sourceTicker,
        destinationId: selectedCoin.currencyId || selectedCoin.id,
        canonicalKey: sourceCanonicalKey || canonicalizeBridgeTicker(sourceTicker),
        hasOnChainPath: true,
        isCrossChain: mergedExportOptions.length > 0,
        viaOptions: [],
        exportOptions: mergedExportOptions,
        gateway: mergedExportOptions.some((option) => option.gateway),
        isEthDestination: false,
      };
    })()
  );
  const sendCrossChainAvailable = $derived(
    !!sendSameAssetOption && sendSameAssetOption.exportOptions.length > 0
  );
  const activeSendExportSystemId = $derived(
    (() => {
      if (!sendSameAssetOption) return null;
      if (
        selectedSendExportSystemId &&
        sendSameAssetOption.exportOptions.some(
          (option) => option.exportTo === selectedSendExportSystemId
        )
      ) {
        return selectedSendExportSystemId;
      }
      return null;
    })()
  );
  const selectedSendExportRouteOption = $derived(
    sendSameAssetOption && activeSendExportSystemId
      ? (sendSameAssetOption.exportOptions.find(
          (option) => option.exportTo === activeSendExportSystemId
        ) ?? null)
      : null
  );
  const activeSendMapTo = $derived(
    (() => {
      if (!activeSendExportSystemId) return null;
      const exportKey = normalizeSendExportKey(activeSendExportSystemId);
      return sendRawMapToByExportKey.get(exportKey) ?? null;
    })()
  );
  const sendSameAssetViaOptions = $derived(
    sendSameAssetOption
      ? filterViaOptionsByExport(sendSameAssetOption.viaOptions, activeSendExportSystemId)
      : []
  );
  const activeSendRoute = $derived<ViaRouteOption | null>(
    (() => {
      if (sendSameAssetViaOptions.length === 0) return null;
      if (!isPositiveAmount(amount)) return sendSameAssetViaOptions[0];
      return (
        sortViaOptionsByScore(sendSameAssetViaOptions, amount, routeEstimateOutputs)[0] ?? null
      );
    })()
  );
  const sendDestinationNetworkValue = $derived(
    activeSendExportSystemId
      ? normalizeNetworkDisplayName(
          activeSendExportSystemId,
          selectedSendExportRouteOption?.exportToName ?? activeSendExportSystemId
        )
      : i18n.t('wallet.transfer.keepOnNetwork', { value: sourceNetworkDisplayName })
  );
  const sameAssetOption = $derived<SameAssetOption | null>(
    selectedCoin && selectedCoinPresentation
      ? {
          id: `same-${selectedCoin.id}${activeSendExportSystemId ? `|${activeSendExportSystemId}` : ''}`,
          label: i18n.t('wallet.transfer.sameAssetOption', {
            ticker: selectedCoinPresentation.displayTicker,
          }),
          destinationId: sendSameAssetOption?.destinationId ?? selectedCoin.id,
          receiveLabel:
            sendSameAssetOption && activeSendExportSystemId
              ? resolveReceiveLabel(sendSameAssetOption, activeSendExportSystemId)
              : selectedCoinPresentation.displayTicker,
          ethDestination: activeSendRoute?.ethDestination ?? false,
          convertTo: null,
          exportTo: activeSendExportSystemId,
          via: null,
          mapTo: activeSendMapTo ?? activeSendRoute?.mapTo ?? null,
          price: selectedSendExportRouteOption?.price ?? activeSendRoute?.price ?? null,
        }
      : null
  );
  const activeTargetOption = $derived(conversionEnabled ? activeConvertRoute : sameAssetOption);
  const activeExportSystemId = $derived(
    conversionEnabled ? (activeConvertRoute?.exportTo ?? null) : (sameAssetOption?.exportTo ?? null)
  );
  const directSendRoute = $derived<DirectSendRouteState>({
    operation:
      (conversionEnabled && !!activeConvertRoute) ||
      (!conversionEnabled && !!activeTargetOption?.exportTo)
        ? 'bridge_transfer'
        : 'preflight_send',
    channelId: selectedChannelId,
    conversionEnabled,
    exportSystemId: activeExportSystemId,
  });
  const directSendFeeEligible = $derived(isDirectSendFeeEligible(directSendRoute));
  const directSendFeeOptions = $derived(getDirectSendFeeOptions(i18n.t));
  const authoritativeDirectSendFeeMode = $derived(simplePreflightResult?.feeMode ?? null);
  const pendingFeeModeMatchesApplied = $derived(
    !directSendFeeSelectionNeedsRefresh(authoritativeDirectSendFeeMode, pendingDirectSendFeeMode)
  );
  const bridgeFeeParityEligible = $derived(
    !!selectedCoin &&
      !!selectedChannelId &&
      selectedChannelPrefix === 'vrpc' &&
      selectedCoin.proto === 'vrsc'
  );
  const selectedExportRouteOption = $derived(
    selectedReceiveAssetOption && selectedExportSystemId
      ? (selectedReceiveAssetOption.exportOptions.find(
          (option) => option.exportTo === selectedExportSystemId
        ) ?? null)
      : null
  );
  const selectedDestinationNetworkValue = $derived(
    conversionEnabled
      ? selectedExportSystemId
        ? normalizeNetworkDisplayName(
            selectedExportSystemId,
            selectedExportRouteOption?.exportToName ?? selectedExportSystemId
          )
        : ''
      : activeExportSystemId
        ? sendDestinationNetworkValue
        : ''
  );
  const selectedReceiveLabel = $derived(
    resolveReceiveLabel(selectedReceiveAssetOption, selectedExportSystemId)
  );
  const bridgeFeeInsufficient = $derived(
    (() => {
      if (!bridgeFeeParityEligible) return false;
      if (!bridgeFeeInfo.feeCoins || !bridgeFeeInfo.balanceCoins) return false;
      if (bridgeFeeInfo.loading || bridgeFeeInfo.error) return false;
      const required = toFiniteNumber(bridgeFeeInfo.feeCoins);
      const available = toFiniteNumber(bridgeFeeInfo.balanceCoins);
      if (required <= 0) return false;
      return available < required;
    })()
  );
  const bridgeFeeFiatDisplay = $derived(
    formatFiatEstimate(
      bridgeFeeInfo.feeCoins ?? null,
      getDisplayCurrencyRateForCurrencyLabel(bridgeFeeInfo.currencyTicker)
    )
  );
  const bridgeFeeEstimateValue = $derived(
    (() => {
      if (bridgeFeeInfo.loading) return i18n.t('wallet.transfer.bridgeFeeCalculating');
      if (bridgeFeeInfo.error || !bridgeFeeInfo.feeCoins) {
        return i18n.t('wallet.transfer.bridgeFeeUnavailable');
      }
      return `${bridgeFeeInfo.feeCoins} ${bridgeFeeInfo.currencyTicker}`;
    })()
  );
  const bridgeFeeEstimateSecondary = $derived(
    (() => {
      if (bridgeFeeInsufficient) {
        return i18n.t('wallet.transfer.bridgeFeeInsufficient', {
          ticker: bridgeFeeInfo.currencyTicker,
        });
      }
      if (
        !bridgeFeeInfo.loading &&
        !bridgeFeeInfo.error &&
        bridgeFeeInfo.feeCoins &&
        bridgeFeeFiatDisplay !== '≈ —'
      ) {
        return bridgeFeeFiatDisplay;
      }
      return undefined;
    })()
  );

  const convertUnavailableForSource = $derived(
    !!selectedCoin &&
      !!selectedChannelId &&
      (!transferSourceSupportsConversion(selectedCoinOption) ||
        (!!bridgeCapabilities && !bridgeCapabilities.conversionSupported))
  );

  const showConvertUnavailable = $derived(
    convertUnavailableForSource && (entryIntent === 'convert' || conversionEnabled)
  );

  const convertUnavailableMessage = $derived(
    (() => {
      const reasonCode = bridgeCapabilities?.reasonCode ?? '';
      if (reasonCode === 'feature_disabled') {
        return i18n.t('wallet.transfer.convertUnavailableFeatureDisabled');
      }
      if (reasonCode === 'eth_not_configured') {
        return i18n.t('wallet.transfer.convertUnavailableEthNotConfigured');
      }
      if (reasonCode === 'unsupported_channel') {
        return i18n.t('wallet.transfer.convertUnavailableUnsupportedChannel');
      }
      return i18n.t('wallet.transfer.convertUnavailableEvm');
    })()
  );

  const destinationAddressKind = $derived<DestinationAddressKind>(
    isEthereumExport(activeExportSystemId)
      ? 'eth'
      : !activeExportSystemId &&
          (selectedChannelPrefix === 'eth' || selectedChannelPrefix === 'erc20')
        ? 'eth'
        : selectedChannelPrefix === 'btc'
          ? 'btc'
          : selectedChannelPrefix === 'dlight_private'
            ? 'dlight'
            : 'vrpc'
  );

  const selfDestinationAddress = $derived(
    destinationAddressKind === 'dlight'
      ? selectedDlightScopeAddress
      : !addresses
        ? ''
        : destinationAddressKind === 'eth'
          ? addresses.eth_address
          : destinationAddressKind === 'btc'
            ? addresses.btc_address
            : addresses.vrsc_address
  );

  const addressBookEndpointOptions = $derived<AddressBookEndpointOption[]>(
    (() => {
      const query = addressBookSearchTerm.trim().toLowerCase();
      const options = addressBookContacts.flatMap((contact: AddressBookContact) =>
        contact.endpoints
          .filter((endpoint) =>
            isEndpointCompatibleWithDestinationKind(endpoint, destinationAddressKind)
          )
          .map((endpoint) => ({
            contactId: contact.id,
            contactName: contact.displayName,
            endpointId: endpoint.id,
            endpointKind: endpoint.kind,
            endpointLabel: endpoint.label,
            endpointAddress: endpoint.address,
            normalizedAddress: endpoint.normalizedAddress,
            lastUsedAt: endpoint.lastUsedAt,
          }))
      );

      const filtered = query
        ? options.filter(
            (option) =>
              option.contactName.toLowerCase().includes(query) ||
              option.endpointLabel.toLowerCase().includes(query) ||
              option.endpointAddress.toLowerCase().includes(query)
          )
        : options;

      return filtered.sort((a, b) => {
        if ((a.lastUsedAt ?? 0) !== (b.lastUsedAt ?? 0)) {
          return (b.lastUsedAt ?? 0) - (a.lastUsedAt ?? 0);
        }
        return a.contactName.localeCompare(b.contactName);
      });
    })()
  );

  const matchedSavedRecipient = $derived(
    findMatchingSavedEndpoint(addressBookContacts, destinationAddressKind, destinationAddress)
  );
  const isSelfRecipient = $derived(
    (() => {
      const normalizedDestination = normalizeAddressByDestinationKind(
        destinationAddressKind,
        destinationAddress
      );
      const normalizedSelf = normalizeAddressByDestinationKind(
        destinationAddressKind,
        selfDestinationAddress
      );
      return (
        !!normalizedDestination && !!normalizedSelf && normalizedDestination === normalizedSelf
      );
    })()
  );
  const isSavedRecipient = $derived(!!matchedSavedRecipient);
  const hasRecipientSimilarityWarning = $derived(
    !isSavedRecipient &&
      sharesSuspiciousPrefixSuffix(addressBookContacts, destinationAddressKind, destinationAddress)
  );
  const activePreflight = $derived(simplePreflightResult ?? bridgePreflightResult);
  const submittedTransferRoute = $derived(
    submittedTransferSnapshot ? classifySubmittedTransferRoute(submittedTransferSnapshot) : null
  );
  const submittedReceiptCopy = $derived(
    submittedTransferRoute && submittedTransferSnapshot
      ? getSubmittedReceiptCopy(
          submittedTransferRoute,
          submittedTransferSnapshot.conversionEnabled,
          i18n.t
        )
      : null
  );
  const submittedExplorerUrl = $derived(
    submittedTransferSnapshot && sendResult
      ? transactionExplorerUrl(submittedTransferSnapshot, sendResult.txid)
      : null
  );
  const requiresUnsavedRecipientAck = $derived(
    !!destinationAddress.trim() && !!activePreflight && !isSavedRecipient && !isSelfRecipient
  );

  const recipientInputCopy = $derived(getRecipientInputCopy(i18n.t, destinationAddressKind));
  const recipientValid = $derived(
    validateDestinationAddressForKind(destinationAddress, destinationAddressKind)
  );
  const dlightDestinationKind = $derived(
    destinationAddressKind === 'dlight'
      ? classifyDlightDestinationAddress(destinationAddress)
      : null
  );
  const showDlightMemoField = $derived(
    destinationAddressKind === 'dlight' && dlightDestinationKind === 'shielded'
  );
  const amountValid = $derived(isPositiveAmount(amount));

  const estimatedConversionValue = $derived(
    (() => {
      if (!conversionEnabled || !amountValid || !activeConvertRoute) return null;

      const estimatedOutput = parseEstimatedOutput(routeEstimateOutputs[activeConvertRoute.id]);
      if (estimatedOutput !== null) {
        return estimatedOutput.toFixed(8);
      }

      const numericPrice = parsePrice(activeConvertRoute.price);
      if (numericPrice === null) return null;
      return (Number(amount) * numericPrice).toFixed(8);
    })()
  );

  const selectedReceiveAssetPresentation = $derived(
    selectedReceiveAssetOption
      ? resolveCoinPresentationById(selectedReceiveAssetOption.destinationId)
      : null
  );
  const selectedReceiveCompactTicker = $derived(
    (selectedReceiveAssetPresentation?.mappedTo
      ? resolveCoinPresentationById(selectedReceiveAssetPresentation.mappedTo)?.displayTicker
      : null) ||
      selectedReceiveAssetPresentation?.displayTicker ||
      selectedReceiveAssetOption?.ticker ||
      selectedReceiveLabel
  );

  const sourceFiatRate = $derived(
    getDisplayCurrencyRateForCoinIds([
      selectedCoin?.id,
      selectedCoin?.currencyId,
      selectedCoin?.mappedTo,
    ])
  );

  const receiveFiatRate = $derived(
    getDisplayCurrencyRateForCoinIds([
      selectedReceiveAssetOption?.destinationId,
      selectedReceiveAssetPresentation?.id,
      selectedReceiveAssetPresentation?.currencyId,
      selectedReceiveAssetPresentation?.mappedTo,
    ])
  );

  const sourceAmountFiatDisplay = $derived(formatFiatEstimate(amount, sourceFiatRate));
  const receiveAmountFiatDisplay = $derived(
    formatFiatEstimate(estimatedConversionValue ?? '0', receiveFiatRate)
  );

  const effectiveSendAmount = $derived((activePreflight?.value ?? amount.trim()).trim());
  const submittedSendAmount = $derived((activePreflight?.amountSubmitted ?? amount.trim()).trim());
  const preflightIndicatesAmountAdjustment = $derived(
    isSimplePreflight(activePreflight)
      ? !!activePreflight.feeTakenFromAmount
      : isBridgePreflight(activePreflight)
        ? !!activePreflight.amountAdjusted
        : false
  );
  const amountAdjustedForReview = $derived(
    (() => {
      if (!activePreflight) return false;
      if (preflightIndicatesAmountAdjustment) return true;
      const normalizedSubmitted = normalizeAmountString(submittedSendAmount);
      const normalizedActual = normalizeAmountString(effectiveSendAmount);
      return (
        !!normalizedSubmitted && !!normalizedActual && normalizedSubmitted !== normalizedActual
      );
    })()
  );

  const reviewEstimatedConversionValue = $derived(
    (() => {
      if (!conversionEnabled) return null;
      if (!amountAdjustedForReview) return estimatedConversionValue;
      if (!activeConvertRoute) return estimatedConversionValue;
      const numericPrice = parsePrice(activeConvertRoute.price);
      const numericAmount = parseNonNegativeAmount(effectiveSendAmount);
      if (numericPrice === null || numericAmount === null || numericAmount <= 0) {
        return estimatedConversionValue;
      }
      return (numericAmount * numericPrice).toFixed(8);
    })()
  );
  const reviewSourceAmountFiatDisplay = $derived(
    formatFiatEstimate(effectiveSendAmount, sourceFiatRate)
  );
  const reviewReceiveAmountFiatDisplay = $derived(
    formatFiatEstimate(reviewEstimatedConversionValue ?? '0', receiveFiatRate)
  );

  const activeConvertRouteRate = $derived(formatRouteRateValue(activeConvertRoute?.price));

  const activeConvertRouteRateValueText = $derived(
    (() => {
      if (!selectedCoinPresentation || !selectedReceiveAssetOption || !activeConvertRouteRate) {
        return i18n.t('wallet.transfer.summary.notSet');
      }

      return i18n.t('wallet.transfer.ratePair', {
        from: selectedCoinPresentation.displayTicker,
        rate: activeConvertRouteRate,
        to: selectedReceiveCompactTicker || selectedReceiveAssetOption.label,
      });
    })()
  );

  const conversionFeeInfo = $derived<ConversionFeeInfo | null>(
    (() => {
      if (!conversionEnabled || !amountValid || !selectedCoinPresentation) return null;
      const numericAmount = Number(amount.trim());
      if (!Number.isFinite(numericAmount) || numericAmount <= 0) return null;

      const hasVia = !!activeConvertRoute?.via;
      const multiplier = hasVia ? 0.0005 : 0.00025;
      const feeAmount = numericAmount * multiplier;
      if (!Number.isFinite(feeAmount) || feeAmount <= 0) return null;

      return {
        amount: formatDecimalTrimmed(feeAmount),
        currency: selectedCoinPresentation.displayTicker,
        percentage: hasVia ? '0.05%' : '0.025%',
      };
    })()
  );

  const reviewSendingValue = $derived(
    (() => {
      if (!selectedCoinPresentation) return '';
      const baseAmount = effectiveSendAmount;
      if (!baseAmount) return '';
      const formattedAmount = formatAmountForReviewDisplay(
        baseAmount,
        MAX_TRANSFER_AMOUNT_FRACTION_DIGITS
      );
      return `${formattedAmount} ${selectedCoinPresentation.displayTicker}`;
    })()
  );

  const reviewReceivingValue = $derived(
    (() => {
      if (!conversionEnabled || !reviewEstimatedConversionValue) return '';
      const receiveLabel =
        selectedReceiveCompactTicker ||
        resolveReceiveLabel(selectedReceiveAssetOption, activeExportSystemId);
      return receiveLabel
        ? `~${reviewEstimatedConversionValue} ${receiveLabel}`
        : `~${reviewEstimatedConversionValue}`;
    })()
  );

  const reviewRouteValue = $derived(
    conversionEnabled && activeConvertRoute ? getViaOptionLabel(activeConvertRoute) : ''
  );

  const reviewDestinationNetworkValue = $derived(
    conversionEnabled
      ? selectedDestinationNetworkValue ||
          normalizeNetworkDisplayName(
            activeConvertRoute?.exportTo ?? null,
            activeConvertRoute?.exportToLabel ?? activeConvertRoute?.exportTo ?? ''
          )
      : selectedDestinationNetworkValue
  );
  const reviewNetworkFeeCurrencyLabel = $derived(
    activePreflight ? resolveCurrencyLabelForDisplay(activePreflight.feeCurrency) : ''
  );

  const reviewNetworkFeeValue = $derived(
    activePreflight
      ? `${formatAmountForReviewDisplay(activePreflight.fee, MAX_TRANSFER_AMOUNT_FRACTION_DIGITS)} ${reviewNetworkFeeCurrencyLabel || activePreflight.feeCurrency}`
      : ''
  );
  const reviewTotalDebitedValue = $derived(
    (() => {
      if (!activePreflight || !selectedCoinPresentation) return '';
      const feeTicker = reviewNetworkFeeCurrencyLabel || activePreflight.feeCurrency;
      const sentTicker = selectedCoinPresentation.displayTicker;
      const sentAmount = parseNonNegativeAmount(activePreflight.value);
      const feeAmount = parseNonNegativeAmount(activePreflight.fee);
      if (
        sentAmount !== null &&
        feeAmount !== null &&
        feeTicker.trim().toLowerCase() === sentTicker.trim().toLowerCase()
      ) {
        return `${formatDecimalTrimmed(sentAmount + feeAmount)} ${sentTicker}`;
      }
      return `${reviewSendingValue} + ${reviewNetworkFeeValue}`;
    })()
  );
  const reviewTotalFeesFiat = $derived(
    (() => {
      if (!activePreflight) return '≈ —';

      let totalFiat = 0;
      const bridgeFeeRequired =
        !!activeExportSystemId && isEthereumExport(activeExportSystemId) && bridgeFeeParityEligible;

      const networkFeeAmount = parseNonNegativeAmount(activePreflight.fee);
      const networkFeeRate =
        getDisplayCurrencyRateForCurrencyLabel(activePreflight.feeCurrency) ?? sourceFiatRate;
      if (networkFeeAmount !== null && networkFeeRate !== null) {
        totalFiat += networkFeeAmount * networkFeeRate;
      }

      if (conversionEnabled && conversionFeeInfo) {
        const conversionAmount = parseNonNegativeAmount(conversionFeeInfo.amount);
        const conversionRate =
          getDisplayCurrencyRateForCurrencyLabel(conversionFeeInfo.currency) ?? sourceFiatRate;
        if (conversionAmount !== null && conversionRate !== null) {
          totalFiat += conversionAmount * conversionRate;
        }
      }

      if (bridgeFeeRequired) {
        if (bridgeFeeInfo.loading || bridgeFeeInfo.error || !bridgeFeeInfo.feeCoins) return '≈ —';

        const bridgeAmount = parseNonNegativeAmount(bridgeFeeInfo.feeCoins);
        const bridgeRate =
          getDisplayCurrencyRateForCurrencyLabel(bridgeFeeInfo.currencyTicker) ?? sourceFiatRate;
        if (bridgeAmount === null || bridgeRate === null) return '≈ —';
        totalFiat += bridgeAmount * bridgeRate;
      }

      if (!Number.isFinite(totalFiat) || totalFiat <= 0) return '≈ —';
      return `≈ ${formatDisplayFiatAmountDynamic(totalFiat)}`;
    })()
  );
  const showReviewTotalFeesFiat = $derived(
    (() => {
      if (reviewTotalFeesFiat === '≈ —') return false;
      if (!activeExportSystemId || !isEthereumExport(activeExportSystemId)) return true;
      if (!bridgeFeeParityEligible || !bridgeFeeEstimateSecondary) return true;
      if (conversionEnabled && conversionFeeInfo) return true;
      return bridgeFeeEstimateSecondary !== reviewTotalFeesFiat;
    })()
  );
  const hideBridgeFeeEstimateFiatBreakdown = $derived(
    (() => {
      if (bridgeFeeInsufficient) return false;
      if (!conversionEnabled || !conversionFeeInfo) return false;
      return showReviewTotalFeesFiat;
    })()
  );
  const reviewRecipientName = $derived(matchedSavedRecipient?.contact.displayName?.trim() ?? '');
  const reviewRecipientAddress = $derived(truncateAddressMiddle(destinationAddress));
  const reviewRecipientAddressWithSelf = $derived(
    isSelfRecipient && reviewRecipientAddress
      ? `${reviewRecipientAddress} ${i18n.t('wallet.transfer.review.selfSuffix')}`
      : reviewRecipientAddress
  );

  const stepNumber = $derived(
    currentStep === 'success'
      ? OPERATIONAL_STEPS.length
      : OPERATIONAL_STEPS.indexOf(currentStep as WizardOperationalStepId) + 1
  );

  const stepperSteps = $derived<TransferStepperStep[]>(
    OPERATIONAL_STEPS.map((stepId, index) => {
      const currentIndex =
        currentStep === 'success'
          ? OPERATIONAL_STEPS.length - 1
          : OPERATIONAL_STEPS.indexOf(currentStep as WizardOperationalStepId);

      return {
        id: stepId,
        label: stepLabels[stepId],
        status:
          currentStep === 'success' || index < currentIndex
            ? 'complete'
            : index === currentIndex
              ? 'current'
              : 'upcoming',
      };
    })
  );

  const isBusy = $derived(
    loadingTargets || preflighting || sending || recoveringEthSubmission || !!pendingEthSubmission
  );
  const transferMutationLocked = $derived(sending || recoveringEthSubmission);
  const isDirty = $derived(
    currentStep !== 'details' ||
      !!amount.trim() ||
      !!destinationAddress.trim() ||
      !!memo.trim() ||
      conversionEnabled !== (entryIntent === 'convert') ||
      !!selectedSendExportSystemId ||
      !!simplePreflightResult ||
      !!bridgePreflightResult ||
      !!sendResult
  );

  const primaryDisabled = $derived(
    isBusy ||
      isShieldedSyncBlocked ||
      (currentStep === 'details' &&
        (!selectedCoin ||
          !selectedChannelId ||
          !amountValid ||
          !recipientValid ||
          !activeTargetOption ||
          (conversionEnabled && !selectedReceiveAssetOption) ||
          (activeExportSystemId !== null &&
            isEthereumExport(activeExportSystemId) &&
            bridgeFeeInsufficient))) ||
      (currentStep === 'review' && !activePreflight) ||
      (currentStep === 'review' && requiresUnsavedRecipientAck && !unsavedRecipientConfirmed)
  );

  const primaryLabel = $derived(
    currentStep === 'details'
      ? preflighting
        ? i18n.t('wallet.transfer.preflighting')
        : conversionEnabled
          ? i18n.t('wallet.transfer.reviewConversion')
          : i18n.t('wallet.transfer.reviewSend')
      : currentStep === 'review'
        ? preflighting
          ? i18n.t('wallet.transfer.fee.updating')
          : sending
            ? sendStageLabel(sendStage)
            : i18n.t('wallet.transfer.sendNow')
        : i18n.t('common.continue')
  );
  const sendStageElapsedMs = $derived(
    sendStageStartedAt === null ? 0 : Date.now() - sendStageStartedAt + sendStageTick * 0
  );
  const sendStageExceededThreshold = $derived(
    sending && sendStageStartedAt !== null && sendStageElapsedMs > 25_000
  );
  const sendStageGuidance = $derived(
    sendStageExceededThreshold ? sendStageGuidanceLabel(sendStage) : ''
  );

  const amountAdjustedWarning = $derived(
    (() => {
      if (!activePreflight || !amountAdjustedForReview) return null;
      const submittedDisplay = formatAmountForReviewDisplay(
        submittedSendAmount,
        MAX_TRANSFER_AMOUNT_FRACTION_DIGITS
      );
      const adjustedDisplay = formatAmountForReviewDisplay(
        effectiveSendAmount,
        MAX_TRANSFER_AMOUNT_FRACTION_DIGITS
      );
      if (!submittedDisplay || !adjustedDisplay) return null;
      const ticker = selectedCoinPresentation?.displayTicker?.trim() || selectedCoin?.id || '';
      return i18n.t('wallet.transfer.review.amountAdjustedWarning', {
        submitted: submittedDisplay,
        adjusted: adjustedDisplay,
        ticker,
      });
    })()
  );
  const mergedPreflightWarnings = $derived(
    (() => {
      const warnings =
        activePreflight?.warnings
          .map((warning) => mapPreflightWarningMessage(warning))
          .filter((warning) => warning.trim().length > 0) ?? [];
      if (amountAdjustedWarning && !warnings.includes(amountAdjustedWarning)) {
        warnings.unshift(amountAdjustedWarning);
      }
      return warnings;
    })()
  );
  const warningsSummary = $derived(mergedPreflightWarnings);
  const reviewWarnings = $derived(currentStep === 'review' ? mergedPreflightWarnings : []);

  const summaryRows = $derived<SummaryRow[]>(
    (() => {
      const rows: SummaryRow[] = [];

      const sourcePrimary = selectedCoinPresentation?.displayName?.trim() ?? '';
      const sourceSecondary = normalizeSummarySecondary(
        sourcePrimary,
        selectedCoinPresentation?.displayTicker ?? ''
      );
      if (sourcePrimary) {
        rows.push({
          label: i18n.t('wallet.transfer.summary.from'),
          primary: sourcePrimary,
          secondary: sourceSecondary,
          iconCoinId: selectedCoin?.id,
          iconCoinName: sourcePrimary,
        });
      }

      if (conversionEnabled) {
        const toDisplay = selectedReceiveAssetOption
          ? getReceiveOptionDisplay(
              selectedReceiveAssetOption,
              selectedReceiveLabel,
              selectedExportSystemId ?? activeExportSystemId
            )
          : null;
        const toPrimary = toDisplay?.primary?.trim() ?? '';
        const toSecondary = toDisplay?.secondary;
        if (toPrimary) {
          rows.push({
            label: i18n.t('wallet.transfer.summary.to'),
            primary: toPrimary,
            secondary: toSecondary,
            iconCoinId: selectedReceiveAssetOption?.destinationId,
            iconCoinName: toPrimary,
          });
        }
      }

      if (selectedDestinationNetworkValue) {
        rows.push({
          label: i18n.t('wallet.transfer.summary.destinationNetwork'),
          primary: selectedDestinationNetworkValue,
        });
      }

      if (conversionEnabled && amountValid && activeConvertRoute) {
        const routePrimary = getViaOptionLabel(activeConvertRoute).trim();
        const routeSecondary = selectedDestinationNetworkValue
          ? undefined
          : normalizeRouteSummarySecondary(routePrimary, getRouteSubtitle(activeConvertRoute));
        if (routePrimary) {
          rows.push({
            label: i18n.t('wallet.transfer.summary.route'),
            primary: routePrimary,
            secondary: routeSecondary,
          });
        }
      }

      if (amountValid) {
        const amountValue = amount.trim();
        const amountFqn = selectedCoinPresentation?.displayTicker?.trim() ?? '';
        if (amountValue) {
          const formattedAmount = formatAmountForReviewDisplay(
            amountValue,
            MAX_TRANSFER_AMOUNT_FRACTION_DIGITS
          );
          const amountPrimary = amountFqn ? `${formattedAmount} ${amountFqn}` : formattedAmount;
          rows.push({
            label: i18n.t('wallet.transfer.summary.amount'),
            primary: amountPrimary,
          });
        }
      }

      if (conversionEnabled && estimatedConversionValue && selectedReceiveAssetOption) {
        const estimatedPrimaryValue = estimatedConversionValue.trim();
        if (estimatedPrimaryValue) {
          const estimatedReceiveLabel = resolveReceiveLabel(
            selectedReceiveAssetOption,
            selectedExportSystemId ?? activeExportSystemId
          );
          const estimatedPrimary = estimatedReceiveLabel
            ? `~${estimatedPrimaryValue} ${estimatedReceiveLabel}`
            : `~${estimatedPrimaryValue}`;
          rows.push({
            label: i18n.t('wallet.transfer.summary.estimatedReceive'),
            primary: estimatedPrimary,
          });
        }
      }

      if (activeExportSystemId && isEthereumExport(activeExportSystemId)) {
        rows.push({
          label: i18n.t('wallet.transfer.summary.bridgeFeeEstimate'),
          primary: bridgeFeeParityEligible
            ? bridgeFeeEstimateValue
            : i18n.t('wallet.transfer.bridgeFeeUnavailable'),
          secondary: bridgeFeeParityEligible ? bridgeFeeEstimateSecondary : undefined,
        });
      }

      const recipientAddress = destinationAddress.trim();
      if (recipientAddress) {
        const truncatedRecipientAddress = truncateAddressMiddle(recipientAddress);
        const recipientName = matchedSavedRecipient?.contact.displayName?.trim() ?? '';
        const recipientPrimary = recipientName || truncatedRecipientAddress;
        const recipientSecondary = normalizeSummarySecondary(
          recipientPrimary,
          truncatedRecipientAddress
        );
        rows.push({
          label: i18n.t('wallet.transfer.summary.recipient'),
          primary: recipientPrimary,
          secondary: recipientSecondary,
          primaryIdentifier: !recipientName,
          secondaryIdentifier: true,
        });
      }

      if (activePreflight) {
        const feePrimary = formatAmountForReviewDisplay(
          activePreflight.fee.trim(),
          MAX_TRANSFER_AMOUNT_FRACTION_DIGITS
        );
        const feeSecondary = normalizeSummarySecondary(
          feePrimary,
          reviewNetworkFeeCurrencyLabel || activePreflight.feeCurrency
        );
        if (feePrimary) {
          rows.push({
            label: i18n.t('wallet.transfer.summary.networkFee'),
            primary: feePrimary,
            secondary: feeSecondary,
          });
        }
      }

      return rows;
    })()
  );

  const resolvedPreflightRequest = $derived<ResolvedPreflightRequest | null>(
    (() => {
      if (!selectedCoin || !selectedChannelId || !activeTargetOption) return null;
      const useBridgePreflight = conversionEnabled
        ? !!activeConvertRoute
        : !!activeTargetOption.exportTo;
      if (useBridgePreflight) {
        return {
          kind: 'bridge',
          params: {
            coinId: selectedCoin.id,
            channelId: selectedChannelId,
            sourceAddress: selectedSourceAddress || null,
            destination: destinationAddress.trim(),
            amount: amount.trim(),
            convertTo: activeTargetOption.convertTo ?? null,
            exportTo: activeTargetOption.exportTo ?? null,
            via: activeTargetOption.via ?? null,
            mapTo: activeTargetOption.mapTo ?? null,
            preconvert: null,
          },
        };
      }
      return {
        kind: 'direct',
        params: withDirectSendFeeMode(
          {
            coinId: selectedCoin.id,
            channelId: selectedChannelId,
            toAddress: destinationAddress.trim(),
            amount: amount.trim(),
            memo: showDlightMemoField ? memo.trim() || null : null,
          },
          directSendRoute,
          directSendFeeMode
        ),
      };
    })()
  );
  const preflightInputSignature = $derived(preflightRequestSignature(resolvedPreflightRequest));

  let previousPreflightInputSignature = $state<string | null>(null);

  $effect(() => {
    if (conversionInitialized) return;
    conversionEnabled = entryIntent === 'convert';
    conversionInitialized = true;
  });

  $effect(() => {
    const requestedCoins = sendableCoins;
    let cancelled = false;

    void Promise.all(
      requestedCoins.map(async (coin) => {
        try {
          const result = await getDisplayCoinScopes(coin.id);
          return { coinId: coin.id, scopes: result.scopes };
        } catch (error) {
          if (isWalletDisplayRequestInvalidated(error)) return null;
          return { coinId: coin.id, scopes: [] };
        }
      })
    ).then((results) => {
      if (cancelled) return;
      const nextScopes: Record<string, CoinScope[]> = {};
      const nextLoaded: Record<string, true> = {};
      for (const result of results) {
        if (!result) continue;
        nextScopes[result.coinId] = result.scopes;
        nextLoaded[result.coinId] = true;
      }
      coinScopesByCoinId = nextScopes;
      loadedScopeCoinIds = nextLoaded;
    });

    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    const coin = selectedCoin;
    const channelId = selectedChannelId;
    let cancelled = false;

    bridgeCapabilities = null;

    if (!coin || !channelId) {
      return () => {
        cancelled = true;
      };
    }

    void (async () => {
      try {
        const capabilities = await getBridgeCapabilities({
          coinId: coin.id,
          channelId,
        });
        if (cancelled) return;
        bridgeCapabilities = capabilities;
      } catch (error) {
        if (cancelled) return;
        bridgeCapabilities = {
          conversionSupported: false,
          executionEngine: 'unknown',
          reasonCode: extractWalletErrorType(error),
        };
      }
    })();

    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    const coin = selectedCoin;
    const channelId = selectedChannelId;
    const defaultTicker = resolveCoinPresentationById('VRSC')?.displayTicker?.trim() || 'VRSC';
    let cancelled = false;

    if (!bridgeFeeParityEligible || !coin || !channelId) {
      bridgeFeeInfo = {
        loading: false,
        feeCoins: null,
        feeSats: null,
        balanceCoins: null,
        currencyTicker: defaultTicker,
        systemId: null,
        error: null,
      };
      return () => {
        cancelled = true;
      };
    }

    bridgeFeeInfo = {
      loading: true,
      feeCoins: null,
      feeSats: null,
      balanceCoins: null,
      currencyTicker: defaultTicker,
      systemId: null,
      error: null,
    };

    void (async () => {
      try {
        const estimate: BridgeExportFeeEstimateResult = await estimateBridgeExportFee({
          coinId: coin.id,
          channelId,
        });
        if (cancelled) return;
        bridgeFeeInfo = {
          loading: false,
          feeCoins: estimate.feeCoins?.trim() || null,
          feeSats: estimate.feeSats?.trim() || null,
          balanceCoins: estimate.balanceCoins?.trim() || null,
          currencyTicker: estimate.currencyTicker?.trim() || defaultTicker,
          systemId: estimate.systemId?.trim() || null,
          error: null,
        };
      } catch (error) {
        if (cancelled) return;
        bridgeFeeInfo = {
          loading: false,
          feeCoins: null,
          feeSats: null,
          balanceCoins: null,
          currencyTicker: defaultTicker,
          systemId: null,
          error: mapWalletError(error),
        };
      }
    })();

    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    if (positiveSendableCoinOptions.length === 0) {
      selectedCoinId = '';
      selectedSourceChannelId = '';
      sourceCoinManuallyChosen = false;
      return;
    }

    const selectedStillAvailable = positiveSendableCoinOptions.some(
      (option) => option.coin.id === selectedCoinId && option.channelId === selectedSourceChannelId
    );

    if (!selectedStillAvailable) {
      selectedCoinId = '';
      selectedSourceChannelId = '';
      sourceCoinManuallyChosen = false;
    }
  });

  $effect(() => {
    const context = entryContext;
    if (!context) return;
    const contextKey = `${context.coinId}|${context.channelId}|${context.scopeKind}|${context.readOnly ? 'read-only' : 'owned'}`;
    const contextOption = sendableCoinOptions.find(
      (option) => option.coin.id === context.coinId && option.channelId === context.channelId
    );
    const ready = !!loadedScopeCoinIds[context.coinId] && (context.readOnly || !!contextOption);
    if (!entryContextGuard.claim(contextKey, ready)) return;
    if (context.readOnly) return;
    if (!contextOption) return;

    selectedCoinId = context.coinId;
    selectedSourceChannelId = context.channelId;
    sourceCoinManuallyChosen = true;
  });

  $effect(() => {
    if (!sourceSupportsConversion) {
      discoveredPathQuotes = {};
      loadingTargets = false;
      targetsError = '';
      selectedReceiveAssetId = '';
      selectedExportSystemId = null;
      selectedSendExportSystemId = null;
      selectedViaOptionId = '';
      manualViaLocked = false;
      receiveSearchTerm = '';
      pendingGroupedReceiveOption = null;
      pendingTargetOption = null;
      showReceiveAssetSheet = false;
      showNetworkSheet = false;
      showExportSheet = false;
    }
  });

  $effect(() => {
    if (showConvertUnavailable && conversionEnabled) {
      conversionEnabled = false;
      manualViaLocked = false;
      selectedViaOptionId = '';
    }
  });

  $effect(() => {
    if (showNetworkSheet) return;
    pendingGroupedReceiveOption = null;
  });

  $effect(() => {
    if (showExportSheet) return;
    pendingTargetOption = null;
    exportSheetMode = 'convert';
  });

  $effect(() => {
    if (rawReceiveAssetSections.allOptions.length === 0) {
      selectedReceiveAssetId = '';
      selectedExportSystemId = null;
      selectedSendExportSystemId = null;
      selectedViaOptionId = '';
      manualViaLocked = false;
      pendingGroupedReceiveOption = null;
      pendingTargetOption = null;
      showReceiveAssetSheet = false;
      showNetworkSheet = false;
      showExportSheet = false;
      return;
    }

    if (!selectableReceiveAssetOptions.some((option) => option.id === selectedReceiveAssetId)) {
      selectedReceiveAssetId = '';
      selectedExportSystemId = null;
      selectedViaOptionId = '';
      manualViaLocked = false;
    }
  });

  $effect(() => {
    if (!selectedReceiveAssetOption) return;
    if (selectedExportSystemId === null) return;

    const exportStillValid = selectedReceiveAssetOption.exportOptions.some(
      (option) => option.exportTo === selectedExportSystemId
    );
    if (exportStillValid) return;

    selectedExportSystemId = selectedReceiveAssetOption.hasOnChainPath
      ? null
      : (selectedReceiveAssetOption.exportOptions[0]?.exportTo ?? null);
    selectedViaOptionId = '';
    manualViaLocked = false;
  });

  $effect(() => {
    if (!sendSameAssetOption) {
      selectedSendExportSystemId = null;
      return;
    }
    if (selectedSendExportSystemId === null) return;

    const exportStillValid = sendSameAssetOption.exportOptions.some(
      (option) => option.exportTo === selectedSendExportSystemId
    );
    if (exportStillValid) return;

    selectedSendExportSystemId = null;
  });

  $effect(() => {
    if (!conversionEnabled) return;
    if (!selectedReceiveAssetOption) return;
    if (!amountValid) {
      selectedViaOptionId = '';
      manualViaLocked = false;
      return;
    }

    const selectedStillValid = selectedReceiveAssetViaOptions.some(
      (option) => option.id === selectedViaOptionId
    );

    if (manualViaLocked && selectedStillValid) return;

    const best =
      sortViaOptionsByScore(selectedReceiveAssetViaOptions, amount, routeEstimateOutputs)[0] ??
      null;
    if (!best) {
      selectedViaOptionId = '';
      manualViaLocked = false;
      return;
    }

    if (selectedViaOptionId !== best.id) {
      selectedViaOptionId = best.id;
    }

    if (manualViaLocked && !selectedStillValid) {
      manualViaLocked = false;
    }
  });

  $effect(() => {
    const signature = preflightInputSignature;
    if (previousPreflightInputSignature === null) {
      previousPreflightInputSignature = signature;
      return;
    }

    if (signature === previousPreflightInputSignature) return;

    previousPreflightInputSignature = signature;
    clearPreflightState();
  });

  $effect(() => {
    const sanitizedAmount = sanitizeAmountInput(amount, MAX_TRANSFER_AMOUNT_FRACTION_DIGITS);
    if (sanitizedAmount === amount) return;
    amount = sanitizedAmount;
  });

  $effect(() => {
    if (!selectedCoinOption) return;
    const trimmedAmount = amount.trim();
    if (!trimmedAmount) return;
    const numericAmount = Number(trimmedAmount);
    if (!Number.isFinite(numericAmount)) return;
    if (numericAmount <= selectedBalanceValue) return;
    amount = selectedBalance;
  });

  $effect(() => {
    const coin = selectedCoin;
    const channelId = selectedChannelId;
    const sourceCurrency = coin?.currencyId || coin?.id || '';
    const normalizedAmount = amount.trim();
    const numericAmount = Number(normalizedAmount);
    const viaOptions = selectedReceiveAssetViaOptions;
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | null = null;

    if (
      !conversionEnabled ||
      !coin ||
      !channelId ||
      !sourceCurrency ||
      !amountValid ||
      !Number.isFinite(numericAmount) ||
      numericAmount <= 0 ||
      viaOptions.length === 0
    ) {
      routeEstimateOutputs = {};
      return () => {
        cancelled = true;
      };
    }

    routeEstimateOutputs = {};

    const fallbackEstimateOutput = (option: ViaRouteOption): string | null => {
      const price = parsePrice(option.price);
      if (price === null) return null;
      return (numericAmount * price).toString();
    };

    timer = setTimeout(() => {
      void (async () => {
        const estimateCache = new Map<string, Promise<string | null>>();
        const nextOutputs: Record<string, string> = {};

        await Promise.all(
          viaOptions.map(async (option) => {
            const convertTo = option.convertTo?.trim();
            if (!convertTo) {
              const fallback = fallbackEstimateOutput(option);
              if (fallback) nextOutputs[option.id] = fallback;
              return;
            }

            const via = option.via?.trim() || null;
            const estimateKey = `${convertTo.toLowerCase()}|${(via ?? '').toLowerCase()}`;

            let estimatePromise = estimateCache.get(estimateKey);
            if (!estimatePromise) {
              estimatePromise = (async () => {
                try {
                  const response = await estimateBridgeConversion({
                    coinId: coin.id,
                    channelId,
                    sourceCurrency,
                    convertTo,
                    amount: normalizedAmount,
                    via,
                    preconvert: false,
                  });
                  const estimatedOut = parseEstimatedOutput(response.estimatedCurrencyOut ?? null);
                  if (estimatedOut !== null) return estimatedOut.toString();

                  const estimatedPrice = parsePrice(response.price ?? null);
                  if (estimatedPrice !== null) return (numericAmount * estimatedPrice).toString();
                } catch {
                  // Fall through to price-based fallback below.
                }
                return null;
              })();
              estimateCache.set(estimateKey, estimatePromise);
            }

            const estimateOutput = await estimatePromise;
            if (estimateOutput) {
              nextOutputs[option.id] = estimateOutput;
              return;
            }

            const fallback = fallbackEstimateOutput(option);
            if (fallback) {
              nextOutputs[option.id] = fallback;
            }
          })
        );

        if (cancelled) return;
        routeEstimateOutputs = nextOutputs;
      })();
    }, 350);

    return () => {
      cancelled = true;
      if (timer) {
        clearTimeout(timer);
      }
    };
  });

  $effect(() => {
    const coin = selectedCoin;
    const channelId = selectedChannelId;
    const sourceCurrency = coin?.currencyId || coin?.id || '';
    let cancelled = false;

    if (
      currentStep !== 'details' ||
      !sourceSupportsConversion ||
      !coin ||
      !channelId ||
      !sourceCurrency
    ) {
      return () => {
        cancelled = true;
      };
    }

    loadingTargets = true;
    targetsError = '';

    void (async () => {
      try {
        const response = await getBridgeConversionPaths({
          coinId: coin.id,
          channelId,
          sourceCurrency,
        });
        if (cancelled) return;
        discoveredPathQuotes = response.paths;
      } catch (error) {
        if (cancelled) return;
        discoveredPathQuotes = {};
        targetsError = mapWalletError(error);
      } finally {
        if (!cancelled) loadingTargets = false;
      }
    })();

    return () => {
      cancelled = true;
    };
  });

  function isTxSendProgressStage(value: unknown): value is TxSendProgressStage {
    return (
      value === 'syncing_spend_state' ||
      value === 'loading_prover' ||
      value === 'building_proof' ||
      value === 'broadcasting'
    );
  }

  onMount(() => {
    let disposed = false;
    let unlistenTxSendProgress: (() => void) | null = null;
    const tickInterval = setInterval(() => {
      if (!sending || sendStageStartedAt === null) return;
      sendStageTick = Date.now();
    }, 1000);

    void (async () => {
      try {
        addresses = await walletService.getAddresses();
      } catch {
        addresses = null;
      }

      try {
        const pendingSubmission = await getPendingEthSubmission();
        if (!disposed) pendingEthSubmission = pendingSubmission;
      } catch (error) {
        const errorType = extractWalletErrorType(error);
        if (!disposed && errorType !== 'WalletSessionChanged' && errorType !== 'WalletLocked') {
          transferError = mapWalletError(error);
        }
      }

      try {
        const unlisten = await listen<TxSendProgressEventPayload>(
          'wallet://tx-send-progress',
          (event) => {
            const stageValue = event.payload?.stage;
            if (!isTxSendProgressStage(stageValue)) return;

            const payloadChannel = event.payload?.channel?.trim();
            if (payloadChannel && selectedChannelId && payloadChannel !== selectedChannelId) return;

            sendStage = stageValue;
            sendStageStartedAt = Date.now();
            sendStageTick = Date.now();
          }
        );
        if (disposed) {
          unlisten();
        } else {
          unlistenTxSendProgress = unlisten;
        }
      } catch {
        // Best-effort progress updates only.
      }
    })();

    return () => {
      disposed = true;
      clearInterval(tickInterval);
      if (unlistenTxSendProgress) unlistenTxSendProgress();
      ethRecoveryLifetime.dispose();
      copiedSuccessFieldState.destroy();
      preflightRequestGuard.dispose();
    };
  });

  $effect(() => {
    destinationAddress;
    unsavedRecipientConfirmed = false;
  });

  $effect(() => {
    if (!showDlightMemoField && memo) {
      memo = '';
    }
  });

  $effect(() => {
    if (currentStep !== 'details') return;
    void tick().then(() => amountInputEl?.focus());
  });

  function clearPreflightState() {
    preflightRequestGuard.invalidate();
    simplePreflightResult = null;
    bridgePreflightResult = null;
    preflighting = false;
    transferError = '';
  }

  function shortRecipientAddress(value: string): string {
    return truncateAddressMiddle(value, 10, 10);
  }

  function endpointBadgeLabel(kind: AddressEndpointKind): string {
    if (kind === 'vrpc') return 'VERUS';
    if (kind === 'zs') return 'ZS';
    return kind.toUpperCase();
  }

  function selectAddressBookRecipient(option: AddressBookEndpointOption) {
    destinationAddress = option.endpointAddress;
    showAddressBookSheet = false;
    addressBookSearchTerm = '';
    transferError = '';
  }

  function selectSelfRecipient() {
    if (!selfDestinationAddress) return;
    destinationAddress = selfDestinationAddress;
    transferError = '';
  }

  async function pasteRecipientAddress() {
    try {
      const pastedAddress = (await walletService.readClipboardText()).trim();
      if (!pastedAddress) return;
      destinationAddress = pastedAddress;
      transferError = '';
    } catch {
      // Ignore clipboard read errors and keep manual entry available.
    }
  }

  async function copySuccessFieldValue(value: string, field: 'recipient' | 'txid') {
    if (await writeClipboardText(value)) {
      copiedSuccessFieldState.set(field, 1800);
      return;
    }
    copiedSuccessFieldState.clear();
  }

  function filterViaOptionsByExport(
    options: ViaRouteOption[],
    exportSystemId: string | null
  ): ViaRouteOption[] {
    return options.filter((option) => {
      if (exportSystemId === null) {
        return option.exportTo === null || option.exportTo === undefined;
      }
      return option.exportTo === exportSystemId;
    });
  }

  function getRouteSubtitle(option: ViaRouteOption): string {
    const subtitleParts: string[] = [];
    if (option.exportTo) {
      subtitleParts.push(
        i18n.t('wallet.transfer.pathExportTo', { value: option.exportToLabel ?? option.exportTo })
      );
    }
    if (option.via) {
      subtitleParts.push(
        i18n.t('wallet.transfer.pathVia', { value: option.viaLabel ?? option.via })
      );
    }
    if (option.mapTo) {
      subtitleParts.push(i18n.t('wallet.transfer.pathMapTo', { value: option.mapTo }));
    }
    return subtitleParts.join(' • ');
  }

  function getViaSheetSubtitle(option: ViaRouteOption): string {
    const subtitleParts: string[] = [];
    if (option.exportTo) {
      subtitleParts.push(
        i18n.t('wallet.transfer.pathExportTo', { value: option.exportToLabel ?? option.exportTo })
      );
    }
    if (option.mapTo) {
      subtitleParts.push(i18n.t('wallet.transfer.pathMapTo', { value: option.mapTo }));
    }
    return subtitleParts.join(' • ');
  }

  function getViaOptionLabel(option: ViaRouteOption): string {
    return option.viaLabel ?? option.via ?? i18n.t('wallet.transfer.viaBest');
  }

  function isReceiveOptionSelected(option: ReceiveAssetOption): boolean {
    if (option.isGrouped && option.networkOptions?.length) {
      return option.networkOptions.some(
        (networkOption) => networkOption.id === selectedReceiveAssetId
      );
    }
    return option.id === selectedReceiveAssetId;
  }

  function closeAssetSelectionSheets() {
    showReceiveAssetSheet = false;
    showNetworkSheet = false;
    showExportSheet = false;
    exportSheetMode = 'convert';
  }

  function finalizeReceiveSelection(option: ReceiveAssetOption, exportSystemId: string | null) {
    selectedReceiveAssetId = option.id;
    selectedExportSystemId = exportSystemId;
    manualViaLocked = false;

    if (amountValid) {
      const viaCandidates = filterViaOptionsByExport(option.viaOptions, exportSystemId);
      const best = sortViaOptionsByScore(viaCandidates, amount, routeEstimateOutputs)[0] ?? null;
      selectedViaOptionId = best?.id ?? '';
    } else {
      selectedViaOptionId = '';
    }

    pendingGroupedReceiveOption = null;
    pendingTargetOption = null;
    closeAssetSelectionSheets();
    transferError = '';
  }

  function finalizeSendExportSelection(exportSystemId: string | null) {
    selectedSendExportSystemId = exportSystemId;
    manualViaLocked = false;
    pendingGroupedReceiveOption = null;
    pendingTargetOption = null;
    closeAssetSelectionSheets();
    transferError = '';
  }

  function beginReceiveSelection(option: ReceiveAssetOption) {
    exportSheetMode = 'convert';
    pendingTargetOption = option;
    showReceiveAssetSheet = false;
    showNetworkSheet = false;

    if (option.isCrossChain && option.exportOptions.length > 0) {
      showExportSheet = true;
      return;
    }

    finalizeReceiveSelection(option, null);
  }

  function openSendDestinationNetworkSheet() {
    if (!sendSameAssetOption) return;
    exportSheetMode = 'send';
    pendingGroupedReceiveOption = null;
    pendingTargetOption = sendSameAssetOption;
    showSourceAssetSheet = false;
    showReceiveAssetSheet = false;
    showNetworkSheet = false;
    showExportSheet = true;
    transferError = '';
  }

  function isPositiveAmount(input: string): boolean {
    const value = Number(input);
    return Number.isFinite(value) && value > 0;
  }

  function toFiniteNumber(value: unknown): number {
    if (typeof value === 'number') {
      return Number.isFinite(value) ? value : 0;
    }

    if (typeof value === 'string') {
      const parsed = Number(value.trim());
      return Number.isFinite(parsed) ? parsed : 0;
    }

    return 0;
  }

  function toOptionalFiniteNumber(value: unknown): number | null {
    if (typeof value === 'number') {
      return Number.isFinite(value) ? value : null;
    }

    if (typeof value === 'string') {
      const parsed = Number(value.trim());
      return Number.isFinite(parsed) ? parsed : null;
    }

    return null;
  }

  function formatSyncPercent(percent: number): string {
    const clamped = Math.max(0, Math.min(percent, 100));
    if (clamped > 0 && clamped < 1) return '<1';
    const floored = Math.floor(clamped * 10) / 10;
    return i18n.formatNumber(floored, {
      minimumFractionDigits: 0,
      maximumFractionDigits: 1,
    });
  }

  function formatSheetBalance(value: string): string {
    const numeric = toFiniteNumber(value);
    return i18n.formatNumber(numeric, {
      minimumFractionDigits: 0,
      maximumFractionDigits: 8,
    });
  }

  function isEthereumExport(systemId: string | null | undefined): boolean {
    const normalized = systemId?.trim().toLowerCase() ?? '';
    if (!normalized) return false;
    return (
      normalized === VETH_SYSTEM_ID.toLowerCase() ||
      normalized === '.eth' ||
      normalized === 'veth' ||
      normalized === 'bridge.veth'
    );
  }

  function normalizeNetworkDisplayName(
    systemId: string | null | undefined,
    fallback: string
  ): string {
    const normalizedSystemId = systemId?.trim().toLowerCase() ?? '';
    const fallbackTrimmed = fallback.trim();
    const normalizedFallback = fallbackTrimmed.toLowerCase();
    if (
      normalizedSystemId === VRSC_SYSTEM_ID.toLowerCase() ||
      normalizedFallback === 'vrsc' ||
      normalizedFallback === 'verus'
    ) {
      return resolveCoinPresentationById('VRSC')?.displayName?.trim() || 'Verus';
    }
    if (
      normalizedSystemId === VRSCTEST_SYSTEM_ID.toLowerCase() ||
      normalizedFallback === 'vrsctest' ||
      normalizedFallback === 'verus testnet'
    ) {
      return resolveCoinPresentationById('VRSCTEST')?.displayName?.trim() || 'Verus Testnet';
    }
    if (
      isEthereumExport(systemId) ||
      normalizedSystemId === '.eth' ||
      normalizedFallback === 'veth' ||
      normalizedFallback === 'ethereum on verus'
    ) {
      return (
        resolveCoinPresentationById('ETH')?.displayName?.trim() ||
        fallbackTrimmed ||
        systemId?.trim() ||
        ''
      );
    }
    return fallbackTrimmed || systemId?.trim() || '';
  }

  function networkLabelForExportOption(exportTo: string, exportToName: string): string {
    return normalizeNetworkDisplayName(exportTo, exportToName || exportTo);
  }

  function networkIconCoinIdForExportOption(exportTo: string, exportToName: string): string {
    const normalizedExportTo = exportTo.trim().toLowerCase();
    const normalizedName = (exportToName || '').trim().toLowerCase();

    if (
      isEthereumExport(exportTo) ||
      normalizedExportTo === '.eth' ||
      normalizedName === 'veth' ||
      normalizedName === 'ethereum on verus' ||
      normalizedName === 'ethereum'
    ) {
      return 'ETH';
    }

    if (
      normalizedExportTo === VRSC_SYSTEM_ID.toLowerCase() ||
      normalizedName === 'vrsc' ||
      normalizedName === 'verus'
    ) {
      return 'VRSC';
    }

    if (
      normalizedExportTo === VRSCTEST_SYSTEM_ID.toLowerCase() ||
      normalizedName === 'vrsctest' ||
      normalizedName === 'verus testnet'
    ) {
      return 'VRSCTEST';
    }

    return exportTo;
  }

  function networkLabelForGroupedOption(option: ReceiveAssetOption): string {
    if (option.hasOnChainPath) {
      return i18n.t('wallet.transfer.keepOnNetwork', { value: sourceNetworkDisplayName });
    }
    const firstExportOption = option.exportOptions[0];
    if (!firstExportOption) return option.label;
    return networkLabelForExportOption(firstExportOption.exportTo, firstExportOption.exportToName);
  }

  function resolveSourceNetworkDisplayName(
    systemId: string | null | undefined,
    channelPrefix: string
  ): string {
    const normalizedSystemId = systemId?.trim() ?? '';
    const normalizedSystemIdLc = normalizedSystemId.toLowerCase();
    const normalizedPrefix = channelPrefix.trim().toLowerCase();

    if (normalizedSystemIdLc === VRSC_SYSTEM_ID.toLowerCase()) {
      return resolveCoinPresentationById('VRSC')?.displayName?.trim() || 'Verus';
    }
    if (normalizedSystemIdLc === VRSCTEST_SYSTEM_ID.toLowerCase()) {
      return resolveCoinPresentationById('VRSCTEST')?.displayName?.trim() || 'Verus Testnet';
    }

    // vETH is a Verus PBaaS system for same-network semantics.
    if (isEthereumExport(normalizedSystemId)) {
      return resolveCoinPresentationById('VRSC')?.displayName?.trim() || 'Verus';
    }

    if (
      normalizedPrefix === 'eth' ||
      normalizedPrefix === 'erc20' ||
      normalizedSystemId.toLowerCase() === '.eth'
    ) {
      return resolveCoinPresentationById('ETH')?.displayName?.trim() || 'Ethereum';
    }

    if (normalizedPrefix === 'btc' || normalizedSystemId.toLowerCase() === '.btc') {
      return resolveCoinPresentationById('BTC')?.displayName?.trim() || 'Bitcoin';
    }

    const systemCoin =
      coins.find(
        (coin) =>
          coin.systemId.toLowerCase() === normalizedSystemIdLc &&
          coin.currencyId.toLowerCase() === normalizedSystemIdLc
      ) ??
      coins.find((coin) => coin.systemId.toLowerCase() === normalizedSystemIdLc) ??
      null;

    const presentation = systemCoin
      ? resolveCoinPresentation(systemCoin)
      : normalizedSystemId
        ? resolveCoinPresentationById(normalizedSystemId)
        : null;
    const displayName =
      presentation?.displayName?.trim() || presentation?.displayTicker?.trim() || '';
    if (displayName) {
      return displayName;
    }

    // Avoid exposing raw i-addresses in user-facing copy.
    if (/^i[a-km-zA-HJ-NP-Z1-9]{24,60}$/.test(normalizedSystemId)) {
      if (normalizedPrefix === 'vrpc') {
        return resolveCoinPresentationById('VRSC')?.displayName?.trim() || 'Verus';
      }
      return 'Network';
    }

    return normalizedSystemId || 'Verus';
  }

  function stripBridgeSuffix(value: string): string {
    const trimmed = value.trim();
    if (!trimmed) return '';
    const withoutErc20 = trimmed.replace(/\s*\[ERC20\]\s*/gi, '').trim();
    const withoutNetworkSuffix = withoutErc20
      .replace(/\s+on\s+Ethereum$/i, '')
      .replace(/\s+on\s+Verus$/i, '')
      .trim();
    if (!withoutNetworkSuffix.toUpperCase().endsWith('.VETH')) {
      return withoutNetworkSuffix;
    }
    const withoutVeth = withoutNetworkSuffix.slice(0, -'.vETH'.length);
    if (/^v[A-Za-z0-9]+$/.test(withoutVeth)) {
      return withoutVeth.slice(1);
    }
    return withoutVeth;
  }

  function canonicalizeBridgeTicker(value: string): string {
    const stripped = stripBridgeSuffix(value).trim();
    if (!stripped) return '';
    if (/^v[A-Za-z0-9]+$/.test(stripped)) {
      return stripped.slice(1).toUpperCase();
    }
    return stripped.toUpperCase();
  }

  function isLikelyEvmAddress(value: string): boolean {
    return /^0x[a-fA-F0-9]{40}$/.test(value.trim());
  }

  function normalizeSendExportKey(value: string): string {
    const normalized = value.trim().toLowerCase();
    if (!normalized) return '';
    if (
      isEthereumExport(value) ||
      normalized === '.eth' ||
      normalized === 'bridge.veth' ||
      normalized === 'veth'
    ) {
      return 'ethereum';
    }
    return normalized;
  }

  function upsertSendExportOption(
    dedupe: Map<string, ExportRouteOption>,
    incoming: ExportRouteOption
  ): void {
    const key = normalizeSendExportKey(incoming.exportTo);
    if (!key) return;

    const existing = dedupe.get(key);
    if (!existing) {
      dedupe.set(key, incoming);
      return;
    }

    const preferIncomingExportTo =
      (!isEthereumExport(existing.exportTo) || existing.exportTo.toLowerCase() === 'bridge.veth') &&
      isEthereumExport(incoming.exportTo);

    dedupe.set(key, {
      exportTo: preferIncomingExportTo ? incoming.exportTo : existing.exportTo,
      exportToName:
        (preferIncomingExportTo ? incoming.exportToName : existing.exportToName) ||
        incoming.exportToName ||
        existing.exportToName,
      gateway: existing.gateway || incoming.gateway,
      via: existing.via ?? incoming.via ?? null,
      price: existing.price ?? incoming.price ?? null,
      mappingDestination: existing.mappingDestination ?? incoming.mappingDestination,
    });
  }

  function mergeSendExportOptions(
    primary: ExportRouteOption[],
    fallback: ExportRouteOption[]
  ): ExportRouteOption[] {
    const dedupe = new Map<string, ExportRouteOption>();
    for (const option of primary) upsertSendExportOption(dedupe, option);
    for (const option of fallback) upsertSendExportOption(dedupe, option);
    return Array.from(dedupe.values());
  }

  function resolveSendMappingDestination(quote: BridgeConversionPathQuote): string | undefined {
    const candidates = [
      quote.destinationDisplayTicker,
      quote.convertToDisplayName,
      quote.destinationDisplayName,
      quote.convertTo,
      quote.destinationId,
      quote.mapTo,
    ];
    let fallback: string | undefined;

    for (const candidate of candidates) {
      if (!candidate) continue;
      const trimmed = candidate.trim();
      if (!trimmed) continue;
      if (!fallback) fallback = trimmed;
      if (!isLikelyEvmAddress(trimmed)) return trimmed;
    }

    return fallback;
  }

  function targetOptionLikelyBridgedAsset(targetOption: ReceiveAssetOption): boolean {
    const candidates = [targetOption.fullyqualifiedname, targetOption.ticker, targetOption.label];
    return candidates.some((candidate) => candidate?.toLowerCase().includes('.veth'));
  }

  function normalizeEthereumReceiveLabel(value: string, targetOption: ReceiveAssetOption): string {
    const stripped = stripBridgeSuffix(value).trim();
    if (!stripped) return '';
    if (/^bridge$/i.test(stripped)) return 'ETH';
    if (targetOptionLikelyBridgedAsset(targetOption) && /^v[A-Za-z0-9]+$/.test(stripped)) {
      return stripped.slice(1);
    }
    return stripped;
  }

  function receiveLabelForExportOption(
    option: ExportRouteOption,
    targetOption: ReceiveAssetOption
  ): string {
    const exportTarget = option.exportTo?.trim() ?? '';
    const ethereumExport = isEthereumExport(exportTarget) || exportTarget.toLowerCase() === '.eth';

    if (ethereumExport) {
      const ethDestinationLabel =
        targetOption.ethDisplayTicker?.trim() || targetOption.ethDisplayName?.trim() || '';
      if (ethDestinationLabel) {
        const normalizedEthDestinationLabel = normalizeEthereumReceiveLabel(
          ethDestinationLabel,
          targetOption
        );
        if (normalizedEthDestinationLabel && !isLikelyEvmAddress(normalizedEthDestinationLabel)) {
          return normalizedEthDestinationLabel;
        }
      }

      const mappingDestinationRef = option.mappingDestination?.trim() ?? '';
      if (mappingDestinationRef) {
        const mappedDestinationPresentation = resolveCoinPresentationById(mappingDestinationRef);
        const mappedDestinationLabel =
          mappedDestinationPresentation?.displayTicker?.trim() ||
          mappedDestinationPresentation?.displayName?.trim() ||
          mappingDestinationRef;
        const normalizedMappedDestinationLabel = normalizeEthereumReceiveLabel(
          mappedDestinationLabel,
          targetOption
        );
        if (
          normalizedMappedDestinationLabel &&
          !isLikelyEvmAddress(normalizedMappedDestinationLabel)
        ) {
          return normalizedMappedDestinationLabel;
        }
      }

      const targetLabel =
        targetOption.ticker?.trim() ||
        targetOption.fullyqualifiedname?.trim() ||
        targetOption.label?.trim() ||
        '';
      const targetPresentation = resolveCoinPresentationById(targetOption.destinationId);
      const mappedToId = targetPresentation?.mappedTo?.trim() ?? '';
      if (mappedToId) {
        const mappedPresentation = resolveCoinPresentationById(mappedToId);
        const mappedLabel =
          mappedPresentation?.displayTicker?.trim() ||
          mappedPresentation?.displayName?.trim() ||
          '';
        if (mappedLabel) {
          return normalizeEthereumReceiveLabel(mappedLabel, targetOption);
        }
      }

      const normalizedTargetLabel = normalizeEthereumReceiveLabel(targetLabel, targetOption);
      if (normalizedTargetLabel && !isLikelyEvmAddress(normalizedTargetLabel)) {
        return normalizedTargetLabel;
      }

      if (sourceCanonicalKey) {
        const canonicalFallback = sourceCanonicalKey.trim();
        if (canonicalFallback) {
          return canonicalFallback;
        }
      }
    }

    return (
      targetOption.fullyqualifiedname?.trim() ||
      targetOption.ticker?.trim() ||
      targetOption.label.trim()
    );
  }

  function resolveReceiveLabel(
    option: ReceiveAssetOption | null | undefined,
    exportSystemId: string | null | undefined
  ): string {
    if (!option) return '';
    const normalizedExportSystemId = exportSystemId?.trim() ?? '';
    if (normalizedExportSystemId) {
      const matchedExportOption =
        option.exportOptions.find(
          (exportOption) => exportOption.exportTo === normalizedExportSystemId
        ) ?? null;
      if (matchedExportOption) {
        return receiveLabelForExportOption(matchedExportOption, option);
      }
    }

    if (!normalizedExportSystemId && !option.hasOnChainPath && option.exportOptions.length > 0) {
      return receiveLabelForExportOption(option.exportOptions[0], option);
    }

    return option.fullyqualifiedname?.trim() || option.ticker?.trim() || option.label.trim();
  }

  function bridgeFeeLineForExportOption(exportSystemId: string | null | undefined): string | null {
    if (!isEthereumExport(exportSystemId)) return null;
    if (bridgeFeeInfo.loading) return i18n.t('wallet.transfer.bridgeFeeCalculating');
    if (bridgeFeeInfo.error || !bridgeFeeInfo.feeCoins)
      return i18n.t('wallet.transfer.bridgeFeeUnavailable');
    return i18n.t('wallet.transfer.bridgeFeeEstimateLine', {
      value: `${bridgeFeeInfo.feeCoins} ${bridgeFeeInfo.currencyTicker}`,
    });
  }

  function bridgeFeeMetaLineForExportOption(
    exportSystemId: string | null | undefined
  ): string | null {
    if (!isEthereumExport(exportSystemId)) return null;
    if (bridgeFeeInsufficient) {
      return i18n.t('wallet.transfer.bridgeFeeInsufficient', {
        ticker: bridgeFeeInfo.currencyTicker,
      });
    }
    if (
      !bridgeFeeInfo.loading &&
      !bridgeFeeInfo.error &&
      bridgeFeeInfo.feeCoins &&
      bridgeFeeFiatDisplay !== '≈ —'
    ) {
      return bridgeFeeFiatDisplay;
    }
    return null;
  }

  function isBridgeFeeSelectionDisabled(exportSystemId: string | null | undefined): boolean {
    if (!isEthereumExport(exportSystemId)) return false;
    return bridgeFeeInsufficient;
  }

  function getReceiveOptionDisplay(
    option: ReceiveAssetOption,
    preferredReceiveLabel?: string | null,
    exportSystemId?: string | null
  ): { primary: string; secondary?: string } {
    const presentation = resolveCoinPresentationById(option.destinationId);
    const preferredLabel = preferredReceiveLabel?.trim() ?? '';
    const primary =
      preferredLabel ||
      presentation?.displayName?.trim() ||
      option.subtitle?.trim() ||
      option.label.trim();
    if (isEthereumExport(exportSystemId)) {
      return { primary, secondary: undefined };
    }
    const secondaryCandidate =
      presentation?.displayTicker?.trim() ||
      option.fullyqualifiedname?.trim() ||
      option.ticker?.trim();
    const normalizedSecondaryCandidate = secondaryCandidate
      ? stripBridgeSuffix(secondaryCandidate)
      : '';
    if (
      normalizedSecondaryCandidate &&
      normalizedSecondaryCandidate.toLowerCase() === primary.toLowerCase()
    ) {
      return { primary, secondary: undefined };
    }
    const secondary =
      secondaryCandidate && secondaryCandidate.toLowerCase() !== primary.toLowerCase()
        ? secondaryCandidate
        : undefined;

    return { primary, secondary };
  }

  function viaLexicalKey(option: ViaRouteOption): string {
    return `${option.via ?? ''}|${option.exportTo ?? ''}|${option.mapTo ?? ''}`.toLowerCase();
  }

  function sanitizeAmountInput(
    rawValue: string,
    maxFractionDigits = MAX_TRANSFER_AMOUNT_FRACTION_DIGITS
  ): string {
    const normalizedInput = rawValue.replace(/,/g, '.').replace(/[^\d.]/g, '');
    if (!normalizedInput) return '';

    const hasDecimalSeparator = normalizedInput.includes('.');
    const [integerPartRaw, ...fractionSegments] = normalizedInput.split('.');
    const fractionRaw = fractionSegments.join('');
    const normalizedInteger = normalizeIntegerPart(integerPartRaw || '0');

    if (!hasDecimalSeparator || maxFractionDigits <= 0) {
      return normalizedInteger;
    }

    const truncatedFraction = fractionRaw.slice(0, maxFractionDigits);
    if (!truncatedFraction && normalizedInput.endsWith('.')) {
      return `${normalizedInteger}.`;
    }

    return truncatedFraction ? `${normalizedInteger}.${truncatedFraction}` : normalizedInteger;
  }

  function isSimplePreflight(
    result: PreflightResult | BridgeTransferPreflightResult | null
  ): result is PreflightResult {
    return !!result && 'feeTakenFromAmount' in result;
  }

  function isBridgePreflight(
    result: PreflightResult | BridgeTransferPreflightResult | null
  ): result is BridgeTransferPreflightResult {
    return !!result && 'amountAdjusted' in result;
  }

  function normalizeAmountString(value?: string | null): string | null {
    if (typeof value !== 'string') return null;
    const trimmed = value.trim();
    if (!trimmed) return null;
    const parsed = parseDecimalParts(trimmed);
    if (!parsed) return trimmed;
    const normalizedInteger = normalizeIntegerPart(parsed.integerPart);
    const normalizedFraction = parsed.fractionPart.replace(/0+$/, '');
    return normalizedFraction ? `${normalizedInteger}.${normalizedFraction}` : normalizedInteger;
  }

  function parseNonNegativeAmount(value?: string | null): number | null {
    if (typeof value !== 'string') return null;
    const trimmed = value.trim();
    if (!trimmed) return 0;
    const parsed = Number(trimmed);
    if (!Number.isFinite(parsed) || parsed < 0) return null;
    return parsed;
  }

  function formatAmountForReviewDisplay(
    rawValue: string | null | undefined,
    maxFractionDigits = MAX_TRANSFER_AMOUNT_FRACTION_DIGITS
  ): string {
    if (typeof rawValue !== 'string') return '';
    const trimmed = rawValue.trim();
    if (!trimmed) return '';
    if (maxFractionDigits < 0) return trimmed;

    const parsed = parseDecimalParts(trimmed);
    if (!parsed) return trimmed;

    const normalizedInteger = normalizeIntegerPart(parsed.integerPart);
    const truncatedFraction = parsed.fractionPart.slice(0, maxFractionDigits).replace(/0+$/, '');

    if (
      normalizedInteger === '0' &&
      !truncatedFraction &&
      isTinyNonZero(parsed.fractionPart, maxFractionDigits)
    ) {
      return `<${formatDisplayFloor(maxFractionDigits)}`;
    }

    if (!truncatedFraction) return normalizedInteger;
    return `${normalizedInteger}.${truncatedFraction}`;
  }

  function parseDecimalParts(value: string): { integerPart: string; fractionPart: string } | null {
    const match = value.match(/^(\d+)(?:\.(\d+))?$/);
    if (!match) return null;
    return {
      integerPart: match[1],
      fractionPart: match[2] ?? '',
    };
  }

  function normalizeIntegerPart(integerPart: string): string {
    const normalized = integerPart.replace(/^0+(?=\d)/, '');
    return normalized || '0';
  }

  function isTinyNonZero(fractionPart: string, maxFractionDigits: number): boolean {
    if (!fractionPart) return false;
    if (maxFractionDigits <= 0) return /[1-9]/.test(fractionPart);

    const withinDisplay = fractionPart.slice(0, maxFractionDigits);
    if (/[1-9]/.test(withinDisplay)) return false;
    return /[1-9]/.test(fractionPart.slice(maxFractionDigits));
  }

  function formatDisplayFloor(maxFractionDigits: number): string {
    if (maxFractionDigits <= 0) return '1';
    return `0.${'0'.repeat(maxFractionDigits - 1)}1`;
  }

  function getDisplayCurrencyRate(rateMap?: Record<string, number>): number | null {
    const rate = getRateForCurrency(rateMap, displayCurrency);
    if (rate === null || !Number.isFinite(rate) || rate <= 0) {
      return null;
    }
    return rate;
  }

  function getDisplayCurrencyRateForCoinIds(
    coinIds: Array<string | null | undefined>
  ): number | null {
    const seen = new Set<string>();

    for (const rawCoinId of coinIds) {
      if (typeof rawCoinId !== 'string') continue;
      const coinId = rawCoinId.trim();
      if (!coinId || seen.has(coinId)) continue;
      seen.add(coinId);

      const fiatRate = getDisplayCurrencyRate(rates[coinId]?.rates);
      if (fiatRate !== null) return fiatRate;
    }

    return null;
  }

  function getDisplayCurrencyRateForCurrencyLabel(label?: string | null): number | null {
    if (typeof label !== 'string') return null;
    const normalizedLabel = label.trim().toLowerCase();
    if (!normalizedLabel) return null;

    for (const coin of coins) {
      const presentation = resolveCoinPresentation(coin);
      if (
        coin.id.toLowerCase() === normalizedLabel ||
        coin.currencyId.toLowerCase() === normalizedLabel ||
        coin.systemId.toLowerCase() === normalizedLabel ||
        presentation.displayTicker.toLowerCase() === normalizedLabel ||
        presentation.displayName.toLowerCase() === normalizedLabel
      ) {
        return getDisplayCurrencyRateForCoinIds([coin.id, coin.currencyId, coin.mappedTo]);
      }
    }

    return null;
  }

  function resolveCurrencyLabelForDisplay(label?: string | null): string {
    if (typeof label !== 'string') return '';
    const trimmedLabel = label.trim();
    if (!trimmedLabel) return '';
    const normalizedLabel = trimmedLabel.toLowerCase();

    for (const coin of coins) {
      const presentation = resolveCoinPresentation(coin);
      if (
        coin.id.toLowerCase() === normalizedLabel ||
        coin.currencyId.toLowerCase() === normalizedLabel ||
        coin.systemId.toLowerCase() === normalizedLabel ||
        presentation.displayTicker.toLowerCase() === normalizedLabel ||
        presentation.displayName.toLowerCase() === normalizedLabel
      ) {
        return presentation.displayTicker;
      }
    }

    return trimmedLabel;
  }

  function formatFiatEstimate(
    amountValue: string | null | undefined,
    fiatRate: number | null
  ): string {
    const numericAmount = parseNonNegativeAmount(amountValue);
    if (numericAmount === null || fiatRate === null) return '≈ —';
    return `≈ ${formatFiatAmount(numericAmount * fiatRate, i18n.intlLocale, displayCurrency)}`;
  }

  function formatDisplayFiatAmountDynamic(value: number): string {
    const absoluteValue = Math.abs(value);
    const maximumFractionDigits =
      absoluteValue < 0.0001
        ? 8
        : absoluteValue < 0.001
          ? 7
          : absoluteValue < 0.01
            ? 6
            : absoluteValue < 0.1
              ? 4
              : 2;

    return formatFiatAmount(value, i18n.intlLocale, displayCurrency, {
      minimumFractionDigits: 2,
      maximumFractionDigits,
    });
  }

  function parsePrice(value?: string | null): number | null {
    if (!value) return null;
    const parsed = Number(value);
    if (!Number.isFinite(parsed) || parsed <= 0) return null;
    return parsed;
  }

  function parseEstimatedOutput(value?: string | null): number | null {
    if (!value) return null;
    const parsed = Number(value);
    if (!Number.isFinite(parsed) || parsed <= 0) return null;
    return parsed;
  }

  function formatRouteRateValue(value?: string | null): string | null {
    const numeric = parsePrice(value);
    if (numeric === null) return null;
    return numeric.toFixed(8).replace(/\.?0+$/, '');
  }

  function formatDecimalTrimmed(value: number, digits = 8): string {
    return value.toFixed(digits).replace(/\.?0+$/, '');
  }

  function routeScore(
    option: ViaRouteOption,
    amountInput: string,
    estimatedOutputs: Record<string, string> = {}
  ): number | null {
    const estimatedOutput = parseEstimatedOutput(estimatedOutputs[option.id]);
    if (estimatedOutput !== null) {
      return estimatedOutput;
    }

    const price = parsePrice(option.price);
    if (price === null) return null;

    const numericAmount = Number(amountInput);
    if (Number.isFinite(numericAmount) && numericAmount > 0) {
      return price * numericAmount;
    }

    return price;
  }

  function sortViaOptionsByScore(
    options: ViaRouteOption[],
    amountInput: string,
    estimatedOutputs: Record<string, string> = {}
  ): ViaRouteOption[] {
    return [...options].sort((a, b) => {
      const scoreA = routeScore(a, amountInput, estimatedOutputs);
      const scoreB = routeScore(b, amountInput, estimatedOutputs);

      if (scoreA !== null && scoreB !== null && scoreA !== scoreB) {
        return scoreB - scoreA;
      }

      if (scoreA !== null && scoreB === null) return -1;
      if (scoreA === null && scoreB !== null) return 1;

      return viaLexicalKey(a).localeCompare(viaLexicalKey(b));
    });
  }

  function formatEstimatedReceive(option: ViaRouteOption): string | null {
    if (!amountValid) return null;
    const estimatedOutput = parseEstimatedOutput(routeEstimateOutputs[option.id]);
    if (estimatedOutput !== null) {
      return estimatedOutput.toFixed(8);
    }
    const price = parsePrice(option.price);
    if (price === null) return null;
    return (Number(amount) * price).toFixed(8);
  }

  function mapPreflightWarningMessage(warning: PreflightWarning): string {
    if (warning.warningType === 'estimated_fee') {
      return i18n.t('wallet.transfer.warning.finalAmountMayVary');
    }
    return warning.message.trim();
  }

  function mapWalletError(error: unknown): string {
    if (isForcedWalletLockError(error)) {
      return '';
    }

    const errorType = extractWalletErrorType(error);
    const rawMessage = extractWalletErrorMessage(error);
    if (errorType === 'InvalidPreflight') return i18n.t('wallet.transfer.reviewUnavailable');
    if (errorType === 'BridgeNotImplemented')
      return i18n.t('wallet.transfer.error.bridgeNotImplemented');
    if (errorType === 'BridgeRouteInvalid')
      return i18n.t('wallet.transfer.error.bridgeRouteInvalid');
    if (errorType === 'BridgeUnsupportedDestinationCombination') {
      return i18n.t('wallet.transfer.error.bridgeUnsupportedDestinationCombination');
    }
    if (errorType === 'BridgeApprovalFailed')
      return i18n.t('wallet.transfer.error.bridgeApprovalFailed');
    if (errorType === 'BridgeInsufficientEthFeeEnvelope') {
      return i18n.t('wallet.transfer.error.bridgeInsufficientEthFeeEnvelope');
    }
    if (errorType === 'BridgeGasDriftExceeded')
      return i18n.t('wallet.transfer.error.bridgeGasDriftExceeded');
    if (errorType === 'DlightSynchronizerNotReady') {
      return shieldedSyncBlockedHelper || i18n.t('wallet.transfer.privateSyncBlockedUnknown');
    }
    if (errorType === 'DlightProverUnavailable') {
      return i18n.t('wallet.transfer.error.dlightProverUnavailable');
    }
    if (errorType === 'DlightBroadcastUncertain') {
      return i18n.t('wallet.transfer.error.dlightBroadcastUncertain');
    }
    if (errorType === 'EthBroadcastUncertain') {
      return i18n.t('wallet.transfer.error.ethBroadcastUncertain');
    }
    if (errorType === 'EthBroadcastRecovered') {
      return i18n.t('wallet.transfer.error.ethBroadcastRecovered');
    }
    if (errorType === 'EthRecoveryRequired') {
      return i18n.t('wallet.transfer.error.ethRecoveryRequired');
    }
    if (errorType === 'DlightSpendCacheNotReady') {
      return i18n.t('wallet.transfer.error.dlightSpendCacheNotReady');
    }
    if (errorType === 'DirectSendFeeModeUnsupported') {
      return i18n.t('wallet.transfer.error.directSendFeeModeUnsupported');
    }
    if (errorType === 'UnsupportedChannel')
      return i18n.t('wallet.transfer.error.unsupportedChannel');
    if (errorType === 'InvalidAddress') return i18n.t('wallet.transfer.error.invalidAddress');
    if (errorType === 'InsufficientEthForGas')
      return i18n.t('wallet.transfer.error.insufficientEthForGas');
    if (errorType === 'InsufficientFunds') return i18n.t('wallet.transfer.error.insufficientFunds');
    if (errorType === 'NetworkError') return i18n.t('wallet.transfer.error.network');
    if (errorType === 'OperationFailed') {
      if (rawMessage && rawMessage.toLowerCase() !== 'operation failed') return rawMessage;
      return i18n.t('wallet.transfer.error.operationFailed');
    }

    if (rawMessage) return rawMessage;
    if (error instanceof Error && error.message) return error.message;
    return i18n.t('common.unknownError');
  }

  async function getSelectedDlightRuntimeStatus(): Promise<DlightRuntimeStatusResult | null> {
    if (!selectedChannelId || !selectedChannelId.startsWith('dlight_private.')) return null;
    if (!selectedCoin) return null;
    return walletService.getDlightRuntimeStatus(selectedChannelId, selectedCoin.id);
  }

  async function ensureDlightSpendReady(): Promise<boolean> {
    if (!selectedChannelId || !selectedChannelId.startsWith('dlight_private.')) return true;

    try {
      const status = await getSelectedDlightRuntimeStatus();
      if (!status) return true;

      const statusKind = status.statusKind.trim().toLowerCase();
      const spendCacheReady = status.spendCacheReady === true;
      const spendCachePercentText =
        typeof status.spendCachePercent === 'number' && Number.isFinite(status.spendCachePercent)
          ? formatSyncPercent(status.spendCachePercent)
          : null;
      if (statusKind !== 'synced') {
        if (!spendCacheReady) {
          transferError =
            spendCachePercentText !== null
              ? i18n.t('wallet.transfer.privateSpendCacheBlocked', {
                  percent: spendCachePercentText,
                })
              : i18n.t('wallet.transfer.error.dlightSpendCacheNotReady');
          return false;
        }
        const percentText =
          typeof status.percent === 'number' && Number.isFinite(status.percent)
            ? formatSyncPercent(status.percent)
            : null;
        transferError =
          percentText !== null
            ? i18n.t('wallet.transfer.privateSyncBlocked', { percent: percentText })
            : shieldedSyncBlockedHelper || i18n.t('wallet.transfer.privateSyncBlockedUnknown');
        return false;
      }

      if (!spendCacheReady) {
        transferError =
          spendCachePercentText !== null
            ? i18n.t('wallet.transfer.privateSpendCacheBlocked', { percent: spendCachePercentText })
            : i18n.t('wallet.transfer.error.dlightSpendCacheNotReady');
        return false;
      }

      return true;
    } catch (error) {
      console.error('[TransferWizard] dlight runtime status check failed', {
        type: extractWalletErrorType(error),
        message: extractWalletErrorMessage(error),
        error,
      });
      transferError = mapWalletError(error);
      return false;
    }
  }

  function sendStageLabel(stage: TxSendProgressStage | null): string {
    if (stage === 'syncing_spend_state')
      return i18n.t('wallet.transfer.sendStage.syncingSpendState');
    if (stage === 'loading_prover') return i18n.t('wallet.transfer.sendStage.loadingProver');
    if (stage === 'building_proof') return i18n.t('wallet.transfer.sendStage.buildingProof');
    if (stage === 'broadcasting') return i18n.t('wallet.transfer.sendStage.broadcasting');
    return i18n.t('wallet.transfer.sendingNow');
  }

  function sendStageGuidanceLabel(stage: TxSendProgressStage | null): string {
    if (stage === 'syncing_spend_state') {
      return i18n.t('wallet.transfer.sendStageHint.syncingSpendState');
    }
    if (stage === 'loading_prover') return i18n.t('wallet.transfer.sendStageHint.loadingProver');
    if (stage === 'building_proof') return i18n.t('wallet.transfer.sendStageHint.buildingProof');
    if (stage === 'broadcasting') return i18n.t('wallet.transfer.sendStageHint.broadcasting');
    return '';
  }

  function normalizeSummarySecondary(
    primary: string,
    secondary: string | null | undefined
  ): string | undefined {
    const primaryTrimmed = primary.trim();
    const secondaryTrimmed = (secondary ?? '').trim();
    if (!secondaryTrimmed) return undefined;
    if (secondaryTrimmed.toLowerCase() === primaryTrimmed.toLowerCase()) return undefined;
    return secondaryTrimmed;
  }

  function normalizeRouteSummarySecondary(
    primary: string,
    secondary: string | null | undefined
  ): string | undefined {
    const normalized = normalizeSummarySecondary(primary, secondary);
    if (!normalized) return undefined;

    const viaOnly = i18n.t('wallet.transfer.pathVia', { value: primary }).trim();
    if (normalized.toLowerCase() === viaOnly.toLowerCase()) return undefined;

    return normalized;
  }

  function truncateAddressMiddle(value: string, startLength = 10, endLength = 10): string {
    const trimmed = value.trim();
    if (!trimmed) return '';
    const minimumVisibleLength = startLength + endLength + 3;
    if (trimmed.length <= minimumVisibleLength) return trimmed;
    return `${trimmed.slice(0, startLength)}...${trimmed.slice(-endLength)}`;
  }

  function goBack() {
    transferError = '';

    if (currentStep === 'details') {
      onClose();
      return;
    }

    if (currentStep === 'review') {
      clearPreflightState();
      currentStep = 'details';
    }
  }

  function continueFlow() {
    transferError = '';

    if (currentStep === 'details') {
      void runPreflight();
      return;
    }

    if (currentStep === 'review') {
      void broadcast();
    }
  }

  function jumpToStep(step: WizardOperationalStepId) {
    if (transferMutationLocked) return;
    clearPreflightState();
    currentStep = step;
  }

  async function runPreflight(keepReviewOnError = false) {
    if (isShieldedSyncBlocked) {
      transferError =
        shieldedSyncBlockedHelper || i18n.t('wallet.transfer.privateSyncBlockedUnknown');
      return;
    }
    const request = resolvedPreflightRequest;
    if (!request || !recipientValid) return;
    if (!(await ensureDlightSpendReady())) return;

    const requestSignature = preflightRequestSignature(request);
    preflighting = true;
    transferError = '';

    try {
      if (request.kind === 'bridge') {
        const outcome = await runGuardedPreflight({
          guard: preflightRequestGuard,
          signature: requestSignature,
          currentSignature: () => preflightInputSignature,
          execute: () => preflightBridgeTransfer(request.params),
        });
        if (outcome.status === 'stale') return;
        if (outcome.status === 'failed') throw outcome.error;
        bridgePreflightResult = outcome.value;
        simplePreflightResult = null;
      } else {
        const outcome = await runGuardedPreflight({
          guard: preflightRequestGuard,
          signature: requestSignature,
          currentSignature: () => preflightInputSignature,
          execute: () => preflightSend(request.params),
        });
        if (outcome.status === 'stale') return;
        if (outcome.status === 'failed') throw outcome.error;
        simplePreflightResult = outcome.value;
        bridgePreflightResult = null;
      }
      currentStep = 'review';
    } catch (error) {
      console.error('[TransferWizard] preflight failed', {
        type: extractWalletErrorType(error),
        message: extractWalletErrorMessage(error),
        error,
      });
      transferError = mapWalletError(error);
      if (!keepReviewOnError) currentStep = 'details';
    } finally {
      if (requestSignature === preflightInputSignature) {
        preflighting = false;
      }
    }
  }

  async function broadcast() {
    if (sending) return;
    if (isShieldedSyncBlocked) {
      transferError =
        shieldedSyncBlockedHelper || i18n.t('wallet.transfer.privateSyncBlockedUnknown');
      return;
    }
    if (!activePreflight) return;
    if (!(await ensureDlightSpendReady())) return;
    const receiptSnapshot = buildSubmittedTransferSnapshot();
    if (!receiptSnapshot) return;

    sending = true;
    sendStage = null;
    sendStageStartedAt = null;
    sendStageTick = Date.now();
    transferError = '';

    try {
      const result = await sendTransaction({ preflightId: activePreflight.preflightId });
      sendResult = result;
      submittedTransferSnapshot = finalizeSubmittedTransferSnapshot(receiptSnapshot, result);
      if (matchedSavedRecipient) {
        void addressBookService.markAddressBookEndpointUsed(matchedSavedRecipient.endpoint.id);
      }
      await refreshTxHistory();
      currentStep = 'success';
    } catch (error) {
      console.error('[TransferWizard] broadcast failed', {
        type: extractWalletErrorType(error),
        message: extractWalletErrorMessage(error),
        error,
      });
      const errorType = extractWalletErrorType(error);
      // Preflight ids are single-use on the backend; always force a fresh preflight after send failure.
      clearPreflightState();
      currentStep = 'details';
      transferError = mapWalletError(error);
      if (errorType === 'EthRecoveryRequired' || errorType === 'EthBroadcastUncertain') {
        try {
          pendingEthSubmission = await getPendingEthSubmission();
        } catch (recoveryError) {
          transferError = mapWalletError(recoveryError);
        }
      }
    } finally {
      sending = false;
      sendStage = null;
      sendStageStartedAt = null;
    }
  }

  async function recoverPendingEthSubmission() {
    if (!pendingEthSubmission || recoveringEthSubmission) return;
    const reviewedSubmission = pendingEthSubmission;
    recoveringEthSubmission = true;
    transferError = '';
    await runEthRecovery({
      lifetime: ethRecoveryLifetime,
      resume: () => resumePendingEthSubmission(reviewedSubmission.recoveryId),
      reload: getPendingEthSubmission,
      complete: (result) => {
        sendResult = result;
        submittedTransferSnapshot = {
          walletNetwork: reviewedSubmission.context.walletNetwork,
          sourceChannelId: reviewedSubmission.context.channelId,
          sourceSystemId: 'ethereum',
          conversionEnabled: reviewedSubmission.context.assetKind === 'bridge',
          exportSystemId: reviewedSubmission.context.destinationSystemId ?? null,
          ethereumDestination: false,
          sourceAddress: reviewedSubmission.context.fromAddress,
          sourceLabel: i18n.t('wallet.transfer.source.publicAddress'),
          sourceNetworkLabel: 'Ethereum',
          destinationAddress: result.toAddress,
          destinationNetworkLabel:
            reviewedSubmission.context.assetKind === 'bridge' ? 'Verus' : 'Ethereum',
          submittedAmount: result.value,
          submittedTicker:
            resolveCoinPresentationById(reviewedSubmission.context.coinId)?.displayTicker ??
            reviewedSubmission.context.feeCurrency,
          estimatedReceive: null,
        };
        pendingEthSubmission = null;
        const savedRecipient = findMatchingSavedEndpoint(
          addressBookContacts,
          reviewedSubmission.context.destinationKind,
          reviewedSubmission.context.toAddress
        );
        if (savedRecipient) {
          void addressBookService.markAddressBookEndpointUsed(savedRecipient.endpoint.id);
        }
        void refreshTxHistory(
          reviewedSubmission.context.channelId,
          reviewedSubmission.context.coinId
        );
        currentStep = 'success';
      },
      refresh: (review) => {
        pendingEthSubmission = review;
      },
      fail: (_error, recovery) => {
        if (recovery.reloadFailed) {
          transferError = i18n.t('wallet.transfer.ethRecovery.error.refreshFailed');
        } else if (!recovery.review) {
          transferError = i18n.t('wallet.transfer.ethRecovery.error.noPending');
        } else if (
          recovery.review.recoveryId !== reviewedSubmission.recoveryId ||
          recovery.review.stage !== reviewedSubmission.stage
        ) {
          transferError = i18n.t('wallet.transfer.ethRecovery.error.stageAdvanced');
        } else {
          transferError = i18n.t('wallet.transfer.ethRecovery.error.retry');
        }
      },
      settle: () => {
        recoveringEthSubmission = false;
      },
    });
  }

  async function refreshTxHistory(
    channelId = selectedChannelId ?? '',
    coinId = selectedCoin?.id ?? ''
  ) {
    if (!coinId || !channelId) return;
    try {
      const transactions = await walletService.getTransactionHistory(channelId, coinId);
      transactionStore.update((state) => ({
        ...state,
        [channelId]: {
          ...(state[channelId] ?? {}),
          [coinId]: transactions,
        },
      }));
    } catch {
      // Best effort refresh only.
    }
  }

  async function handleDone() {
    if (sendResult?.recoveryId) {
      try {
        await acknowledgePendingEthSubmission(sendResult.recoveryId);
      } catch {
        // Closing is safe: the durable result will be offered again on the next transfer screen.
      }
    }
    onClose();
  }

  function setMaxAmount() {
    amount = selectedBalance;
  }

  function clearAmount() {
    amount = '';
    transferError = '';
  }

  function setTransferMode(mode: 'send' | 'convert') {
    if (transferMutationLocked) return;
    if (mode === 'convert') {
      if (selectedCoin && !sourceSupportsConversion) return;
      conversionEnabled = true;
      manualViaLocked = false;
      return;
    }

    conversionEnabled = false;
    manualViaLocked = false;
    selectedViaOptionId = '';
  }

  function buildSubmittedTransferSnapshot(): SubmittedTransferSnapshot | null {
    if (!selectedCoin || !selectedCoinOption || !selectedChannelId || !activePreflight) return null;
    return {
      walletNetwork,
      sourceChannelId: selectedChannelId,
      sourceSystemId: selectedSourceSystemId,
      conversionEnabled,
      exportSystemId: activeExportSystemId,
      ethereumDestination: activeTargetOption?.ethDestination ?? false,
      sourceAddress: activePreflight.fromAddress || selectedSourceAddress,
      sourceLabel: i18n.t(
        selectedCoinOption.sourceKind === 'private'
          ? 'wallet.transfer.source.privateAddress'
          : 'wallet.transfer.source.publicAddress'
      ),
      sourceNetworkLabel: sourceNetworkDisplayName,
      destinationAddress: activePreflight.toAddress,
      destinationNetworkLabel: reviewDestinationNetworkValue || sourceNetworkDisplayName,
      submittedAmount: activePreflight.value,
      submittedTicker: selectedCoinPresentation?.displayTicker ?? selectedCoin.id,
      estimatedReceive:
        conversionEnabled && reviewEstimatedConversionValue
          ? `${reviewEstimatedConversionValue} ${selectedReceiveCompactTicker}`
          : null,
    };
  }

  function openDirectSendFeeSheet() {
    if (
      sending ||
      recoveringEthSubmission ||
      !directSendFeeEligible ||
      !authoritativeDirectSendFeeMode
    ) {
      return;
    }
    pendingDirectSendFeeMode = directSendFeeDraftForOpen(
      authoritativeDirectSendFeeMode,
      directSendFeeMode
    );
    showFeeSheet = true;
  }

  async function applyDirectSendFeeMode() {
    if (sending || recoveringEthSubmission) return;
    if (
      !directSendFeeEligible ||
      !directSendFeeSelectionNeedsRefresh(authoritativeDirectSendFeeMode, pendingDirectSendFeeMode)
    ) {
      showFeeSheet = false;
      return;
    }
    directSendFeeMode = pendingDirectSendFeeMode;
    clearPreflightState();
    showFeeSheet = false;
    await tick();
    await runPreflight(true);
  }

  function selectSourceCoin(coinId: string, channelId: string) {
    if (sending || recoveringEthSubmission) return;
    selectedCoinId = coinId;
    selectedSourceChannelId = channelId;
    selectedSendExportSystemId = null;
    sourceCoinManuallyChosen = true;
    showSourceAssetSheet = false;
    transferError = '';
  }

  function selectReceiveAsset(optionId: string) {
    const selected = receiveAssetOptions.find((option) => option.id === optionId);
    if (!selected) return;

    if (selected.isGrouped && selected.networkOptions?.length) {
      pendingGroupedReceiveOption = selected;
      pendingTargetOption = null;
      showReceiveAssetSheet = false;
      showExportSheet = false;
      showNetworkSheet = true;
      transferError = '';
      return;
    }

    pendingGroupedReceiveOption = null;
    beginReceiveSelection(selected);
  }

  function selectReceiveNetworkOption(optionId: string) {
    const selected = pendingGroupedReceiveOption?.networkOptions?.find(
      (option) => option.id === optionId
    );
    if (!selected) return;
    const selectedExportSystemId = selected.hasOnChainPath
      ? null
      : (selected.exportOptions[0]?.exportTo ?? null);
    finalizeReceiveSelection(selected, selectedExportSystemId);
  }

  function selectExportOption(exportSystemId: string) {
    if (!pendingTargetOption) return;
    if (exportSheetMode === 'send') {
      finalizeSendExportSelection(exportSystemId);
      return;
    }
    finalizeReceiveSelection(pendingTargetOption, exportSystemId);
  }

  function selectSameNetworkOption() {
    if (!pendingTargetOption) return;
    if (exportSheetMode === 'send') {
      finalizeSendExportSelection(null);
      return;
    }
    finalizeReceiveSelection(pendingTargetOption, null);
  }

  function selectViaOption(viaOptionId: string) {
    selectedViaOptionId = viaOptionId;
    manualViaLocked = true;
    showViaSheet = false;
    transferError = '';
  }
</script>

<WalletTransferStepperShell
  currentStep={stepNumber}
  totalSteps={OPERATIONAL_STEPS.length}
  steps={stepperSteps}
  showProgress={false}
  {onClose}
  closeDisabled={loadingTargets || preflighting || sending || recoveringEthSubmission}
  dirty={isDirty}
  showAside={false}
  mobileAsideLabel={i18n.t('wallet.transfer.viewSummary')}
  mobileAsideTitle={i18n.t('wallet.transfer.summary.title')}
>
  {#snippet aside()}
    <TransferSummaryRail rows={summaryRows} warnings={warningsSummary} class="h-full" />
  {/snippet}

  {#snippet footer()}
    {#if pendingEthSubmission}
      <div class="flex justify-end">
        <Button variant="secondary" onclick={onClose} disabled={recoveringEthSubmission}>
          {i18n.t('common.cancel')}
        </Button>
      </div>
    {:else if currentStep === 'success'}
      <div class="flex justify-end">
        <Button class="w-full sm:w-[368px]" onclick={handleDone}>
          {i18n.t('common.done')}
        </Button>
      </div>
    {:else}
      <div class="flex items-center justify-between gap-3">
        <Button variant="secondary" onclick={goBack} disabled={isBusy}>
          {currentStep === 'details' ? i18n.t('common.cancel') : i18n.t('common.back')}
        </Button>
        {#if currentStep === 'review'}
          <div class="flex items-center gap-3">
            {#if requiresUnsavedRecipientAck}
              <div class="flex items-center gap-1.5">
                <Checkbox
                  id="review-unsaved-recipient-footer"
                  bind:checked={unsavedRecipientConfirmed}
                />
                <Label
                  for="review-unsaved-recipient-footer"
                  class="text-[11px] whitespace-nowrap text-muted-foreground"
                >
                  {i18n.t('wallet.transfer.review.unsavedConfirmShort')}
                </Label>
              </div>
            {/if}
            <Button class="w-[min(368px,45vw)]" onclick={continueFlow} disabled={primaryDisabled}>
              {primaryLabel}
            </Button>
          </div>
        {:else}
          <Button class="w-[min(368px,45vw)]" onclick={continueFlow} disabled={primaryDisabled}>
            {primaryLabel}
          </Button>
        {/if}
      </div>
    {/if}
  {/snippet}

  {#snippet footerAside()}
    {#if pendingEthSubmission}
      <div class="hidden w-full justify-end md:flex">
        <Button variant="secondary" onclick={onClose} disabled={recoveringEthSubmission}>
          {i18n.t('common.cancel')}
        </Button>
      </div>
    {:else if currentStep === 'success'}
      <div class="hidden w-full justify-end md:flex">
        <Button onclick={handleDone}>
          {i18n.t('common.done')}
        </Button>
      </div>
    {:else}
      <div class="hidden w-full justify-end md:flex">
        <Button onclick={continueFlow} disabled={primaryDisabled}>
          {primaryLabel}
        </Button>
      </div>
    {/if}
  {/snippet}

  <div
    class={currentStep === 'details'
      ? 'space-y-4'
      : currentStep === 'review'
        ? 'space-y-3'
        : 'space-y-5'}
  >
    {#if transferError}
      <div
        class="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive"
      >
        {transferError}
      </div>
    {/if}

    {#if pendingEthSubmission}
      <EthRecoveryReviewCard
        review={pendingEthSubmission}
        recovering={recoveringEthSubmission}
        onrecover={recoverPendingEthSubmission}
      />
    {:else}
      {#if !transferError && sendStageGuidance}
        <div
          class="rounded-md border border-amber-300/70 bg-amber-50 px-3 py-2 text-sm text-amber-900 dark:border-amber-500/35 dark:bg-amber-500/12 dark:text-amber-200"
        >
          {sendStageGuidance}
        </div>
      {/if}
      {#if shieldedSyncBlockedHelper}
        <div
          class="rounded-md border border-amber-300/70 bg-amber-50 px-3 py-2 text-sm text-amber-900 dark:border-amber-500/35 dark:bg-amber-500/12 dark:text-amber-200"
        >
          {shieldedSyncBlockedHelper}
        </div>
      {/if}

      {#if currentStep === 'details'}
        <Card.Root class="border-0 bg-transparent py-0 shadow-none">
          <Card.Content class="space-y-3 px-0">
            <div class="-mt-1 flex w-full justify-start border-b border-border/70">
              <Tabs.Root value={conversionEnabled ? 'convert' : 'send'} class="w-full">
                <Tabs.List class="h-9 w-full justify-start rounded-none bg-transparent p-0">
                  <Tabs.Trigger
                    value="send"
                    class="h-9 min-w-[6.5rem] rounded-none border-b-2 border-transparent px-1 text-[22px] font-normal shadow-none data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:font-medium data-[state=active]:shadow-none"
                    onclick={() => setTransferMode('send')}
                  >
                    {i18n.t('wallet.overview.send')}
                  </Tabs.Trigger>
                  <Tabs.Trigger
                    value="convert"
                    class="h-9 min-w-[6.5rem] rounded-none border-b-2 border-transparent px-1 text-[22px] font-normal shadow-none data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:font-medium data-[state=active]:shadow-none"
                    disabled={!!selectedCoin && !sourceSupportsConversion}
                    onclick={() => setTransferMode('convert')}
                  >
                    {i18n.t('wallet.overview.convert')}
                  </Tabs.Trigger>
                </Tabs.List>
              </Tabs.Root>
            </div>

            <div
              class={`relative mx-auto w-full max-w-[840px] gap-x-6 gap-y-3 ${conversionEnabled ? 'grid md:grid-cols-2' : 'space-y-3.5'}`}
            >
              {#if selectedCoinOption}
                <div class={`flex items-center gap-4 ${conversionEnabled ? 'md:col-span-2' : ''}`}>
                  <span class="text-sm text-muted-foreground">
                    {i18n.t('wallet.transfer.source.sendFrom')}
                  </span>
                  <button
                    type="button"
                    class="flex h-10 max-w-[34rem] min-w-0 items-center gap-4 rounded-lg border border-border/70 bg-background px-3 text-left outline-none hover:bg-muted/35 focus-visible:ring-2 focus-visible:ring-ring/60"
                    onclick={() => (showSourceAssetSheet = true)}
                  >
                    <span class="shrink-0 text-sm font-medium">
                      {i18n.t(
                        selectedCoinOption.sourceKind === 'private'
                          ? 'wallet.transfer.source.privateAddress'
                          : 'wallet.transfer.source.publicAddress'
                      )}
                    </span>
                    {#if selectedSourceAddress}
                      <span class="identifier-text truncate text-xs text-muted-foreground">
                        {truncateAddressMiddle(selectedSourceAddress, 12, 12)}
                      </span>
                    {/if}
                    <ChevronRightIcon class="size-4 text-muted-foreground" />
                  </button>
                </div>
              {/if}
              <div class="space-y-1.5">
                <p class="px-1 text-sm text-muted-foreground">
                  {i18n.t('wallet.transfer.youSend')}
                </p>
                <section class="rounded-xl border border-border/70 bg-background p-4">
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0 flex-1">
                      <Label for="transfer-amount" class="sr-only"
                        >{i18n.t('wallet.transfer.amountLabel')}</Label
                      >
                      <div class="relative">
                        <Input
                          bind:ref={amountInputEl}
                          id="transfer-amount"
                          type="text"
                          inputmode="decimal"
                          placeholder={i18n.t('wallet.transfer.amountPlaceholder')}
                          bind:value={amount}
                          class="h-auto min-h-0 border-0 !bg-transparent px-0 py-0 pr-8 text-[clamp(26px,4vw,34px)] leading-10 font-medium tracking-[-0.035em] text-foreground placeholder:text-foreground focus-visible:ring-0 md:text-[34px] dark:!bg-transparent dark:placeholder:text-foreground"
                        />
                        {#if amount.trim()}
                          <button
                            type="button"
                            class="absolute top-1/2 right-0 -translate-y-1/2 rounded-sm p-1 text-muted-foreground transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:outline-none"
                            onclick={clearAmount}
                            aria-label={i18n.t('wallet.transfer.amountClear')}
                            title={i18n.t('wallet.transfer.amountClear')}
                          >
                            <XIcon class="size-3.5" />
                          </button>
                        {/if}
                      </div>
                      <p class="mt-1 px-0.5 text-xs text-muted-foreground tabular-nums">
                        {sourceAmountFiatDisplay}
                      </p>
                    </div>

                    <div class="flex max-w-[72%] shrink-0 flex-col items-end gap-1">
                      <Button
                        variant={showChooseCurrencyCallToAction ? 'default' : 'ghost'}
                        class="max-w-full
                        {showChooseCurrencyCallToAction
                          ? 'h-9 justify-center self-center rounded-md px-4 text-sm font-semibold'
                          : 'h-10 justify-start gap-2 rounded-lg border border-border/70 bg-background px-3 hover:bg-muted/35 dark:bg-background dark:hover:bg-muted/35'}"
                        onclick={() => (showSourceAssetSheet = true)}
                      >
                        {#if showChooseCurrencyCallToAction}
                          <span class="truncate text-sm font-semibold"
                            >{i18n.t('wallet.transfer.chooseCurrency')}</span
                          >
                        {:else}
                          {#if selectedCoin}
                            <CoinIcon
                              coinId={selectedCoin.id}
                              coinName={selectedCoinPresentation?.displayName}
                              size={24}
                              decorative={true}
                            />
                          {/if}
                          <span class="truncate text-[15px] font-medium">
                            {selectedCoinPresentation?.displayTicker ||
                              i18n.t('wallet.transfer.sourceAsset')}
                          </span>
                          <ChevronRightIcon class="ml-auto size-4 shrink-0 text-foreground/45" />
                        {/if}
                      </Button>
                    </div>
                  </div>

                  <div class="mt-2 flex min-h-6 items-center justify-between gap-2">
                    <div class="min-w-0 flex-1">
                      {#if !amountValid && amount.trim()}
                        <p class="truncate text-xs text-destructive">
                          {i18n.t('wallet.transfer.amountInvalid')}
                        </p>
                      {/if}
                    </div>

                    {#if selectedCoinOption}
                      <div class="flex shrink-0 items-center gap-2">
                        <p class="truncate text-xs text-muted-foreground">
                          {i18n.t('wallet.transfer.availableBalance', {
                            value: formatSheetBalance(selectedBalance),
                            ticker:
                              selectedCoinPresentation?.displayTicker ?? selectedCoin?.id ?? '',
                          })}
                        </p>
                        <Button
                          variant="secondary"
                          size="sm"
                          class="h-6 rounded-full px-2.5 text-[11px]"
                          onclick={setMaxAmount}
                        >
                          {i18n.t('wallet.transfer.max')}
                        </Button>
                      </div>
                    {/if}
                  </div>
                </section>
              </div>

              {#if conversionEnabled}
                <div class="space-y-1.5">
                  <p class="px-1 text-sm text-muted-foreground">
                    {i18n.t('wallet.transfer.summary.estimatedReceive')}
                  </p>
                  <section class="rounded-xl bg-muted/40 p-4 dark:bg-muted/30">
                    <div class="flex items-start justify-between gap-3">
                      <div class="min-w-0 flex-1">
                        <p
                          class="text-[34px] leading-10 font-medium tracking-[-0.035em] text-muted-foreground"
                        >
                          {estimatedConversionValue || '0'}
                        </p>
                        <p class="mt-1 px-0.5 text-xs text-muted-foreground tabular-nums">
                          {receiveAmountFiatDisplay}
                        </p>
                      </div>
                      <Button
                        variant={selectedReceiveAssetOption ? 'ghost' : 'default'}
                        class="max-w-[72%] shrink-0
                        {selectedReceiveAssetOption
                          ? 'h-10 justify-start gap-2 rounded-lg border border-border/70 bg-background px-3 hover:bg-muted/35 dark:bg-background dark:hover:bg-muted/35'
                          : 'h-9 justify-center rounded-md px-4 text-sm font-semibold'}"
                        disabled={!receiveAssetSelectionEnabled}
                        onclick={() => {
                          if (!receiveAssetSelectionEnabled) return;
                          showNetworkSheet = false;
                          showExportSheet = false;
                          pendingGroupedReceiveOption = null;
                          pendingTargetOption = null;
                          showReceiveAssetSheet = true;
                        }}
                      >
                        {#if selectedReceiveAssetOption}
                          {@const receiveDisplay = getReceiveOptionDisplay(
                            selectedReceiveAssetOption,
                            selectedReceiveLabel,
                            selectedExportSystemId ?? activeExportSystemId
                          )}
                          <CoinIcon
                            coinId={selectedReceiveAssetOption.destinationId}
                            coinName={receiveDisplay.primary}
                            size={24}
                            decorative={true}
                          />
                          <span class="truncate text-[15px] font-medium">
                            {selectedReceiveCompactTicker || receiveDisplay.primary}
                          </span>
                          <ChevronRightIcon class="ml-auto size-4 shrink-0 text-foreground/45" />
                        {:else}
                          <span class="truncate text-sm font-semibold"
                            >{i18n.t('wallet.transfer.receiveAsset')}</span
                          >
                        {/if}
                      </Button>
                    </div>

                    {#if targetsError}
                      <p class="mt-2 text-xs text-destructive">{targetsError}</p>
                    {/if}
                  </section>
                </div>
              {/if}
            </div>

            {#if conversionEnabled && selectedReceiveAssetOption && amountValid}
              <div class="mx-auto w-full max-w-[840px] border-t border-border/70 pt-4">
                <button
                  type="button"
                  class="flex w-full items-center justify-between gap-6 rounded-lg px-1 py-2 text-left outline-none hover:bg-muted/35 focus-visible:ring-2 focus-visible:ring-ring/60"
                  onclick={() => (showViaSheet = true)}
                >
                  <span>
                    <span class="block text-sm font-medium text-muted-foreground">
                      {i18n.t('wallet.transfer.conversionRoute')}
                    </span>
                    <span class="mt-0.5 block text-xs text-muted-foreground tabular-nums">
                      {activeConvertRouteRateValueText}
                    </span>
                  </span>
                  <span class="flex items-center gap-2 text-sm font-semibold">
                    {activeConvertRoute
                      ? getViaOptionLabel(activeConvertRoute)
                      : i18n.t('wallet.transfer.viaBest')}
                    <ChevronRightIcon class="size-4 shrink-0 text-muted-foreground" />
                  </span>
                </button>
              </div>
            {/if}

            {#if showConvertUnavailable}
              <div
                class="rounded-md border border-border/60 px-3 py-2 text-sm text-muted-foreground"
              >
                {convertUnavailableMessage}
              </div>
            {/if}

            {#if !loadingTargets && conversionEnabled && rawReceiveAssetSections.allOptions.length === 0 && sourceSupportsConversion}
              <p class="text-sm text-muted-foreground">{i18n.t('wallet.transfer.noRoutes')}</p>
            {/if}

            {#if positiveSendableCoinOptions.length === 0}
              <p class="text-sm text-muted-foreground">{i18n.t('wallet.transfer.noAssets')}</p>
            {/if}
          </Card.Content>
        </Card.Root>
      {/if}

      {#if currentStep === 'details'}
        <div class="mx-auto w-full max-w-[840px]">
          <Card.Root class="w-full border-0 bg-transparent py-0 shadow-none">
            <Card.Content class="px-0">
              <div class="grid gap-4 md:grid-cols-[248px_minmax(0,1fr)]">
                <div class="space-y-2">
                  <p class="text-sm font-medium text-muted-foreground">
                    {i18n.t('wallet.transfer.summary.destinationNetwork')}
                  </p>
                  {#if !conversionEnabled && sendCrossChainAvailable}
                    <button
                      type="button"
                      class="flex h-10 w-full items-center justify-between rounded-lg border border-border/70 bg-background px-3 text-left text-sm font-medium transition-colors outline-none hover:bg-muted/50 focus-visible:ring-2 focus-visible:ring-ring/60"
                      onclick={openSendDestinationNetworkSheet}
                    >
                      <span class="truncate">
                        {reviewDestinationNetworkValue || sourceNetworkDisplayName}
                      </span>
                      <ChevronRightIcon class="size-4 shrink-0 text-muted-foreground" />
                    </button>
                  {:else}
                    <div
                      class="flex h-10 items-center rounded-lg border border-border/70 bg-muted/25 px-3 text-sm font-medium"
                    >
                      <span class="truncate">
                        {reviewDestinationNetworkValue || sourceNetworkDisplayName}
                      </span>
                    </div>
                  {/if}
                </div>

                <div class="flex min-w-0 flex-col gap-2 text-left">
                  <p class="text-sm font-medium text-muted-foreground">
                    {i18n.t('wallet.transfer.recipientLabel')}
                  </p>
                  <Label for="transfer-recipient" class="sr-only"
                    >{i18n.t('wallet.transfer.recipientLabel')}</Label
                  >
                  <div class="relative w-full">
                    <Input
                      id="transfer-recipient"
                      class="identifier-text h-10 rounded-lg bg-muted/85 px-4 pr-14 text-left text-sm font-medium dark:bg-muted/55"
                      bind:value={destinationAddress}
                      placeholder={recipientInputCopy.placeholder}
                    />
                    <button
                      type="button"
                      class="absolute top-1/2 right-2 -translate-y-1/2 rounded px-1.5 py-0.5 text-[10px] leading-none font-semibold text-muted-foreground transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:outline-none"
                      onclick={pasteRecipientAddress}
                      aria-label={i18n.t('wallet.transfer.recipient.paste')}
                      title={i18n.t('wallet.transfer.recipient.paste')}
                    >
                      {i18n.t('wallet.transfer.recipient.paste')}
                    </button>
                  </div>
                  {#if destinationAddress.trim() && !recipientValid}
                    <p class="text-xs text-destructive">
                      {i18n.t('wallet.transfer.recipientInvalid')}
                    </p>
                  {/if}
                  {#if showDlightMemoField}
                    <div class="w-full text-left">
                      <Label for="transfer-memo" class="text-xs font-medium">
                        {i18n.t('wallet.send.memoLabel')}
                      </Label>
                      <Input
                        id="transfer-memo"
                        class="mt-1 h-10 rounded-xl bg-muted/85 px-3 text-sm dark:bg-muted/55"
                        bind:value={memo}
                        maxlength={512}
                        placeholder={i18n.t('wallet.send.memoPlaceholder')}
                      />
                      <p class="mt-1 text-[11px] text-muted-foreground">
                        {i18n.t('wallet.transfer.memoHintDlight')}
                      </p>
                    </div>
                  {/if}

                  <div class="mt-1 flex items-center justify-start gap-6">
                    <InlineTextActionButton onclick={() => (showAddressBookSheet = true)}>
                      {i18n.t('wallet.transfer.addressBook.open')}
                    </InlineTextActionButton>
                    <InlineTextActionButton
                      onclick={selectSelfRecipient}
                      disabled={!selfDestinationAddress}
                    >
                      {i18n.t('wallet.transfer.recipient.sendToSelf')}
                    </InlineTextActionButton>
                  </div>

                  <div class="min-h-5">
                    {#if hasRecipientSimilarityWarning}
                      <p class="text-xs text-amber-700 dark:text-amber-300">
                        {i18n.t('wallet.transfer.addressBook.similarWarning')}
                      </p>
                    {/if}
                  </div>
                </div>
              </div>
            </Card.Content>
          </Card.Root>
        </div>
      {/if}

      {#if currentStep === 'review'}
        <Card.Root class="border-0 bg-transparent py-0 shadow-none">
          <Card.Content class="space-y-3 px-0 pt-0">
            {#if activePreflight}
              <div class="mx-auto w-full max-w-[840px] space-y-5">
                <div class="flex items-center justify-between gap-4">
                  <h2 class="text-2xl font-semibold tracking-tight">
                    {i18n.t(
                      conversionEnabled
                        ? 'wallet.transfer.review.conversionTitle'
                        : 'wallet.transfer.review.sendTitle'
                    )}
                  </h2>
                  <Button
                    variant="ghost"
                    size="sm"
                    class="gap-1.5"
                    onclick={() => jumpToStep('details')}
                    disabled={transferMutationLocked}
                  >
                    <PencilIcon class="size-3.5" />
                    {i18n.t('wallet.transfer.review.changeDetails')}
                  </Button>
                </div>
                <div class={`grid gap-4 ${conversionEnabled ? 'sm:grid-cols-2' : ''}`}>
                  <div
                    class="min-h-24 space-y-1 rounded-xl bg-muted/40 p-4 text-left dark:bg-muted/35"
                  >
                    <p class="text-sm font-medium text-muted-foreground">
                      {i18n.t('wallet.transfer.review.sending')}
                    </p>
                    <div class="flex items-center gap-2">
                      {#if selectedCoin}
                        <CoinIcon
                          coinId={selectedCoin.id}
                          coinName={selectedCoinPresentation?.displayName}
                          size={20}
                          decorative={true}
                        />
                      {/if}
                      <p class="min-w-0 truncate text-[1.35rem] font-semibold tabular-nums">
                        {reviewSendingValue || i18n.t('wallet.transfer.summary.notSet')}
                      </p>
                    </div>
                    {#if reviewSourceAmountFiatDisplay && reviewSourceAmountFiatDisplay !== '≈ —'}
                      <p class="text-[11px] text-muted-foreground tabular-nums">
                        {reviewSourceAmountFiatDisplay}
                      </p>
                    {/if}
                  </div>

                  {#if conversionEnabled}
                    <div
                      class="min-h-24 space-y-1 rounded-xl bg-muted/40 p-4 text-left dark:bg-muted/35"
                    >
                      <p class="text-sm font-medium text-muted-foreground">
                        {i18n.t('wallet.transfer.review.receiving')}
                      </p>
                      <div class="flex items-center gap-2">
                        {#if selectedReceiveAssetOption?.destinationId}
                          <CoinIcon
                            coinId={selectedReceiveAssetOption.destinationId}
                            coinName={resolveReceiveLabel(
                              selectedReceiveAssetOption,
                              activeExportSystemId
                            )}
                            size={20}
                            decorative={true}
                          />
                        {/if}
                        <p class="min-w-0 truncate text-[1.35rem] font-semibold tabular-nums">
                          {reviewReceivingValue || i18n.t('wallet.transfer.summary.notSet')}
                        </p>
                      </div>
                      {#if reviewReceiveAmountFiatDisplay && reviewReceiveAmountFiatDisplay !== '≈ —'}
                        <p class="text-[11px] text-muted-foreground tabular-nums">
                          {reviewReceiveAmountFiatDisplay}
                        </p>
                      {/if}
                    </div>
                  {/if}
                </div>

                <div class="grid gap-8 py-1 sm:grid-cols-2">
                  <div class="min-w-0 space-y-1">
                    <p class="text-sm text-muted-foreground">
                      {i18n.t('wallet.transfer.summary.from')}
                    </p>
                    <p class="truncate text-sm font-semibold">
                      {i18n.t(
                        selectedCoinOption?.sourceKind === 'private'
                          ? 'wallet.transfer.source.privateAddress'
                          : 'wallet.transfer.source.publicAddress'
                      )} · {sourceNetworkDisplayName}
                    </p>
                    <p class="identifier-text truncate text-xs text-muted-foreground">
                      {truncateAddressMiddle(selectedSourceAddress, 12, 12)}
                    </p>
                  </div>
                  <div class="min-w-0 space-y-1">
                    <p class="text-sm text-muted-foreground">
                      {i18n.t('wallet.transfer.summary.to')} · {reviewDestinationNetworkValue ||
                        sourceNetworkDisplayName}
                    </p>
                    {#if reviewRecipientName}
                      <p class="truncate text-sm font-semibold">{reviewRecipientName}</p>
                    {/if}
                    <p class="identifier-text truncate text-xs text-muted-foreground">
                      {reviewRecipientAddressWithSelf || i18n.t('wallet.transfer.summary.notSet')}
                    </p>
                    {#if !matchedSavedRecipient && !isSelfRecipient}
                      <p class="text-xs text-amber-700 dark:text-amber-300">
                        {i18n.t('wallet.transfer.review.unsavedRecipient')}
                      </p>
                    {/if}
                  </div>
                </div>

                <div class="space-y-1 border-t border-border/70 pt-4">
                  {#if conversionEnabled}
                    <div class="px-1 py-2">
                      <div class="flex items-center justify-between gap-3">
                        <p class="text-sm text-muted-foreground">
                          {i18n.t('wallet.transfer.summary.route')}
                        </p>
                        <p class="text-right text-sm font-medium">
                          {reviewRouteValue || i18n.t('wallet.transfer.viaBest')}
                        </p>
                      </div>
                    </div>
                  {/if}

                  <div class="space-y-2 px-1 py-2">
                    {#if directSendFeeEligible && authoritativeDirectSendFeeMode}
                      <button
                        type="button"
                        class="-mx-1 flex w-[calc(100%+0.5rem)] items-center justify-between gap-3 rounded-lg px-1 py-1 text-left transition-colors outline-none hover:bg-muted/60 focus-visible:ring-2 focus-visible:ring-ring/60"
                        onclick={openDirectSendFeeSheet}
                        disabled={transferMutationLocked}
                      >
                        <span class="min-w-0">
                          <span class="block text-sm text-muted-foreground">
                            {selectedChannelPrefix === 'eth' || selectedChannelPrefix === 'erc20'
                              ? i18n.t('wallet.transfer.fee.maximumNetworkFee')
                              : i18n.t('wallet.transfer.summary.networkFee')}
                          </span>
                          <span class="block text-xs font-medium">
                            {i18n.t(`wallet.transfer.fee.${authoritativeDirectSendFeeMode}`)}
                          </span>
                        </span>
                        <span class="flex items-center gap-2">
                          <span class="text-sm font-medium tabular-nums">
                            {reviewNetworkFeeValue || i18n.t('wallet.transfer.summary.notSet')}
                          </span>
                          <ChevronRightIcon class="size-4 text-muted-foreground" />
                        </span>
                      </button>
                      {#if simplePreflightResult?.feeRateSatsPerVbyte}
                        <div class="flex items-center justify-between gap-3 pt-1">
                          <p class="text-[11px] text-muted-foreground">
                            {i18n.t('wallet.transfer.fee.feeRate')}
                          </p>
                          <p class="text-[11px] font-medium tabular-nums">
                            {simplePreflightResult.feeRateSatsPerVbyte} sat/vB
                          </p>
                        </div>
                      {/if}
                      <div
                        class="flex items-center justify-between gap-3 border-t border-border/60 pt-2"
                      >
                        <p class="text-sm text-muted-foreground">
                          {i18n.t(
                            selectedChannelPrefix === 'eth' || selectedChannelPrefix === 'erc20'
                              ? 'wallet.transfer.fee.maximumTotalDebited'
                              : 'wallet.transfer.fee.totalDebited'
                          )}
                        </p>
                        <p class="text-sm font-semibold tabular-nums">
                          {reviewTotalDebitedValue}
                        </p>
                      </div>
                    {:else}
                      <div class="flex items-start justify-between gap-3">
                        <p class="text-[11px] text-muted-foreground">
                          {i18n.t('wallet.transfer.summary.networkFee')}
                        </p>
                        <p class="text-[13px] font-medium tabular-nums">
                          {reviewNetworkFeeValue || i18n.t('wallet.transfer.summary.notSet')}
                        </p>
                      </div>
                    {/if}

                    {#if activeExportSystemId && isEthereumExport(activeExportSystemId)}
                      <div class="flex items-start justify-between gap-3">
                        <p class="text-[11px] text-muted-foreground">
                          {i18n.t('wallet.transfer.summary.bridgeFeeEstimate')}
                        </p>
                        <div class="min-w-0 text-right">
                          <p class="text-[13px] font-medium tabular-nums">
                            {bridgeFeeParityEligible
                              ? bridgeFeeEstimateValue
                              : i18n.t('wallet.transfer.bridgeFeeUnavailable')}
                          </p>
                          {#if bridgeFeeParityEligible && bridgeFeeEstimateSecondary && !hideBridgeFeeEstimateFiatBreakdown}
                            <p
                              class={`mt-0.5 text-[11px] ${bridgeFeeInsufficient ? 'text-destructive' : 'text-muted-foreground'} tabular-nums`}
                            >
                              {bridgeFeeEstimateSecondary}
                            </p>
                          {/if}
                        </div>
                      </div>
                    {/if}

                    {#if conversionEnabled && conversionFeeInfo}
                      <div class="flex items-start justify-between gap-3">
                        <p class="text-[11px] text-muted-foreground">
                          {i18n.t('wallet.transfer.summary.conversionFeeWithRate', {
                            rate: conversionFeeInfo.percentage,
                          })}
                        </p>
                        <p class="text-[13px] font-medium tabular-nums">
                          {conversionFeeInfo.amount}
                          {conversionFeeInfo.currency}
                        </p>
                      </div>
                    {/if}

                    {#if showReviewTotalFeesFiat && !directSendFeeEligible}
                      <p class="text-right text-[11px] text-muted-foreground tabular-nums">
                        {reviewTotalFeesFiat}
                      </p>
                    {/if}
                  </div>

                  {#if reviewWarnings.length > 0}
                    <div class="rounded-lg border border-amber-500/35 bg-amber-500/10 px-2.5 py-2">
                      <p class="text-[11px] font-semibold text-amber-700 dark:text-amber-300">
                        {i18n.t('wallet.transfer.warningsTitle')}
                      </p>
                      <div class="mt-1 space-y-1">
                        {#each reviewWarnings as warning}
                          <p class="text-[11px] leading-snug text-amber-700 dark:text-amber-300">
                            {warning}
                          </p>
                        {/each}
                      </div>
                    </div>
                  {/if}
                </div>
              </div>
            {:else}
              <div class="space-y-3">
                <p class="text-sm text-muted-foreground">
                  {i18n.t('wallet.transfer.reviewUnavailable')}
                </p>
                <Button
                  variant="outline"
                  onclick={() => void runPreflight()}
                  disabled={preflighting || !recipientValid}
                >
                  {i18n.t('wallet.transfer.review.refresh')}
                </Button>
              </div>
            {/if}
          </Card.Content>
        </Card.Root>
      {/if}

      {#if currentStep === 'success'}
        <Card.Root class="border-0 bg-transparent py-0 shadow-none">
          <Card.Content class="px-0 py-6">
            <div class="mx-auto w-full max-w-[760px] space-y-6">
              {#if sendResult && submittedTransferSnapshot && submittedReceiptCopy}
                {@const submittedResult = sendResult}
                <div class="space-y-1 text-left">
                  <h2 class="text-[1.75rem] leading-tight font-semibold tracking-tight">
                    {submittedReceiptCopy.title}
                  </h2>
                  <p class="max-w-xl text-sm leading-5 text-muted-foreground">
                    {submittedReceiptCopy.timing}
                  </p>
                </div>

                <div
                  class={`grid gap-4 ${submittedTransferSnapshot.estimatedReceive ? 'sm:grid-cols-2' : ''}`}
                >
                  <div class="rounded-2xl bg-muted/35 p-4 dark:bg-muted/40">
                    <p class="text-xs text-muted-foreground">
                      {i18n.t('wallet.transfer.receipt.submittedAmount')}
                    </p>
                    <p class="mt-1 text-xl font-semibold tabular-nums">
                      {formatAmountForReviewDisplay(
                        submittedTransferSnapshot.submittedAmount,
                        MAX_TRANSFER_AMOUNT_FRACTION_DIGITS
                      )}
                      <span class="ml-1">{submittedTransferSnapshot.submittedTicker}</span>
                    </p>
                  </div>

                  {#if submittedTransferSnapshot.estimatedReceive}
                    <div class="rounded-2xl bg-muted/35 p-4 dark:bg-muted/40">
                      <p class="text-xs text-muted-foreground">
                        {i18n.t('wallet.transfer.summary.estimatedReceive')}
                      </p>
                      <p class="mt-1 text-xl font-semibold tabular-nums">
                        {submittedTransferSnapshot.estimatedReceive}
                      </p>
                    </div>
                  {/if}
                </div>

                <div class="grid gap-8 sm:grid-cols-2">
                  <div class="min-w-0 space-y-1">
                    <p class="text-sm text-muted-foreground">
                      {i18n.t('wallet.transfer.receipt.from')}
                    </p>
                    <p class="truncate text-sm font-semibold">
                      {submittedTransferSnapshot.sourceLabel} · {submittedTransferSnapshot.sourceNetworkLabel}
                    </p>
                    <p class="identifier-text truncate text-xs text-muted-foreground">
                      {truncateAddressMiddle(submittedTransferSnapshot.sourceAddress, 12, 12)}
                    </p>
                  </div>
                  <div class="min-w-0 space-y-1">
                    <p class="text-sm text-muted-foreground">
                      {i18n.t('wallet.transfer.receipt.to')} · {submittedTransferSnapshot.destinationNetworkLabel}
                    </p>
                    <p class="identifier-text truncate text-xs text-muted-foreground">
                      {truncateAddressMiddle(submittedTransferSnapshot.destinationAddress, 12, 12)}
                    </p>
                  </div>
                </div>

                <div class="rounded-xl bg-muted/35 px-4 py-3 dark:bg-muted/40">
                  <p class="text-xs text-muted-foreground">
                    {i18n.t('wallet.transfer.receipt.transactionId')}
                  </p>
                  <p class="identifier-text mt-1 text-xs leading-5 break-all">
                    {submittedResult.txid}
                  </p>
                  <div class="mt-2 flex flex-wrap items-center gap-4">
                    <InlineTextActionButton
                      onclick={() => copySuccessFieldValue(submittedResult.txid, 'txid')}
                    >
                      {copiedSuccessField === 'txid'
                        ? i18n.t('wallet.transfer.receipt.copied')
                        : i18n.t('wallet.transfer.receipt.copy')}
                    </InlineTextActionButton>
                    {#if submittedExplorerUrl}
                      <InlineTextActionButton
                        onclick={() => void openTrustedExternalUrl(submittedExplorerUrl)}
                      >
                        <ExternalLinkIcon class="size-3.5" />
                        {i18n.t('wallet.transfer.receipt.viewExplorer')}
                      </InlineTextActionButton>
                    {/if}
                  </div>
                </div>
              {/if}
            </div>
          </Card.Content>
        </Card.Root>
      {/if}
    {/if}
  </div>
</WalletTransferStepperShell>

<StandardRightSheet
  bind:isOpen={showSourceAssetSheet}
  title={i18n.t('wallet.transfer.source.sendFrom')}
>
  <div class="flex h-full min-h-0 flex-col">
    {#if positiveSendableCoinOptions.length === 0}
      <p class="text-sm text-muted-foreground">{i18n.t('wallet.transfer.noAssets')}</p>
    {:else}
      <ScrollArea.Root class="min-h-0 flex-1">
        <ScrollArea.Viewport class="h-full pr-1">
          <div class="space-y-2 pb-1">
            {#each positiveSendableCoinOptions as option}
              {@const optionAddress = option.scope.address}
              <button
                type="button"
                class="group flex w-full items-center justify-between rounded-lg p-3 text-left transition-colors
                focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2
                {selectedCoinId === option.coin.id && selectedSourceChannelId === option.channelId
                  ? 'bg-primary/14 hover:bg-primary/20 dark:bg-primary/28 dark:hover:bg-primary/36'
                  : 'bg-muted/65 hover:bg-muted/70 dark:bg-muted/55 dark:hover:bg-muted/65'}"
                onclick={() => selectSourceCoin(option.coin.id, option.channelId)}
              >
                <div class="flex min-w-0 items-center gap-2.5">
                  <CoinIcon
                    coinId={option.coin.id}
                    coinName={option.displayName}
                    size={18}
                    decorative={true}
                  />
                  <div class="min-w-0">
                    <p class="truncate text-sm font-semibold">{option.displayName}</p>
                    <p class="truncate text-xs text-muted-foreground">
                      {i18n.t(
                        option.sourceKind === 'private'
                          ? 'wallet.transfer.source.privateAddress'
                          : 'wallet.transfer.source.publicAddress'
                      )}
                      · {resolveSourceNetworkDisplayName(
                        option.coin.systemId ?? option.coin.id,
                        option.channelId.split('.')[0] ?? ''
                      )}
                    </p>
                    {#if optionAddress}
                      <p class="identifier-text truncate text-[11px] text-muted-foreground">
                        {truncateAddressMiddle(optionAddress, 10, 10)}
                      </p>
                    {/if}
                  </div>
                </div>
                <p class="ml-3 shrink-0 text-sm font-medium">
                  {formatSheetBalance(option.balanceTotal)}
                </p>
              </button>
            {/each}
          </div>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>
    {/if}
  </div>
</StandardRightSheet>

<StandardRightSheet bind:isOpen={showFeeSheet} title={i18n.t('wallet.transfer.fee.networkFee')}>
  <div class="flex h-full min-h-0 flex-col">
    <div class="space-y-4">
      {#if selectedChannelPrefix === 'eth' || selectedChannelPrefix === 'erc20'}
        <p class="text-sm leading-5 text-muted-foreground">
          {i18n.t('wallet.transfer.fee.maximumExplanation')}
        </p>
      {/if}

      <RadioGroup.Root
        bind:value={pendingDirectSendFeeMode}
        disabled={transferMutationLocked}
        name="direct-send-fee-mode"
        aria-label={i18n.t('wallet.transfer.fee.networkFee')}
      >
        {#each directSendFeeOptions as option}
          <RadioGroup.Item
            value={option.value}
            aria-label={`${option.label} ${option.description}`}
          >
            <span>
              <span class="block text-sm font-semibold">{option.label}</span>
              <span class="block text-xs text-muted-foreground">{option.description}</span>
            </span>
          </RadioGroup.Item>
        {/each}
      </RadioGroup.Root>

      {#if activePreflight && pendingFeeModeMatchesApplied}
        <div class="rounded-xl bg-muted/35 px-4 py-3 dark:bg-muted/40">
          <div class="flex items-center justify-between gap-4">
            <p class="text-xs text-muted-foreground">
              {selectedChannelPrefix === 'eth' || selectedChannelPrefix === 'erc20'
                ? i18n.t('wallet.transfer.fee.maximumNetworkFee')
                : i18n.t('wallet.transfer.fee.networkFee')}
            </p>
            <p class="text-sm font-semibold tabular-nums">{reviewNetworkFeeValue}</p>
          </div>
          <p class="mt-1 text-xs text-muted-foreground">
            {i18n.t('wallet.transfer.fee.paidIn', {
              currency: reviewNetworkFeeCurrencyLabel || activePreflight.feeCurrency,
            })}
          </p>
        </div>
      {:else if activePreflight}
        <p class="rounded-xl bg-muted/35 px-4 py-3 text-sm text-muted-foreground dark:bg-muted/40">
          {i18n.t('wallet.transfer.fee.refreshRequired')}
        </p>
      {/if}
    </div>

    <div class="mt-auto pt-6">
      <Button
        class="w-full"
        onclick={() => void applyDirectSendFeeMode()}
        disabled={transferMutationLocked || preflighting}
      >
        {i18n.t('wallet.transfer.fee.apply')}
      </Button>
    </div>
  </div>
</StandardRightSheet>

<StandardRightSheet
  bind:isOpen={showAddressBookSheet}
  title={i18n.t('wallet.transfer.addressBook.sheetTitle')}
>
  <div class="flex h-full min-h-0 flex-col gap-3">
    <SearchInput
      bind:value={addressBookSearchTerm}
      placeholder={i18n.t('wallet.transfer.addressBook.searchPlaceholder')}
      inputClass="h-10"
    />

    {#if addressBookEndpointOptions.length === 0}
      <p class="text-sm text-muted-foreground">{i18n.t('wallet.transfer.addressBook.empty')}</p>
    {:else}
      <ScrollArea.Root class="min-h-0 flex-1">
        <ScrollArea.Viewport class="h-full pr-1">
          <div class="space-y-2 pb-1">
            {#each addressBookEndpointOptions as option}
              <button
                type="button"
                class="flex w-full items-start justify-between rounded-lg bg-muted/65 px-3.5 py-3 text-left transition-colors
                  hover:bg-muted/70 focus:outline-none focus-visible:ring-2 focus-visible:ring-ring/60
                  dark:bg-muted/55 dark:hover:bg-muted/65"
                onclick={() => selectAddressBookRecipient(option)}
              >
                <div class="min-w-0">
                  <p class="truncate text-[15px] leading-tight font-medium">{option.contactName}</p>
                  <p
                    class="mt-0.5 flex items-center gap-1.5 truncate text-sm text-muted-foreground"
                  >
                    <span
                      class="inline-flex shrink-0 rounded-full bg-background/60 px-2.5 py-0.5 text-[10px] font-semibold tracking-wide text-muted-foreground uppercase dark:bg-background/45"
                    >
                      {endpointBadgeLabel(option.endpointKind)}
                    </span>
                    <span class="identifier-text truncate"
                      >{shortRecipientAddress(option.endpointAddress)}</span
                    >
                  </p>
                </div>
                {#if option.lastUsedAt}
                  <p class="mt-0.5 ml-3 text-xs text-muted-foreground">
                    {i18n.t('wallet.transfer.addressBook.recent')}
                  </p>
                {/if}
              </button>
            {/each}
          </div>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>
    {/if}
  </div>
</StandardRightSheet>

<StandardRightSheet
  bind:isOpen={showReceiveAssetSheet}
  title={i18n.t('wallet.transfer.receiveSheetTitle')}
>
  <div class="flex h-full min-h-0 flex-col gap-3">
    <SearchInput
      bind:value={receiveSearchTerm}
      placeholder={i18n.t('wallet.transfer.receiveSearchPlaceholder')}
      inputClass="h-10"
    />

    {#if targetsError}
      <p class="text-sm text-destructive">{targetsError}</p>
    {/if}

    {#if !loadingTargets && rawReceiveAssetSections.allOptions.length === 0}
      <p class="text-sm text-muted-foreground">{i18n.t('wallet.transfer.viaNoOptions')}</p>
    {:else if !loadingTargets && receiveAssetOptions.length === 0}
      <p class="text-sm text-muted-foreground">{i18n.t('wallet.transfer.routeNoMatches')}</p>
    {:else}
      <ScrollArea.Root class="min-h-0 flex-1">
        <ScrollArea.Viewport class="h-full pr-1">
          <div class="space-y-4 pb-1">
            {#if popularReceiveAssetOptions.length > 0}
              <section class="space-y-2">
                <p class="px-1 text-xs font-semibold text-muted-foreground">
                  {i18n.t('wallet.transfer.routeGroupPopular')}
                </p>
                {#each popularReceiveAssetOptions as option}
                  {@const display = getReceiveOptionDisplay(option)}
                  <button
                    type="button"
                    class="group flex w-full items-center justify-between rounded-lg p-3 text-left transition-colors
                      focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2
                      {isReceiveOptionSelected(option)
                      ? 'bg-primary/14 hover:bg-primary/20 dark:bg-primary/28 dark:hover:bg-primary/36'
                      : 'bg-muted/65 hover:bg-muted/70 dark:bg-muted/55 dark:hover:bg-muted/65'}"
                    onclick={() => selectReceiveAsset(option.id)}
                  >
                    <div class="flex min-w-0 items-center gap-2.5">
                      <CoinIcon
                        coinId={option.destinationId}
                        coinName={display.primary}
                        size={18}
                        decorative={true}
                      />
                      <div class="min-w-0">
                        <p class="truncate text-sm font-semibold">{display.primary}</p>
                        {#if display.secondary}
                          <p class="truncate text-xs text-muted-foreground">{display.secondary}</p>
                        {/if}
                      </div>
                    </div>
                  </button>
                {/each}
              </section>
            {/if}

            {#if otherReceiveAssetOptions.length > 0}
              <section class="space-y-2">
                <p class="px-1 text-xs font-semibold text-muted-foreground">
                  {popularReceiveAssetOptions.length > 0
                    ? i18n.t('wallet.transfer.routeGroupMore')
                    : i18n.t('wallet.transfer.routeGroupConversions')}
                </p>
                {#each otherReceiveAssetOptions as option}
                  {@const display = getReceiveOptionDisplay(option)}
                  <button
                    type="button"
                    class="group flex w-full items-center justify-between rounded-lg p-3 text-left transition-colors
                      focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2
                      {isReceiveOptionSelected(option)
                      ? 'bg-primary/14 hover:bg-primary/20 dark:bg-primary/28 dark:hover:bg-primary/36'
                      : 'bg-muted/65 hover:bg-muted/70 dark:bg-muted/55 dark:hover:bg-muted/65'}"
                    onclick={() => selectReceiveAsset(option.id)}
                  >
                    <div class="flex min-w-0 items-center gap-2.5">
                      <CoinIcon
                        coinId={option.destinationId}
                        coinName={display.primary}
                        size={18}
                        decorative={true}
                      />
                      <div class="min-w-0">
                        <p class="truncate text-sm font-semibold">{display.primary}</p>
                        {#if display.secondary}
                          <p class="truncate text-xs text-muted-foreground">{display.secondary}</p>
                        {/if}
                      </div>
                    </div>
                  </button>
                {/each}
              </section>
            {/if}
          </div>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>
    {/if}
  </div>
</StandardRightSheet>

<StandardRightSheet
  bind:isOpen={showNetworkSheet}
  title={i18n.t('wallet.transfer.networkSheetTitle')}
>
  <div class="flex h-full min-h-0 flex-col gap-3">
    {#if pendingGroupedReceiveOption}
      <p class="text-sm text-muted-foreground">
        {i18n.t('wallet.transfer.networkSheetDescription', {
          value: pendingGroupedReceiveOption.label,
        })}
      </p>
      <ScrollArea.Root class="min-h-0 flex-1">
        <ScrollArea.Viewport class="h-full pr-1">
          <div class="space-y-2 pb-1">
            {#each pendingGroupedReceiveOption.networkOptions ?? [] as option}
              {@const optionExportSystemId = option.hasOnChainPath
                ? null
                : (option.exportOptions[0]?.exportTo ?? null)}
              {@const optionNetworkIconId = networkIconCoinIdForExportOption(
                optionExportSystemId ?? selectedSourceSystemId,
                optionExportSystemId
                  ? (option.exportOptions[0]?.exportToName ?? optionExportSystemId)
                  : sourceNetworkDisplayName
              )}
              {@const optionDisabled = isBridgeFeeSelectionDisabled(optionExportSystemId)}
              {@const optionBridgeFeeLine = bridgeFeeLineForExportOption(optionExportSystemId)}
              {@const optionBridgeFeeMeta = bridgeFeeMetaLineForExportOption(optionExportSystemId)}
              <button
                type="button"
                class="group flex w-full items-center justify-between rounded-lg p-3 text-left transition-colors
                  focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2
                  {optionDisabled ? 'cursor-not-allowed opacity-60' : ''}
                  {selectedReceiveAssetId === option.id &&
                selectedExportSystemId === optionExportSystemId
                  ? 'bg-primary/14 hover:bg-primary/20 dark:bg-primary/28 dark:hover:bg-primary/36'
                  : 'bg-muted/65 hover:bg-muted/70 dark:bg-muted/55 dark:hover:bg-muted/65'}"
                disabled={optionDisabled}
                onclick={() => {
                  if (optionDisabled) return;
                  selectReceiveNetworkOption(option.id);
                }}
              >
                <div class="flex min-w-0 items-center gap-2">
                  <CoinIcon
                    coinId={optionNetworkIconId}
                    coinName={networkLabelForGroupedOption(option)}
                    size={18}
                    decorative={true}
                  />
                  <div class="min-w-0">
                    <p class="truncate text-sm font-medium">
                      {networkLabelForGroupedOption(option)}
                    </p>
                    <p class="truncate text-xs text-muted-foreground">
                      {i18n.t('wallet.transfer.receiveAs', {
                        value: resolveReceiveLabel(option, optionExportSystemId),
                      })}
                    </p>
                    {#if optionBridgeFeeLine}
                      <p
                        class={`truncate text-xs ${bridgeFeeInsufficient ? 'text-destructive' : 'text-muted-foreground'}`}
                      >
                        {optionBridgeFeeLine}
                      </p>
                    {/if}
                    {#if optionBridgeFeeMeta}
                      <p
                        class={`truncate text-xs ${bridgeFeeInsufficient ? 'text-destructive' : 'text-muted-foreground'}`}
                      >
                        {optionBridgeFeeMeta}
                      </p>
                    {/if}
                  </div>
                </div>
              </button>
            {/each}
          </div>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>
    {:else}
      <p class="text-sm text-muted-foreground">{i18n.t('wallet.transfer.viaNoOptions')}</p>
    {/if}
  </div>
</StandardRightSheet>

<StandardRightSheet
  bind:isOpen={showExportSheet}
  title={i18n.t('wallet.transfer.exportSheetTitle')}
>
  <div class="flex h-full min-h-0 flex-col gap-3">
    {#if pendingTargetOption}
      <p class="text-sm text-muted-foreground">
        {#if exportSheetMode !== 'send' && !pendingTargetOption.hasOnChainPath && pendingTargetOption.exportOptions.length === 1}
          {i18n.t('wallet.transfer.onlyAvailableOnNetwork', {
            value: networkLabelForExportOption(
              pendingTargetOption.exportOptions[0].exportTo,
              pendingTargetOption.exportOptions[0].exportToName
            ),
          })}
        {:else}
          {i18n.t('wallet.transfer.exportSheetDescription', {
            value:
              exportSheetMode === 'send'
                ? selectedCoinPresentation?.displayTicker?.trim() ||
                  selectedCoinPresentation?.displayName?.trim() ||
                  pendingTargetOption.label
                : pendingTargetOption.label,
          })}
        {/if}
      </p>
      <ScrollArea.Root class="min-h-0 flex-1">
        <ScrollArea.Viewport class="h-full pr-1">
          <div class="space-y-2 pb-1">
            {#if pendingTargetOption.hasOnChainPath || exportSheetMode === 'send'}
              {@const sameNetworkSelected =
                exportSheetMode === 'send'
                  ? selectedSendExportSystemId === null
                  : selectedReceiveAssetId === pendingTargetOption.id &&
                    selectedExportSystemId === null}
              <button
                type="button"
                class="group flex w-full items-center justify-between rounded-lg p-3 text-left transition-colors
                  focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2
                  {sameNetworkSelected
                  ? 'bg-primary/14 hover:bg-primary/20 dark:bg-primary/28 dark:hover:bg-primary/36'
                  : 'bg-muted/65 hover:bg-muted/70 dark:bg-muted/55 dark:hover:bg-muted/65'}"
                onclick={selectSameNetworkOption}
              >
                <div class="flex min-w-0 items-center gap-2">
                  <CoinIcon
                    coinId={selectedCoin?.id || pendingTargetOption.destinationId}
                    coinName={pendingTargetOption.label}
                    size={18}
                    decorative={true}
                  />
                  <div class="min-w-0">
                    <p class="truncate text-sm font-medium">
                      {i18n.t('wallet.transfer.keepOnNetwork', { value: sourceNetworkDisplayName })}
                    </p>
                    <p class="truncate text-xs text-muted-foreground">
                      {i18n.t('wallet.transfer.receiveAs', {
                        value:
                          exportSheetMode === 'send'
                            ? selectedCoinPresentation?.displayTicker?.trim() ||
                              resolveReceiveLabel(pendingTargetOption, null)
                            : resolveReceiveLabel(pendingTargetOption, null),
                      })}
                    </p>
                  </div>
                </div>
              </button>
            {/if}

            {#each pendingTargetOption.exportOptions as option}
              {@const optionDisabled = isBridgeFeeSelectionDisabled(option.exportTo)}
              {@const optionBridgeFeeLine = bridgeFeeLineForExportOption(option.exportTo)}
              {@const optionBridgeFeeMeta = bridgeFeeMetaLineForExportOption(option.exportTo)}
              {@const optionSelected =
                exportSheetMode === 'send'
                  ? selectedSendExportSystemId === option.exportTo
                  : selectedReceiveAssetId === pendingTargetOption.id &&
                    selectedExportSystemId === option.exportTo}
              <button
                type="button"
                class="group flex w-full items-center justify-between rounded-lg p-3 text-left transition-colors
                  focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2
                  {optionDisabled ? 'cursor-not-allowed opacity-60' : ''}
                  {optionSelected
                  ? 'bg-primary/14 hover:bg-primary/20 dark:bg-primary/28 dark:hover:bg-primary/36'
                  : 'bg-muted/65 hover:bg-muted/70 dark:bg-muted/55 dark:hover:bg-muted/65'}"
                disabled={optionDisabled}
                onclick={() => {
                  if (optionDisabled) return;
                  selectExportOption(option.exportTo);
                }}
              >
                <div class="flex min-w-0 items-center gap-2">
                  <CoinIcon
                    coinId={networkIconCoinIdForExportOption(option.exportTo, option.exportToName)}
                    coinName={networkLabelForExportOption(option.exportTo, option.exportToName)}
                    size={18}
                    decorative={true}
                  />
                  <div class="min-w-0">
                    <p class="truncate text-sm font-medium">
                      {networkLabelForExportOption(option.exportTo, option.exportToName)}
                    </p>
                    <p class="truncate text-xs text-muted-foreground">
                      {i18n.t('wallet.transfer.receiveAs', {
                        value: resolveReceiveLabel(pendingTargetOption, option.exportTo),
                      })}
                    </p>
                    {#if optionBridgeFeeLine}
                      <p
                        class={`truncate text-xs ${bridgeFeeInsufficient ? 'text-destructive' : 'text-muted-foreground'}`}
                      >
                        {optionBridgeFeeLine}
                      </p>
                    {/if}
                    {#if optionBridgeFeeMeta}
                      <p
                        class={`truncate text-xs ${bridgeFeeInsufficient ? 'text-destructive' : 'text-muted-foreground'}`}
                      >
                        {optionBridgeFeeMeta}
                      </p>
                    {/if}
                  </div>
                </div>
              </button>
            {/each}
          </div>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>
    {:else}
      <p class="text-sm text-muted-foreground">{i18n.t('wallet.transfer.viaNoOptions')}</p>
    {/if}
  </div>
</StandardRightSheet>

<StandardRightSheet bind:isOpen={showViaSheet} title={i18n.t('wallet.transfer.viaSheetTitle')}>
  <div class="space-y-3">
    {#if selectedReceiveAssetOption}
      {#if rankedViaOptions.length === 0}
        <p class="text-sm text-muted-foreground">{i18n.t('wallet.transfer.viaNoOptions')}</p>
      {:else}
        <div class="space-y-2">
          {#each rankedViaOptions as option}
            {@const estimatedValue = formatEstimatedReceive(option)}
            {@const routeSubtitle = getViaSheetSubtitle(option)}
            <button
              type="button"
              class="group flex w-full items-center justify-between rounded-lg p-3 text-left transition-colors
                focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2
                {selectedViaOptionId === option.id
                ? 'bg-primary/14 hover:bg-primary/20 dark:bg-primary/28 dark:hover:bg-primary/36'
                : 'bg-muted/65 hover:bg-muted/70 dark:bg-muted/55 dark:hover:bg-muted/65'}"
              onclick={() => selectViaOption(option.id)}
            >
              <div class="flex min-w-0 items-start gap-2.5">
                <CoinIcon
                  coinId={option.via ?? option.id}
                  coinName={getViaOptionLabel(option)}
                  size={18}
                  decorative={true}
                />
                <div class="min-w-0">
                  <div class="flex items-center gap-2">
                    <p class="truncate text-sm font-semibold">{getViaOptionLabel(option)}</p>
                    {#if bestViaOption?.id === option.id}
                      <span
                        class="rounded-full bg-emerald-500/12 px-2 py-0.5 text-[10px] font-semibold text-emerald-700 dark:text-emerald-300"
                      >
                        {i18n.t('wallet.transfer.viaBest')}
                      </span>
                    {/if}
                  </div>
                  {#if routeSubtitle}
                    <p class="truncate text-xs text-muted-foreground">{routeSubtitle}</p>
                  {/if}
                </div>
              </div>
              {#if estimatedValue}
                <div class="ml-3 min-w-0 text-right">
                  <p
                    class="inline-flex items-center gap-1 text-[11px] font-medium text-muted-foreground"
                  >
                    <ArrowDownIcon class="size-3" />
                    <span>{i18n.t('wallet.transfer.estimatedLabel')}</span>
                  </p>
                  <p class="text-lg leading-none font-semibold text-foreground tabular-nums">
                    {estimatedValue}
                  </p>
                </div>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    {:else}
      <p class="text-sm text-muted-foreground">{i18n.t('wallet.transfer.viaNoOptions')}</p>
    {/if}
  </div>
</StandardRightSheet>
