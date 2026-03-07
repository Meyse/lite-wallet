<script lang="ts">
  import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';
  import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
  import SearchInput from '$lib/components/common/SearchInput.svelte';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { i18nStore } from '$lib/i18n';
  import * as identityLinkService from '$lib/services/identityLinkService.js';
  import type { LinkableIdentity, LinkedIdentity } from '$lib/types/wallet.js';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay';
  import { extractWalletErrorMessage, extractWalletErrorType } from '$lib/utils/walletErrors.js';
  import LinkIdentityRow from './LinkIdentityRow.svelte';

  const noop = (linked: LinkedIdentity[]): void => {
    void linked;
  };

  type LinkIdentitySheetProps = {
    isOpen?: boolean;
    onLinkedChange?: typeof noop;
    allowManualLinkEntry?: boolean;
  };

   
  let {
    isOpen = $bindable(false),
    onLinkedChange = noop,
    allowManualLinkEntry = false
  }: LinkIdentitySheetProps = $props();
   

  const i18n = $derived($i18nStore);

  let searchInput = $state('');
  let debouncedSearch = $state('');
  let loading = $state(false);
  let sheetError = $state('');
  let candidates = $state<LinkableIdentity[]>([]);
  let busyIdentityAddress = $state<string | null>(null);
  let manualIdentityInput = $state('');
  let manualLinkBusy = $state(false);

  const filteredCandidates = $derived(
    (() => {
      const query = debouncedSearch.trim().toLowerCase();
      if (!query) return candidates;

      return candidates.filter((candidate) => {
        const fields = [
          candidate.name,
          candidate.fullyQualifiedName,
          candidate.identityAddress,
          formatIdentityDisplayName(candidate)
        ]
          .map((value) => value?.toLowerCase() ?? '');

        return fields.some((value) => value.includes(query));
      });
    })()
  );

  $effect(() => {
    const query = searchInput;
    const timer = setTimeout(() => {
      debouncedSearch = query;
    }, 150);

    return () => clearTimeout(timer);
  });

  $effect(() => {
    if (!isOpen) {
      searchInput = '';
      debouncedSearch = '';
      sheetError = '';
      busyIdentityAddress = null;
      manualIdentityInput = '';
      manualLinkBusy = false;
      return;
    }

    void hydrateCandidates();
  });

  function handleOpenAutoFocus(event: Event) {
    event.preventDefault();
  }

  function mapSheetError(error: unknown, fallbackKey: string): string {
    const errorType = extractWalletErrorType(error);

    switch (errorType) {
      case 'WalletLocked':
        return i18n.t('wallet.identity.error.walletLocked');
      case 'IdentityOwnershipMismatch':
        return i18n.t('wallet.identity.error.ownershipMismatch');
      case 'IdentityNotFound':
        return i18n.t('wallet.identity.error.notFound');
      case 'NetworkError':
        return i18n.t('wallet.identity.error.network');
      default:
        break;
    }

    const message = extractWalletErrorMessage(error);
    if (message) return message;

    return i18n.t(fallbackKey);
  }

  async function hydrateCandidates() {
    loading = true;
    sheetError = '';

    try {
      candidates = await identityLinkService.discoverLinkableIdentities();
    } catch (error) {
      sheetError = mapSheetError(error, 'wallet.identity.sheet.errorLoad');
    } finally {
      loading = false;
    }
  }

  async function handleLink(candidate: LinkableIdentity) {
    if (candidate.linked || busyIdentityAddress || manualLinkBusy) return;

    busyIdentityAddress = candidate.identityAddress;
    sheetError = '';

    try {
      const updatedLinked = await identityLinkService.linkIdentity({
        identityAddress: candidate.identityAddress
      });

      onLinkedChange(updatedLinked);
      candidates = candidates.map((entry) =>
        entry.identityAddress.toLowerCase() === candidate.identityAddress.toLowerCase()
          ? { ...entry, linked: true }
          : entry
      );
    } catch (error) {
      sheetError = mapSheetError(error, 'wallet.identity.sheet.errorLink');
    } finally {
      busyIdentityAddress = null;
    }
  }

  async function handleManualLink() {
    const identityAddress = manualIdentityInput.trim();
    if (!identityAddress || manualLinkBusy || busyIdentityAddress) return;

    manualLinkBusy = true;
    sheetError = '';

    try {
      const updatedLinked = await identityLinkService.linkIdentity({
        identityAddress
      });

      onLinkedChange(updatedLinked);
      manualIdentityInput = '';
      candidates = candidates.map((entry) =>
        entry.identityAddress.toLowerCase() === identityAddress.toLowerCase()
          ? { ...entry, linked: true }
          : entry
      );
    } catch (error) {
      sheetError = mapSheetError(error, 'wallet.identity.sheet.errorLink');
    } finally {
      manualLinkBusy = false;
    }
  }
