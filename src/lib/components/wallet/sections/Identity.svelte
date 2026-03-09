<!--
  Component: Identity
  Purpose: Linked VerusID management with discovery, linking, and in-section detail view.
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
  import StarIcon from '@lucide/svelte/icons/star';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
  import Link2Icon from '@lucide/svelte/icons/link-2';
  import { toast } from 'svelte-sonner';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import VerusIdAtIcon from '$lib/components/icons/VerusIdAtIcon.svelte';
  import SearchInput from '$lib/components/common/SearchInput.svelte';
  import { Button } from '$lib/components/ui/button';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { i18nStore } from '$lib/i18n';
  import { queueGenericRequest } from '$lib/stores/genericRequest.js';
  import * as genericRequestService from '$lib/services/genericRequestService.js';
  import * as identityLinkService from '$lib/services/identityLinkService.js';
  import type { IdentityDetails, LinkedIdentity, ProvisioningJobRecord } from '$lib/types/wallet.js';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay';
  import { extractWalletErrorMessage, extractWalletErrorType } from '$lib/utils/walletErrors.js';
  import IdentityDetailView from './identity/IdentityDetailView.svelte';
  import LinkIdentitySheet from './identity/LinkIdentitySheet.svelte';
  import LinkedIdentityCard from './identity/LinkedIdentityCard.svelte';
  import LinkedIdentityRow from './identity/LinkedIdentityRow.svelte';

   
  let { walletNetwork = 'mainnet' }: { walletNetwork?: 'mainnet' | 'testnet' } = $props();
   

  const i18n = $derived($i18nStore);

  let loading = $state(true);
  let error = $state('');
  let linkedIdentities = $state<LinkedIdentity[]>([]);
  let linkSheetOpen = $state(false);
  let provisioningJobs = $state<ProvisioningJobRecord[]>([]);
  let provisioningLoading = $state(false);
  let provisioningError = $state('');
  let provisioningBusyJobId = $state<string | null>(null);
  let provisioningRefreshBusy = $state(false);

  let selectedIdentityAddress = $state<string | null>(null);
  let detailsLoading = $state(false);
  let detailsError = $state('');
  let details = $state<IdentityDetails | null>(null);
  let unlinking = $state(false);
  let favoriteBusyIdentityAddress = $state<string | null>(null);

  let listSearchInput = $state('');
  let listDebouncedSearch = $state('');

  const showingDetail = $derived(Boolean(selectedIdentityAddress));
  const compactMode = $derived(linkedIdentities.length >= 7);
  const selectedLinkedIdentity = $derived(
    selectedIdentityAddress
      ? linkedIdentities.find(
          (identity) => identity.identityAddress.toLowerCase() === selectedIdentityAddress?.toLowerCase()
        ) ?? null
      : null
  );

  function normalizeLinkedIdentities(records: LinkedIdentity[]): LinkedIdentity[] {
    return records.map((identity) => ({ ...identity, favorite: Boolean(identity.favorite) }));
  }

  function sortLinkedIdentities(left: LinkedIdentity, right: LinkedIdentity): number {
    const leftDisplay = formatIdentityDisplayName(left).toLowerCase();
    const rightDisplay = formatIdentityDisplayName(right).toLowerCase();
    return leftDisplay.localeCompare(rightDisplay) || left.identityAddress.localeCompare(right.identityAddress);
  }

  function identityMatchesQuery(identity: LinkedIdentity, query: string): boolean {
    if (!query) return true;

    const fields = [
      formatIdentityDisplayName(identity),
      identity.name,
      identity.fullyQualifiedName,
      identity.identityAddress
    ]
      .map((value) => value?.toLowerCase() ?? '')
      .filter(Boolean);

    return fields.some((value) => value.includes(query));
  }

  const sortedLinkedIdentities = $derived([...linkedIdentities].sort(sortLinkedIdentities));
  const favoriteIdentities = $derived(sortedLinkedIdentities.filter((identity) => identity.favorite));
  const nonFavoriteIdentities = $derived(sortedLinkedIdentities.filter((identity) => !identity.favorite));

  const filteredFavoriteIdentities = $derived(
    favoriteIdentities.filter((identity) => identityMatchesQuery(identity, listDebouncedSearch.trim().toLowerCase()))
  );
  const filteredNonFavoriteIdentities = $derived(
    nonFavoriteIdentities.filter((identity) => identityMatchesQuery(identity, listDebouncedSearch.trim().toLowerCase()))
  );
  const hasVisibleIdentities = $derived(
    filteredFavoriteIdentities.length + filteredNonFavoriteIdentities.length > 0
  );
  const visibleProvisioningJobs = $derived(
    provisioningJobs.filter((job) => job.status !== 'linked')
  );

  $effect(() => {
    const query = listSearchInput;
    const timer = setTimeout(() => {
      listDebouncedSearch = query;
    }, 150);

    return () => clearTimeout(timer);
  });

  onMount(async () => {
    await Promise.all([loadLinkedIdentities(), loadProvisioningJobs(false)]);
  });

  function mapIdentityError(errorValue: unknown, fallbackKey: string): string {
    const errorType = extractWalletErrorType(errorValue);

    switch (errorType) {
      case 'WalletLocked':
        return i18n.t('wallet.identity.error.walletLocked');
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

  async function loadLinkedIdentities() {
    loading = true;
    error = '';

    try {
      linkedIdentities = normalizeLinkedIdentities(await identityLinkService.getLinkedIdentities());
    } catch (errorValue) {
      error = mapIdentityError(errorValue, 'wallet.identity.error.load');
    } finally {
      loading = false;
    }
  }

  function applyLinkedIdentities(updatedLinked: LinkedIdentity[]) {
    linkedIdentities = normalizeLinkedIdentities(updatedLinked);
    if (!selectedIdentityAddress) return;

    const stillExists = linkedIdentities.some(
      (identity) => identity.identityAddress.toLowerCase() === selectedIdentityAddress?.toLowerCase()
    );

    if (!stillExists) {
      selectedIdentityAddress = null;
      details = null;
      detailsError = '';
    }
  }

  async function toggleFavorite(identity: LinkedIdentity) {
    if (favoriteBusyIdentityAddress) return;

    favoriteBusyIdentityAddress = identity.identityAddress;

    try {
      const updated = await identityLinkService.setLinkedIdentityFavorite({
        identityAddress: identity.identityAddress,
        favorite: !identity.favorite
      });
      applyLinkedIdentities(updated);
    } catch (errorValue) {
      toast.error(mapIdentityError(errorValue, 'wallet.identity.error.load'));
    } finally {
      favoriteBusyIdentityAddress = null;
    }
  }

  async function openIdentityDetails(identityAddress: string) {
    selectedIdentityAddress = identityAddress;
    details = null;
    detailsError = '';
    detailsLoading = true;

    try {
      details = await identityLinkService.getIdentityDetails(identityAddress);
    } catch (errorValue) {
      detailsError = mapIdentityError(errorValue, 'wallet.identity.error.details');
    } finally {
      detailsLoading = false;
    }
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
        identityAddress: selectedIdentityAddress
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
    const errorType = extractWalletErrorType(errorValue);

    switch (errorType) {
      case 'WalletLocked':
        return i18n.t('wallet.identity.error.walletLocked');
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
      toast.error(mapProvisioningError(errorValue));
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
          source: 'provisioning'
        });
        toast.success(i18n.t('wallet.identity.provisioning.linkAndContinueQueued'));
      } else {
        toast.success(i18n.t('wallet.identity.provisioning.linked'));
      }
    } catch (errorValue) {
      const message = mapProvisioningError(errorValue);
      provisioningError = message;
      toast.error(message);
    } finally {
      provisioningBusyJobId = null;
    }
  }
