<script lang="ts">
  import { onDestroy, onMount, tick, untrack } from 'svelte';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import Globe2Icon from '@lucide/svelte/icons/globe-2';
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
  import SearchIcon from '@lucide/svelte/icons/search';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { identityKey } from '$lib/contacts/identity';
  import { identityProfiles, loadIdentityProfile, profileImage } from '$lib/contacts/profiles';
  import { resolveContactIdentity } from '$lib/contacts/service';
  import { i18nStore } from '$lib/i18n';
  import type { ResolvedContactIdentity } from '$lib/types/addressBook';
  import type { WalletNetwork } from '$lib/types/wallet';
  import { extractWalletErrorType } from '$lib/utils/walletErrors';
  import IdentityAvatar from './IdentityAvatar.svelte';
  import type { VerusIdLookupState, VerusIdLookupStatus } from './verusIdPublicProfile';

  const noopState = (_state: VerusIdLookupState): void => {};
  const noopIdentity = (_identity: ResolvedContactIdentity): void => {};
  const noop = (): void => {};

  let {
    walletNetwork,
    initialState,
    restoreResultFocus = false,
    onStateChange = noopState,
    onOpenProfile = noopIdentity,
    onFocusRestored = noop,
  }: {
    walletNetwork: WalletNetwork;
    initialState: VerusIdLookupState;
    restoreResultFocus?: boolean;
    onStateChange?: (state: VerusIdLookupState) => void;
    onOpenProfile?: (identity: ResolvedContactIdentity) => void;
    onFocusRestored?: () => void;
  } = $props();

  const i18n = $derived($i18nStore);
  const restoredState = untrack(() => initialState);
  const restoredLookupWasInterrupted = restoredState.status === 'looking-up';
  let query = $state(restoredState.query);
  let submittedQuery = $state(restoredLookupWasInterrupted ? '' : restoredState.submittedQuery);
  let status = $state<VerusIdLookupStatus>(
    restoredLookupWasInterrupted ? 'idle' : restoredState.status
  );
  let result = $state<ResolvedContactIdentity | null>(
    restoredLookupWasInterrupted ? null : restoredState.result
  );
  let scrollTop = $state(restoredState.scrollTop);
  let generation = 0;
  let alive = true;
  let inputElement = $state<HTMLInputElement | null>(null);
  let resultButton = $state<HTMLButtonElement | null>(null);
  let viewport = $state<HTMLDivElement | null>(null);

  const resultEntry = $derived(result ? $identityProfiles[identityKey(result)] : null);
  const resultDescription = $derived(resultEntry?.profile?.description?.value?.trim() ?? '');
  const resultAvatar = $derived(profileImage(resultEntry?.profile));
  const busy = $derived(status === 'looking-up');
  const networkLabel = $derived(
    i18n.t(
      walletNetwork === 'testnet'
        ? 'wallet.identity.lookup.network.testnet'
        : 'wallet.identity.lookup.network.mainnet'
    )
  );

  function publish(): void {
    onStateChange({ query, submittedQuery, status, result, scrollTop });
  }

  function invalidateResult(): void {
    if (query.trim() === submittedQuery) {
      publish();
      return;
    }

    generation += 1;
    submittedQuery = '';
    status = 'idle';
    result = null;
    publish();
  }

  async function lookup(): Promise<void> {
    const normalized = query.trim();
    if (!normalized || busy) return;

    const request = ++generation;
    submittedQuery = normalized;
    status = 'looking-up';
    result = null;
    publish();

    try {
      const resolved = await resolveContactIdentity(normalized);
      if (!alive || request !== generation) return;
      result = resolved;
      status = 'resolved';
      void loadIdentityProfile(resolved, false, true).catch(() => {});
    } catch (error) {
      if (!alive || request !== generation) return;
      status = extractWalletErrorType(error) === 'IdentityNotFound' ? 'not-found' : 'unavailable';
      result = null;
    } finally {
      if (alive && request === generation) publish();
    }
  }

  function openResult(): void {
    if (result) onOpenProfile(result);
  }

  onMount(() => {
    if (restoredLookupWasInterrupted) publish();
    if (viewport) viewport.scrollTop = scrollTop;
    if (restoreResultFocus) {
      void tick().then(() => {
        (resultButton ?? inputElement)?.focus();
        onFocusRestored();
      });
    }
  });

  onDestroy(() => {
    alive = false;
    generation += 1;
    if (status === 'looking-up') {
      submittedQuery = '';
      status = 'idle';
      result = null;
      publish();
    }
  });
