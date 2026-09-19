<!--
  Component: Identity
  Purpose: Linked VerusID management with discovery, linking, and in-section detail view.
-->

<script lang="ts">
  import { onDestroy, onMount, untrack } from 'svelte';
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
  import StarIcon from '@lucide/svelte/icons/star';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
  import Link2Icon from '@lucide/svelte/icons/link-2';
  import { toast } from 'svelte-sonner';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import SearchInput from '$lib/components/common/SearchInput.svelte';
  import WalletEmptyState from '$lib/components/wallet/WalletEmptyState.svelte';
  import { Button } from '$lib/components/ui/button';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { i18nStore } from '$lib/i18n';
  import { queueGenericRequest } from '$lib/stores/genericRequest.js';
  import * as genericRequestService from '$lib/services/genericRequestService.js';
  import * as identityLinkService from '$lib/services/identityLinkService.js';
  import { isForcedWalletLockError } from '$lib/services/walletLockCoordinator.js';
  import type {
    IdentityDetails,
    IdentityProfileLoadResult,
    LinkedIdentity,
    PendingIdentityProfileUpdate,
    ProvisioningJobRecord,
  } from '$lib/types/wallet.js';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay';
  import {
    isCompleteProfileRemoval,
    pendingProfileMatches,
    profileMatchesSnapshot,
  } from '$lib/utils/identityProfileUpdate';
  import { extractWalletErrorMessage, extractWalletErrorType } from '$lib/utils/walletErrors.js';
  import IdentityDetailView from './identity/IdentityDetailView.svelte';
  import IdentityDetailSkeleton from './identity/IdentityDetailSkeleton.svelte';
  import IdentityListSkeleton from './identity/IdentityListSkeleton.svelte';
  import LinkIdentitySheet from './identity/LinkIdentitySheet.svelte';
  import LinkedIdentityCard from './identity/LinkedIdentityCard.svelte';
  import LinkedIdentityRow from './identity/LinkedIdentityRow.svelte';
  import {
    createIdentitySectionSessionState,
    type IdentitySectionSessionState,
  } from './identity/identitySectionSessionState.js';

  const IDENTITY_SKELETON_DELAY_MS = 240;

  const noop = (_nextState?: IdentitySectionSessionState): void => {};

  let {
    walletNetwork = 'mainnet',
    sessionState = createIdentitySectionSessionState(),
    onSessionStateChange = noop,
  }: {
    walletNetwork?: 'mainnet' | 'testnet';
    sessionState?: IdentitySectionSessionState;
    onSessionStateChange?: (nextState: IdentitySectionSessionState) => void;
  } = $props();

  const i18n = $derived($i18nStore);
  import { identityProfiles } from '$lib/contacts/profiles';
  import { contactSession, isContactSessionCurrent } from '$lib/contacts/session';
  import { contactChainId, identityKey } from '$lib/contacts/identity';

  let alive = true;
  onDestroy(() => {
    alive = false;
  });
  const initialSessionState = untrack(() => sessionState);
  const announcedProfileConfirmations = new Set<string>();

  let loading = $state(!initialSessionState.hasLoadedLinkedIdentitiesOnce);
  let hasLoadedLinkedIdentitiesOnce = $state(initialSessionState.hasLoadedLinkedIdentitiesOnce);
  let showDelayedIdentitySkeleton = $state(false);
  let error = $state(initialSessionState.linkedIdentityError);
  let linkedIdentities = $state<LinkedIdentity[]>([...initialSessionState.linkedIdentities]);
  let linkSheetOpen = $state(false);
  let provisioningJobs = $state<ProvisioningJobRecord[]>([...initialSessionState.provisioningJobs]);
  let provisioningLoading = $state(!initialSessionState.hasLoadedProvisioningOnce);
  let provisioningError = $state(initialSessionState.provisioningError);
  let hasLoadedProvisioningOnce = $state(initialSessionState.hasLoadedProvisioningOnce);
  let provisioningBusyJobId = $state<string | null>(null);
  let provisioningRefreshBusy = $state(false);

  let selectedIdentityAddress = $state<string | null>(null);
  let detailsLoading = $state(false);
  let detailsError = $state('');
  let details = $state<IdentityDetails | null>(null);
  let unlinking = $state(false);
  let favoriteBusyIdentityAddress = $state<string | null>(null);
  let profilesByAddress = $state.raw<Record<string, IdentityProfileLoadResult>>({
    ...initialSessionState.profilesByAddress,
  });
  let profileLoadingByAddress = $state<Record<string, boolean>>({});
  let pendingProfilesByAddress = $state<Record<string, PendingIdentityProfileUpdate>>({
    ...initialSessionState.pendingProfilesByAddress,
  });

  let listSearchInput = $state('');
  let listDebouncedSearch = $state('');

  const showingDetail = $derived(Boolean(selectedIdentityAddress));
  const compactMode = $derived(linkedIdentities.length >= 7);
  const favoriteToggleDisabled = $derived(favoriteBusyIdentityAddress !== null);
  const selectedLinkedIdentity = $derived(
    selectedIdentityAddress
      ? (linkedIdentities.find(
          (identity) => identity.identityAddress === selectedIdentityAddress
        ) ?? null)
      : null
  );
  const selectedProfile = $derived(
    selectedIdentityAddress ? (profilesByAddress[selectedIdentityAddress] ?? null) : null
  );
  const selectedProfileLoading = $derived(
    selectedIdentityAddress ? Boolean(profileLoadingByAddress[selectedIdentityAddress]) : false
  );
  const selectedPendingProfile = $derived(
    selectedIdentityAddress ? (pendingProfilesByAddress[selectedIdentityAddress] ?? null) : null
  );

  function normalizeLinkedIdentities(records: LinkedIdentity[]): LinkedIdentity[] {
    return records.map((identity) => ({ ...identity, favorite: Boolean(identity.favorite) }));
  }

  function sortLinkedIdentities(left: LinkedIdentity, right: LinkedIdentity): number {
    const leftDisplay = formatIdentityDisplayName(left).toLowerCase();
    const rightDisplay = formatIdentityDisplayName(right).toLowerCase();
    return (
      leftDisplay.localeCompare(rightDisplay) ||
      left.identityAddress.localeCompare(right.identityAddress)
    );
  }

  function identityMatchesQuery(identity: LinkedIdentity, query: string): boolean {
    if (!query) return true;

    const fields = [
      formatIdentityDisplayName(identity),
      identity.name,
      identity.fullyQualifiedName,
      identity.identityAddress,
    ]
      .map((value) => value?.toLowerCase() ?? '')
      .filter(Boolean);

    return fields.some((value) => value.includes(query));
  }

  const sortedLinkedIdentities = $derived([...linkedIdentities].sort(sortLinkedIdentities));
  const favoriteIdentities = $derived(
    sortedLinkedIdentities.filter((identity) => identity.favorite)
  );
  const nonFavoriteIdentities = $derived(
    sortedLinkedIdentities.filter((identity) => !identity.favorite)
  );

  const filteredFavoriteIdentities = $derived(
    favoriteIdentities.filter((identity) =>
      identityMatchesQuery(identity, listDebouncedSearch.trim().toLowerCase())
    )
  );
  const filteredNonFavoriteIdentities = $derived(
    nonFavoriteIdentities.filter((identity) =>
      identityMatchesQuery(identity, listDebouncedSearch.trim().toLowerCase())
    )
  );
  const hasVisibleIdentities = $derived(
    filteredFavoriteIdentities.length + filteredNonFavoriteIdentities.length > 0
  );
  const showingInitialIdentityLoad = $derived(loading && !hasLoadedLinkedIdentitiesOnce);
  const showBlockingIdentityError = $derived(
    Boolean(error) && !hasLoadedLinkedIdentitiesOnce && linkedIdentities.length === 0
  );
  const showInlineIdentityError = $derived(Boolean(error) && !showBlockingIdentityError);
  const visibleProvisioningJobs = $derived(
    provisioningJobs.filter((job) => job.status !== 'linked')
  );

  $effect(() => {
    onSessionStateChange({
      linkedIdentities,
      linkedIdentityError: error,
      hasLoadedLinkedIdentitiesOnce,
      provisioningJobs,
      provisioningError,
      hasLoadedProvisioningOnce,
      profilesByAddress,
      pendingProfilesByAddress,
    });
  });

  $effect(() => {
    const pending = Object.values(pendingProfilesByAddress);
    if (pending.length === 0) return undefined;

    const timer = setInterval(() => {
      for (const record of pending) {
        void loadIdentityProfile(record.identityAddress, true);
      }
    }, 30_000);
    return () => clearInterval(timer);
  });

  $effect(() => {
    const query = listSearchInput;
    const timer = setTimeout(() => {
      listDebouncedSearch = query;
    }, 150);

    return () => clearTimeout(timer);
  });

  $effect(() => {
    let timer: ReturnType<typeof setTimeout> | null = null;

    if (!showingInitialIdentityLoad || linkedIdentities.length > 0) {
      showDelayedIdentitySkeleton = false;
    } else {
      showDelayedIdentitySkeleton = false;
      timer = setTimeout(() => {
        showDelayedIdentitySkeleton = true;
      }, IDENTITY_SKELETON_DELAY_MS);
    }

    return () => {
      if (timer) clearTimeout(timer);
    };
  });

  onMount(() => {
    void loadPendingProfiles();
    if (hasLoadedLinkedIdentitiesOnce) {
      void loadLinkedIdentities({ background: true });
    } else {
      void loadLinkedIdentities();
    }

    if (!hasLoadedProvisioningOnce) {
      void loadProvisioningJobs(false);
    }
  });

  async function loadPendingProfiles(): Promise<void> {
    try {
      const records = await identityLinkService.getPendingIdentityProfileUpdates();
      const activeRecords: Array<[string, PendingIdentityProfileUpdate]> = [];
      const confirmedRecords: PendingIdentityProfileUpdate[] = [];
      for (const record of records) {
        const key = record.identityAddress;
        const profile = profilesByAddress[key];
        if (profile && pendingProfileMatches(record, profile)) {
          showProfileConfirmation(record);
          confirmedRecords.push(record);
        } else {
          activeRecords.push([key, record]);
        }
      }
      pendingProfilesByAddress = Object.fromEntries(
        activeRecords.filter(
          ([, record]) => !announcedProfileConfirmations.has(profileConfirmationKey(record))
        )
      );
      for (const record of confirmedRecords) {
        try {
          await identityLinkService.clearPendingIdentityProfileUpdate(
            record.identityAddress,
            record.txid
          );
        } catch {
          // The confirmed profile remains authoritative. A later session can
          // retry cleanup of this non-secret local pending marker.
        }
      }
    } catch {
      // Pending metadata is a progressive enhancement. Profile and identity
      // reads remain available when local public state cannot be loaded.
    }
  }

  function mapIdentityError(errorValue: unknown, fallbackKey: string): string {
    if (isForcedWalletLockError(errorValue)) {
      return '';
    }

    const errorType = extractWalletErrorType(errorValue);

    switch (errorType) {
      case 'IdentityOwnershipMismatch':
        return i18n.t('wallet.identity.error.ownershipMismatch');
      case 'IdentityNotFound':
        return i18n.t('wallet.identity.error.notFound');
      case 'NetworkError':
        return i18n.t('wallet.identity.error.network');
      case 'IdentityFavoriteLimitReached':
        return i18n.t('wallet.identity.favorite.limitReached');
      default:
        break;
    }

    const extractedMessage = extractWalletErrorMessage(errorValue);
    if (extractedMessage) return extractedMessage;

    return i18n.t(fallbackKey);
  }

  async function loadLinkedIdentities({ background = false }: { background?: boolean } = {}) {
    loading = true;
    error = '';

    try {
      linkedIdentities = normalizeLinkedIdentities(await identityLinkService.getLinkedIdentities());
      hasLoadedLinkedIdentitiesOnce = true;
      void loadIdentityProfiles(linkedIdentities, background);
    } catch (errorValue) {
      error = mapIdentityError(errorValue, 'wallet.identity.error.load');
    } finally {
      loading = false;
    }
  }

  function profileConfirmationKey(pending: PendingIdentityProfileUpdate): string {
    return `${pending.identityAddress}:${pending.txid.toLowerCase()}`;
  }

  function showProfileConfirmation(pending: PendingIdentityProfileUpdate): void {
    const confirmationKey = profileConfirmationKey(pending);
    if (announcedProfileConfirmations.has(confirmationKey)) return;
    announcedProfileConfirmations.add(confirmationKey);
    toast.success(
      i18n.t(
        isCompleteProfileRemoval(pending.previousProfile, pending.proposedProfile)
          ? 'wallet.identity.profile.confirmed.removalToast'
          : 'wallet.identity.profile.confirmed.title'
      )
    );
  }

  $effect(() => {
    const session = $contactSession;
    if (!session) return;
    const shared = $identityProfiles;
    for (const identity of linkedIdentities) {
      const entry =
        shared[
          identityKey({
            identityAddress: identity.identityAddress,
            fullyQualifiedName: '',
            network: session.network,
            chainId: contactChainId(session.network),
          })
        ];
      const pending = pendingProfilesByAddress[identity.identityAddress];
      if (
        entry?.profile &&
        !entry.loading &&
        (!pending ||
          pendingProfileMatches(pending, entry.profile) ||
          profileMatchesSnapshot(pending.previousProfile, entry.profile)) &&
        profilesByAddress[identity.identityAddress] !== entry.profile
      ) {
        profilesByAddress = { ...profilesByAddress, [identity.identityAddress]: entry.profile };
      }
    }
  });

  async function loadIdentityProfile(
    identityAddress: string,
    preserveOnFailure = false
  ): Promise<void> {
    const key = identityAddress;
    const session = $contactSession;
    const current = () => alive && (!session || isContactSessionCurrent(session));
    if (profileLoadingByAddress[key]) return;
    profileLoadingByAddress = { ...profileLoadingByAddress, [key]: true };
    try {
      const profile = await identityLinkService.getIdentityProfile(
        identityAddress,
        preserveOnFailure,
        identityAddress === selectedIdentityAddress
      );
      if (!current()) return;
      const pending = pendingProfilesByAddress[key];
      if (pending && pendingProfileMatches(pending, profile)) {
        profilesByAddress = { ...profilesByAddress, [key]: profile };
        showProfileConfirmation(pending);
        const { [key]: _confirmed, ...remaining } = pendingProfilesByAddress;
        pendingProfilesByAddress = remaining;
        try {
          await identityLinkService.clearPendingIdentityProfileUpdate(
            pending.identityAddress,
            pending.txid
          );
        } catch {
          // The verified profile is already confirmed; only local marker cleanup failed.
        }
      } else if (!pending || profileMatchesSnapshot(pending.previousProfile, profile)) {
        // While a publication is pending, only replace the visible profile
        // with the last prepared snapshot or the proven submitted revision.
        profilesByAddress = { ...profilesByAddress, [key]: profile };
      }
    } catch {
      if (!current()) return;
      if ((preserveOnFailure || pendingProfilesByAddress[key]) && profilesByAddress[key]) return;
      profilesByAddress = {
        ...profilesByAddress,
        [key]: {
          state: 'unavailable',
          avatar: null,
          description: null,
          issues: [{ field: null, code: 'profile_load_failed' }],
          readHeight: null,
          revisionTxid: null,
        },
      };
    } finally {
      if (current()) {
        const { [key]: _finished, ...remaining } = profileLoadingByAddress;
        profileLoadingByAddress = remaining;
      }
    }
  }

  async function loadIdentityProfiles(
    identities: LinkedIdentity[],
    refreshExisting = false
  ): Promise<void> {
    const candidates = identities.filter(
      (identity) =>
        identity.identityAddress === selectedIdentityAddress ||
        (refreshExisting && profilesByAddress[identity.identityAddress])
    );
    for (let index = 0; index < candidates.length; index += 4) {
      await Promise.all(
        candidates
          .slice(index, index + 4)
          .map((identity) => loadIdentityProfile(identity.identityAddress, refreshExisting))
      );
    }
  }

  function applyLinkedIdentities(updatedLinked: LinkedIdentity[]) {
    linkedIdentities = normalizeLinkedIdentities(updatedLinked);
    hasLoadedLinkedIdentitiesOnce = true;
    error = '';
    if (!selectedIdentityAddress) return;

    const stillExists = linkedIdentities.some(
      (identity) => identity.identityAddress === selectedIdentityAddress
    );

    if (!stillExists) {
      selectedIdentityAddress = null;
      details = null;
      detailsError = '';
    }
  }

  function isFavoriteToggleBusy(identity: LinkedIdentity): boolean {
    return favoriteBusyIdentityAddress === identity.identityAddress;
  }

  async function toggleFavorite(identity: LinkedIdentity) {
    if (favoriteBusyIdentityAddress) return;

    favoriteBusyIdentityAddress = identity.identityAddress;

    try {
      const updated = await identityLinkService.setLinkedIdentityFavorite({
        identityAddress: identity.identityAddress,
        favorite: !identity.favorite,
      });
      applyLinkedIdentities(updated);
    } catch (errorValue) {
      const message = mapIdentityError(errorValue, 'wallet.identity.error.load');
      if (message) {
        toast.error(message);
      }
    } finally {
      favoriteBusyIdentityAddress = null;
    }
  }

  async function openIdentityDetails(identityAddress: string) {
    selectedIdentityAddress = identityAddress;
    details = null;
    detailsError = '';
    detailsLoading = true;
    void loadIdentityProfile(identityAddress);

    try {
      details = await identityLinkService.getIdentityDetails(identityAddress);
    } catch (errorValue) {
      detailsError = mapIdentityError(errorValue, 'wallet.identity.error.details');
    } finally {
      detailsLoading = false;
    }
  }

  function handleProfileSubmitted(update: PendingIdentityProfileUpdate): void {
    if (!selectedIdentityAddress) return;
    const key = selectedIdentityAddress;
    pendingProfilesByAddress = {
      ...pendingProfilesByAddress,
      [key]: update,
    };
  }

  function closeDetailView() {
    selectedIdentityAddress = null;
    details = null;
    detailsError = '';
    detailsLoading = false;
  }

  async function unlinkSelectedIdentity() {
    if (!selectedIdentityAddress || unlinking) return;

    unlinking = true;
    detailsError = '';

    try {
      const updatedLinked = await identityLinkService.unlinkIdentity({
        identityAddress: selectedIdentityAddress,
      });

      applyLinkedIdentities(updatedLinked);
      selectedIdentityAddress = null;
      details = null;
      detailsError = '';
    } catch (errorValue) {
      detailsError = mapIdentityError(errorValue, 'wallet.identity.error.unlink');
    } finally {
      unlinking = false;
    }
  }

  function mapProvisioningError(errorValue: unknown): string {
    if (isForcedWalletLockError(errorValue)) {
      return '';
    }

    const errorType = extractWalletErrorType(errorValue);

    switch (errorType) {
      case 'IdentityOwnershipMismatch':
        return i18n.t('wallet.identity.error.ownershipMismatch');
      default:
        break;
    }

    const extractedMessage = extractWalletErrorMessage(errorValue);
    if (extractedMessage) return extractedMessage;

    return i18n.t('wallet.identity.provisioning.errorLoad');
  }

  async function loadProvisioningJobs(refresh: boolean): Promise<void> {
    if (refresh) {
      provisioningRefreshBusy = true;
    } else {
      provisioningLoading = true;
    }
    provisioningError = '';

    try {
      provisioningJobs = refresh
        ? await genericRequestService.refreshIdentityProvisioningJobs()
        : await genericRequestService.listIdentityProvisioningJobs();
      hasLoadedProvisioningOnce = true;
    } catch (errorValue) {
      provisioningError = mapProvisioningError(errorValue);
    } finally {
      provisioningLoading = false;
      provisioningRefreshBusy = false;
    }
  }

  function provisioningStatusLabel(status: string): string {
    switch (status) {
      case 'ready':
        return i18n.t('wallet.identity.provisioning.status.ready');
      case 'expired':
        return i18n.t('wallet.identity.provisioning.status.expired');
      case 'linked':
        return i18n.t('wallet.identity.provisioning.status.linked');
      default:
        return i18n.t('wallet.identity.provisioning.status.pending');
    }
  }

  function provisioningStatusBadgeClass(status: string): string {
    switch (status) {
      case 'ready':
        return 'bg-emerald-500/10 text-emerald-700 dark:bg-emerald-500/15 dark:text-emerald-300';
      case 'expired':
        return 'bg-destructive/10 text-destructive';
      case 'linked':
        return 'bg-primary/10 text-primary';
      default:
        return 'bg-amber-500/12 text-amber-700 dark:bg-amber-500/15 dark:text-amber-300';
    }
  }

  async function openProvisioningInfo(url: string): Promise<void> {
    try {
      await openUrl(url);
    } catch (errorValue) {
      const message = mapProvisioningError(errorValue);
      if (message) {
        toast.error(message);
      }
    }
  }

  async function handleLinkProvisioningJob(job: ProvisioningJobRecord): Promise<void> {
    if (provisioningBusyJobId) return;

    provisioningBusyJobId = job.jobId;
    provisioningError = '';

    try {
      const result = await genericRequestService.linkReadyIdentityProvisioning(job.jobId);
      applyLinkedIdentities(result.linkedIdentities);
      provisioningJobs = provisioningJobs.map((entry) =>
        entry.jobId === result.job.jobId ? result.job : entry
      );

      if (job.hasResponseUris) {
        queueGenericRequest({
          input: job.requestHex,
          passthroughAutoLinkFqn: job.requestedFqn,
          source: 'provisioning',
        });
        toast.success(i18n.t('wallet.identity.provisioning.linkAndContinueQueued'));
      } else {
        toast.success(i18n.t('wallet.identity.provisioning.linked'));
      }
    } catch (errorValue) {
      const message = mapProvisioningError(errorValue);
      provisioningError = message;
      if (message) {
        toast.error(message);
      }
    } finally {
      provisioningBusyJobId = null;
    }
  }