</script>

{#if showingDetail}
  {#if detailsLoading}
    <div class="mx-auto flex h-full w-full max-w-4xl flex-col gap-3 p-6">
      <p class="text-sm text-muted-foreground">{i18n.t('wallet.identity.detail.loading')}</p>
    </div>
  {:else if detailsError}
    <div class="mx-auto flex h-full w-full max-w-4xl flex-col gap-3 p-6">
      <button
        type="button"
        class="text-muted-foreground hover:text-foreground inline-flex items-center gap-1.5 text-sm transition-colors"
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
      {unlinking}
      onBack={closeDetailView}
      onUnlink={unlinkSelectedIdentity}
    />
  {/if}
{:else}
  <div class="mx-auto flex h-full w-full max-w-5xl flex-col p-6">
    {#if loading}
      <p class="text-sm text-muted-foreground">{i18n.t('wallet.identity.loading')}</p>
    {:else if error}
      <div class="space-y-3">
        <p class="rounded-md bg-destructive/12 px-3 py-2 text-sm text-destructive">{error}</p>
        <Button variant="secondary" class="w-fit" onclick={loadLinkedIdentities}>
          {i18n.t('common.retry')}
        </Button>
      </div>
    {:else}
      {#if provisioningLoading || provisioningError || visibleProvisioningJobs.length > 0}
        <section class={`${linkedIdentities.length === 0 ? 'mb-8' : 'mb-6'} rounded-2xl border border-border/70 bg-muted/18 p-4`}>
          <div class="flex items-start justify-between gap-3">
            <div>
              <p class="text-sm font-semibold text-foreground">{i18n.t('wallet.identity.provisioning.title')}</p>
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
                <div class="rounded-xl border border-border/70 bg-background/85 p-4">
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <div class="flex flex-wrap items-center gap-2">
                        <span class={`rounded-full px-2.5 py-1 text-[11px] font-semibold uppercase tracking-wide ${provisioningStatusBadgeClass(job.status)}`}>
                          {provisioningStatusLabel(job.status)}
                        </span>
                        {#if job.hasResponseUris}
                          <span class="rounded-full bg-primary/8 px-2.5 py-1 text-[11px] font-semibold uppercase tracking-wide text-primary">
                            {i18n.t('wallet.identity.provisioning.callbackPending')}
                          </span>
                        {/if}
                      </div>

                      <p class="mt-3 truncate text-sm font-semibold text-foreground">{job.requestedFqn}</p>

                      {#if job.signingId}
                        <p class="mt-1 break-all text-xs text-muted-foreground">
                          {i18n.t('wallet.identity.provisioning.serviceLabel', { value: job.signingId })}
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
                        <p class="mt-3 rounded-md bg-destructive/12 px-3 py-2 text-xs text-destructive">
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

      {#if linkedIdentities.length === 0}
      <div class="-mt-6 flex h-full flex-col items-center justify-center px-6 py-12 text-center">
        <div class="bg-background/70 text-primary inline-flex size-14 items-center justify-center rounded-full dark:bg-background/40">
          <VerusIdAtIcon class="size-6" inverted />
        </div>
        <h2 class="mt-4 text-xl font-semibold text-foreground">{i18n.t('wallet.identity.empty.title')}</h2>
        <p class="mt-2 max-w-lg text-sm text-muted-foreground">
          {i18n.t('wallet.identity.empty.description')}
        </p>
        <Button class="mt-5" onclick={() => (linkSheetOpen = true)}>
          {i18n.t('wallet.identity.empty.cta')}
        </Button>
      </div>
      {:else}
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

      {#if !hasVisibleIdentities}
        <p class="mt-4 rounded-lg bg-muted/55 px-3 py-2.5 text-sm text-muted-foreground dark:bg-muted/50">
          {i18n.t('wallet.identity.sheet.emptySearch')}
        </p>
      {:else}
        <div class="mt-4 min-h-0 flex-1">
          <ScrollArea.Root class="h-full" type="scroll">
            <ScrollArea.Viewport class="h-full pr-1">
              {#if filteredFavoriteIdentities.length > 0}
                <section>
                  <div class="flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                    <StarIcon class="size-3.5 fill-current text-amber-500" />
                    <span>{i18n.t('wallet.identity.list.favorites')}</span>
                    <span class="text-[11px] text-muted-foreground/80">{favoriteIdentities.length}/2</span>
                  </div>

                  <div class={`${compactMode ? 'mt-2 space-y-2' : 'mt-2 grid gap-3 md:grid-cols-2 xl:grid-cols-3'}`}>
                    {#each filteredFavoriteIdentities as identity (identity.identityAddress)}
                      {#if compactMode}
                        <LinkedIdentityRow
                          {identity}
                          onSelect={(selected) => openIdentityDetails(selected.identityAddress)}
                          onToggleFavorite={toggleFavorite}
                        />
                      {:else}
                        <LinkedIdentityCard
                          {identity}
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
                    <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      {i18n.t('wallet.identity.list.all')}
                    </p>
                  {/if}

                  <div class={`${filteredFavoriteIdentities.length > 0 ? 'mt-2' : ''} ${compactMode
                    ? 'space-y-2'
                    : 'grid gap-3 md:grid-cols-2 xl:grid-cols-3'}`}>
                    {#each filteredNonFavoriteIdentities as identity (identity.identityAddress)}
                      {#if compactMode}
                        <LinkedIdentityRow
                          {identity}
                          onSelect={(selected) => openIdentityDetails(selected.identityAddress)}
                          onToggleFavorite={toggleFavorite}
                        />
                      {:else}
                        <LinkedIdentityCard
                          {identity}
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
