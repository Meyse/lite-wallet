<script lang="ts">
  import { onDestroy, untrack } from 'svelte';
  import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import { Button } from '$lib/components/ui/button';
  import { CopyButton } from '$lib/components/ui/copy-button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Spinner } from '$lib/components/ui/spinner';
  import LinkIdentitySheet from '$lib/components/wallet/sections/identity/LinkIdentitySheet.svelte';
  import { contactChainId } from '$lib/contacts/identity';
  import { contactSession } from '$lib/contacts/session';
  import {
    ApprovalRun,
    type ApprovalStageId,
    approvalStages,
    DETAIL_IMPACT,
    expiresSoon,
    planRequestFlow,
    type RequestStepId,
    timeLeft,
  } from '$lib/genericRequest/flowPlan';
  import { buildGenericIdentityUpdateReview } from '$lib/genericRequest/identityUpdateReview';
  import { submitGenericProvisioningRequest } from '$lib/genericRequest/provisioning';
  import type {
    AuthenticationDetailSession,
    GenericRequestFlowSession,
    IdentityUpdateDetailSession,
    ProvisionIdentityDetailSession,
  } from '$lib/genericRequest/session';
  import { i18nStore, networkLocaleKey } from '$lib/i18n';
  import * as genericRequestService from '$lib/services/genericRequestService.js';
  import * as identityLinkService from '$lib/services/identityLinkService.js';
  import { isForcedWalletLockError } from '$lib/services/walletLockCoordinator.js';
  import * as walletService from '$lib/services/walletService.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { walletChannelsStore } from '$lib/stores/walletChannels.js';
  import type { ContactIdentity } from '$lib/types/addressBook';
  import type {
    BalanceResult,
    BuildAndSignGenericResponseResult,
    CoinDefinition,
    CoinScope,
    GenericIdentityUpdatePreflightResult,
    GenericIdentityUpdateReviewResult,
    IdentityDetails,
    LinkedIdentity,
  } from '$lib/types/wallet.js';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import {
    formatIdentityDisplayName,
    formatIdentityFullyQualifiedName,
  } from '$lib/utils/identityDisplay.js';
  import { extractWalletErrorMessage, extractWalletErrorType } from '$lib/utils/walletErrors.js';
  import { rejectionCode } from '$lib/genericRequest/rejections';
  import ConfirmPlan from './ConfirmPlan.svelte';
  import DeliveryLine from './DeliveryLine.svelte';
  import NetworkFeePicker from './NetworkFeePicker.svelte';
  import RequesterHeader from './RequesterHeader.svelte';
  import RequestFacts from './RequestFacts.svelte';
  import RequestFrame from './RequestFrame.svelte';
  import RequestOutcome from './RequestOutcome.svelte';
  import RequestRail from './RequestRail.svelte';
  import ResponderPicker from './ResponderPicker.svelte';
  import type {
    FundingSource,
    PlanLine,
    RailStep,
    RequesterView,
    RequestFact,
    StageView,
  } from './types';
  import UpdateChanges from './UpdateChanges.svelte';
  import PayInvoiceStep from './PayInvoiceStep.svelte';
  import { PayInvoiceModule } from './modules/PayInvoiceModule.svelte';
  import { SignDataModule } from './modules/SignDataModule.svelte';
  import { EncryptionKeyModule } from './modules/EncryptionKeyModule.svelte';
  import { ShareDataModule } from './modules/ShareDataModule.svelte';
  import { GiftCardModule } from './modules/GiftCardModule.svelte';
  import { legacyLoginWebhookBody } from '$lib/genericRequest/legacy';
  import EncryptionKeyStep from './EncryptionKeyStep.svelte';
  import ShareDataStep from './ShareDataStep.svelte';
  import GiftCardStep from './GiftCardStep.svelte';
  import SignDataStep from './SignDataStep.svelte';
  import type { DetailModule, RequestContext } from './modules/types';

  type Phase = 'review' | 'working' | 'done' | 'failed' | 'declined';

  const MAINNET_SYSTEM_ID = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
  const TESTNET_SYSTEM_ID = 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq';
  const AUTH_REQUIRED_ID = 1;
  const AUTH_REQUIRED_SYSTEM = 2;
  const AUTH_REQUIRED_PARENT = 3;
  // The backend keeps a fee preflight for 15 minutes. Refresh it on Confirm
  // when it is older than this, so Approval never uses an expired one.
  const PREFLIGHT_REFRESH_MS = 12 * 60 * 1000;

  let {
    isOpen = $bindable(false),
    approved = $bindable(false),
    session,
    onClose = () => {},
    onSetupPrivate,
  }: {
    isOpen?: boolean;
    /** True from Approval until the request closes; a second link then waits. */
    approved?: boolean;
    session: GenericRequestFlowSession | null;
    onClose?: () => void;
    /** Leaves the request for the private address setup. */
    onSetupPrivate?: () => void;
  } = $props();

  const i18n = $derived($i18nStore);
  const coins = $derived($coinsStore);
  const walletChannels = $derived($walletChannelsStore);
  const authDetail = $derived(
    (session?.details.find((detail) => detail.type === 'authentication') as
      AuthenticationDetailSession | undefined) ?? null
  );
  const updateDetail = $derived(
    (session?.details.find((detail) => detail.type === 'identityUpdate') as
      IdentityUpdateDetailSession | undefined) ?? null
  );
  const provisioningDetail = $derived(
    (session?.details.find((detail) => detail.type === 'provisionIdentity') as
      ProvisionIdentityDetailSession | undefined) ?? null
  );

  // Review state. Nothing here is signed, broadcast, or sent (ADR 0001).
  let linkedIdentities = $state<LinkedIdentity[]>([]);
  let identityDetailsByAddress = $state<Record<string, IdentityDetails>>({});
  let loadingLinkedIdentities = $state(false);
  let responderAddress = $state('');
  let newIdentity = $state(false);
  let linkSheetOpen = $state(false);
  let requesterIdentityDetails = $state<IdentityDetails | null>(null);
  let updateAnalysis = $state<GenericIdentityUpdateReviewResult | null>(null);
  let updateAnalysisLoading = $state(false);
  let updatePreflight = $state<GenericIdentityUpdatePreflightResult | null>(null);
  let preflightAt = 0;
  let updateLoading = $state(false);
  let fundingSources = $state<FundingSource[]>([]);
  let fundingSourcesLoading = $state(false);
  let fundingSourcesLoadedKey = $state('');
  let selectedSourceChannelId = $state('');
  let highRiskAcknowledged = $state(false);
  let stepIndex = $state(0);
  let flowError = $state('');
  let nowSeconds = $state(Math.floor(Date.now() / 1000));
  let fundingSourceLoadSequence = 0;
  let updatePreflightRequestSequence = 0;

  // Detail modules: each new detail type plugs into the shell (plan step 7).
  let invoiceModule = $state.raw<PayInvoiceModule | null>(null);
  let signDataModule = $state.raw<SignDataModule | null>(null);
  let encryptionKeyModule = $state.raw<EncryptionKeyModule | null>(null);
  let shareDataModule = $state.raw<ShareDataModule | null>(null);
  let giftCardModule = $state.raw<GiftCardModule | null>(null);
  const modules = $derived<DetailModule[]>(
    [invoiceModule, signDataModule, encryptionKeyModule, shareDataModule, giftCardModule].filter(
      (module) => module !== null
    )
  );
  const transactionModule = $derived(modules.find((module) => module.transaction) ?? null);
  /** The response carries a detail; invoices and New VerusID add none, as on mobile. */
  const respondsWithDetails = $derived(
    !!authDetail ||
      !!updateDetail ||
      !!signDataModule ||
      !!encryptionKeyModule ||
      (shareDataModule?.count ?? 0) > 0
  );
  /** Details that the chosen responder answers: a sign in, signing or sharing data, or a key. */
  const needsResponder = $derived(
    !!authDetail || !!signDataModule || !!encryptionKeyModule || !!shareDataModule
  );

  // Approval state.
  let phase = $state<Phase>('review');
  let run = $state<ApprovalRun | null>(null);
  let runTick = $state(0);
  let txid = $state('');
  let signed = $state<BuildAndSignGenericResponseResult | null>(null);
  let openedSite = $state(false);
  let openingSite = $state(false);
  let openGeneration = 0;
  const copiedTxid = new TimedValueState<'txid'>();

  onDestroy(() => {
    openGeneration += 1;
  });

  const plan = $derived(session ? planRequestFlow(session.details, { newIdentity }) : null);
  const currentStep = $derived<RequestStepId | null>(plan ? (plan.steps[stepIndex] ?? null) : null);
  const updateReviewSource = $derived(updatePreflight ?? updateAnalysis);
  const updateReview = $derived(
    updateReviewSource && updateDetail
      ? buildGenericIdentityUpdateReview({
          currentIdentity: updateReviewSource.currentIdentity,
          requestedIdentity: updateReviewSource.requestedIdentity,
          rawRequestedIdentity:
            updateDetail.requestedIdentityJson ?? updateReviewSource.requestedIdentity,
          friendlyNames: updateReviewSource.friendlyNames,
          signerCmmKeyLabels: updateReviewSource.signerCmmKeyLabels,
          primaryAddressAfterUpdateInfo: updateReviewSource.primaryAddressAfterUpdateInfo,
          currentAuthorities: updateReviewSource.currentAuthorities,
          t: i18n.t.bind(i18n),
        })
      : null
  );
  const eligibleIdentities = $derived(filterEligibleLinkedIdentities());
  const responder = $derived(
    eligibleIdentities.find(
      (identity) => identity.identityAddress.toLowerCase() === responderAddress.toLowerCase()
    ) ?? null
  );
  const postUri = $derived(session?.responseUris.find((item) => item.mode === 'post')?.uri ?? '');
  const redirectUri = $derived(
    session?.responseUris.find((item) => item.mode === 'redirect')?.uri ?? ''
  );
  const appLabel = $derived(resolveAppLabel());
  const requesterView = $derived<RequesterView>(buildRequesterView());
  const targetName = $derived(
    formatIdentityDisplayName({
      identityAddress: updateDetail?.targetIdentityAddress ?? '',
      fullyQualifiedName: updateReviewSource?.fullyQualifiedName ?? null,
    })
  );
  // An offered VerusID or name that exists on chain fixes the name; otherwise
  // the user chooses one under the parent, as in Verus Mobile's provisioning
  // form (`ProvisionIdentityForm.js`).
  let offeredFqn = $state<string | null>(null);
  let offerChecked = $state(false);
  const choosesName = $derived(!!provisioningDetail && offerChecked && !offeredFqn);
  let chosenName = $state('');
  let parentFqn = $state<string | null>(null);
  let chosenNameStatus = $state<'idle' | 'checking' | 'available' | 'taken' | 'failed'>('idle');
  let chosenNameSequence = 0;
  const chosenNameValid = $derived(isValidChosenName(chosenName));
  const chosenFqn = $derived(
    choosesName && chosenNameValid && parentFqn ? `${chosenName}.${parentFqn}` : null
  );
  // Empty until the name is known: offered and found on chain, or chosen and
  // valid. Copy that names the new VerusID waits for it.
  const requestedFqn = $derived(formatIdentityFullyQualifiedName(offeredFqn ?? chosenFqn) || '');
  const expiresAt = $derived(authDetail?.expiryTime ?? invoiceModule?.expiresAt() ?? null);
  const expiry = $derived(expiresAt ? timeLeft(expiresAt, nowSeconds) : null);
  const expired = $derived(expiry?.kind === 'expired');
  const expiryUrgent = $derived(!!expiresAt && expiresSoon(expiresAt, nowSeconds));
  const timeLeftLabel = $derived(formatTimeLeft());
  const failedStage = $derived.by(() => {
    void runTick;
    return run?.failedStage ?? null;
  });
  const stagesView = $derived.by(() => {
    void runTick;
    return buildStagesView();
  });
  const deliveryView = $derived(buildDeliveryLine());
  const commitLabel = $derived(buildCommitLabel());

  $effect(() => {
    if (!isOpen || !session) {
      resetFlowState();
      return;
    }
    resetFlowState();
    void hydrateLinkedIdentities();
    if (updateDetail) void hydrateUpdateReview();
    // Create the modules without reading them back, so this effect does not
    // depend on the state it sets.
    const activeSession = session;
    untrack(() => {
      if (activeSession.details.some((detail) => detail.type === 'invoice')) {
        const module = new PayInvoiceModule(moduleContext(activeSession));
        invoiceModule = module;
        void module.load();
      }
      const dataPacket = activeSession.details.find((detail) => detail.type === 'dataPacket');
      if (dataPacket) signDataModule = new SignDataModule(moduleContext(activeSession), dataPacket);
      const appEncryption = activeSession.details.find((detail) => detail.type === 'appEncryption');
      if (appEncryption) {
        const module = new EncryptionKeyModule(moduleContext(activeSession), appEncryption);
        encryptionKeyModule = module;
        void module.load();
      }
      const userData = activeSession.details.find((detail) => detail.type === 'userData');
      if (userData) {
        const module = new ShareDataModule(moduleContext(activeSession), userData);
        shareDataModule = module;
        void module.load();
      }
      const giftCard = activeSession.details.find((detail) => detail.type === 'giftCard');
      if (giftCard) {
        const module = new GiftCardModule(moduleContext(activeSession), giftCard);
        giftCardModule = module;
        void module.load();
      }
    });
  });

  $effect(() => {
    // Share data reads the credentials of the chosen responder.
    const module = shareDataModule;
    const identityId = responder?.identityAddress ?? '';
    if (!isOpen || !module || !identityId || module.privateReady !== true) return;
    untrack(() => void module.lookUp(identityId));
  });

  function moduleContext(activeSession: GenericRequestFlowSession): RequestContext {
    return {
      session: activeSession,
      t: (key, params) => i18n.t(key, params),
      formatNumber: (value, options) => i18n.formatNumber(value, options),
      appLabel: () => appLabel,
      responder: () => responder,
      coins: () => coins,
      errorMessage: (error, fallbackKey) => errorMessage(error, fallbackKey),
    };
  }

  function moduleFor(step: RequestStepId | null): DetailModule | null {
    return step ? (modules.find((module) => module.steps.includes(step)) ?? null) : null;
  }

  $effect(() => {
    if (!isOpen || !session) return undefined;
    const timer = setInterval(() => (nowSeconds = Math.floor(Date.now() / 1000)), 15_000);
    return () => clearInterval(timer);
  });

  $effect(() => {
    // One eligible VerusID is chosen for the user; a choice that no longer
    // qualifies is cleared.
    if (!isOpen || !needsResponder || newIdentity) return;
    if (responderAddress) {
      if (
        !eligibleIdentities.some(
          (identity) => identity.identityAddress.toLowerCase() === responderAddress.toLowerCase()
        )
      ) {
        responderAddress = '';
      }
      return;
    }
    if (eligibleIdentities.length === 1) responderAddress = eligibleIdentities[0].identityAddress;
  });

  $effect(() => {
    const signerIdentityId = session?.signer.identityId?.trim() ?? '';
    if (!isOpen || !signerIdentityId) {
      requesterIdentityDetails = null;
      return undefined;
    }
    let cancelled = false;
    requesterIdentityDetails = null;
    void identityLinkService
      .getIdentityDetails(signerIdentityId)
      .then((details) => {
        if (!cancelled) requesterIdentityDetails = details;
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    if (!isOpen || currentStep !== 'networkFee' || !updateDetail) return;
    const systemId = reviewSystemId();
    const loadKey = `${updateDetail.targetIdentityAddress.toLowerCase()}::${systemId.toLowerCase()}`;
    if (fundingSourcesLoadedKey === loadKey || fundingSourcesLoading) return;
    void hydrateFundingSources(loadKey, systemId);
  });

  function resetFlowState() {
    openGeneration += 1;
    invoiceModule = null;
    signDataModule = null;
    encryptionKeyModule = null;
    shareDataModule = null;
    giftCardModule = null;
    linkedIdentities = [];
    identityDetailsByAddress = {};
    loadingLinkedIdentities = false;
    responderAddress = '';
    newIdentity = false;
    chosenName = '';
    parentFqn = null;
    offeredFqn = null;
    offerChecked = false;
    chosenNameStatus = 'idle';
    chosenNameSequence += 1;
    linkSheetOpen = false;
    updateAnalysis = null;
    updateAnalysisLoading = false;
    updatePreflight = null;
    preflightAt = 0;
    updateLoading = false;
    fundingSources = [];
    fundingSourcesLoading = false;
    fundingSourcesLoadedKey = '';
    selectedSourceChannelId = '';
    highRiskAcknowledged = false;
    stepIndex = 0;
    flowError = '';
    phase = 'review';
    approved = false;
    run = null;
    txid = '';
    signed = null;
    openedSite = false;
    openingSite = false;
    nowSeconds = Math.floor(Date.now() / 1000);
    copiedTxid.clear();
  }

  function errorMessage(
    error: unknown,
    fallbackKey = 'genericRequest.error.generic',
    fallbackParams: Record<string, string> = {}
  ): string {
    if (isForcedWalletLockError(error)) return '';
    const rejection = rejectionCode(error);
    if (rejection) return i18n.t(`genericRequest.rejected.${rejection}`);
    switch (extractWalletErrorType(error)) {
      case 'IdentityOwnershipMismatch':
        return i18n.t('genericRequest.error.identityOwnershipMismatch');
      case 'IdentityRequestExpired':
        return i18n.t('genericRequest.error.identityRequestExpired');
      case 'IdentityUnsupportedAuthority':
        return i18n.t('genericRequest.error.identityUnsupportedAuthority');
      case 'IdentityUnsupportedActiveCurrencyChange':
        return i18n.t('genericRequest.error.identityUnsupportedActiveCurrencyChange');
      case 'IdentityUnsupportedTokenizedControlChange':
        return i18n.t('genericRequest.error.identityUnsupportedTokenizedControlChange');
      case 'InsufficientFunds':
        return i18n.t('genericRequest.error.insufficientFunds');
      case 'InvalidAddress':
      case 'UnsupportedChannel':
        return i18n.t('genericRequest.error.noSourceChannel');
      case 'NetworkError':
      case 'OperationFailed':
        return i18n.t(fallbackKey, fallbackParams);
      default:
        break;
    }
    if (error instanceof Error && error.message.startsWith('genericRequest.'))
      return i18n.t(error.message);
    return extractWalletErrorMessage(error) || i18n.t(fallbackKey, fallbackParams);
  }

  function setError(error: unknown, fallbackKey?: string) {
    flowError = errorMessage(error, fallbackKey);
  }

  function reviewSystemId(): string {
    return (
      updateDetail?.sourceSystemId?.trim() ||
      (session?.testnet ? TESTNET_SYSTEM_ID : MAINNET_SYSTEM_ID)
    );
  }

  function uriHost(uri: string): string {
    try {
      return new URL(uri).host;
    } catch {
      return '';
    }
  }

  function requesterDisplayName(): string {
    if (requesterIdentityDetails) {
      const name = formatIdentityDisplayName(requesterIdentityDetails);
      if (name && name !== requesterIdentityDetails.identityAddress) return name;
    }
    return '';
  }

  function resolveAppLabel(): string {
    return (
      uriHost(postUri || redirectUri || session?.responseUris[0]?.uri || '') ||
      requesterDisplayName() ||
      session?.signer.identityId?.trim() ||
      i18n.t('genericRequest.auth.requesterFallback')
    );
  }

  function toContactIdentity(identity: {
    identityAddress: string;
    fullyQualifiedName?: string | null;
  }): ContactIdentity | null {
    const contacts = $contactSession;
    if (!contacts) return null;
    return {
      identityAddress: identity.identityAddress,
      fullyQualifiedName: formatIdentityDisplayName(identity),
      network: contacts.network,
      chainId: contactChainId(contacts.network),
    };
  }

  function buildRequesterView(): RequesterView {
    const contacts = $contactSession;
    const identityLabel = requesterDisplayName() || session?.signer.identityId?.trim() || '';
    const sameChain = !!contacts && session?.signer.systemId === contactChainId(contacts.network);
    return {
      appLabel,
      identity:
        requesterIdentityDetails && sameChain ? toContactIdentity(requesterIdentityDetails) : null,
      identityLabel,
      signed: !!session?.signer.isSigned,
      giftCard: !!session?.details.some((detail) => detail.type === 'giftCard'),
    };
  }

  function formatTimeLeft(): string {
    if (!expiry) return '';
    if (expiry.kind === 'expired') return i18n.t('genericRequest.flow.expired');
    if (expiry.kind === 'minutes')
      return i18n.t('genericRequest.flow.timeLeft.minutes', { count: expiry.minutes });
    return i18n.t('genericRequest.flow.timeLeft.hours', { count: expiry.hours });
  }

  function systemDisplayLabel(systemId: string): string {
    const normalized = systemId.trim().toLowerCase();
    if (normalized === MAINNET_SYSTEM_ID.toLowerCase()) {
      return coins.find((coin) => coin.id === 'VRSC')?.displayName?.trim() || 'Verus';
    }
    if (normalized === TESTNET_SYSTEM_ID.toLowerCase()) {
      return coins.find((coin) => coin.id === 'VRSCTEST')?.displayName?.trim() || 'Verus Testnet';
    }
    const coin =
      coins.find(
        (item) =>
          item.systemId.toLowerCase() === normalized && item.currencyId.toLowerCase() === normalized
      ) ?? coins.find((item) => item.systemId.toLowerCase() === normalized);
    return coin?.displayName?.trim() || coin?.displayTicker?.trim() || systemId.trim();
  }

  function systemTicker(systemId: string): string {
    const normalized = systemId.trim().toLowerCase();
    const coin = coins.find(
      (item) =>
        item.systemId.toLowerCase() === normalized && item.currencyId.toLowerCase() === normalized
    );
    return coin?.displayTicker?.trim() || systemDisplayLabel(systemId);
  }

  // Responder ------------------------------------------------------------------

  function filterEligibleLinkedIdentities(): LinkedIdentity[] {
    let identities = linkedIdentities;
    // One responder answers every detail, so a sign in next to an update must
    // use the VerusID being updated (Verus Mobile ensureGenericResponseSigner).
    if (updateDetail) {
      const target = updateDetail.targetIdentityAddress.toLowerCase();
      identities = identities.filter(
        (identity) => identity.identityAddress.toLowerCase() === target
      );
    }
    // A credential request may name the VerusID that must answer
    // (`userDataRequestedSignerMatchesIdentity`).
    const requiredIdentity = shareDataModule?.detail.requiredIdentity?.toLowerCase();
    if (requiredIdentity) {
      identities = identities.filter(
        (identity) => identity.identityAddress.toLowerCase() === requiredIdentity
      );
    }
    if (!authDetail) return identities;
    const of = (type: number) =>
      new Set(
        authDetail.constraints
          .filter((constraint) => constraint.type === type)
          .map((constraint) => constraint.identity.toLowerCase())
      );
    const requiredIds = of(AUTH_REQUIRED_ID);
    const requiredSystems = of(AUTH_REQUIRED_SYSTEM);
    const requiredParents = of(AUTH_REQUIRED_PARENT);
    return identities.filter((identity) => {
      if (requiredIds.size > 0 && !requiredIds.has(identity.identityAddress.toLowerCase()))
        return false;
      if (
        requiredSystems.size > 0 &&
        (!identity.systemId || !requiredSystems.has(identity.systemId.toLowerCase()))
      ) {
        return false;
      }
      if (requiredParents.size > 0) {
        const parent =
          identityDetailsByAddress[identity.identityAddress.toLowerCase()]?.parent?.toLowerCase();
        if (!parent || !requiredParents.has(parent)) return false;
      }
      return true;
    });
  }

  async function loadIdentityDetailsMap(
    identities: LinkedIdentity[]
  ): Promise<Record<string, IdentityDetails>> {
    const requiresParent = authDetail?.constraints.some(
      (constraint) => constraint.type === AUTH_REQUIRED_PARENT
    );
    if (!requiresParent) return {};
    const entries = await Promise.all(
      identities.map(async (identity) => {
        try {
          return [
            identity.identityAddress.toLowerCase(),
            await identityLinkService.getIdentityDetails(identity.identityAddress),
          ] as const;
        } catch {
          return [identity.identityAddress.toLowerCase(), null] as const;
        }
      })
    );
    return Object.fromEntries(
      entries.filter((entry): entry is readonly [string, IdentityDetails] => entry[1] !== null)
    );
  }

  function applyPassthroughSelection(identities: LinkedIdentity[]) {
    const fqn = session?.passthroughAutoLinkFqn?.toLowerCase();
    if (!fqn) return;
    const match = identities.find(
      (identity) =>
        formatIdentityDisplayName(identity).toLowerCase() === fqn ||
        identity.fullyQualifiedName?.toLowerCase() === fqn
    );
    if (match) responderAddress = match.identityAddress;
  }

  async function hydrateLinkedIdentities() {
    loadingLinkedIdentities = true;
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

  function chooseResponder(identityAddress: string) {
    responderAddress = identityAddress;
    newIdentity = false;
  }

  // Update identity -------------------------------------------------------------

  async function hydrateUpdateReview() {
    if (!session || !updateDetail) return;
    updateAnalysisLoading = true;
    try {
      updateAnalysis = await genericRequestService.reviewGenericIdentityUpdate(
        session.requestHex,
        updateDetail.requestedIdentityJson
      );
    } catch (error) {
      updateAnalysis = null;
      setError(error, 'genericRequest.error.updatePreflight');
    } finally {
      updateAnalysisLoading = false;
    }
  }

  function resolveFundingCoin(systemId: string): CoinDefinition | null {
    const normalized = systemId.trim().toLowerCase();
    return (
      coins.find(
        (coin) =>
          coin.compatibleChannels.includes('vrpc') &&
          coin.systemId.toLowerCase() === normalized &&
          coin.currencyId.toLowerCase() === normalized
      ) ?? null
    );
  }

  function buildFundingSource(
    scope: CoinScope,
    balance: BalanceResult,
    coin: CoinDefinition
  ): FundingSource | null {
    const value = Number(balance.total);
    if (!Number.isFinite(value) || value <= 0) return null;
    const maxDigits = Math.max(2, Math.min(8, coin.decimals));
    return {
      channelId: scope.channelId,
      address: scope.address,
      addressLabel: scope.addressLabel,
      systemId: scope.systemId,
      balanceValue: value,
      balanceDisplay: `${i18n.formatNumber(value, { minimumFractionDigits: Math.min(2, maxDigits), maximumFractionDigits: maxDigits })} ${coin.displayTicker}`,
      isPrimaryAddress: scope.isPrimaryAddress,
    };
  }

  async function hydrateFundingSources(loadKey: string, systemId: string) {
    const coin = resolveFundingCoin(systemId);
    const sequence = ++fundingSourceLoadSequence;
    fundingSourcesLoading = true;
    flowError = '';
    try {
      if (!coin) {
        fundingSources = [];
        fundingSourcesLoadedKey = loadKey;
        return;
      }
      const result = await walletService.getCoinScopes(coin.id);
      if (sequence !== fundingSourceLoadSequence) return;
      // Watched addresses are read-only scopes and never fund an update.
      const scopes = result.scopes.filter(
        (scope) =>
          scope.scopeKind === 'transparent' &&
          !scope.isReadOnly &&
          scope.systemId.toLowerCase() === systemId.toLowerCase()
      );
      const balances = await Promise.all(
        scopes.map(async (scope) => ({
          scope,
          balance: await walletService.getBalances(scope.channelId, coin.id),
        }))
      );
      if (sequence !== fundingSourceLoadSequence) return;
      fundingSources = balances
        .map(({ scope, balance }) => buildFundingSource(scope, balance, coin))
        .filter((source): source is FundingSource => source !== null)
        .sort((left, right) =>
          left.isPrimaryAddress !== right.isPrimaryAddress
            ? left.isPrimaryAddress
              ? -1
              : 1
            : right.balanceValue - left.balanceValue
        );
      fundingSourcesLoadedKey = loadKey;
      if (!fundingSources.some((source) => source.channelId === selectedSourceChannelId)) {
        // Nothing is preselected (spec "Network fee").
        selectedSourceChannelId = '';
        updatePreflight = null;
      }
    } catch (error) {
      if (sequence !== fundingSourceLoadSequence) return;
      fundingSources = [];
      fundingSourcesLoadedKey = '';
      selectedSourceChannelId = '';
      updatePreflight = null;
      setError(error, 'genericRequest.error.fundingSources');
    } finally {
      if (sequence === fundingSourceLoadSequence) fundingSourcesLoading = false;
    }
  }

  async function hydrateUpdatePreflight(channelId: string) {
    if (!session || !updateDetail || !channelId) return;
    const sequence = ++updatePreflightRequestSequence;
    updateLoading = true;
    flowError = '';
    try {
      const preflight = await genericRequestService.preflightGenericIdentityUpdate(
        session.requestHex,
        updateDetail.requestedIdentityJson,
        channelId
      );
      if (sequence !== updatePreflightRequestSequence) return;
      updatePreflight = preflight;
      preflightAt = Date.now();
    } catch (error) {
      if (sequence !== updatePreflightRequestSequence) return;
      updatePreflight = null;
      setError(error, 'genericRequest.error.updatePreflight');
    } finally {
      if (sequence === updatePreflightRequestSequence) updateLoading = false;
    }
  }

  async function chooseFundingSource(channelId: string) {
    if (channelId === selectedSourceChannelId && updatePreflight) return;
    selectedSourceChannelId = channelId;
    updatePreflight = null;
    await hydrateUpdatePreflight(channelId);
  }

  const feeLabel = $derived(
    updatePreflight
      ? i18n.t('genericRequest.flow.feeValue', {
          fee: updatePreflight.fee,
          currency: systemTicker(updatePreflight.feeCurrency),
        })
      : ''
  );

  // Steps -----------------------------------------------------------------------

  function stepReady(step: RequestStepId | null): boolean {
    switch (step) {
      case 'signIn':
        return newIdentity ? !!provisioningDetail?.webhook : !!responder?.systemId;
      case 'newIdentity':
        return (
          !!provisioningDetail?.webhook &&
          !!walletChannels.vrpcAddress?.trim() &&
          offerChecked &&
          (!choosesName || chosenNameStatus === 'available')
        );
      case 'updateChanges':
        return !!updateReview && (!updateReview.hasHighRisk || highRiskAcknowledged);
      case 'networkFee':
        return !!updatePreflight && !updateLoading;
      case 'confirm':
        return !!plan && plan.steps.every((item) => item === 'confirm' || stepReady(item));
      default:
        return step ? (moduleFor(step)?.ready(step) ?? false) : false;
    }
  }

  function stepLabel(step: RequestStepId): string {
    return i18n.t(`genericRequest.flow.step.${step}`);
  }

  function stepTitle(step: RequestStepId): string {
    const module = moduleFor(step);
    if (module) return module.title(step);
    switch (step) {
      case 'signIn':
        return i18n.t('genericRequest.flow.title.signIn', { app: appLabel });
      case 'newIdentity':
        return i18n.t('genericRequest.flow.title.newIdentity');
      case 'updateChanges':
        return i18n.t('genericRequest.flow.title.updateChanges', { identity: targetName });
      case 'networkFee':
        return i18n.t('genericRequest.flow.title.networkFee');
      default:
        return i18n.t('genericRequest.flow.title.confirm');
    }
  }

  // The step where the user picks the VerusID that answers: Sign in, or the
  // step that shows the picker itself when the request has no sign-in.
  const responderStep = $derived<RequestStepId | null>(
    plan?.steps.find(
      (id) =>
        id === 'signIn' ||
        id === 'encryptionKey' ||
        (!authDetail && (id === 'signData' || id === 'shareData'))
    ) ?? null
  );

  const railSteps = $derived<RailStep[]>(
    plan
      ? plan.steps.map((id, index) => {
          // Done is a result, not a step: after the stages run every step is
          // finished, and a decline leaves the rail where it was.
          const current = phase === 'done' || phase === 'failed' ? plan.steps.length : stepIndex;
          const status = index < current ? 'done' : index === current ? 'current' : 'upcoming';
          return {
            id,
            label: stepLabel(id),
            detail:
              status === 'done' && id === responderStep && responder && !newIdentity
                ? i18n.t('genericRequest.flow.respondsAs', {
                    identity: formatIdentityDisplayName(responder),
                  })
                : undefined,
            status,
          };
        })
      : []
  );

  async function goNext() {
    if (!plan || !stepReady(currentStep)) return;
    flowError = '';
    stepIndex = Math.min(stepIndex + 1, plan.steps.length - 1);
    if (plan.steps[stepIndex] !== 'confirm') return;
    if (updatePreflight && Date.now() - preflightAt > PREFLIGHT_REFRESH_MS) {
      await hydrateUpdatePreflight(selectedSourceChannelId);
    }
    await Promise.all(modules.map((module) => module.beforeConfirm?.()));
  }

  function goBack() {
    flowError = '';
    stepIndex = Math.max(stepIndex - 1, 0);
  }

  function decline() {
    openGeneration += 1;
    phase = 'declined';
  }

  function closeFlow() {
    openGeneration += 1;
    isOpen = false;
    onClose();
  }

  function startNewIdentity() {
    newIdentity = true;
    responderAddress = '';
    void checkOffer();
  }

  async function checkOffer() {
    if (!offerChecked) {
      const generation = openGeneration;
      const offered = provisioningDetail?.requestedFqn?.trim();
      let found: string | null = null;
      if (offered) {
        try {
          found =
            (await identityLinkService.getIdentityDetails(offered)).fullyQualifiedName ?? null;
        } catch {
          // Not on chain: the user chooses the name.
        }
      }
      if (generation !== openGeneration) return;
      offeredFqn = found;
      offerChecked = true;
    }
    if (choosesName && !parentFqn) void loadParentFqn();
  }

  /** One label without the characters a VerusID name cannot hold (Rust checks it too). */
  function isValidChosenName(name: string): boolean {
    return (
      name.length > 0 &&
      name.trim() === name &&
      new TextEncoder().encode(name).length <= 64 &&
      !/[\\/:*?"<>|@.\p{Cc}]/u.test(name)
    );
  }

  async function loadParentFqn() {
    const parent = provisioningDetail?.parent ?? provisioningDetail?.systemId ?? null;
    if (!parent) {
      parentFqn = session?.testnet ? 'VRSCTEST@' : 'VRSC@';
      return;
    }
    try {
      const details = await identityLinkService.getIdentityDetails(parent);
      parentFqn = details.fullyQualifiedName ?? null;
    } catch {
      parentFqn = null;
    }
  }

  /** A chosen name must be free (Verus Mobile: "Identity name taken"). */
  async function checkChosenName(name: string) {
    chosenName = name;
    const sequence = ++chosenNameSequence;
    if (!isValidChosenName(name) || !parentFqn) {
      chosenNameStatus = 'idle';
      return;
    }
    chosenNameStatus = 'checking';
    await new Promise((resolve) => setTimeout(resolve, 400));
    if (sequence !== chosenNameSequence) return;
    try {
      await identityLinkService.getIdentityDetails(`${name}.${parentFqn}`);
      if (sequence === chosenNameSequence) chosenNameStatus = 'taken';
    } catch (error) {
      if (sequence !== chosenNameSequence) return;
      chosenNameStatus =
        extractWalletErrorType(error) === 'IdentityNotFound' ? 'available' : 'failed';
    }
  }

  // Plan and outcome copy ------------------------------------------------------

  function changeSummary(): string {
    if (!updateReview) return '';
    const total = updateReview.contentCount + updateReview.highRiskCount;
    return i18n.t(`genericRequest.flow.plan.changes.${total === 1 ? 'one' : 'other'}`, {
      count: total,
    });
  }

  const planLines = $derived<PlanLine[]>(
    session
      ? session.details.flatMap((detail): PlanLine[] => {
          if (detail.type === 'authentication' && responder && !encryptionKeyModule) {
            return [
              {
                id: 'signIn',
                icon: 'signIn',
                text: i18n.t('genericRequest.flow.plan.signIn', {
                  app: appLabel,
                  identity: formatIdentityDisplayName(responder),
                }),
                impact: DETAIL_IMPACT.authentication,
              },
            ];
          }
          if (detail.type === 'provisionIdentity' && newIdentity) {
            return [
              {
                id: 'newIdentity',
                icon: 'newIdentity',
                text: i18n.t('genericRequest.flow.plan.newIdentity', {
                  issuer: requesterView.identityLabel,
                  name: requestedFqn,
                }),
                impact: DETAIL_IMPACT.provisionIdentity,
              },
            ];
          }
          if (detail.type === 'invoice') return invoiceModule?.planLines() ?? [];
          if (detail.type === 'dataPacket') return signDataModule?.planLines() ?? [];
          if (detail.type === 'appEncryption') return encryptionKeyModule?.planLines() ?? [];
          if (detail.type === 'userData') return shareDataModule?.planLines() ?? [];
          if (detail.type === 'giftCard') return giftCardModule?.planLines() ?? [];
          if (detail.type === 'identityUpdate') {
            return [
              {
                id: 'update',
                icon: 'updateChanges',
                text: i18n.t('genericRequest.flow.plan.update', { identity: targetName }),
                detail: changeSummary(),
                impact: DETAIL_IMPACT.identityUpdate,
                highRisk: (updateReview?.highRiskCount ?? 0) > 0,
              },
            ];
          }
          return [];
        })
      : []
  );

  /**
   * Where the response goes, when the screen does not already say it. A POST to
   * the requester's own address needs no line: the rail or the header names
   * that site. No response, a new browser page, and a request that goes to the
   * issuer instead of the site do get a line.
   */
  function buildDeliveryLine(): { text: string; sends: boolean } | null {
    if (!respondsWithDetails && !newIdentity)
      return { text: i18n.t('genericRequest.flow.delivery.none'), sends: false };
    if (newIdentity)
      return {
        text: i18n.t('genericRequest.flow.delivery.webhook', {
          issuer: requesterView.identityLabel,
        }),
        sends: true,
      };
    if (postUri) return null;
    if (redirectUri)
      return {
        text: i18n.t('genericRequest.flow.delivery.redirect', { app: appLabel }),
        sends: true,
      };
    return { text: i18n.t('genericRequest.flow.delivery.none'), sends: false };
  }

  function buildCommitLabel(): { idle: string; working: string } {
    if (newIdentity) {
      return {
        idle: i18n.t('genericRequest.flow.action.newIdentity', { name: requestedFqn }),
        working: i18n.t('genericRequest.flow.working.newIdentity'),
      };
    }
    if (updateDetail) {
      return {
        idle: authDetail
          ? i18n.t('genericRequest.flow.action.signInAndUpdate')
          : i18n.t('genericRequest.flow.action.update'),
        working: i18n.t('genericRequest.flow.working.update'),
      };
    }
    if (invoiceModule) {
      return {
        idle: invoiceModule.actionLabel(!!authDetail),
        working: i18n.t('genericRequest.flow.working.pay'),
      };
    }
    if (giftCardModule) {
      return {
        idle: giftCardModule.actionLabel(),
        working: i18n.t('genericRequest.flow.working.claim'),
      };
    }
    if (encryptionKeyModule) {
      return {
        idle: encryptionKeyModule.actionLabel(),
        working: i18n.t('genericRequest.flow.working.shareKey'),
      };
    }
    if (signDataModule) {
      return {
        idle: signDataModule.actionLabel(!!authDetail),
        working: i18n.t('genericRequest.flow.working.sign'),
      };
    }
    if (shareDataModule && shareDataModule.count > 0) {
      return {
        idle: shareDataModule.actionLabel(!!authDetail),
        working: i18n.t('genericRequest.flow.working.share'),
      };
    }
    return {
      idle: i18n.t('genericRequest.flow.action.signIn'),
      working: i18n.t('genericRequest.flow.working.signIn'),
    };
  }

  function stageLabel(id: ApprovalStageId, status: StageView['status']): string {
    if (id === 'delivery') {
      if (newIdentity)
        return i18n.t(`genericRequest.flow.stage.webhook.${status}`, {
          issuer: requesterView.identityLabel,
        });
      if (openedSite || (!postUri && redirectUri)) {
        return i18n.t(`genericRequest.flow.stage.redirect.${status}`, { app: appLabel });
      }
    }
    if (id === 'transaction' && invoiceModule) {
      return i18n.t(`genericRequest.flow.stage.payment.${status}`);
    }
    if (id === 'transaction' && giftCardModule) {
      return i18n.t(`genericRequest.flow.stage.claim.${status}`);
    }
    return i18n.t(`genericRequest.flow.stage.${id}.${status}`, { app: appLabel });
  }

  function stageErrorKey(id: ApprovalStageId): string {
    if (id === 'delivery' && newIdentity) return 'genericRequest.flow.stageError.webhook';
    if (id === 'transaction' && giftCardModule) return 'genericRequest.flow.stageError.claim';
    return `genericRequest.flow.stageError.${id}`;
  }

  function buildStagesView(): StageView[] {
    if (!run) return [];
    const active = run;
    return active.stages.map((id) => ({
      id,
      status: active.status[id],
      label: stageLabel(id, active.status[id]),
      error:
        active.status[id] === 'failed'
          ? errorMessage(active.errors[id], stageErrorKey(id), {
              app: appLabel,
              issuer: requesterView.identityLabel,
            })
          : undefined,
    }));
  }

  const outcomeTitle = $derived(
    phase === 'failed'
      ? i18n.t('genericRequest.flow.outcome.failed')
      : newIdentity
        ? i18n.t('genericRequest.flow.outcome.newIdentity', { name: requestedFqn })
        : updateDetail
          ? i18n.t('genericRequest.flow.outcome.updated', { identity: targetName })
          : invoiceModule
            ? invoiceModule.outcomeTitle()
            : giftCardModule
              ? giftCardModule.outcomeTitle()
              : encryptionKeyModule
                ? encryptionKeyModule.outcomeTitle()
                : signDataModule
                  ? signDataModule.outcomeTitle()
                  : shareDataModule && shareDataModule.count > 0
                    ? shareDataModule.outcomeTitle()
                    : i18n.t('genericRequest.flow.outcome.signedIn', { app: appLabel })
  );

  // Approval ----------------------------------------------------------------------

  function responseSigner(): { systemId: string; identityId: string } | null {
    if (needsResponder) {
      return responder?.systemId
        ? { systemId: responder.systemId, identityId: responder.identityAddress }
        : null;
    }
    if (updateDetail)
      return { systemId: reviewSystemId(), identityId: updateDetail.targetIdentityAddress };
    return null;
  }

  /** Verus Mobile checks the sign-in expiry again before it signs and before it sends. */
  function assertNotExpired() {
    if (authDetail?.expiryTime && Math.floor(Date.now() / 1000) >= authDetail.expiryTime) {
      throw new Error('genericRequest.flow.error.expired');
    }
  }

  async function deliver(
    response: BuildAndSignGenericResponseResult,
    activeSession: GenericRequestFlowSession
  ) {
    const post = activeSession.responseUris.find((item) => item.mode === 'post')?.uri;
    const redirect = activeSession.responseUris.find((item) => item.mode === 'redirect')?.uri;
    if (post) {
      await genericRequestService.postGenericResponseCallback(
        activeSession.requestHex,
        post,
        response.signedResponseHex,
        response.sessionId,
        activeSession.legacy === 'login' ? legacyLoginWebhookBody(response.signedResponseHex) : null
      );
      return;
    }
    if (redirect) {
      await genericRequestService.openGenericRequestCallback(
        activeSession.requestHex,
        redirect,
        response.signedResponseHex,
        response.sessionId
      );
    }
  }

  async function approve() {
    if (
      !session ||
      phase !== 'review' ||
      !stepReady(plan?.layout === 'single' ? currentStep : 'confirm') ||
      expired
    )
      return;
    const activeSession = session;
    const generation = ++openGeneration;
    const live = () => generation === openGeneration && isOpen && session === activeSession;
    flowError = '';

    if (newIdentity) {
      if (!authDetail || !provisioningDetail) return;
      const signingAddress = walletChannels.vrpcAddress?.trim() ?? '';
      run = new ApprovalRun(
        ['delivery'],
        {
          transaction: async () => {},
          signing: async () => {},
          delivery: async () => {
            if (!signingAddress)
              throw new Error('genericRequest.provisioning.error.noSigningAddress');
            await submitGenericProvisioningRequest({
              session: activeSession,
              authenticationDetail: authDetail,
              provisioningDetail,
              signingAddress,
              requestedFqn: offeredFqn ?? chosenFqn,
            });
          },
        },
        () => (runTick += 1)
      );
    } else {
      const signer = responseSigner();
      if (!signer && respondsWithDetails) return;
      const preflightId = updatePreflight?.preflightId ?? '';
      const paying = transactionModule;
      run = new ApprovalRun(
        approvalStages({
          hasTransaction: !!updateDetail || !!paying,
          signsResponse: respondsWithDetails,
          delivers: activeSession.responseUris.length > 0,
        }),
        {
          transaction: async () => {
            // Rust records the broadcast for the open request and reuses it.
            if (updateDetail) {
              const result = await genericRequestService.sendGenericIdentityUpdate(
                activeSession.requestHex,
                preflightId
              );
              txid = result.txid;
            } else if (paying?.transaction) {
              txid = await paying.transaction();
            }
          },
          signing: async () => {
            assertNotExpired();
            if (!signer) throw new Error('genericRequest.error.complete');
            signed = await genericRequestService.buildAndSignGenericResponse(
              activeSession.requestHex,
              signer
            );
          },
          delivery: async () => {
            if (!signed) throw new Error('genericRequest.error.complete');
            assertNotExpired();
            await deliver(signed, activeSession);
          },
        },
        () => (runTick += 1)
      );
    }

    phase = 'working';
    approved = true;
    const approvedRun = run;
    const ok = await genericRequestService.withAutoLockPaused(activeSession.requestHex, () =>
      approvedRun.run()
    );
    if (!live()) return;
    phase = ok ? 'done' : 'failed';
  }

  async function retry() {
    if (!run || run.isRunning || !session) return;
    const activeSession = session;
    const generation = openGeneration;
    phase = 'working';
    const retried = run;
    const ok = await genericRequestService.withAutoLockPaused(activeSession.requestHex, () =>
      retried.run()
    );
    if (generation !== openGeneration || session !== activeSession) return;
    phase = ok ? 'done' : 'failed';
  }

  async function openSiteInstead() {
    if (!session || !signed || !redirectUri || !run || openingSite) return;
    openingSite = true;
    try {
      await genericRequestService.openGenericRequestCallback(
        session.requestHex,
        redirectUri,
        signed.signedResponseHex,
        signed.sessionId
      );
      openedSite = true;
      run.status.delivery = 'done';
      runTick += 1;
      phase = 'done';
    } catch (error) {
      run.errors.delivery = error;
      runTick += 1;
    } finally {
      openingSite = false;
    }
  }

  async function copyTxid() {
    if (txid && (await writeClipboardText(txid))) copiedTxid.set('txid');
  }

  const receiptFacts = $derived.by<RequestFact[]>(() => {
    const rows: RequestFact[] = [];
    if (responder && authDetail && !newIdentity && !encryptionKeyModule) {
      rows.push({
        label: i18n.t('genericRequest.flow.receipt.signedInAs'),
        value: formatIdentityDisplayName(responder),
      });
    }
    if (invoiceModule && phase === 'done') rows.push(...invoiceModule.receiptFacts());
    if (giftCardModule && phase === 'done') rows.push(...giftCardModule.receiptFacts());
    if (signDataModule && !authDetail && phase === 'done')
      rows.push(...signDataModule.receiptFacts());
    if (encryptionKeyModule && phase === 'done') rows.push(...encryptionKeyModule.receiptFacts());
    if (shareDataModule && shareDataModule.count > 0 && !authDetail && phase === 'done')
      rows.push(...shareDataModule.receiptFacts());
    if (txid) rows.push({ label: i18n.t('genericRequest.complete.txidLabel'), content: txidValue });
    return rows;
  });

  // A required system that is the open wallet's own network limits nothing: the
  // wallet refuses a request for another network before this screen.
  const signInConstraints = $derived(
    (authDetail?.constraints ?? []).filter(
      (constraint) =>
        constraint.type !== AUTH_REQUIRED_SYSTEM ||
        constraint.identity.toLowerCase() !==
          (session?.testnet ? TESTNET_SYSTEM_ID : MAINNET_SYSTEM_ID).toLowerCase()
    )
  );

  const newIdentityFacts = $derived<RequestFact[]>([
    ...(requestedFqn
      ? [{ label: i18n.t('genericRequest.flow.newIdentity.name'), value: requestedFqn }]
      : []),
    { label: i18n.t('genericRequest.flow.newIdentity.issuer'), value: requesterView.identityLabel },
    {
      label: i18n.t('genericRequest.flow.newIdentity.network'),
      value: i18n.t(networkLocaleKey(session?.testnet ? 'testnet' : 'mainnet')),
    },
  ]);

  const confirmFacts = $derived<RequestFact[]>([
    ...(updatePreflight
      ? [{ label: i18n.t('genericRequest.flow.networkFee'), value: feeLabel }]
      : []),
    ...modules.flatMap((module) => module.confirmFacts()),
  ]);

  function constraintLabel(type: number): string {
    if (type === AUTH_REQUIRED_ID) return i18n.t('genericRequest.auth.requiredId');
    if (type === AUTH_REQUIRED_SYSTEM) return i18n.t('genericRequest.auth.requiredSystem');
    return i18n.t('genericRequest.auth.requiredParent');
  }
</script>

{#snippet txidValue()}
  <span class="inline-flex items-center gap-1.5">
    <IdentifierText value={txid} mode="compact" />
    <CopyButton
      copied={copiedTxid.current === 'txid'}
      size="xs"
      iconClass="size-3"
      copiedIconClass="size-3 text-emerald-600 dark:text-emerald-400"
      onclick={() => void copyTxid()}
      title={i18n.t('genericRequest.flow.copyTransaction')}
      aria-label={i18n.t('genericRequest.flow.copyTransaction')}
    />
  </span>
{/snippet}

{#snippet requirementsValue()}
  <span class="flex flex-col items-end gap-1 font-normal">
    {#each signInConstraints as constraint (`${constraint.type}:${constraint.identity}`)}
      <span class="inline-flex items-center gap-1.5">
        <span class="text-muted-foreground">{constraintLabel(constraint.type)}</span>
        {#if constraint.type === AUTH_REQUIRED_SYSTEM}
          <span class="font-medium">{systemDisplayLabel(constraint.identity)}</span>
        {:else}
          <IdentifierText value={constraint.identity} mode="compact" class="font-medium" />
        {/if}
      </span>
    {/each}
  </span>
{/snippet}

{#snippet errorBox()}
  {#if flowError}
    <div
      class="flex items-start gap-2 rounded-xl border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
    >
      <AlertCircleIcon class="mt-0.5 size-4 shrink-0" />
      <p>{flowError}</p>
    </div>
  {/if}
{/snippet}

{#snippet responderPicker()}
  <ResponderPicker
    identities={eligibleIdentities}
    selectedAddress={responderAddress}
    loading={loadingLinkedIdentities}
    {toContactIdentity}
    onSelect={chooseResponder}
    onLink={() => (linkSheetOpen = true)}
    newIdentityLabel={provisioningDetail ? i18n.t('genericRequest.flow.newIdentity.offer') : ''}
    onNewIdentity={provisioningDetail ? startNewIdentity : undefined}
  />
{/snippet}

{#snippet stepBody(step: RequestStepId)}
  {#if step === 'signIn'}
    {#if newIdentity}
      <p class="text-[13.5px] text-foreground/80">
        {requestedFqn
          ? i18n.t('genericRequest.flow.newIdentity.chosen', { name: requestedFqn })
          : i18n.t('genericRequest.flow.newIdentity.chosenUnnamed')}
      </p>
    {:else}
      {@render responderPicker()}
    {/if}
    <p class="text-[13px] text-muted-foreground">
      {i18n.t('genericRequest.flow.signIn.learns')}
    </p>
    {#if signInConstraints.length > 0}
      <RequestFacts
        rows={[
          { label: i18n.t('genericRequest.flow.signIn.requirements'), content: requirementsValue },
        ]}
      />
    {/if}
  {:else if step === 'newIdentity'}
    {#if choosesName}
      <div class="flex flex-col gap-1.5">
        <Label for="request-new-identity-name" class="text-xs font-medium text-muted-foreground">
          {i18n.t('genericRequest.flow.newIdentity.chooseName')}
        </Label>
        <div class="flex items-center gap-2">
          <Input
            id="request-new-identity-name"
            value={chosenName}
            oninput={(event) => void checkChosenName((event.target as HTMLInputElement).value)}
            autocomplete="off"
            aria-invalid={chosenName !== '' && (!chosenNameValid || chosenNameStatus === 'taken')}
          />
          {#if parentFqn}
            <span class="shrink-0 text-sm text-muted-foreground"
              >.{formatIdentityFullyQualifiedName(parentFqn)}</span
            >
          {/if}
        </div>
        <p class="text-xs text-muted-foreground">
          {#if chosenName !== '' && !chosenNameValid}
            {i18n.t('genericRequest.flow.newIdentity.nameInvalid')}
          {:else if chosenNameStatus === 'checking'}
            {i18n.t('genericRequest.flow.newIdentity.nameChecking')}
          {:else if chosenNameStatus === 'taken'}
            {i18n.t('genericRequest.flow.newIdentity.nameTaken')}
          {:else if chosenNameStatus === 'failed'}
            {i18n.t('genericRequest.flow.newIdentity.nameCheckFailed')}
          {:else if chosenNameStatus === 'available'}
            {i18n.t('genericRequest.flow.newIdentity.nameAvailable')}
          {:else}
            {i18n.t('genericRequest.flow.newIdentity.nameHint')}
          {/if}
        </p>
      </div>
    {/if}
    <RequestFacts rows={newIdentityFacts} />
    <p class="text-[13.5px] text-foreground/80">
      {i18n.t('genericRequest.flow.newIdentity.note', { issuer: requesterView.identityLabel })}
    </p>
  {:else if step === 'updateChanges'}
    {#if updateReview && updateReviewSource}
      <UpdateChanges
        review={updateReview}
        warnings={updateReviewSource.warnings}
        bind:acknowledged={highRiskAcknowledged}
      />
    {:else if updateAnalysisLoading}
      <p class="flex items-center gap-2 text-sm text-muted-foreground">
        <Spinner class="size-4" />
        {i18n.t('genericRequest.update.preflightLoading')}
      </p>
    {/if}
  {:else if step === 'networkFee'}
    <NetworkFeePicker
      sources={fundingSources}
      selectedChannelId={selectedSourceChannelId}
      loading={fundingSourcesLoading}
      feeLoading={updateLoading}
      {feeLabel}
      networkLabel={(source) => systemDisplayLabel(source.systemId)}
      onSelect={(channelId) => void chooseFundingSource(channelId)}
    />
  {:else if step === 'encryptionKey' && encryptionKeyModule}
    <EncryptionKeyStep
      module={encryptionKeyModule}
      responder={responderPicker}
      onSetupPrivate={onSetupPrivate
        ? () => {
            closeFlow();
            onSetupPrivate?.();
          }
        : undefined}
    />
  {:else if step === 'signData' && signDataModule}
    <SignDataStep
      module={signDataModule}
      {appLabel}
      responder={authDetail ? undefined : responderPicker}
    />
  {:else if step === 'shareData' && shareDataModule}
    {@const module = shareDataModule}
    <ShareDataStep
      {module}
      {appLabel}
      responderName={responder ? formatIdentityDisplayName(responder) : ''}
      responder={authDetail ? undefined : responderPicker}
      onSetupPrivate={onSetupPrivate
        ? () => {
            closeFlow();
            onSetupPrivate?.();
          }
        : undefined}
      onRetry={() => {
        if (responder) void module.lookUp(responder.identityAddress, true);
      }}
    />
  {:else if step === 'giftCard' && giftCardModule}
    <GiftCardStep module={giftCardModule} />
  {:else if step === 'payInvoice' && invoiceModule}
    <PayInvoiceStep module={invoiceModule} networkLabel={systemDisplayLabel} />
  {:else if step === 'confirm'}
    <ConfirmPlan lines={planLines} facts={confirmFacts} delivery={deliveryView} />
  {/if}
{/snippet}

{#snippet outcome()}
  {#if phase === 'declined'}
    <RequestOutcome
      kind="declined"
      title={i18n.t('genericRequest.flow.outcome.declined')}
      body={i18n.t('genericRequest.flow.outcome.declinedBody', { app: appLabel })}
    />
  {:else}
    <RequestOutcome
      kind={phase === 'failed' ? 'failed' : 'done'}
      title={outcomeTitle}
      body={phase === 'done' && newIdentity
        ? i18n.t('genericRequest.flow.outcome.newIdentityAfter', { name: requestedFqn })
        : ''}
      facts={receiptFacts}
      stages={stagesView}
      retrying={phase === 'working'}
      copyValue={failedStage === 'delivery' ? (signed?.signedResponseHex ?? '') : ''}
      onRetry={() => void retry()}
      onOpenSite={failedStage === 'delivery' && postUri && redirectUri && signed
        ? () => void openSiteInstead()
        : undefined}
    />
  {/if}
{/snippet}

{#snippet footer()}
  {#if phase === 'review' && !expired}
    {#if plan?.layout === 'rail' && stepIndex > 0}
      <Button variant="secondary" onclick={goBack}>{i18n.t('common.back')}</Button>
    {:else}
      <Button variant="secondary" onclick={decline}>{i18n.t('genericRequest.flow.decline')}</Button>
    {/if}
    {#if plan?.layout === 'single' || currentStep === 'confirm'}
      <Button
        class="min-w-48"
        onclick={() => void approve()}
        disabled={!stepReady(plan?.layout === 'single' ? currentStep : 'confirm') || updateLoading}
      >
        {commitLabel.idle}
      </Button>
    {:else}
      <Button class="min-w-48" onclick={() => void goNext()} disabled={!stepReady(currentStep)}>
        {i18n.t('common.continue')}
      </Button>
    {/if}
  {:else if phase === 'working'}
    <span></span>
    <Button class="min-w-48" disabled>
      <Spinner class="size-4" />
      {stagesView.find((stage) => stage.status === 'running')?.label ?? commitLabel.working}
    </Button>
  {:else}
    <span></span>
    <Button
      class="min-w-48"
      variant={phase === 'done' || phase === 'declined' ? 'default' : 'secondary'}
      onclick={closeFlow}
    >
      {phase === 'done' || phase === 'declined' ? i18n.t('common.done') : i18n.t('common.close')}
    </Button>
  {/if}
{/snippet}

{#if isOpen && session && plan}
  <RequestFrame
    label={i18n.t('genericRequest.flow.requestFrom', { app: appLabel })}
    onClose={closeFlow}
    closeDisabled={phase === 'working'}
    rail={plan.layout === 'rail' ? railSnippet : undefined}
    {footer}
  >
    {#if phase === 'done' || phase === 'failed' || phase === 'declined'}
      {@render outcome()}
    {:else if expired && phase === 'review'}
      <RequestOutcome
        kind="expired"
        title={i18n.t('genericRequest.flow.outcome.expired')}
        body={i18n.t('genericRequest.flow.outcome.expiredBody')}
      />
    {:else if currentStep}
      <!-- While the stages run, the reviewed request stays in view and is locked. -->
      <fieldset class="contents" disabled={phase === 'working'}>
        {#if plan.layout === 'single'}
          <RequesterHeader
            requester={requesterView}
            title={stepTitle(currentStep)}
            {timeLeftLabel}
            urgent={expiryUrgent}
          />
        {:else}
          <h1
            class="text-2xl leading-tight font-semibold tracking-tight text-balance text-foreground"
          >
            {stepTitle(currentStep)}
          </h1>
        {/if}
        {@render errorBox()}
        {@render stepBody(currentStep)}
        {#if plan.layout === 'single' && deliveryView}
          <DeliveryLine text={deliveryView.text} sends={deliveryView.sends} />
        {/if}
      </fieldset>
    {/if}
  </RequestFrame>

  <LinkIdentitySheet
    bind:isOpen={linkSheetOpen}
    onLinkedChange={(updated) => {
      linkedIdentities = updated;
      applyPassthroughSelection(updated);
      void loadIdentityDetailsMap(updated).then((details) => (identityDetailsByAddress = details));
    }}
    allowManualLinkEntry={session.testnet}
  />
{/if}

{#snippet railSnippet()}
  <RequestRail
    requester={requesterView}
    steps={railSteps}
    timeLeftLabel={phase === 'review' || phase === 'working' ? timeLeftLabel : ''}
    urgent={expiryUrgent}
  />
{/snippet}