</script>

<StandardRightSheet
  bind:isOpen
  title={i18n.t('wallet.identity.sheet.title')}
  hideTitle
  bodyClass="mt-0"
  onOpenAutoFocus={handleOpenAutoFocus}
>
  <div class="flex h-full min-h-0 flex-col">
    <div class="pr-8 pt-4">
      <h2 class="text-base font-semibold text-foreground">{i18n.t('wallet.identity.sheet.title')}</h2>
    </div>

    <div class="mt-4 min-h-0 flex-1">
      <div class="pb-3 pr-1">
        <SearchInput
          bind:value={searchInput}
          placeholder={i18n.t('wallet.identity.sheet.searchPlaceholder')}
          inputClass="focus-visible:ring-0 focus-visible:ring-transparent"
        />

        {#if allowManualLinkEntry}
          <div class="mt-3 rounded-lg bg-muted/35 p-2.5 dark:bg-muted/28">
            <p class="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
              {i18n.t('wallet.identity.sheet.manualTitle')}
            </p>
            <p class="mt-1 text-xs text-muted-foreground">
              {i18n.t('wallet.identity.sheet.manualDescription')}
            </p>
            <div class="mt-2 flex items-center gap-2">
              <Input
                bind:value={manualIdentityInput}
                placeholder={i18n.t('wallet.identity.sheet.manualPlaceholder')}
                class="h-9"
                disabled={manualLinkBusy || busyIdentityAddress !== null}
                onkeydown={(event) => {
                  if (event.key === 'Enter') {
                    event.preventDefault();
                    void handleManualLink();
                  }
                }}
              />
              <Button
                size="sm"
                class="h-9 shrink-0 px-3"
                disabled={!manualIdentityInput.trim() || manualLinkBusy || busyIdentityAddress !== null}
                onclick={handleManualLink}
              >
                {manualLinkBusy ? i18n.t('wallet.identity.sheet.linking') : i18n.t('wallet.identity.sheet.manualLink')}
              </Button>
            </div>
          </div>
        {/if}

        {#if sheetError}
          <div class="mt-2 space-y-2 rounded-md bg-destructive/12 px-2.5 py-2 text-xs text-destructive">
            <div class="flex items-start gap-2">
              <AlertCircleIcon class="mt-0.5 h-4 w-4 shrink-0" />
              <p>{sheetError}</p>
            </div>
            <Button
              variant="ghost"
              size="sm"
              class="h-7 px-2 text-xs"
              onclick={hydrateCandidates}
              disabled={loading}
            >
              <RefreshCwIcon class={`size-3.5 ${loading ? 'animate-spin' : ''}`} />
              {i18n.t('common.retry')}
            </Button>
          </div>
        {/if}
      </div>

      <ScrollArea.Root class="min-h-0 flex-1">
        <ScrollArea.Viewport class="h-full pr-1">
          {#if loading}
            <p class="mt-2 rounded-lg bg-muted/55 px-3 py-2.5 text-xs text-muted-foreground dark:bg-muted/50">
              {i18n.t('wallet.identity.sheet.loading')}
            </p>
          {:else if filteredCandidates.length === 0}
            <p class="mt-2 rounded-lg bg-muted/55 px-3 py-2.5 text-xs text-muted-foreground dark:bg-muted/50">
              {debouncedSearch
                ? i18n.t('wallet.identity.sheet.emptySearch')
                : i18n.t('wallet.identity.sheet.empty')}
            </p>
          {:else}
            <ul class="mt-2 space-y-2 pb-1">
              {#each filteredCandidates as candidate (candidate.identityAddress)}
                <LinkIdentityRow
                  identity={candidate}
                  busy={busyIdentityAddress === candidate.identityAddress}
                  onLink={handleLink}
                />
              {/each}
            </ul>
          {/if}
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>
    </div>
  </div>
</StandardRightSheet>
