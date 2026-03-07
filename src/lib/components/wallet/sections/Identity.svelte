<!--
  Component: Identity
  Purpose: Linked VerusID management with discovery, linking, and in-section detail view.
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import StarIcon from '@lucide/svelte/icons/star';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import { toast } from 'svelte-sonner';
  import VerusIdAtIcon from '$lib/components/icons/VerusIdAtIcon.svelte';
  import SearchInput from '$lib/components/common/SearchInput.svelte';
  import { Button } from '$lib/components/ui/button';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { i18nStore } from '$lib/i18n';
  import * as identityLinkService from '$lib/services/identityLinkService.js';
  import type { IdentityDetails, LinkedIdentity } from '$lib/types/wallet.js';
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

  $effect(() => {
    const query = listSearchInput;
    const timer = setTimeout(() => {
      listDebouncedSearch = query;
    }, 150);

    return () => clearTimeout(timer);
  });

  onMount(async () => {
    await loadLinkedIdentities();
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
    {:else if linkedIdentities.length === 0}
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
  </div>
{/if}

<LinkIdentitySheet
  bind:isOpen={linkSheetOpen}
  onLinkedChange={applyLinkedIdentities}
  allowManualLinkEntry={walletNetwork === 'testnet'}
/>
