<script lang="ts">
  import CheckIcon from '@lucide/svelte/icons/check';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import UserRoundIcon from '@lucide/svelte/icons/user-round';
  import UserRoundXIcon from '@lucide/svelte/icons/user-round-x';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import ContactAvatar from '$lib/components/wallet/contacts/ContactAvatar.svelte';
  import { Button } from '$lib/components/ui/button';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { Spinner } from '$lib/components/ui/spinner';
  import { i18nStore } from '$lib/i18n';
  import type { ContactIdentity } from '$lib/types/addressBook';
  import type { LinkedIdentity } from '$lib/types/wallet.js';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay.js';

  /** "Respond as": the VerusID in this wallet that answers the request. */
  let {
    identities,
    selectedAddress,
    loading = false,
    disabled = false,
    toContactIdentity,
    onSelect,
    onLink,
    newIdentityLabel = '',
    onNewIdentity,
  }: {
    identities: LinkedIdentity[];
    selectedAddress: string;
    loading?: boolean;
    disabled?: boolean;
    toContactIdentity: (identity: LinkedIdentity) => ContactIdentity | null;
    onSelect: (identityAddress: string) => void;
    onLink: () => void;
    /** Set when the request offers a new VerusID (spec "Sign in"). */
    newIdentityLabel?: string;
    onNewIdentity?: () => void;
  } = $props();

  const i18n = $derived($i18nStore);
  let sheetOpen = $state(false);
  const selected = $derived(
    identities.find(
      (identity) => identity.identityAddress.toLowerCase() === selectedAddress.toLowerCase()
    ) ?? null
  );
  const noneEligible = $derived(!loading && identities.length === 0);

  function choose(identityAddress: string) {
    onSelect(identityAddress);
    sheetOpen = false;
  }

  function link() {
    sheetOpen = false;
    onLink();
  }
</script>

<div class="flex flex-col gap-1.5">
  <span class="text-xs font-medium text-muted-foreground"
    >{i18n.t('genericRequest.flow.respondAs')}</span
  >
  {#if loading}
    <div
      class="flex min-h-[52px] items-center gap-2 rounded-lg bg-muted/60 px-3 text-sm text-muted-foreground"
    >
      <Spinner class="size-4" />
      {i18n.t('genericRequest.auth.loading')}
    </div>
  {:else if noneEligible}
    <div
      class="flex min-h-[52px] items-center gap-2.5 rounded-lg border border-dashed border-border px-3 py-2"
    >
      <UserRoundXIcon class="size-5 shrink-0 text-muted-foreground" />
      <span class="min-w-0">
        <span class="block text-sm font-medium text-foreground"
          >{i18n.t('genericRequest.flow.noMatchingIdentity')}</span
        >
        <span class="block text-xs text-muted-foreground"
          >{i18n.t('genericRequest.flow.noMatchingIdentityHint')}</span
        >
      </span>
    </div>
    <div class="flex flex-wrap gap-2">
      <Button variant="secondary" size="sm" onclick={onLink} {disabled}>
        {i18n.t('genericRequest.auth.linkIdentity')}
      </Button>
      {#if newIdentityLabel && onNewIdentity}
        <Button variant="secondary" size="sm" onclick={onNewIdentity} {disabled}
          >{newIdentityLabel}</Button
        >
      {/if}
    </div>
  {:else}
    <button
      type="button"
      class={`flex min-h-[52px] w-full items-center gap-2.5 rounded-lg px-3 py-2 text-left transition-colors focus-visible:ring-2 focus-visible:ring-ring/60 focus-visible:outline-hidden disabled:opacity-60 ${
        selected
          ? 'bg-muted/70 hover:bg-muted'
          : 'border border-dashed border-border hover:bg-muted/40'
      }`}
      onclick={() => (sheetOpen = true)}
      {disabled}
    >
      {#if selected}
        {@const name = formatIdentityDisplayName(selected)}
        <ContactAvatar identity={toContactIdentity(selected)} {name} class="size-7" />
        <span class="flex min-w-0 flex-1 flex-col leading-tight">
          <span class="truncate text-sm font-medium text-foreground">{name}</span>
          {#if name !== selected.identityAddress}
            <IdentifierText
              value={selected.identityAddress}
              mode="compact"
              class="text-[11.5px] text-muted-foreground"
            />
          {/if}
        </span>
      {:else}
        <UserRoundIcon class="size-5 shrink-0 text-muted-foreground" />
        <span class="min-w-0 flex-1 text-sm font-medium text-foreground"
          >{i18n.t('genericRequest.flow.chooseIdentity')}</span
        >
      {/if}
      <ChevronDownIcon class="size-4 shrink-0 text-muted-foreground" />
    </button>
  {/if}
</div>

<StandardRightSheet
  bind:isOpen={sheetOpen}
  title={i18n.t('genericRequest.flow.respondAs')}
  onOpenAutoFocus={(event: Event) => event.preventDefault()}
>
  <div class="flex h-full min-h-0 flex-col">
    <p class="pr-8 text-sm text-muted-foreground">
      {i18n.t('genericRequest.auth.selectorDescription')}
    </p>
    <ScrollArea.Root class="mt-4 min-h-0 flex-1">
      <ScrollArea.Viewport class="h-full pr-1">
        <ul class="space-y-1.5 pb-4">
          {#each identities as identity (identity.identityAddress)}
            {@const name = formatIdentityDisplayName(identity)}
            {@const isSelected =
              identity.identityAddress.toLowerCase() === selectedAddress.toLowerCase()}
            <li>
              <button
                type="button"
                class={`flex w-full items-center gap-2.5 rounded-lg px-3 py-2.5 text-left transition-colors focus-visible:ring-2 focus-visible:ring-ring/60 focus-visible:outline-hidden ${
                  isSelected ? 'bg-primary/12 dark:bg-primary/24' : 'hover:bg-muted/70'
                }`}
                onclick={() => choose(identity.identityAddress)}
              >
                <ContactAvatar identity={toContactIdentity(identity)} {name} class="size-7" />
                <span class="flex min-w-0 flex-1 flex-col leading-tight">
                  <span class="truncate text-sm font-medium text-foreground">{name}</span>
                  {#if name !== identity.identityAddress}
                    <IdentifierText
                      value={identity.identityAddress}
                      mode="compact"
                      class="text-xs text-muted-foreground"
                    />
                  {/if}
                </span>
                {#if isSelected}<CheckIcon class="size-4 shrink-0 text-primary" />{/if}
              </button>
            </li>
          {/each}
        </ul>
      </ScrollArea.Viewport>
      <ScrollArea.Scrollbar orientation="vertical" />
    </ScrollArea.Root>
    <div class="flex flex-col gap-2 pt-3">
      <Button variant="secondary" onclick={link}
        >{i18n.t('genericRequest.auth.linkIdentity')}</Button
      >
    </div>
  </div>
</StandardRightSheet>
