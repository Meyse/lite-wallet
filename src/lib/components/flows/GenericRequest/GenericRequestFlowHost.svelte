<script lang="ts">
  import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';
  import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import CheckCircle2Icon from '@lucide/svelte/icons/check-circle-2';
  import CircleMinusIcon from '@lucide/svelte/icons/circle-minus';
  import CirclePlusIcon from '@lucide/svelte/icons/circle-plus';
  import CircleHelpIcon from '@lucide/svelte/icons/circle-help';
  import EyeIcon from '@lucide/svelte/icons/eye';
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
  import GlobeIcon from '@lucide/svelte/icons/globe';
  import Link2Icon from '@lucide/svelte/icons/link-2';
  import ShieldAlertIcon from '@lucide/svelte/icons/shield-alert';
  import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
  import WalletIcon from '@lucide/svelte/icons/wallet';
  import XIcon from '@lucide/svelte/icons/x';
  import { toast } from 'svelte-sonner';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import InlineTextActionButton from '$lib/components/common/InlineTextActionButton.svelte';
  import { Button } from '$lib/components/ui/button';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Label } from '$lib/components/ui/label';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';
  import { Spinner } from '$lib/components/ui/spinner';
  import { CopyButton } from '$lib/components/ui/copy-button';
  import WalletTransferStepperShell from '$lib/components/shared/WalletTransferStepperShell.svelte';
  import LinkIdentitySheet from '$lib/components/wallet/sections/identity/LinkIdentitySheet.svelte';
  import { i18nStore, networkLocaleKey } from '$lib/i18n';
  import {
    buildGenericIdentityUpdateReview,
    type IdentityUpdateContentItem
  } from '$lib/genericRequest/identityUpdateReview';
  import { submitGenericProvisioningRequest } from '$lib/genericRequest/provisioning';
  import {
    type AuthenticationDetailSession,
    type GenericRequestFlowSession,
    type IdentityUpdateDetailSession,
    type ProvisionIdentityDetailSession
  } from '$lib/genericRequest/session';
  import * as genericRequestService from '$lib/services/genericRequestService.js';
  import * as identityLinkService from '$lib/services/identityLinkService.js';
  import * as identityService from '$lib/services/identityService.js';
  import * as walletService from '$lib/services/walletService.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { walletChannelsStore } from '$lib/stores/walletChannels.js';
  import type {
    BalanceResult,
    BuildAndSignGenericResponseRequest,
    CoinDefinition,
    CoinScope,
    GenericIdentityUpdatePreflightResult,
    GenericIdentityUpdateReviewResult,
    IdentityDetails,
    LinkedIdentity
  } from '$lib/types/wallet.js';
  import { truncateIdentityAddress } from '$lib/utils/identityDisplay.js';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay.js';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import { extractWalletErrorMessage, extractWalletErrorType } from '$lib/utils/walletErrors.js';

  type FlowStepId =
    | 'auth'
    | 'updateOverview'
    | 'updateRisk'
    | 'updateContent'
    | 'updateFunding'
    | 'complete';

  type FlowStep = {
    id: FlowStepId;
    label: string;
    status: 'complete' | 'current' | 'upcoming';
  };

  type FundingSource = {
    channelId: string;
    address: string;
    addressLabel: string;
    systemId: string;
    balanceValue: number;
    balanceDisplay: string;
    isPrimaryAddress: boolean;
  };

  type ResponseDraft = Omit<BuildAndSignGenericResponseRequest, 'requestHex'>;

  const MAINNET_SYSTEM_ID = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
  const TESTNET_SYSTEM_ID = 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq';
  const AUTH_REQUIRED_ID = 1;
  const AUTH_REQUIRED_SYSTEM = 2;
  const AUTH_REQUIRED_PARENT = 3;

  type GenericRequestFlowHostProps = {
    isOpen?: boolean;
    session: GenericRequestFlowSession | null;
    onClose?: () => void;
  };

  const defaultClose = () => {};

  let { isOpen = $bindable(false), session, onClose = defaultClose }: GenericRequestFlowHostProps = $props();

  const i18n = $derived($i18nStore);
  const coins = $derived($coinsStore);
  const walletChannels = $derived($walletChannelsStore);
  const authDetail = $derived(
    (session?.details.find((detail) => detail.type === 'authentication') as AuthenticationDetailSession | undefined) ??
      null
  );
  const updateDetail = $derived(
    (session?.details.find((detail) => detail.type === 'identityUpdate') as IdentityUpdateDetailSession | undefined) ??
      null
  );
  const provisioningDetail = $derived(
    (session?.details.find(
      (detail) => detail.type === 'provisionIdentity'
    ) as ProvisionIdentityDetailSession | undefined) ?? null
  );

  let currentStepId = $state<FlowStepId>('auth');
  let linkedIdentities = $state<LinkedIdentity[]>([]);
  let identityDetailsByAddress = $state<Record<string, IdentityDetails>>({});
  let selectedAuthIdentityAddress = $state('');
  let authIdentitySheetOpen = $state(false);
  let linkSheetOpen = $state(false);
  let loadingLinkedIdentities = $state(false);
  let updateAnalysisLoading = $state(false);
  let updateLoading = $state(false);
  let updateSubmitting = $state(false);
  let provisioningSubmitting = $state(false);
  let provisioningNotice = $state('');
  let completing = $state(false);
  let requesterIdentityDetails = $state<IdentityDetails | null>(null);
  let requesterIdentityLoading = $state(false);
  let flowError = $state('');
  let responseDraft = $state<ResponseDraft | null>(null);
  let completionMessage = $state('');
  let updateAnalysis = $state<GenericIdentityUpdateReviewResult | null>(null);
  let updatePreflight = $state<GenericIdentityUpdatePreflightResult | null>(null);
  let fundingSources = $state<FundingSource[]>([]);
  let fundingSourcesLoading = $state(false);
  let fundingSourcesLoadedKey = $state('');
  let selectedSourceChannelId = $state('');
  let highRiskAcknowledged = $state(false);
  let authorityInfoExpanded = $state(false);
  let contentInspectSheetOpen = $state(false);
  let selectedContentChange = $state<IdentityUpdateContentItem | null>(null);
  let fundingSourceLoadSequence = 0;
  let updatePreflightRequestSequence = 0;
  const copiedCompletionFieldState = new TimedValueState<'txid'>();

  const updateReviewSource = $derived(updatePreflight ?? updateAnalysis);
  const updateReview = $derived(
    updateReviewSource
      ? buildGenericIdentityUpdateReview({
          currentIdentity: updateReviewSource.currentIdentity,
          requestedIdentity: updateReviewSource.requestedIdentity,
          rawRequestedIdentity: updateDetail?.requestedIdentityJson ?? updateReviewSource.requestedIdentity,
          friendlyNames: updateReviewSource.friendlyNames,
          signerCmmKeyLabels: updateReviewSource.signerCmmKeyLabels,
          primaryAddressAfterUpdateInfo: updateReviewSource.primaryAddressAfterUpdateInfo,
          currentAuthorities: updateReviewSource.currentAuthorities,
          t: i18n.t.bind(i18n)
        })
      : null
  );
  const eligibleLinkedIdentities = $derived(filterEligibleLinkedIdentities());
  const selectedFundingSource = $derived(
    selectedSourceChannelId
      ? fundingSources.find((source) => source.channelId === selectedSourceChannelId) ?? null
      : null
  );
  const activeAuthIdentity = $derived(
    selectedAuthIdentityAddress
      ? eligibleLinkedIdentities.find(
          (identity) =>
            identity.identityAddress.toLowerCase() === selectedAuthIdentityAddress.toLowerCase()
        ) ?? null
      : null
  );
  const visibleSteps = $derived(buildVisibleSteps());
  const currentStepIndex = $derived(
    Math.max(
      visibleSteps.findIndex((step) => step.id === currentStepId),
      0
    ) + 1
  );
  const completionNotice = $derived(
    session?.responseUris.find((item) => item.mode === 'post')?.uri ||
      session?.responseUris.find((item) => item.mode === 'redirect')?.uri ||
      ''
  );
  const completionTxid = $derived(responseDraft?.identityUpdate?.txid?.trim() ?? '');
  const copiedCompletionField = $derived(copiedCompletionFieldState.current);
  const finishStepLocked = $derived(currentStepId === 'complete' && !!updateDetail && !!responseDraft);
  const stepperContentWidthClass = $derived(
    currentStepId === 'updateFunding' ? 'max-w-[560px]' : 'max-w-[720px]'
  );

  $effect(() => {
    if (!isOpen || !session) {
      resetFlowState();
      return;
    }

    currentStepId = authDetail ? 'auth' : 'updateOverview';
    flowError = '';
    completionMessage = '';
    provisioningNotice = '';
    responseDraft = null;
    updateAnalysis = null;
    updatePreflight = null;
    fundingSources = [];
    fundingSourcesLoading = false;
    fundingSourcesLoadedKey = '';
    highRiskAcknowledged = false;
    authorityInfoExpanded = false;
    contentInspectSheetOpen = false;
    selectedContentChange = null;
    selectedAuthIdentityAddress = '';
    selectedSourceChannelId = '';

    void hydrateLinkedIdentities();
    if (!authDetail && updateDetail) {
      void hydrateUpdateReview();
    }
  });

  $effect(() => {
    if (!isOpen || currentStepId !== 'auth') return;

    if (selectedAuthIdentityAddress) {
      const stillEligible = eligibleLinkedIdentities.some(
        (identity) => identity.identityAddress.toLowerCase() === selectedAuthIdentityAddress.toLowerCase()
      );
      if (!stillEligible) {
        selectedAuthIdentityAddress = '';
      }
      return;
    }

    if (eligibleLinkedIdentities.length === 1) {
      selectedAuthIdentityAddress = eligibleLinkedIdentities[0].identityAddress;
    }
  });

  $effect(() => {
    const signerIdentityId = session?.signer.identityId?.trim() ?? '';
    if (!isOpen || !signerIdentityId) {
      requesterIdentityDetails = null;
      requesterIdentityLoading = false;
      return undefined;
    }

    let cancelled = false;
    requesterIdentityDetails = null;
    requesterIdentityLoading = true;

    void (async () => {
      try {
        const details = await identityLinkService.getIdentityDetails(signerIdentityId);
        if (!cancelled) {
          requesterIdentityDetails = details;
          requesterIdentityLoading = false;
        }
      } catch {
        if (!cancelled) {
          requesterIdentityDetails = null;
          requesterIdentityLoading = false;
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  });

  function resetFlowState() {
    currentStepId = 'auth';
    linkedIdentities = [];
    identityDetailsByAddress = {};
    selectedAuthIdentityAddress = '';
    loadingLinkedIdentities = false;
    updateAnalysisLoading = false;
    updateLoading = false;
    updateSubmitting = false;
    provisioningSubmitting = false;
    completing = false;
    requesterIdentityDetails = null;
    requesterIdentityLoading = false;
    flowError = '';
    responseDraft = null;
    completionMessage = '';
    provisioningNotice = '';
    updateAnalysis = null;
    updatePreflight = null;
    fundingSources = [];
    fundingSourcesLoading = false;
    fundingSourcesLoadedKey = '';
    highRiskAcknowledged = false;
    authorityInfoExpanded = false;
    contentInspectSheetOpen = false;
    selectedContentChange = null;
    selectedSourceChannelId = '';
    authIdentitySheetOpen = false;
    linkSheetOpen = false;
    copiedCompletionFieldState.clear();
  }

  function setError(error: unknown, fallbackKey = 'genericRequest.error.generic') {
    const type = extractWalletErrorType(error);
    switch (type) {
      case 'WalletLocked':
        flowError = i18n.t('genericRequest.error.walletLocked');
        return;
      case 'IdentityOwnershipMismatch':
        flowError = i18n.t('genericRequest.error.identityOwnershipMismatch');
        return;
      case 'IdentityRequestExpired':
        flowError = i18n.t('genericRequest.error.identityRequestExpired');
        return;
      case 'IdentityUnsupportedAuthority':
        flowError = i18n.t('genericRequest.error.identityUnsupportedAuthority');
        return;
      case 'IdentityUnsupportedActiveCurrencyChange':
        flowError = i18n.t('genericRequest.error.identityUnsupportedActiveCurrencyChange');
        return;
      case 'IdentityUnsupportedTokenizedControlChange':
        flowError = i18n.t('genericRequest.error.identityUnsupportedTokenizedControlChange');
        return;
      case 'InsufficientFunds':
        flowError = i18n.t('genericRequest.error.insufficientFunds');
        return;
      case 'InvalidAddress':
      case 'UnsupportedChannel':
        flowError = i18n.t('genericRequest.error.noSourceChannel');
        return;
      case 'NetworkError':
      case 'OperationFailed':
        flowError = i18n.t(fallbackKey);
        return;
      default:
        break;
    }

    if (error instanceof Error && error.message.startsWith('genericRequest.')) {
      flowError = i18n.t(error.message);
      return;
    }

    flowError = extractWalletErrorMessage(error) || i18n.t(fallbackKey);
  }

  function reviewSystemId(): string {
    return (
      updateDetail?.sourceSystemId?.trim() ||
      (session?.testnet ? TESTNET_SYSTEM_ID : MAINNET_SYSTEM_ID)
    );
  }

  function responseUriLabel(uri: string): string {
    try {
      const parsed = new URL(uri);
      return `${parsed.protocol}//${parsed.host}`;
    } catch {
      return uri;
    }
  }

  function responseUriHost(uri: string): string {
    try {
      return new URL(uri).host;
    } catch {
      return uri;
    }
  }

  function authRequesterLabel(): string {
    if (requesterIdentityDetails) {
      const displayName = formatIdentityDisplayName(requesterIdentityDetails);
      if (displayName && displayName !== requesterIdentityDetails.identityAddress) {
        return displayName;
      }
    }

    const fallbackUri = completionNotice || session?.responseUris[0]?.uri || '';
    const host = responseUriHost(fallbackUri);
    if (host) return host;

    const signerIdentityId = session?.signer.identityId?.trim();
    return signerIdentityId || i18n.t('genericRequest.auth.requesterFallback');
  }

  function authAppLabel(): string {
    const host = responseUriHost(completionNotice || session?.responseUris[0]?.uri || '');
    return host || i18n.t('genericRequest.auth.requesterFallback');
  }

  function authConstraintLabel(type: number): string {
    if (type === AUTH_REQUIRED_ID) return i18n.t('genericRequest.auth.requiredId');
    if (type === AUTH_REQUIRED_SYSTEM) return i18n.t('genericRequest.auth.requiredSystem');
    return i18n.t('genericRequest.auth.requiredParent');
  }

  function formatExpiryTime(expiryTime: number | null): string {
    if (!expiryTime) return '';

    return new Intl.DateTimeFormat(undefined, {
      dateStyle: 'medium',
      timeStyle: 'short'
    }).format(expiryTime * 1000);
  }

  function equalsIgnoreCase(left: string, right: string): boolean {
    return left.trim().toLowerCase() === right.trim().toLowerCase();
  }

  function toFiniteNumber(value: BalanceResult['total']): number | null {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : null;
  }

  function resolveFundingCoin(systemId: string): CoinDefinition | null {
    const normalizedSystemId = systemId.trim().toLowerCase();
    return (
      coins.find(
        (coin) =>
          coin.compatibleChannels.includes('vrpc') &&
          coin.systemId.toLowerCase() === normalizedSystemId &&
          coin.currencyId.toLowerCase() === normalizedSystemId
      ) ?? null
    );
  }

  function formatFundingSourceBalance(value: number, coin: CoinDefinition): string {
    const maxFractionDigits = Math.max(2, Math.min(8, coin.decimals));
    const minimumFractionDigits = value === 0 ? 0 : Math.min(2, maxFractionDigits);
    return `${i18n.formatNumber(value, {
      minimumFractionDigits,
      maximumFractionDigits: maxFractionDigits
    })} ${coin.displayTicker}`;
  }

  function buildFundingSource(
    scope: CoinScope,
    balance: BalanceResult,
    coin: CoinDefinition
  ): FundingSource | null {
    const balanceValue = toFiniteNumber(balance.total);
    if (balanceValue === null || balanceValue <= 0) {
      return null;
    }

    return {
      channelId: scope.channelId,
      address: scope.address,
      addressLabel: scope.addressLabel,
      systemId: scope.systemId,
      balanceValue,
      balanceDisplay: formatFundingSourceBalance(balanceValue, coin),
      isPrimaryAddress: scope.isPrimaryAddress
    };
  }

  function fundingSourcePrimaryLabel(source: FundingSource): string {
    const label = source.addressLabel.trim();
    if (label && !equalsIgnoreCase(label, source.address)) {
      return label;
    }

    return truncateIdentityAddress(source.address, 10, 8);
  }

  function fundingSourceSecondaryLabel(source: FundingSource): string {
    const networkLabel = resolveSystemDisplayLabel(source.systemId);
    const label = source.addressLabel.trim();
    if (!label || equalsIgnoreCase(label, source.address)) {
      return networkLabel;
    }

    const shortAddress = truncateIdentityAddress(source.address, 8, 6);
    return `${shortAddress} • ${networkLabel}`;
  }

  async function loadIdentityDetailsMap(identities: LinkedIdentity[]): Promise<Record<string, IdentityDetails>> {
    const requiredParentIds = new Set(
      authDetail?.constraints
        .filter((constraint) => constraint.type === AUTH_REQUIRED_PARENT)
        .map((constraint) => constraint.identity.toLowerCase()) ?? []
    );

    if (requiredParentIds.size === 0) {
      return {};
    }

    const detailEntries = await Promise.all(
      identities.map(async (identity) => {
        try {
          const details = await identityLinkService.getIdentityDetails(identity.identityAddress);
          return [identity.identityAddress.toLowerCase(), details] as const;
        } catch {
          return [identity.identityAddress.toLowerCase(), null] as const;
        }
      })
    );

    return Object.fromEntries(
      detailEntries.filter((entry): entry is readonly [string, IdentityDetails] => entry[1] !== null)
    );
  }

  function applyPassthroughSelection(identities: LinkedIdentity[]) {
    const passthroughFqn = session?.passthroughAutoLinkFqn?.toLowerCase();
    if (!passthroughFqn) return;

    const passthroughIdentity = identities.find((identity) => {
      const displayName = formatIdentityDisplayName(identity).toLowerCase();
      return displayName === passthroughFqn || identity.fullyQualifiedName?.toLowerCase() === passthroughFqn;
    });

    if (passthroughIdentity) {
      selectedAuthIdentityAddress = passthroughIdentity.identityAddress;
    }
  }

  function buildVisibleSteps(): FlowStep[] {
    if (!session) return [];

    const order: FlowStepId[] = [];
    if (authDetail) {
      order.push('auth');
    }
    if (updateDetail) {
      order.push('updateOverview');
      if (updateReview?.hasHighRisk) {
        order.push('updateRisk');
      }
      if (updateReview?.hasContent) {
        order.push('updateContent');
      }
      order.push('updateFunding');
    }
    if (responseDraft) {
      order.push('complete');
    }

    return order.map((id, index) => ({
      id,
      label: i18n.t(`genericRequest.steps.${id}`),
      status:
        id === currentStepId
          ? 'current'
          : index < order.indexOf(currentStepId)
            ? 'complete'
            : 'upcoming'
    }));
  }

  async function hydrateLinkedIdentities() {
    loadingLinkedIdentities = true;
    flowError = '';

    try {
      const identities = await identityLinkService.getLinkedIdentities();
      linkedIdentities = identities;
      identityDetailsByAddress = await loadIdentityDetailsMap(identities);
      applyPassthroughSelection(identities);
    } catch (error) {
      setError(error, 'genericRequest.error.linkedIdentities');
    } finally {
      loadingLinkedIdentities = false;
    }
  }

  async function hydrateUpdateReview() {
    if (!session || !updateDetail) return;

    updateAnalysisLoading = true;
    flowError = '';

    try {
      updateAnalysis = await genericRequestService.reviewGenericIdentityUpdate(
        updateDetail.requestedIdentityJson,
        updateDetail.targetIdentityAddress,
        reviewSystemId(),
        {
          requestId: updateDetail.requestId,
          signerSystemId: session.signer.systemId,
          signerIdentityId: session.signer.identityId,
          expiryHeight: updateDetail.expiryHeight
        }
      );
    } catch (error) {
      updateAnalysis = null;
      setError(error, 'genericRequest.error.updatePreflight');
    } finally {
      updateAnalysisLoading = false;
    }
  }

  $effect(() => {
    if (!isOpen || currentStepId !== 'updateFunding' || !updateDetail) {
      return;
    }

    const systemId = reviewSystemId();
    const loadKey = `${updateDetail.targetIdentityAddress.toLowerCase()}::${systemId.toLowerCase()}`;
    if (fundingSourcesLoadedKey === loadKey || fundingSourcesLoading) {
      return;
    }

    void hydrateFundingSources(loadKey, systemId);
  });

  function filterEligibleLinkedIdentities(): LinkedIdentity[] {
    if (!authDetail) {
      return linkedIdentities;
    }

    const requiredIds = new Set(
      authDetail.constraints
        .filter((constraint) => constraint.type === AUTH_REQUIRED_ID)
        .map((constraint) => constraint.identity.toLowerCase())
    );
    const requiredSystems = new Set(
      authDetail.constraints
        .filter((constraint) => constraint.type === AUTH_REQUIRED_SYSTEM)
        .map((constraint) => constraint.identity.toLowerCase())
    );
    const requiredParents = new Set(
      authDetail.constraints
        .filter((constraint) => constraint.type === AUTH_REQUIRED_PARENT)
        .map((constraint) => constraint.identity.toLowerCase())
    );

    return linkedIdentities.filter((identity) => {
      if (requiredIds.size > 0 && !requiredIds.has(identity.identityAddress.toLowerCase())) {
        return false;
      }

      if (
        requiredSystems.size > 0 &&
        (!identity.systemId || !requiredSystems.has(identity.systemId.toLowerCase()))
      ) {
        return false;
      }

      if (requiredParents.size > 0) {
        const details = identityDetailsByAddress[identity.identityAddress.toLowerCase()];
        const parent = details?.parent?.toLowerCase();
        if (!parent || !requiredParents.has(parent)) {
          return false;
        }
      }

      return true;
    });
  }

  async function hydrateFundingSources(loadKey: string, systemId: string) {
    const coin = resolveFundingCoin(systemId);
    const requestSequence = ++fundingSourceLoadSequence;

    fundingSourcesLoading = true;
    flowError = '';

    try {
      if (!coin) {
        fundingSources = [];
        fundingSourcesLoadedKey = loadKey;
        selectedSourceChannelId = '';
        updatePreflight = null;
        return;
      }

      const result = await walletService.getCoinScopes(coin.id);
      if (requestSequence !== fundingSourceLoadSequence) return;

      const candidateScopes = result.scopes.filter(
        (scope) =>
          scope.scopeKind === 'transparent' &&
          !scope.isReadOnly &&
          scope.systemId.toLowerCase() === systemId.toLowerCase()
      );
      const balanceResults = await Promise.all(
        candidateScopes.map(async (scope) => ({
          scope,
          balance: await walletService.getBalances(scope.channelId, coin.id)
        }))
      );
      if (requestSequence !== fundingSourceLoadSequence) return;

      const nextSources = balanceResults
        .map(({ scope, balance }) => buildFundingSource(scope, balance, coin))
        .filter((source): source is FundingSource => source !== null)
        .sort((left, right) => {
          if (left.isPrimaryAddress !== right.isPrimaryAddress) {
            return left.isPrimaryAddress ? -1 : 1;
          }

          if (left.balanceValue !== right.balanceValue) {
            return right.balanceValue - left.balanceValue;
          }

          return fundingSourcePrimaryLabel(left)
            .toLowerCase()
            .localeCompare(fundingSourcePrimaryLabel(right).toLowerCase());
        });

      fundingSources = nextSources;
      fundingSourcesLoadedKey = loadKey;

      const existingSelectionStillValid = nextSources.some(
        (source) => source.channelId === selectedSourceChannelId
      );
      if (existingSelectionStillValid) {
        return;
      }

      updatePreflight = null;
      if (nextSources.length === 1) {
        selectedSourceChannelId = nextSources[0].channelId;
        await hydrateUpdatePreflight(nextSources[0].channelId);
        return;
      }

      selectedSourceChannelId = '';
    } catch (error) {
      if (requestSequence !== fundingSourceLoadSequence) return;
      fundingSources = [];
      fundingSourcesLoadedKey = '';
      selectedSourceChannelId = '';
      updatePreflight = null;
      setError(error, 'genericRequest.error.fundingSources');
    } finally {
      if (requestSequence === fundingSourceLoadSequence) {
        fundingSourcesLoading = false;
      }
    }
  }

  async function hydrateUpdatePreflight(sourceChannelIdOverride?: string) {
    if (!session || !updateDetail) return;
    const activeSourceChannelId = sourceChannelIdOverride ?? selectedSourceChannelId;
    const requestSequence = ++updatePreflightRequestSequence;
    if (!activeSourceChannelId) {
      updatePreflight = null;
      flowError = '';
      return;
    }

    updateLoading = true;
    flowError = '';

    try {
      const preflight = await genericRequestService.preflightGenericIdentityUpdate(
        updateDetail.requestedIdentityJson,
        updateDetail.targetIdentityAddress,
        activeSourceChannelId,
        {
          requestId: updateDetail.requestId,
          signerSystemId: session.signer.systemId,
          signerIdentityId: session.signer.identityId,
          expiryHeight: updateDetail.expiryHeight,
          requestSystemId: reviewSystemId()
        }
      );
      if (requestSequence !== updatePreflightRequestSequence) return;
      updatePreflight = preflight;
      highRiskAcknowledged = false;
      authorityInfoExpanded = false;
    } catch (error) {
      if (requestSequence !== updatePreflightRequestSequence) return;
      updatePreflight = null;
      setError(error, 'genericRequest.error.updatePreflight');
    } finally {
      if (requestSequence === updatePreflightRequestSequence) {
        updateLoading = false;
      }
    }
  }

  async function handleAuthContinue() {
    if (!session || !authDetail) {
      return;
    }

    if (!activeAuthIdentity) {
      authIdentitySheetOpen = true;
      return;
    }

    if (!activeAuthIdentity.systemId) {
      return;
    }

    flowError = '';

    responseDraft = {
      signer: {
        systemId: activeAuthIdentity.systemId,
        identityId: activeAuthIdentity.identityAddress
      },
      authentication: {
        requestId: authDetail.requestId || session.requestId
      },
      identityUpdate: responseDraft?.identityUpdate ?? null
    };

    if (updateDetail) {
      currentStepId = 'updateOverview';
      if (!updateReviewSource) {
        await hydrateUpdateReview();
      }
      return;
    }

    await submitResponseDraft({
      signer: {
        systemId: activeAuthIdentity.systemId,
        identityId: activeAuthIdentity.identityAddress
      },
      authentication: {
        requestId: authDetail.requestId || session.requestId
      },
      identityUpdate: responseDraft?.identityUpdate ?? null
    });
  }

  async function handleProvisionIdentity() {
    if (!session || !authDetail || !provisioningDetail) return;

    const signingAddress = walletChannels.vrpcAddress?.trim();
    if (!signingAddress) {
      flowError = i18n.t('genericRequest.provisioning.error.noSigningAddress');
      return;
    }

    provisioningSubmitting = true;
    flowError = '';
    provisioningNotice = '';

    try {
      await submitGenericProvisioningRequest({
        session,
        authenticationDetail: authDetail,
        provisioningDetail,
        signingAddress
      });

      const notice = i18n.t('genericRequest.provisioning.submitted');
      provisioningNotice = notice;
      toast.success(notice);
      isOpen = false;
      onClose();
    } catch (error) {
      setError(error, 'genericRequest.provisioning.error.generic');
    } finally {
      provisioningSubmitting = false;
    }
  }

  function handleAuthSheetOpenAutoFocus(event: Event) {
    event.preventDefault();
  }

  function openLinkIdentityManager() {
    authIdentitySheetOpen = false;
    linkSheetOpen = true;
  }

  async function submitResponseDraft(draft: ResponseDraft) {
    if (!session) return;

    completing = true;
    flowError = '';

    try {
      const { signedResponseHex } = await genericRequestService.buildAndSignGenericResponse({
        requestHex: session.requestHex,
        ...draft
      });

      if (session.responseUris.length > 0) {
        await deliverSignedResponse(signedResponseHex);
      }

      completionMessage = completionMessage || i18n.t('genericRequest.complete.sent');
      isOpen = false;
      onClose();
    } catch (error) {
      setError(error, 'genericRequest.error.complete');
    } finally {
      completing = false;
    }
  }

  function handleUpdateNext() {
    if (!visibleSteps.length) return;
    const currentIndex = visibleSteps.findIndex((step) => step.id === currentStepId);
    if (currentIndex < 0 || currentIndex >= visibleSteps.length - 1) {
      return;
    }
    currentStepId = visibleSteps[currentIndex + 1].id;
  }

  function handleBack() {
    if (finishStepLocked) {
      return;
    }

    const currentIndex = visibleSteps.findIndex((step) => step.id === currentStepId);
    if (currentIndex <= 0) {
      isOpen = false;
      onClose();
      return;
    }

    currentStepId = visibleSteps[currentIndex - 1].id;
  }

  async function handleFundingSourceChange(channelId: string) {
    if (channelId === selectedSourceChannelId && updatePreflight) {
      return;
    }

    selectedSourceChannelId = channelId;
    updatePreflight = null;
    flowError = '';

    if (!channelId) {
      return;
    }

    await hydrateUpdatePreflight(channelId);
  }

  async function handleBroadcastIdentityUpdate() {
    if (!session || !updateDetail || !updatePreflight) {
      return;
    }

    updateSubmitting = true;
    flowError = '';

    try {
      const sendResult = await identityService.sendIdentityUpdate({
        preflightId: updatePreflight.preflightId
      });

      responseDraft = {
        signer:
          responseDraft?.signer ?? {
            systemId:
              updateDetail.sourceSystemId?.trim() ||
              (session.testnet ? TESTNET_SYSTEM_ID : MAINNET_SYSTEM_ID),
            identityId: updateDetail.targetIdentityAddress
          },
        authentication: responseDraft?.authentication ?? null,
        identityUpdate: {
          requestId: updateDetail.requestId,
          txid: sendResult.txid
        }
      };
      completionMessage = '';
      currentStepId = 'complete';
    } catch (error) {
      setError(error, 'genericRequest.error.updateSend');
    } finally {
      updateSubmitting = false;
    }
  }

  async function deliverSignedResponse(signedResponseHex: string) {
    const postUri = session?.responseUris.find((item) => item.mode === 'post');
    const redirectUri = session?.responseUris.find((item) => item.mode === 'redirect');

    if (postUri) {
      const scheme = (() => {
        try {
          return new URL(postUri.uri).protocol;
        } catch {
          return '';
        }
      })();
      if (scheme !== 'http:' && scheme !== 'https:') {
        throw new Error('genericRequest.error.unsupportedPostCallback');
      }
      await genericRequestService.postGenericResponseCallback(postUri.uri, signedResponseHex);
      return;
    }

    if (redirectUri) {
      await genericRequestService.openGenericRequestCallback(redirectUri.uri, signedResponseHex);
    }
  }

  async function handleComplete() {
    if (!responseDraft) {
      isOpen = false;
      onClose();
      return;
    }

    await submitResponseDraft(responseDraft);
  }

  async function copyCompletionTxid() {
    if (!completionTxid) {
      return;
    }

    if (await writeClipboardText(completionTxid)) {
      copiedCompletionFieldState.set('txid');
    }
  }

  function resolveSystemDisplayLabel(systemId: string): string {
    const normalizedSystemId = systemId.trim();
    const normalizedSystemIdLc = normalizedSystemId.toLowerCase();

    if (normalizedSystemIdLc === MAINNET_SYSTEM_ID.toLowerCase()) {
      return coins.find((coin) => coin.id === 'VRSC')?.displayName?.trim() || 'Verus';
    }

    if (normalizedSystemIdLc === TESTNET_SYSTEM_ID.toLowerCase()) {
      return coins.find((coin) => coin.id === 'VRSCTEST')?.displayName?.trim() || 'Verus Testnet';
    }

    const systemCoin =
      coins.find(
        (coin) =>
          coin.systemId.toLowerCase() === normalizedSystemIdLc &&
          coin.currencyId.toLowerCase() === normalizedSystemIdLc
      ) ??
      coins.find((coin) => coin.systemId.toLowerCase() === normalizedSystemIdLc) ??
      null;

    const label =
      systemCoin?.displayName?.trim() ||
      systemCoin?.displayTicker?.trim() ||
      normalizedSystemId;
    if (label) {
      return label;
    }

    if (/^i[a-km-zA-HJ-NP-Z1-9]{24,60}$/.test(normalizedSystemId)) {
      return 'Verus';
    }

    return normalizedSystemId;
  }

  function outcomeCardClass(tone: 'primary' | 'info' | 'warning'): string {
    switch (tone) {
      case 'primary':
        return 'border-primary/30 bg-primary/8 dark:bg-primary/12';
      case 'warning':
        return 'border-amber-300/70 bg-amber-50/80 dark:border-amber-500/30 dark:bg-amber-950/20';
      default:
        return 'border-sky-300/60 bg-sky-50/80 dark:border-sky-500/30 dark:bg-sky-950/20';
    }
  }

  function badgeClass(tone: 'wallet' | 'external'): string {
    return tone === 'wallet'
      ? 'border-primary/30 bg-primary/10 text-primary dark:bg-primary/15'
      : 'border-sky-300/60 bg-sky-50/80 text-sky-700 dark:border-sky-500/30 dark:bg-sky-950/20 dark:text-sky-200';
  }

  function contentBadgeClass(tone: 'add' | 'remove' | 'info'): string {
    switch (tone) {
      case 'add':
        return 'border-emerald-300/70 bg-emerald-50/80 text-emerald-700 dark:border-emerald-500/30 dark:bg-emerald-950/25 dark:text-emerald-200';
      case 'remove':
        return 'border-amber-300/70 bg-amber-50/80 text-amber-700 dark:border-amber-500/30 dark:bg-amber-950/25 dark:text-amber-200';
      default:
        return 'border-sky-300/60 bg-sky-50/80 text-sky-700 dark:border-sky-500/30 dark:bg-sky-950/20 dark:text-sky-200';
    }
  }

  function contentBadgeIcon(changeType: IdentityUpdateContentItem['changeType']) {
    return changeType === 'removed' ? CircleMinusIcon : CirclePlusIcon;
  }

  function formatInspectValue(value: unknown): string {
    if (value === undefined || value === null) {
      return i18n.t('genericRequest.update.emptyValue');
    }
    if (typeof value === 'string') {
      return value;
    }

    try {
      return JSON.stringify(value, null, 2);
    } catch {
      return String(value);
    }
  }

  function openContentInspect(change: IdentityUpdateContentItem) {
    if (!change.isInspectable) return;
    selectedContentChange = change;
    contentInspectSheetOpen = true;
  }

  function authorityInfoTitle(): string {
    if (!updateReview) return i18n.t('genericRequest.update.authority.infoTitle');
    if (updateReview.authorityChanges.length > 1) {
      return i18n.t('genericRequest.update.authority.infoTitleBoth');
    }

    return updateReview.authorityChanges[0]?.id === 'recovery'
      ? i18n.t('genericRequest.update.authority.infoTitleRecovery')
      : i18n.t('genericRequest.update.authority.infoTitleRevocation');
  }

  function authorityInfoBody(): string {
    if (!updateReview) return i18n.t('genericRequest.update.authority.infoBody');
    if (updateReview.authorityChanges.length > 1) {
      return i18n.t('genericRequest.update.authority.infoBodyBoth');
    }

    return updateReview.authorityChanges[0]?.id === 'recovery'
      ? i18n.t('genericRequest.update.authority.infoBodyRecovery')
      : i18n.t('genericRequest.update.authority.infoBodyRevocation');
  }
</script>

{#if isOpen && session}
  <div class="absolute inset-0 z-50">
    {#if currentStepId === 'auth' && authDetail}
      <div class="bg-background relative flex h-full flex-col overflow-hidden">
        <div class="bg-app-canvas absolute inset-0"></div>
        <div class="absolute top-0 right-0 left-0 z-20 h-11" data-tauri-drag-region aria-hidden="true"></div>

        <div class="absolute top-0 right-0 z-30 flex h-[50px] items-center pr-4">
          <button
            type="button"
            class="ring-offset-background focus-visible:ring-ring inline-flex h-8 w-8 items-center justify-center rounded-xs opacity-70 transition-opacity hover:opacity-100 focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-hidden disabled:pointer-events-none"
            onclick={() => {
              isOpen = false;
              onClose();
            }}
            aria-label={i18n.t('common.cancel')}
          >
            <XIcon class="size-5" />
          </button>
        </div>

        <div class="relative z-10 flex min-h-0 flex-1 flex-col">
          <div class="min-h-0 flex-1 overflow-y-auto px-4 py-16 sm:px-6">
            <div class="mx-auto flex min-h-full w-full max-w-[560px] items-center">
              <div class="w-full space-y-8">
                <div class="space-y-6">
                  <div class="space-y-1.5">
                    <h1 class="text-2xl font-semibold tracking-tight text-foreground">
                      {i18n.t('genericRequest.auth.signInTo', { app: authAppLabel() })}
                    </h1>
                    {#if requesterIdentityLoading}
                      <Skeleton class="h-6 w-52 rounded-sm" />
                    {:else}
                      <p class="text-base font-medium text-foreground/70">
                        {i18n.t('genericRequest.auth.requestedBy', { requester: authRequesterLabel() })}
                      </p>
                    {/if}
                  </div>

                  {#if flowError}
                    <div class="rounded-2xl border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
                      <div class="flex items-start gap-2">
                        <AlertCircleIcon class="mt-0.5 size-4 shrink-0" />
                        <p>{flowError}</p>
                      </div>
                    </div>
                  {/if}

                  <div class="space-y-3">
                    <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      {i18n.t('genericRequest.auth.useIdentity')}
                    </p>
                    <button
                      type="button"
                      class={`group w-full rounded-2xl text-left transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 ${
                        activeAuthIdentity
                          ? 'bg-primary/12 hover:bg-primary/16 dark:bg-primary/18 dark:hover:bg-primary/22'
                          : 'bg-muted/40 hover:bg-muted/50 dark:bg-muted/28 dark:hover:bg-muted/38'
                      }`}
                      onclick={() => (authIdentitySheetOpen = true)}
                      disabled={loadingLinkedIdentities}
                    >
                      <div class="flex items-start justify-between gap-4 px-4 py-4 sm:px-5">
                        <div class="min-w-0 flex-1">
                          {#if loadingLinkedIdentities}
                            <div class="flex items-center gap-2 text-sm text-muted-foreground">
                              <Spinner class="size-4" />
                              <span>{i18n.t('genericRequest.auth.loading')}</span>
                            </div>
                          {:else if activeAuthIdentity}
                            <div class="flex items-start gap-3">
                              <div class="mt-0.5 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-background/70 text-primary shadow-sm dark:bg-background/40">
                                <ShieldCheckIcon class="size-5" />
                              </div>
                              <div class="min-w-0 flex-1">
                                <div class="flex items-start justify-between gap-3">
                                  <div class="min-w-0">
                                    <p class="truncate text-base font-semibold text-foreground">
                                      {formatIdentityDisplayName(activeAuthIdentity)}
                                    </p>
                                    <p class="identifier-text mt-1 truncate text-xs text-foreground/72">
                                      {activeAuthIdentity.identityAddress}
                                    </p>
                                  </div>

                                  <div class="flex shrink-0 items-center gap-1 text-sm font-medium text-primary">
                                    <span>{i18n.t('genericRequest.auth.changeSelection')}</span>
                                    <ChevronRightIcon class="size-4 transition-transform group-hover:translate-x-0.5" />
                                  </div>
                                </div>

                                <p class="mt-3 max-w-[32rem] text-sm text-foreground/72">
                                  {i18n.t('genericRequest.auth.selectedDescription')}
                                </p>
                              </div>
                            </div>
                          {:else}
                            <div class="flex items-start justify-between gap-3">
                              <div class="min-w-0">
                                <p class="text-base font-semibold text-foreground">
                                  {i18n.t('genericRequest.auth.selectIdentity')}
                                </p>
                                <p class="mt-1 max-w-[30rem] text-sm text-muted-foreground">
                                  {eligibleLinkedIdentities.length === 0
                                    ? i18n.t('genericRequest.auth.empty')
                                    : i18n.t('genericRequest.auth.noSelection')}
                                </p>
                              </div>

                              <div class="flex shrink-0 items-center gap-1 text-sm font-medium text-primary">
                                <span>{i18n.t('genericRequest.auth.selectIdentity')}</span>
                                <ChevronRightIcon class="size-4 transition-transform group-hover:translate-x-0.5" />
                              </div>
                            </div>
                          {/if}
                        </div>
                      </div>
                    </button>

                    <div class="flex flex-wrap items-center gap-2">
                      <Button variant="secondary" size="sm" onclick={openLinkIdentityManager} disabled={loadingLinkedIdentities}>
                        {i18n.t('genericRequest.auth.linkIdentity')}
                      </Button>
                      {#if provisioningDetail}
                        <Button
                          variant="outline"
                          size="sm"
                          onclick={handleProvisionIdentity}
                          disabled={provisioningSubmitting || !provisioningDetail.webhook}
                        >
                          {#if provisioningSubmitting}
                            <Spinner class="size-4" />
                          {/if}
                          {i18n.t('genericRequest.provisioning.cta')}
                        </Button>
                      {/if}
                    </div>

                    {#if provisioningNotice}
                      <p class="text-sm text-primary">{provisioningNotice}</p>
                    {/if}
                  </div>

                  <div class="space-y-3">
                    <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      {i18n.t('genericRequest.auth.detailsTitle')}
                    </p>
                    <div class="rounded-2xl bg-muted/25 p-4 dark:bg-muted/18">
                      <dl class="space-y-3 text-sm">
                        <div class="flex items-start justify-between gap-4">
                          <dt class="text-muted-foreground">{i18n.t('genericRequest.summary.network')}</dt>
                          <dd class="font-medium text-foreground">
                            {i18n.t(networkLocaleKey(session.testnet ? 'testnet' : 'mainnet'))}
                          </dd>
                        </div>

                        {#if completionNotice}
                          <div class="flex items-start justify-between gap-4">
                            <dt class="text-muted-foreground">{i18n.t('genericRequest.summary.destination')}</dt>
                            <dd class="text-right font-medium text-foreground">{responseUriHost(completionNotice)}</dd>
                          </div>
                        {/if}

                        {#if authDetail.expiryTime}
                          <div class="flex items-start justify-between gap-4">
                            <dt class="text-muted-foreground">{i18n.t('genericRequest.auth.expires')}</dt>
                            <dd class="text-right font-medium text-foreground">
                              {formatExpiryTime(authDetail.expiryTime)}
                            </dd>
                          </div>
                        {/if}
                      </dl>

                      {#if authDetail.constraints.length > 0}
                        <div class="mt-4 space-y-2 rounded-xl bg-background/45 px-3 py-3 dark:bg-background/25">
                          <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                            {i18n.t('genericRequest.auth.constraints')}
                          </p>
                          <ul class="space-y-2 text-sm">
                            {#each authDetail.constraints as constraint (`${constraint.type}:${constraint.identity}`)}
                              <li class="flex items-start justify-between gap-4">
                                <p class="text-muted-foreground">{authConstraintLabel(constraint.type)}</p>
                                <p class="identifier-text max-w-[16rem] break-all text-right font-medium text-foreground">
                                  {constraint.identity}
                                </p>
                              </li>
                            {/each}
                          </ul>
                        </div>
                      {/if}
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="border-black/10 bg-muted/10 dark:border-white/20 border-t">
            <div class="flex w-full items-center justify-between gap-4 px-4 py-4 sm:px-6">
              <Button variant="secondary" class="min-w-40 px-4 sm:min-w-48 sm:px-6" onclick={() => {
                isOpen = false;
                onClose();
              }}>
                {i18n.t('common.cancel')}
              </Button>

              <Button
                class="min-w-40 px-4 sm:min-w-48 sm:px-6"
                onclick={handleAuthContinue}
                disabled={completing || loadingLinkedIdentities || (!activeAuthIdentity && eligibleLinkedIdentities.length === 0)}
              >
                {#if completing}
                  <Spinner class="size-4" />
                {/if}
                {activeAuthIdentity
                  ? updateDetail
                    ? i18n.t('genericRequest.auth.continue')
                    : i18n.t('genericRequest.auth.submit')
                  : i18n.t('genericRequest.auth.selectIdentity')}
              </Button>
            </div>
          </div>
        </div>
      </div>
    {:else}
      <div class="bg-background h-full">
        <WalletTransferStepperShell
          currentStep={currentStepIndex}
          totalSteps={Math.max(visibleSteps.length, 1)}
          steps={visibleSteps}
          closeDisabled={finishStepLocked}
          onClose={() => {
            isOpen = false;
            onClose();
          }}
          showCloseButton={!finishStepLocked}
          showAside={false}
        >
          {#snippet footer()}
            <div class={`flex items-center gap-3 ${finishStepLocked ? 'justify-end' : 'justify-between'}`}>
              {#if !finishStepLocked}
                <Button variant="secondary" class="min-w-40 px-4 sm:min-w-48 sm:px-6" onclick={handleBack} disabled={updateSubmitting || completing}>
                  {currentStepIndex <= 1 ? i18n.t('common.cancel') : i18n.t('common.back')}
                </Button>
              {/if}

              {#if currentStepId === 'updateOverview' || currentStepId === 'updateContent'}
                <Button class="min-w-40 px-4 sm:min-w-48 sm:px-6" onclick={handleUpdateNext} disabled={updateAnalysisLoading || !updateReviewSource}>
                  {i18n.t('common.continue')}
                </Button>
              {:else if currentStepId === 'updateRisk'}
                <Button class="min-w-40 px-4 sm:min-w-48 sm:px-6" onclick={handleUpdateNext} disabled={!highRiskAcknowledged}>
                  {i18n.t('common.continue')}
                </Button>
              {:else if currentStepId === 'updateFunding'}
                <Button class="min-w-40 px-4 sm:min-w-48 sm:px-6" onclick={handleBroadcastIdentityUpdate} disabled={updateSubmitting || updateLoading || !updatePreflight}>
                  {#if updateSubmitting}
                    <Spinner class="size-4" />
                  {/if}
                  {i18n.t('genericRequest.update.broadcast')}
                </Button>
              {:else if currentStepId === 'complete'}
                <Button class="min-w-40 px-4 sm:min-w-48 sm:px-6" onclick={handleComplete} disabled={completing}>
                  {#if completing}
                    <Spinner class="size-4" />
                  {/if}
                  {i18n.t('genericRequest.complete.cta')}
                </Button>
              {/if}
            </div>
          {/snippet}

          <div class={`mx-auto w-full space-y-6 py-4 ${stepperContentWidthClass}`}>
            {#if flowError}
              <div class="rounded-xl border border-destructive/30 bg-destructive/10 p-4 text-sm text-destructive">
                <div class="flex items-start gap-2">
                  <AlertCircleIcon class="mt-0.5 size-4 shrink-0" />
                  <p>{flowError}</p>
                </div>
              </div>
            {/if}

            {#if currentStepId === 'updateOverview'}
          <div class="space-y-4">
            <div class="space-y-2">
              <h1 class="text-2xl font-semibold tracking-tight text-foreground">
                {i18n.t('genericRequest.update.title')}
              </h1>
              <p class="text-sm text-muted-foreground">
                {i18n.t('genericRequest.update.description')}
              </p>
            </div>

            {#if updateAnalysisLoading && !updateReviewSource}
              <div class="flex items-center gap-2 rounded-xl border border-border/70 bg-muted/20 p-4 text-sm text-muted-foreground">
                <Spinner class="size-4" />
                <span>{i18n.t('genericRequest.update.preflightLoading')}</span>
              </div>
            {:else if updateReviewSource && updateReview}
              <div class="rounded-2xl bg-muted/25 p-4 dark:bg-muted/18">
                <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  {i18n.t('genericRequest.summary.title')}
                </p>
                <dl class="mt-3 space-y-3 text-sm">
                  <div class="flex items-start justify-between gap-4">
                    <dt class="text-muted-foreground">{i18n.t('genericRequest.summary.network')}</dt>
                    <dd class="font-medium text-foreground">
                      {i18n.t(networkLocaleKey(session.testnet ? 'testnet' : 'mainnet'))}
                    </dd>
                  </div>

                  {#if session.signer.identityId}
                    <div class="flex items-start justify-between gap-4">
                      <dt class="text-muted-foreground">{i18n.t('genericRequest.summary.signer')}</dt>
                      <dd class="max-w-[22rem] text-right font-medium text-foreground">
                        <p>{authRequesterLabel()}</p>
                        {#if authRequesterLabel() !== session.signer.identityId}
                          <p class="identifier-text mt-1 break-all text-xs text-muted-foreground">
                            {session.signer.identityId}
                          </p>
                        {/if}
                      </dd>
                    </div>
                  {/if}

                  {#if completionNotice}
                    <div class="flex items-start justify-between gap-4">
                      <dt class="text-muted-foreground">{i18n.t('genericRequest.summary.destination')}</dt>
                      <dd class="max-w-[22rem] break-all text-right font-medium text-foreground">
                        {responseUriLabel(completionNotice)}
                      </dd>
                    </div>
                  {/if}
                </dl>
              </div>

              {#if provisioningDetail}
                <div class="rounded-2xl bg-muted/25 p-4 text-sm dark:bg-muted/18">
                  <div class="flex items-start gap-2">
                    <Link2Icon class="mt-0.5 size-4 shrink-0 text-primary" />
                    <div>
                      <p class="font-medium text-foreground">{i18n.t('genericRequest.summary.provisioningTitle')}</p>
                      <p class="mt-1 text-muted-foreground">
                        {provisioningDetail.requestedFqn || i18n.t('genericRequest.summary.provisioningRequested')}
                      </p>
                    </div>
                  </div>
                </div>
              {/if}

              <div class="grid gap-4 md:grid-cols-2">
                <div class="rounded-xl border border-border/70 bg-muted/20 p-4">
                  <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                    {i18n.t('genericRequest.update.targetIdentity')}
                  </p>
                  <p class="mt-2 text-sm font-semibold text-foreground">
                    {updateReviewSource.fullyQualifiedName || updateReviewSource.targetIdentity}
                  </p>
                </div>

                <div class="rounded-xl border border-border/70 bg-muted/20 p-4">
                  <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                    {i18n.t('genericRequest.update.summary')}
                  </p>
                  <div class="mt-2 flex items-center gap-6">
                    <div>
                      <p class="text-xl font-semibold text-foreground">{updateReview.highRiskCount}</p>
                      <p class="text-xs text-muted-foreground">{i18n.t('genericRequest.update.highRiskCount')}</p>
                    </div>
                    <div>
                      <p class="text-xl font-semibold text-foreground">{updateReview.contentCount}</p>
                      <p class="text-xs text-muted-foreground">{i18n.t('genericRequest.update.contentCount')}</p>
                    </div>
                  </div>
                </div>
              </div>

              {#if updateReviewSource.warnings.length > 0}
                <div class="rounded-xl border border-border/70 bg-muted/20 p-4">
                  <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                    {i18n.t('genericRequest.update.warnings')}
                  </p>
                  <ul class="mt-3 space-y-2 text-sm text-foreground">
                    {#each updateReviewSource.warnings as warning (`${warning.warningType}:${warning.message}`)}
                      <li class="rounded-lg bg-background/80 px-3 py-2">
                        <p class="font-medium">{warning.warningType}</p>
                        <p class="mt-1 text-muted-foreground">{warning.message}</p>
                      </li>
                    {/each}
                  </ul>
                </div>
              {/if}
            {/if}
          </div>
        {:else if currentStepId === 'updateRisk' && updateReviewSource && updateReview}
          <div class="space-y-4">
            <div class="space-y-2">
              <h1 class="text-2xl font-semibold tracking-tight text-foreground">
                {i18n.t('genericRequest.update.highRiskTitle')}
              </h1>
              <p class="text-sm text-muted-foreground">
                {updateReview.isContentClearOnly
                  ? i18n.t('genericRequest.update.highRiskDescriptionContentClear')
                  : i18n.t('genericRequest.update.highRiskDescription')}
              </p>
            </div>

            <div class="space-y-3">
              {#if updateReview.outcome && !updateReview.isAuthorityOnly && !updateReview.isContentClearOnly}
                <div class={`rounded-xl border p-4 ${outcomeCardClass(updateReview.outcome.tone)}`}>
                  <div class="flex items-start gap-3">
                    <ShieldAlertIcon class="mt-0.5 size-5 shrink-0 text-foreground" />
                    <div class="space-y-1">
                      <p class="text-sm font-semibold text-foreground">{updateReview.outcome.title}</p>
                      <p class="text-sm text-muted-foreground">{updateReview.outcome.description}</p>
                    </div>
                  </div>
                </div>
              {/if}

              {#if updateReview.authorityChanges.length > 0}
                <div class="rounded-xl border border-border/70 bg-muted/20 p-4">
                  <div class="flex items-start justify-between gap-3">
                    <div>
                      <p class="text-sm font-semibold text-foreground">{i18n.t('genericRequest.update.authority.sectionTitle')}</p>
                      <p class="mt-1 text-sm text-muted-foreground">{i18n.t('genericRequest.update.authority.sectionDescription')}</p>
                    </div>
                    <button
                      type="button"
                      class="inline-flex items-center gap-1 rounded-md border border-border/70 px-2.5 py-1.5 text-xs font-medium text-muted-foreground transition-colors hover:text-foreground"
                      onclick={() => (authorityInfoExpanded = !authorityInfoExpanded)}
                    >
                      <CircleHelpIcon class="size-3.5" />
                      <span>{i18n.t('genericRequest.update.authority.learnMore')}</span>
                    </button>
                  </div>

                  {#if authorityInfoExpanded}
                    <div class="mt-3 rounded-lg border border-border/70 bg-background/80 p-3 text-sm">
                      <p class="font-medium text-foreground">{authorityInfoTitle()}</p>
                      <p class="mt-1 text-muted-foreground">{authorityInfoBody()}</p>
                    </div>
                  {/if}

                  <div class="mt-4 space-y-3">
                    {#each updateReview.authorityChanges as change (change.id)}
                      <div class="rounded-lg bg-background/80 p-3">
                        <div class="flex items-center gap-2">
                          <ShieldAlertIcon class="size-4 shrink-0 text-sky-700 dark:text-sky-200" />
                          <p class="text-sm font-semibold text-foreground">{change.title}</p>
                        </div>
                        <div class="mt-3 flex gap-3">
                          <div class="flex w-4 shrink-0 flex-col items-center pt-0.5">
                            <div class="h-full w-px bg-border"></div>
                            <ArrowDownIcon class="my-1 size-3.5 text-muted-foreground" />
                            <div class="h-full w-px bg-border"></div>
                          </div>
                          <div class="min-w-0 flex-1 space-y-2">
                            {#if change.currentValue}
                              <div>
                                <p class="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
                                  {i18n.t('genericRequest.update.currentValue')}
                                </p>
                                <p
                                  class="identifier-text truncate text-sm text-muted-foreground"
                                  title={change.currentValue}
                                >
                                  {truncateIdentityAddress(change.currentValue, 14, 12)}
                                </p>
                              </div>
                            {/if}
                            <div>
                              <p class="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
                                {i18n.t('genericRequest.update.newValue')}
                              </p>
                              <p class="identifier-text break-all text-sm font-medium text-foreground">{change.nextValue}</p>
                            </div>
                          </div>
                        </div>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}

              {#if updateReview.primaryAddressChanges.length > 0}
                <div class="rounded-xl border border-border/70 bg-muted/20 p-4">
                  <p class="text-sm font-semibold text-foreground">{i18n.t('genericRequest.update.primaryAddress.sectionTitle')}</p>
                  <div class="mt-4 space-y-3">
                    {#each updateReview.primaryAddressChanges as change (change.id)}
                      <div class="rounded-lg bg-background/80 p-3">
                        <div class="flex items-start justify-between gap-3">
                          <div class="space-y-1">
                            <p class="text-sm font-semibold text-foreground">{change.title}</p>
                            <p class="text-sm text-muted-foreground">{change.description}</p>
                          </div>
                          {#if change.badge}
                            <span class={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-medium ${badgeClass(change.badge.tone)}`}>
                              {#if change.badge.tone === 'wallet'}
                                <WalletIcon class="size-3.5" />
                              {:else}
                                <GlobeIcon class="size-3.5" />
                              {/if}
                              <span>{change.badge.label}</span>
                            </span>
                          {/if}
                        </div>
                        <p class="identifier-text mt-3 break-all text-sm text-foreground">{change.address}</p>
                      </div>
                    {/each}
                  </div>

                  <div class="mt-4 rounded-lg bg-background/80 p-3">
                    <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      {i18n.t('genericRequest.update.primaryAddress.afterUpdateTitle')}
                    </p>
                    <div class="mt-3 space-y-2">
                      {#each updateReview.primaryAddressesAfterUpdate as entry (entry.address)}
                        <div class="flex items-center justify-between gap-3 rounded-lg border border-border/70 px-3 py-2">
                          <p class="identifier-text min-w-0 flex-1 truncate text-sm text-foreground" title={entry.displayAddress}>
                            {entry.displayAddress}
                          </p>
                          <span class={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-medium ${badgeClass(entry.badge.tone)}`}>
                            {#if entry.badge.tone === 'wallet'}
                              <WalletIcon class="size-3.5" />
                            {:else}
                              <GlobeIcon class="size-3.5" />
                            {/if}
                            <span>{entry.badge.label}</span>
                          </span>
                        </div>
                      {/each}
                    </div>
                  </div>
                </div>
              {/if}

              {#each updateReview.otherHighRiskItems as item (item.id)}
                <div class="rounded-xl border border-amber-300/70 bg-amber-50/80 p-4 dark:border-amber-500/30 dark:bg-amber-950/20">
                  <p class="text-sm font-semibold text-foreground">{item.title}</p>
                  <p class="mt-2 text-sm text-muted-foreground">{item.description}</p>
                  {#if item.currentValue}
                    <p class="mt-3 text-xs text-muted-foreground">
                      {i18n.t('genericRequest.update.beforeValue')}: <span class="identifier-text">{item.currentValue}</span>
                    </p>
                  {/if}
                  {#if item.nextValue}
                    <p class="mt-1 text-xs text-muted-foreground">
                      {i18n.t('genericRequest.update.afterValue')}: <span class="identifier-text">{item.nextValue}</span>
                    </p>
                  {/if}
                </div>
              {/each}
            </div>

            <Label for="high-risk-acknowledge" class="flex items-start gap-3 rounded-xl border border-border/70 bg-muted/20 p-4">
              <Checkbox
                id="high-risk-acknowledge"
                checked={highRiskAcknowledged}
                onCheckedChange={(checked) => (highRiskAcknowledged = checked === true)}
              />
              <span class="text-sm text-foreground">{i18n.t('genericRequest.update.highRiskAcknowledge')}</span>
            </Label>
          </div>
        {:else if currentStepId === 'updateContent' && updateReviewSource && updateReview}
          <div class="space-y-4">
            <div class="space-y-2">
              <h1 class="text-2xl font-semibold tracking-tight text-foreground">
                {i18n.t('genericRequest.update.contentTitle')}
              </h1>
              <p class="text-sm text-muted-foreground">
                {i18n.t('genericRequest.update.contentDescription')}
              </p>
            </div>

            <div class="space-y-3">
              {#each updateReview.contentChanges as change (change.id)}
                <div class="rounded-xl border border-border/70 bg-muted/20 p-4">
                  <div class="flex items-start justify-between gap-3">
                    <p class="text-sm font-semibold text-foreground">{change.title}</p>
                    {#if change.isInspectable}
                      <InlineTextActionButton
                        class="shrink-0"
                        onclick={() => openContentInspect(change)}
                      >
                        <EyeIcon class="size-3.5" />
                        <span>{i18n.t('genericRequest.update.content.viewDetails')}</span>
                      </InlineTextActionButton>
                    {/if}
                  </div>

                  {#if change.badges.length > 0}
                    <div class="mt-3 flex flex-wrap items-center gap-2">
                      {#each change.badges as badge (`${change.id}:${badge.id}`)}
                        {@const BadgeIcon = contentBadgeIcon(change.changeType)}
                        <span class={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-medium ${contentBadgeClass(badge.tone)}`}>
                          <BadgeIcon class="size-3.5" />
                          <span>{badge.label}</span>
                        </span>
                      {/each}
                    </div>
                  {/if}

                  <div class="mt-3 space-y-2">
                    {#if change.currentLabel}
                      <div class="rounded-lg border border-border/70 bg-background/80 px-3 py-3">
                        <p class="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
                          {change.currentLabel}
                        </p>
                        <p class="mt-2 whitespace-pre-wrap break-words text-sm text-foreground">
                          {change.currentPreview || i18n.t('genericRequest.update.emptyValue')}
                        </p>
                      </div>
                    {/if}

                    {#if change.nextLabel}
                      <div class={`rounded-lg border px-3 py-3 ${
                        change.changeType === 'removed'
                          ? 'border-amber-300/60 bg-amber-50/60 dark:border-amber-500/30 dark:bg-amber-950/20'
                          : change.changeType === 'updated'
                            ? 'border-sky-300/60 bg-sky-50/60 dark:border-sky-500/30 dark:bg-sky-950/20'
                            : 'border-emerald-300/60 bg-emerald-50/60 dark:border-emerald-500/30 dark:bg-emerald-950/20'
                      }`}>
                        <p class="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
                          {change.nextLabel}
                        </p>
                        <p class="mt-2 whitespace-pre-wrap break-words text-sm text-foreground">
                          {change.nextPreview || i18n.t('genericRequest.update.emptyValue')}
                        </p>
                      </div>
                    {/if}
                  </div>

                  {#if change.detailBody || change.effectNote || change.historyNote}
                    <div class="mt-3 space-y-2 rounded-lg bg-background/80 px-3 py-3 text-sm text-muted-foreground">
                      {#if change.detailBody}
                        <p>{change.detailBody}</p>
                      {/if}
                      {#if change.effectNote}
                        <p>{change.effectNote}</p>
                      {/if}
                      {#if change.historyNote}
                        <p>{change.historyNote}</p>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {:else if currentStepId === 'updateFunding'}
          <div class="space-y-6">
            <div class="space-y-2">
              <h1 class="text-2xl font-semibold tracking-tight text-foreground">
                {i18n.t('genericRequest.update.fundingTitle')}
              </h1>
              <p class="text-sm text-muted-foreground">
                {i18n.t('genericRequest.update.fundingDescription')}
              </p>
            </div>

            <div class="rounded-2xl bg-muted/25 p-4 dark:bg-muted/18">
              <p class="block text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                {i18n.t('genericRequest.update.fundingSource')}
              </p>

              {#if fundingSourcesLoading}
                <div class="mt-3 flex items-center gap-2 rounded-lg bg-background/80 px-3 py-3 text-sm text-muted-foreground">
                  <Spinner class="size-4" />
                  <span>{i18n.t('genericRequest.update.fundingSourcesLoading')}</span>
                </div>
              {:else if fundingSources.length === 0}
                <div class="mt-3 rounded-xl border border-dashed border-border/70 bg-background/70 px-4 py-4 text-sm text-muted-foreground">
                  <p>{i18n.t('genericRequest.update.fundingEmpty')}</p>
                </div>
              {:else if fundingSources.length === 1 && selectedFundingSource}
                <div class="mt-3 rounded-2xl bg-primary/12 dark:bg-primary/18">
                  <div class="flex items-start justify-between gap-4 px-4 py-4 sm:px-5">
                    <div class="min-w-0 flex-1">
                      <div class="flex items-start gap-3">
                        <div class="mt-0.5 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-background/70 text-primary shadow-sm dark:bg-background/40">
                          <WalletIcon class="size-5" />
                        </div>
                        <div class="min-w-0 flex-1">
                          <div class="flex items-start justify-between gap-3">
                            <div class="min-w-0">
                              <p class="truncate text-base font-semibold text-foreground">
                                {fundingSourcePrimaryLabel(selectedFundingSource)}
                              </p>
                              <p class="identifier-text mt-1 truncate text-xs text-foreground/72">
                                {fundingSourceSecondaryLabel(selectedFundingSource)}
                              </p>
                            </div>

                            <p class="shrink-0 text-sm font-medium text-primary">
                              {selectedFundingSource.balanceDisplay}
                            </p>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              {:else}
                <div class="mt-3 space-y-3">
                  {#each fundingSources as source (source.channelId)}
                    <button
                      type="button"
                      class={`group w-full rounded-2xl text-left transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 ${
                        selectedSourceChannelId === source.channelId
                          ? 'bg-primary/12 hover:bg-primary/16 dark:bg-primary/18 dark:hover:bg-primary/22'
                          : 'bg-muted/40 hover:bg-muted/50 dark:bg-muted/28 dark:hover:bg-muted/38'
                      }`}
                      aria-pressed={selectedSourceChannelId === source.channelId}
                      onclick={() => void handleFundingSourceChange(source.channelId)}
                    >
                      <div class="flex items-start justify-between gap-4 px-4 py-4 sm:px-5">
                        <div class="min-w-0 flex-1">
                          <div class="flex items-start gap-3">
                            <div class="mt-0.5 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-background/70 text-primary shadow-sm dark:bg-background/40">
                              <WalletIcon class="size-5" />
                            </div>
                            <div class="min-w-0 flex-1">
                              <div class="flex items-start justify-between gap-3">
                                <div class="min-w-0">
                                  <p class="truncate text-base font-semibold text-foreground">
                                    {fundingSourcePrimaryLabel(source)}
                                  </p>
                                  <p class="identifier-text mt-1 truncate text-xs text-foreground/72">
                                    {fundingSourceSecondaryLabel(source)}
                                  </p>
                                </div>

                                <p class="shrink-0 text-sm font-medium text-primary">
                                  {source.balanceDisplay}
                                </p>
                              </div>
                            </div>
                          </div>
                        </div>
                      </div>
                    </button>
                  {/each}
                </div>
              {/if}
            </div>

            <div class="rounded-2xl bg-muted/25 p-4 dark:bg-muted/18">
              <p class="block text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                {i18n.t('genericRequest.update.feeTitle')}
              </p>

              {#if updateLoading}
                <div class="mt-4 flex items-center gap-2 rounded-lg bg-background/80 px-3 py-3 text-sm text-muted-foreground">
                  <Spinner class="size-4" />
                  <span>{i18n.t('genericRequest.update.fundingCalculating')}</span>
                </div>
              {:else if updatePreflight}
                <div class="mt-4 rounded-lg bg-background/80 px-3 py-3 text-sm">
                  <p class="font-medium text-foreground">
                    {i18n.t('genericRequest.update.feeLabel', {
                      fee: updatePreflight.fee,
                      currency: resolveSystemDisplayLabel(updatePreflight.feeCurrency)
                    })}
                  </p>
                </div>
              {:else if fundingSources.length > 0}
                <div class="mt-4 rounded-lg bg-background/80 px-3 py-3 text-sm text-muted-foreground">
                  <p>{i18n.t('genericRequest.update.fundingFeePlaceholder')}</p>
                </div>
              {/if}
            </div>
          </div>
        {:else if currentStepId === 'complete'}
          <div class="space-y-4">
            <div class="space-y-2">
              <h1 class="text-2xl font-semibold tracking-tight text-foreground">
                {i18n.t('genericRequest.complete.title')}
              </h1>
              <p class="text-sm text-muted-foreground">
                {session.responseUris.length > 0
                  ? i18n.t('genericRequest.complete.descriptionWithCallback')
                  : i18n.t('genericRequest.complete.description')}
              </p>
            </div>

            <div class="rounded-xl border border-border/70 bg-muted/20 p-4">
              <div class="flex items-start gap-3">
                <CheckCircle2Icon class="mt-0.5 size-5 shrink-0 text-primary" />
                <div class="space-y-2">
                  <p class="text-sm font-medium text-foreground">
                    {completionMessage || i18n.t('genericRequest.complete.ready')}
                  </p>
                  {#if session.responseUris.length > 0}
                    <p class="text-sm text-muted-foreground">
                      {i18n.t('genericRequest.complete.deliveryNotice', {
                        destination: responseUriLabel(completionNotice)
                      })}
                    </p>
                  {/if}
                </div>
              </div>
            </div>

            {#if completionTxid}
              <div class="rounded-xl border border-border/70 bg-muted/20 p-4">
                <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  {i18n.t('genericRequest.complete.txidLabel')}
                </p>
                <div class="mt-2 flex items-start justify-between gap-2">
                  <p class="identifier-text min-w-0 flex-1 break-all text-sm font-medium text-foreground">{completionTxid}</p>
                  <CopyButton
                    copied={copiedCompletionField === 'txid'}
                    size="xs"
                    class="mt-0.5"
                    iconClass="size-3"
                    copiedIconClass="size-3 text-emerald-600 dark:text-emerald-400"
                    onclick={() => void copyCompletionTxid()}
                    title={i18n.t('wallet.receive.copy')}
                    aria-label={i18n.t('wallet.receive.copy')}
                  />
                </div>
              </div>
            {/if}

            {#if completionNotice}
              <div class="inline-flex items-center gap-2 text-sm text-muted-foreground">
                <ExternalLinkIcon class="size-4" />
                <span>{responseUriLabel(completionNotice)}</span>
              </div>
            {/if}
          </div>
            {/if}
          </div>
        </WalletTransferStepperShell>
      </div>
    {/if}

    <StandardRightSheet
      bind:isOpen={authIdentitySheetOpen}
      title={i18n.t('genericRequest.auth.selectIdentity')}
      onOpenAutoFocus={handleAuthSheetOpenAutoFocus}
    >
      <div class="flex h-full min-h-0 flex-col">
        <p class="pr-8 text-sm text-muted-foreground">
          {i18n.t('genericRequest.auth.selectorDescription')}
        </p>

        <div class="mt-4 flex min-h-0 flex-1 flex-col">
          <ScrollArea.Root class="min-h-0 flex-1">
            <ScrollArea.Viewport class="h-full pr-1">
              {#if loadingLinkedIdentities}
                <div class="mt-2 flex items-center gap-2 rounded-lg bg-muted/55 px-3 py-3 text-sm text-muted-foreground dark:bg-muted/50">
                  <Spinner class="size-4" />
                  <span>{i18n.t('genericRequest.auth.loading')}</span>
                </div>
              {:else if eligibleLinkedIdentities.length === 0}
                <p class="mt-2 rounded-lg bg-muted/55 px-3 py-3 text-sm text-muted-foreground dark:bg-muted/50">
                  {i18n.t('genericRequest.auth.empty')}
                </p>
              {:else}
                <ul class="mt-2 space-y-2 pb-4">
                  {#each eligibleLinkedIdentities as identity (identity.identityAddress)}
                    <li>
                      <button
                        type="button"
                        class={`w-full rounded-lg px-3.5 py-3 text-left transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 ${
                          selectedAuthIdentityAddress === identity.identityAddress
                            ? 'bg-primary/14 hover:bg-primary/20 dark:bg-primary/28 dark:hover:bg-primary/36'
                            : 'bg-muted/65 hover:bg-muted/70 dark:bg-muted/55 dark:hover:bg-muted/65'
                        }`}
                        onclick={() => {
                          selectedAuthIdentityAddress = identity.identityAddress;
                          authIdentitySheetOpen = false;
                        }}
                      >
                        <div class="flex items-center justify-between gap-3">
                          <div class="min-w-0">
                            <p class="truncate text-sm font-semibold text-foreground">
                              {formatIdentityDisplayName(identity)}
                            </p>
                            <p class="identifier-text mt-1 truncate text-xs text-muted-foreground">
                              {identity.identityAddress}
                            </p>
                          </div>

                          {#if selectedAuthIdentityAddress === identity.identityAddress}
                            <ShieldCheckIcon class="size-4 shrink-0 text-primary" />
                          {:else}
                            <ChevronRightIcon class="size-4 shrink-0 text-muted-foreground" />
                          {/if}
                        </div>
                      </button>
                    </li>
                  {/each}
                </ul>
              {/if}
            </ScrollArea.Viewport>
            <ScrollArea.Scrollbar orientation="vertical" />
          </ScrollArea.Root>
        </div>

        <div class="mt-3 flex flex-col gap-2 pt-2">
          <Button variant="secondary" onclick={openLinkIdentityManager} disabled={loadingLinkedIdentities}>
            {i18n.t('genericRequest.auth.linkIdentity')}
          </Button>
          {#if provisioningDetail}
            <Button
              variant="outline"
              onclick={handleProvisionIdentity}
              disabled={provisioningSubmitting || !provisioningDetail.webhook}
            >
              {#if provisioningSubmitting}
                <Spinner class="size-4" />
              {/if}
              {i18n.t('genericRequest.provisioning.cta')}
            </Button>
          {/if}
        </div>
      </div>
    </StandardRightSheet>

    <StandardRightSheet
      bind:isOpen={contentInspectSheetOpen}
      title={selectedContentChange?.inspectTitle || i18n.t('genericRequest.update.content.viewDetails')}
    >
      {#if selectedContentChange}
        <div class="flex min-h-0 flex-1 flex-col">
          <ScrollArea.Root class="min-h-0 flex-1">
            <ScrollArea.Viewport class="h-full pr-1">
              <div class="space-y-4 pb-4">
                <div class="rounded-xl border border-border/70 bg-muted/20 p-4">
                  <p class="text-sm font-semibold text-foreground">{selectedContentChange.title}</p>
                  {#if selectedContentChange.badges.length > 0}
                    <div class="mt-3 flex flex-wrap items-center gap-2">
                      {#each selectedContentChange.badges as badge (`inspect:${selectedContentChange.id}:${badge.id}`)}
                        {@const BadgeIcon = contentBadgeIcon(selectedContentChange.changeType)}
                        <span class={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-medium ${contentBadgeClass(badge.tone)}`}>
                          <BadgeIcon class="size-3.5" />
                          <span>{badge.label}</span>
                        </span>
                      {/each}
                    </div>
                  {/if}
                </div>

                {#if selectedContentChange.inspectPayload?.current !== undefined}
                  <div class="space-y-2">
                    <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      {selectedContentChange.currentLabel || i18n.t('genericRequest.update.currentValue')}
                    </p>
                    <pre class="overflow-x-auto whitespace-pre-wrap rounded-xl border border-border/70 bg-background/80 px-3 py-3 text-xs text-foreground">{formatInspectValue(selectedContentChange.inspectPayload.current)}</pre>
                  </div>
                {/if}

                {#if selectedContentChange.inspectPayload?.requested !== undefined}
                  <div class="space-y-2">
                    <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      {selectedContentChange.nextLabel || i18n.t('genericRequest.update.content.requestedValue')}
                    </p>
                    <pre class="overflow-x-auto whitespace-pre-wrap rounded-xl border border-border/70 bg-background/80 px-3 py-3 text-xs text-foreground">{formatInspectValue(selectedContentChange.inspectPayload.requested)}</pre>
                  </div>
                {/if}

                {#if selectedContentChange.detailBody || selectedContentChange.effectNote || selectedContentChange.historyNote}
                  <div class="space-y-2 rounded-xl border border-border/70 bg-muted/20 p-4 text-sm text-muted-foreground">
                    {#if selectedContentChange.detailBody}
                      <p>{selectedContentChange.detailBody}</p>
                    {/if}
                    {#if selectedContentChange.effectNote}
                      <p>{selectedContentChange.effectNote}</p>
                    {/if}
                    {#if selectedContentChange.historyNote}
                      <p>{selectedContentChange.historyNote}</p>
                    {/if}
                  </div>
                {/if}
              </div>
            </ScrollArea.Viewport>
            <ScrollArea.Scrollbar orientation="vertical" />
          </ScrollArea.Root>
        </div>
      {/if}
    </StandardRightSheet>

    <LinkIdentitySheet
      bind:isOpen={linkSheetOpen}
      onLinkedChange={(updated) => {
        linkedIdentities = updated;
        applyPassthroughSelection(updated);
        void (async () => {
          identityDetailsByAddress = await loadIdentityDetailsMap(updated);
        })();
      }}
      allowManualLinkEntry={session.testnet}
    />
  </div>
{/if}