</script>

<ScrollArea.Root class="min-h-0 flex-1" type="scroll">
  <ScrollArea.Viewport
    bind:ref={viewport}
    class="h-full pr-1"
    onscroll={(event) => {
      scrollTop = event.currentTarget.scrollTop;
      publish();
    }}
  >
    <form
      class="space-y-2"
      onsubmit={(event) => {
        event.preventDefault();
        void lookup();
      }}
    >
      <div class="flex items-center justify-between gap-4">
        <Label for="verusid-public-lookup" class="text-[13px] font-medium">
          {i18n.t('wallet.identity.lookup.label')}
        </Label>
        <span class="inline-flex shrink-0 items-center gap-2 text-xs text-muted-foreground">
          <Globe2Icon class="size-3.5" aria-hidden="true" />
          {networkLabel}
        </span>
      </div>

      <div class="flex min-w-0 gap-2.5">
        <div class="relative min-w-0 flex-1">
          <SearchIcon
            class="pointer-events-none absolute top-1/2 left-3.5 z-10 size-4 -translate-y-1/2 text-muted-foreground"
            aria-hidden="true"
          />
          <Input
            id="verusid-public-lookup"
            bind:ref={inputElement}
            bind:value={query}
            class="h-10 pl-10 text-[13px]"
            placeholder={i18n.t('wallet.identity.lookup.placeholder')}
            autocomplete="off"
            spellcheck="false"
            oninput={invalidateResult}
          />
        </div>
        <Button
          type="submit"
          class="h-10 w-[122px] shrink-0 text-[13px]"
          disabled={!query.trim() || busy}
        >
          {#if busy}
            <LoaderCircleIcon
              class="size-3.5 animate-spin motion-reduce:animate-none"
              aria-hidden="true"
            />
            {i18n.t('wallet.identity.lookup.lookingUp')}
          {:else}
            {i18n.t('wallet.identity.lookup.submit')}
          {/if}
        </Button>
      </div>
      <p class="text-xs text-muted-foreground">{i18n.t('wallet.identity.lookup.example')}</p>
    </form>

    <div aria-live="polite" class="mt-6">
      {#if status === 'resolved' && result}
        <div
          class="flex items-center gap-3.5 rounded-[10px] bg-muted/70 px-4 py-[18px] dark:bg-muted"
        >
          <button
            bind:this={resultButton}
            type="button"
            class="flex min-w-0 flex-1 items-center gap-3.5 rounded-sm text-left outline-none focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
            onclick={openResult}
          >
            <IdentityAvatar
              seed={result.identityAddress}
              label={result.fullyQualifiedName}
              imageUrl={resultAvatar}
              class="size-12 text-sm"
            />
            <span class="min-w-0 flex-1">
              <span class="block truncate text-[17px] leading-[21px] font-semibold">
                {result.fullyQualifiedName}
              </span>
              {#if resultDescription}
                <span class="mt-1 block truncate text-[13px] text-muted-foreground">
                  {resultDescription}
                </span>
              {:else if resultEntry?.unavailable}
                <span class="mt-1 block truncate text-xs text-muted-foreground">
                  {i18n.t('wallet.identity.lookup.profileUnavailable')}
                </span>
              {/if}
            </span>
          </button>
          <Button
            variant="ghost"
            class="h-[34px] shrink-0 gap-1.5 px-3 text-[13px] text-text-action hover:text-text-action"
            onclick={openResult}
          >
            {i18n.t('wallet.identity.lookup.viewProfile')}
            <ChevronRightIcon class="size-4" aria-hidden="true" />
          </Button>
        </div>
      {:else if status === 'not-found'}
        <p class="rounded-lg bg-muted/55 px-3 py-2.5 text-sm text-muted-foreground" role="status">
          {i18n.t('wallet.identity.lookup.notFound')}
        </p>
      {:else if status === 'unavailable'}
        <div
          class="flex items-center justify-between gap-4 rounded-lg bg-destructive/10 px-3 py-2.5"
        >
          <p class="text-sm text-destructive" role="alert">
            {i18n.t('wallet.identity.lookup.unavailable')}
          </p>
          <Button variant="ghost" size="sm" class="shrink-0" onclick={() => void lookup()}>
            {i18n.t('common.retry')}
          </Button>
        </div>
      {/if}
    </div>
  </ScrollArea.Viewport>
  <ScrollArea.Scrollbar orientation="vertical" />
</ScrollArea.Root>
