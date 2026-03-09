<script lang="ts">
  import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import CheckCircle2Icon from '@lucide/svelte/icons/check-circle-2';
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
  import Link2Icon from '@lucide/svelte/icons/link-2';
  import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
  import XIcon from '@lucide/svelte/icons/x';
  import { toast } from 'svelte-sonner';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { Button } from '$lib/components/ui/button';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';
  import { Spinner } from '$lib/components/ui/spinner';
  import WalletTransferStepperShell from '$lib/components/shared/WalletTransferStepperShell.svelte';
  import LinkIdentitySheet from '$lib/components/wallet/sections/identity/LinkIdentitySheet.svelte';
  import { i18nStore, networkLocaleKey } from '$lib/i18n';
  import { collectIdentityUpdateContentChanges, humanizeIdentityUpdatePath } from '$lib/genericRequest/identityUpdateDiff';
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
  import { walletChannelsStore } from '$lib/stores/walletChannels.js';
  import type {
    BuildAndSignGenericResponseRequest,
    GenericIdentityUpdatePreflightResult,
    IdentityDetails,
    LinkedIdentity
  } from '$lib/types/wallet.js';
  import { parseVrpcChannelId } from '$lib/utils/channelId.js';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay.js';
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
  let updatePreflight = $state<GenericIdentityUpdatePreflightResult | null>(null);
  let selectedSourceChannelId = $state('');
  let highRiskAcknowledged = $state(false);

  const updateContentChanges = $derived(
    updatePreflight
      ? collectIdentityUpdateContentChanges(updatePreflight.currentIdentity, updatePreflight.requestedIdentity)
      : []
  );
  const eligibleLinkedIdentities = $derived(filterEligibleLinkedIdentities());
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
    updatePreflight = null;
    highRiskAcknowledged = false;
    selectedAuthIdentityAddress = '';
    selectedSourceChannelId = defaultSourceChannelId();

    void hydrateLinkedIdentities();
    if (!authDetail && updateDetail) {
      void hydrateUpdatePreflight();
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
      return;
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
    updatePreflight = null;
    highRiskAcknowledged = false;
    selectedSourceChannelId = '';
    authIdentitySheetOpen = false;
    linkSheetOpen = false;
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

  function defaultSourceChannelId(): string {
    const requestSystemId = updateDetail?.sourceSystemId?.trim();
    const systemId =
      requestSystemId ||
      (session?.testnet ? TESTNET_SYSTEM_ID : MAINNET_SYSTEM_ID);
    const vrpcAddress = walletChannels.vrpcAddress?.trim();
    if (vrpcAddress) {
      return `vrpc.${vrpcAddress}.${systemId}`;
    }
    return walletChannels.primaryChannelId ?? '';
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
      if (updatePreflight && updatePreflight.highRiskChanges.length > 0) {
        order.push('updateRisk');
      }
      if (updateContentChanges.length > 0) {
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

  async function hydrateUpdatePreflight() {
    if (!session || !updateDetail) return;
    if (!selectedSourceChannelId) {
      flowError = i18n.t('genericRequest.error.noSourceChannel');
      return;
    }

    updateLoading = true;
    flowError = '';

    try {
      updatePreflight = await genericRequestService.preflightGenericIdentityUpdate(
        updateDetail.requestedIdentityJson,
        updateDetail.targetIdentityAddress,
        selectedSourceChannelId,
        {
          requestId: updateDetail.requestId,
          signerSystemId: session.signer.systemId,
          signerIdentityId: session.signer.identityId,
          expiryHeight: updateDetail.expiryHeight
        }
      );
      highRiskAcknowledged = false;
    } catch (error) {
      updatePreflight = null;
      setError(error, 'genericRequest.error.updatePreflight');
    } finally {
      updateLoading = false;
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
      if (!updatePreflight) {
        await hydrateUpdatePreflight();
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
    const currentIndex = visibleSteps.findIndex((step) => step.id === currentStepId);
    if (currentIndex <= 0) {
      isOpen = false;
      onClose();
      return;
    }

    currentStepId = visibleSteps[currentIndex - 1].id;
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
      completionMessage = i18n.t('genericRequest.complete.updateBroadcast', {
        txid: sendResult.txid
      });
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

  function sourceChannelLabel(channelId: string): string {
    const parsed = parseVrpcChannelId(channelId);
    if (!parsed) return channelId;
    return parsed.systemId;
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
          onClose={() => {
            isOpen = false;
            onClose();
          }}
          showAside={true}
          mobileAsideLabel={i18n.t('genericRequest.summary.toggle')}
          mobileAsideTitle={i18n.t('genericRequest.summary.title')}
        >
          {#snippet aside()}
            <div class="space-y-4">
              <div class="rounded-xl border border-border/70 bg-muted/20 p-4">
                <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  {i18n.t('genericRequest.summary.title')}
                </p>
                <div class="mt-3 space-y-2 text-sm">
                  <div>
                    <p class="text-xs text-muted-foreground">{i18n.t('genericRequest.summary.network')}</p>
                    <p class="font-medium text-foreground">{i18n.t(networkLocaleKey(session.testnet ? 'testnet' : 'mainnet'))}</p>
                  </div>
                  {#if session.signer.identityId}
                    <div>
                      <p class="text-xs text-muted-foreground">{i18n.t('genericRequest.summary.signer')}</p>
                      <p class="identifier-text break-all font-medium text-foreground">{session.signer.identityId}</p>
                    </div>
                  {/if}
                  {#if completionNotice}
                    <div>
                      <p class="text-xs text-muted-foreground">{i18n.t('genericRequest.summary.destination')}</p>
                      <p class="break-all font-medium text-foreground">{responseUriLabel(completionNotice)}</p>
                    </div>
                  {/if}
                </div>
              </div>

              {#if provisioningDetail}
                <div class="rounded-xl border border-border/70 bg-muted/20 p-4 text-sm">
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
            </div>
          {/snippet}

          {#snippet footer()}
            <div class="flex items-center justify-between gap-3">
              <Button variant="secondary" class="min-w-40 px-4 sm:min-w-48 sm:px-6" onclick={handleBack} disabled={updateSubmitting || completing}>
                {currentStepIndex <= 1 ? i18n.t('common.cancel') : i18n.t('common.back')}
              </Button>

              {#if currentStepId === 'updateOverview' || currentStepId === 'updateContent'}
                <Button class="min-w-40 px-4 sm:min-w-48 sm:px-6" onclick={handleUpdateNext} disabled={updateLoading || !updatePreflight}>
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

          <div class="mx-auto w-full max-w-[720px] space-y-6 py-4">
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

            {#if updateLoading && !updatePreflight}
              <div class="flex items-center gap-2 rounded-xl border border-border/70 bg-muted/20 p-4 text-sm text-muted-foreground">
                <Spinner class="size-4" />
                <span>{i18n.t('genericRequest.update.preflightLoading')}</span>
              </div>
            {:else if updatePreflight}
              <div class="grid gap-4 md:grid-cols-2">
                <div class="rounded-xl border border-border/70 bg-muted/20 p-4">
                  <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                    {i18n.t('genericRequest.update.targetIdentity')}
                  </p>
                  <p class="mt-2 text-sm font-semibold text-foreground">
                    {updatePreflight.fullyQualifiedName || updatePreflight.targetIdentity}
                  </p>
                </div>

                <div class="rounded-xl border border-border/70 bg-muted/20 p-4">
                  <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                    {i18n.t('genericRequest.update.summary')}
                  </p>
                  <div class="mt-2 flex items-center gap-6">
                    <div>
                      <p class="text-xl font-semibold text-foreground">{updatePreflight.highRiskChanges.length}</p>
                      <p class="text-xs text-muted-foreground">{i18n.t('genericRequest.update.highRiskCount')}</p>
                    </div>
                    <div>
                      <p class="text-xl font-semibold text-foreground">{updateContentChanges.length}</p>
                      <p class="text-xs text-muted-foreground">{i18n.t('genericRequest.update.contentCount')}</p>
                    </div>
                  </div>
                </div>
              </div>

              {#if updatePreflight.warnings.length > 0}
                <div class="rounded-xl border border-border/70 bg-muted/20 p-4">
                  <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                    {i18n.t('genericRequest.update.warnings')}
                  </p>
                  <ul class="mt-3 space-y-2 text-sm text-foreground">
                    {#each updatePreflight.warnings as warning (`${warning.warningType}:${warning.message}`)}
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
        {:else if currentStepId === 'updateRisk' && updatePreflight}
          <div class="space-y-4">
            <div class="space-y-2">
              <h1 class="text-2xl font-semibold tracking-tight text-foreground">
                {i18n.t('genericRequest.update.highRiskTitle')}
              </h1>
              <p class="text-sm text-muted-foreground">
                {i18n.t('genericRequest.update.highRiskDescription')}
              </p>
            </div>

            <div class="space-y-3">
              {#each updatePreflight.highRiskChanges as change (`${change.changeType}:${change.afterValue || ''}`)}
                <div class="rounded-xl border border-amber-300/70 bg-amber-50/80 p-4 dark:border-amber-500/30 dark:bg-amber-950/20">
                  <p class="text-sm font-semibold text-foreground">{change.changeType}</p>
                  {#if change.beforeValue}
                    <p class="mt-2 text-xs text-muted-foreground">
                      {i18n.t('genericRequest.update.beforeValue')}: <span class="identifier-text">{change.beforeValue}</span>
                    </p>
                  {/if}
                  {#if change.afterValue}
                    <p class="mt-1 text-xs text-muted-foreground">
                      {i18n.t('genericRequest.update.afterValue')}: <span class="identifier-text">{change.afterValue}</span>
                    </p>
                  {/if}
                </div>
              {/each}
            </div>

            <label class="flex items-start gap-3 rounded-xl border border-border/70 bg-muted/20 p-4">
              <Checkbox checked={highRiskAcknowledged} onCheckedChange={(checked) => (highRiskAcknowledged = checked === true)} />
              <span class="text-sm text-foreground">{i18n.t('genericRequest.update.highRiskAcknowledge')}</span>
            </label>
          </div>
        {:else if currentStepId === 'updateContent' && updatePreflight}
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
              {#each updateContentChanges as change (`${change.path}:${change.afterValue || ''}`)}
                <div class="rounded-xl border border-border/70 bg-muted/20 p-4">
                  <p class="text-sm font-semibold text-foreground">{humanizeIdentityUpdatePath(change.path)}</p>
                  <div class="mt-3 grid gap-3 md:grid-cols-2">
                    <div>
                      <p class="text-xs font-medium text-muted-foreground">{i18n.t('genericRequest.update.beforeValue')}</p>
                      <pre class="mt-1 overflow-x-auto whitespace-pre-wrap rounded-lg bg-background/80 px-3 py-2 text-xs text-foreground">{change.beforeValue || i18n.t('genericRequest.update.emptyValue')}</pre>
                    </div>
                    <div>
                      <p class="text-xs font-medium text-muted-foreground">{i18n.t('genericRequest.update.afterValue')}</p>
                      <pre class="mt-1 overflow-x-auto whitespace-pre-wrap rounded-lg bg-background/80 px-3 py-2 text-xs text-foreground">{change.afterValue || i18n.t('genericRequest.update.emptyValue')}</pre>
                    </div>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {:else if currentStepId === 'updateFunding'}
          <div class="space-y-4">
            <div class="space-y-2">
              <h1 class="text-2xl font-semibold tracking-tight text-foreground">
                {i18n.t('genericRequest.update.fundingTitle')}
              </h1>
              <p class="text-sm text-muted-foreground">
                {i18n.t('genericRequest.update.fundingDescription')}
              </p>
            </div>

            <div class="rounded-xl border border-border/70 bg-muted/20 p-4">
              <label
                for="generic-request-funding-source"
                class="block text-xs font-semibold uppercase tracking-wide text-muted-foreground"
              >
                {i18n.t('genericRequest.update.fundingSource')}
              </label>
              <select
                id="generic-request-funding-source"
                class="mt-3 h-10 w-full rounded-md border border-input bg-background px-3 text-sm text-foreground"
                bind:value={selectedSourceChannelId}
                onchange={() => void hydrateUpdatePreflight()}
              >
                {#each walletChannels.channels.filter((channelId) => channelId.startsWith('vrpc.')) as channelId}
                  <option value={channelId}>{sourceChannelLabel(channelId)}</option>
                {/each}
                {#if selectedSourceChannelId && !walletChannels.channels.includes(selectedSourceChannelId)}
                  <option value={selectedSourceChannelId}>{sourceChannelLabel(selectedSourceChannelId)}</option>
                {/if}
              </select>

              {#if updatePreflight}
                <div class="mt-4 rounded-lg bg-background/80 px-3 py-3 text-sm">
                  <p class="font-medium text-foreground">
                    {i18n.t('genericRequest.update.feeLabel', {
                      fee: updatePreflight.fee,
                      currency: updatePreflight.feeCurrency
                    })}
                  </p>
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