</script>

{#if showingDetail}
  {#if detailsLoading}
    <IdentityDetailSkeleton identity={selectedLinkedIdentity} />
  {:else if detailsError}
    <div class="mx-auto flex h-full w-full max-w-4xl min-w-0 flex-col gap-3 px-6 pt-3 pb-6">
      <button
        type="button"
        class="inline-flex items-center gap-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
        onclick={closeDetailView}
      >
        {i18n.t('wallet.identity.detail.back')}
      </button>

      <p class="rounded-md bg-destructive/12 px-3 py-2 text-sm text-destructive">{detailsError}</p>

      {#if selectedLinkedIdentity}
        <Button
          variant="secondary"
          class="w-fit"
          onclick={() => openIdentityDetails(selectedLinkedIdentity.identityAddress)}
        >
          {i18n.t('common.retry')}
        </Button>
      {/if}
    </div>
  {:else if details}
    <IdentityDetailView
      {details}
      profile={selectedProfile}
      profileLoading={selectedProfileLoading}
      pendingProfile={selectedPendingProfile}
      {unlinking}
      onBack={closeDetailView}
      onUnlink={unlinkSelectedIdentity}
      onProfileSubmitted={handleProfileSubmitted}
    />
  {/if}
{:else}
  <div class="mx-auto flex h-full w-full max-w-5xl flex-col px-8 pb-8" aria-busy={loading}>
    {#if showingInitialIdentityLoad || showBlockingIdentityError || linkedIdentities.length > 0}
      <header class="flex h-[58px] shrink-0 items-center">
        <h2 class="text-2xl leading-8 font-semibold tracking-tight">
          {i18n.t('wallet.sidebar.identities')}
        </h2>
      </header>
    {/if}

    {#if loading}
      <p class="sr-only" role="status">{i18n.t('wallet.identity.loading')}</p>
    {/if}

    {#if provisioningLoading || provisioningError || visibleProvisioningJobs.length > 0}
      <section
        class={`${linkedIdentities.length === 0 ? 'mb-8' : 'mb-6'} rounded-2xl bg-muted/30 p-4`}
      >
        <div class="flex items-start justify-between gap-3">
          <div>
            <p class="text-sm font-semibold text-foreground">
              {i18n.t('wallet.identity.provisioning.title')}
            </p>
            <p class="mt-1 text-sm text-muted-foreground">
              {i18n.t('wallet.identity.provisioning.description')}
            </p>
          </div>

          <Button
            variant="ghost"
            size="sm"
            class="shrink-0 gap-2"
            onclick={() => void loadProvisioningJobs(true)}
            disabled={provisioningRefreshBusy || provisioningBusyJobId !== null}
          >
            <RefreshCwIcon class={`size-4 ${provisioningRefreshBusy ? 'animate-spin' : ''}`} />
            {i18n.t('wallet.identity.provisioning.refresh')}
          </Button>
        </div>

        {#if provisioningError}
          <p class="mt-3 rounded-md bg-destructive/12 px-3 py-2 text-sm text-destructive">
            {provisioningError}
          </p>
        {/if}

        {#if provisioningLoading}
          <p class="mt-3 rounded-lg bg-background/80 px-3 py-2.5 text-sm text-muted-foreground">
            {i18n.t('wallet.identity.provisioning.loading')}
          </p>
        {:else if visibleProvisioningJobs.length > 0}
          <div class="mt-4 space-y-3">
            {#each visibleProvisioningJobs as job (job.jobId)}
              <div class="rounded-xl bg-background/75 p-4 dark:bg-background/35">
                <div class="flex items-start justify-between gap-3">
                  <div class="min-w-0">
                    <div class="flex flex-wrap items-center gap-2">
                      <span
                        class={`rounded-full px-2.5 py-1 text-[11px] font-semibold tracking-wide uppercase ${provisioningStatusBadgeClass(job.status)}`}
                      >
                        {provisioningStatusLabel(job.status)}
                      </span>
                      {#if job.hasResponseUris}
                        <span
                          class="rounded-full bg-primary/8 px-2.5 py-1 text-[11px] font-semibold tracking-wide text-primary uppercase"
                        >
                          {i18n.t('wallet.identity.provisioning.callbackPending')}
                        </span>
                      {/if}
                    </div>

                    <p class="mt-3 truncate text-sm font-semibold text-foreground">
                      {job.requestedFqn}
                    </p>

                    {#if job.signingId}
                      <p class="mt-1 text-xs break-all text-muted-foreground">
                        {i18n.t('wallet.identity.provisioning.serviceLabel', {
                          value: job.signingId,
                        })}
                      </p>
                    {/if}

                    {#if job.infoUri}
                      <button
                        type="button"
                        class="mt-3 inline-flex items-center gap-1.5 text-xs font-medium text-primary hover:underline"
                        onclick={() => void openProvisioningInfo(job.infoUri ?? '')}
                      >
                        <ExternalLinkIcon class="size-3.5" />
                        {i18n.t('wallet.identity.provisioning.info')}
                      </button>
                    {/if}

                    {#if job.error}
                      <p
                        class="mt-3 rounded-md bg-destructive/12 px-3 py-2 text-xs text-destructive"
                      >
                        {job.error}
                      </p>
                    {/if}
                  </div>

                  <div class="flex shrink-0 items-start gap-2">
                    {#if job.status === 'ready'}
                      <Button
                        size="sm"
                        class="gap-2"
                        onclick={() => void handleLinkProvisioningJob(job)}
                        disabled={provisioningBusyJobId !== null}
                      >
                        <Link2Icon class="size-4" />
                        {job.hasResponseUris
                          ? i18n.t('wallet.identity.provisioning.linkAndContinue')
                          : i18n.t('wallet.identity.provisioning.linkIdentity')}
                      </Button>
                    {/if}
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </section>
    {/if}

    {#if showingInitialIdentityLoad && linkedIdentities.length === 0}
      <IdentityListSkeleton
        revealed={showDelayedIdentitySkeleton}
        onOpenLinkSheet={() => (linkSheetOpen = true)}
      />
    {:else if showBlockingIdentityError}
      <div class="space-y-4">
        <IdentityListSkeleton revealed={false} onOpenLinkSheet={() => (linkSheetOpen = true)} />

        <div class="space-y-3">
          <p class="rounded-md bg-destructive/12 px-3 py-2 text-sm text-destructive">{error}</p>
          <Button variant="secondary" class="w-fit" onclick={loadLinkedIdentities}>
            {i18n.t('common.retry')}
          </Button>
        </div>
      </div>
    {:else}
      {#if linkedIdentities.length > 0}
        <div class="flex min-w-0 items-center gap-3">
          <div class="min-w-0 flex-[3]">
            <SearchInput
              bind:value={listSearchInput}
              placeholder={i18n.t('wallet.identity.list.searchPlaceholder')}
            />
          </div>

          <Button
            variant="secondary"
            size="lg"
            class="h-10 min-w-[12rem] flex-1 justify-center gap-1.5 rounded-md px-3"
            onclick={() => (linkSheetOpen = true)}
          >
            <PlusIcon class="size-4" />
            {i18n.t('wallet.identity.list.linkButton')}
          </Button>
        </div>
      {/if}

      {#if showInlineIdentityError}
        <div class={linkedIdentities.length > 0 ? 'mt-4 space-y-3' : 'space-y-3'}>
          <p class="rounded-md bg-destructive/12 px-3 py-2 text-sm text-destructive">{error}</p>
          <Button variant="secondary" class="w-fit" onclick={loadLinkedIdentities}>
            {i18n.t('common.retry')}
          </Button>
        </div>
      {/if}

      {#if linkedIdentities.length === 0}
        <div class="-mb-8 flex min-h-0 flex-1">
          <WalletEmptyState
            illustration="verus-id"
            title={i18n.t('wallet.identity.empty.title')}
            actionLabel={i18n.t('wallet.identity.empty.cta')}
            onAction={() => (linkSheetOpen = true)}
            testId="identity-empty"
          />
        </div>
      {:else}
        {#if !hasVisibleIdentities}
          <p
            class="mt-4 rounded-lg bg-muted/55 px-3 py-2.5 text-sm text-muted-foreground dark:bg-muted/50"
          >
            {i18n.t('wallet.identity.sheet.emptySearch')}
          </p>
        {:else}
          <div class="mt-4 min-h-0 flex-1">
            <ScrollArea.Root class="h-full" type="scroll">
              <ScrollArea.Viewport class="h-full pr-1">
                {#if filteredFavoriteIdentities.length > 0}
                  <section>
                    <div
                      class="flex items-center gap-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase"
                    >
                      <StarIcon class="size-3.5 fill-current text-amber-500" />
                      <span>{i18n.t('wallet.identity.list.favorites')}</span>
                      <span class="text-[11px] text-muted-foreground/80"
                        >{favoriteIdentities.length}/2</span
                      >
                    </div>

                    <div
                      class={`${compactMode ? 'mt-2 space-y-2' : 'mt-2 grid gap-3 md:grid-cols-2 xl:grid-cols-3'}`}
                    >
                      {#each filteredFavoriteIdentities as identity (identity.identityAddress)}
                        {#if compactMode}
                          <LinkedIdentityRow
                            {identity}
                            onProfileVisible={() =>
                              void loadIdentityProfile(identity.identityAddress)}
                            profile={profilesByAddress[identity.identityAddress] ?? null}
                            pendingProfile={pendingProfilesByAddress[identity.identityAddress] ??
                              null}
                            favoriteBusy={isFavoriteToggleBusy(identity)}
                            favoriteDisabled={favoriteToggleDisabled}
                            onSelect={(selected) => openIdentityDetails(selected.identityAddress)}
                            onToggleFavorite={toggleFavorite}
                          />
                        {:else}
                          <LinkedIdentityCard
                            {identity}
                            onProfileVisible={() =>
                              void loadIdentityProfile(identity.identityAddress)}
                            profile={profilesByAddress[identity.identityAddress] ?? null}
                            pendingProfile={pendingProfilesByAddress[identity.identityAddress] ??
                              null}
                            favoriteBusy={isFavoriteToggleBusy(identity)}
                            favoriteDisabled={favoriteToggleDisabled}
                            onSelect={(selected) => openIdentityDetails(selected.identityAddress)}
                            onToggleFavorite={toggleFavorite}
                          />
                        {/if}
                      {/each}
                    </div>
                  </section>
                {/if}

                {#if filteredNonFavoriteIdentities.length > 0}
                  <section class={`${filteredFavoriteIdentities.length > 0 ? 'mt-5' : ''}`}>
                    {#if filteredFavoriteIdentities.length > 0}
                      <p
                        class="text-xs font-semibold tracking-wide text-muted-foreground uppercase"
                      >
                        {i18n.t('wallet.identity.list.all')}
                      </p>
                    {/if}

                    <div
                      class={`${filteredFavoriteIdentities.length > 0 ? 'mt-2' : ''} ${
                        compactMode ? 'space-y-2' : 'grid gap-3 md:grid-cols-2 xl:grid-cols-3'
                      }`}
                    >
                      {#each filteredNonFavoriteIdentities as identity (identity.identityAddress)}
                        {#if compactMode}
                          <LinkedIdentityRow
                            {identity}
                            onProfileVisible={() =>
                              void loadIdentityProfile(identity.identityAddress)}
                            profile={profilesByAddress[identity.identityAddress] ?? null}
                            pendingProfile={pendingProfilesByAddress[identity.identityAddress] ??
                              null}
                            favoriteBusy={isFavoriteToggleBusy(identity)}
                            favoriteDisabled={favoriteToggleDisabled}
                            onSelect={(selected) => openIdentityDetails(selected.identityAddress)}
                            onToggleFavorite={toggleFavorite}
                          />
                        {:else}
                          <LinkedIdentityCard
                            {identity}
                            onProfileVisible={() =>
                              void loadIdentityProfile(identity.identityAddress)}
                            profile={profilesByAddress[identity.identityAddress] ?? null}
                            pendingProfile={pendingProfilesByAddress[identity.identityAddress] ??
                              null}
                            favoriteBusy={isFavoriteToggleBusy(identity)}
                            favoriteDisabled={favoriteToggleDisabled}
                            onSelect={(selected) => openIdentityDetails(selected.identityAddress)}
                            onToggleFavorite={toggleFavorite}
                          />
                        {/if}
                      {/each}
                    </div>
                  </section>
                {/if}
              </ScrollArea.Viewport>
              <ScrollArea.Scrollbar orientation="vertical" />
            </ScrollArea.Root>
          </div>
        {/if}
      {/if}
    {/if}
  </div>
{/if}

<LinkIdentitySheet
  bind:isOpen={linkSheetOpen}
  onLinkedChange={applyLinkedIdentities}
  allowManualLinkEntry={walletNetwork === 'testnet'}
/>
