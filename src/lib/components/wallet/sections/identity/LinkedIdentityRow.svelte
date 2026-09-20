<script lang="ts">
  import { onMount } from 'svelte';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
  import StarIcon from '@lucide/svelte/icons/star';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import { Button } from '$lib/components/ui/button';
  import { i18nStore } from '$lib/i18n';
  import type {
    IdentityProfileLoadResult,
    LinkedIdentity,
    PendingIdentityProfileUpdate,
  } from '$lib/types/wallet.js';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay';
  import IdentityAvatar from './IdentityAvatar.svelte';

  const noop = (identity: LinkedIdentity): void => {
    void identity;
  };

  type LinkedIdentityRowProps = {
    identity: LinkedIdentity;
    profile?: IdentityProfileLoadResult | null;
    pendingProfile?: PendingIdentityProfileUpdate | null;
    favoriteBusy?: boolean;
    favoriteDisabled?: boolean;
    showDivider?: boolean;
    onProfileVisible?: () => void;
    onSelect?: typeof noop;
    onToggleFavorite?: typeof noop;
  };

  let {
    identity,
    profile = null,
    pendingProfile = null,
    favoriteBusy = false,
    favoriteDisabled = false,
    showDivider = false,
    onProfileVisible = () => {},
    onSelect = noop,
    onToggleFavorite = noop,
  }: LinkedIdentityRowProps = $props();

  const i18n = $derived($i18nStore);
  const displayName = $derived(formatIdentityDisplayName(identity));
  const displayNameIsAddress = $derived(displayName === identity.identityAddress);
  const usePendingProfileFallback = $derived(!profile || profile.state === 'unavailable');
  const displayedAvatarBase64 = $derived(
    usePendingProfileFallback
      ? (pendingProfile?.previousProfile.avatarBase64 ?? null)
      : (profile?.avatar?.value.base64 ?? null)
  );
  const avatarUrl = $derived(
    displayedAvatarBase64
      ? `data:${profile?.avatar?.value.mimeType ?? 'image/jpeg'};base64,${displayedAvatarBase64}`
      : null
  );
  const description = $derived(
    usePendingProfileFallback
      ? (pendingProfile?.previousProfile.description?.trim() ?? '')
      : (profile?.description?.value?.trim() ?? '')
  );
  const favoriteActionLabel = $derived(
    favoriteBusy
      ? i18n.t('wallet.identity.favorite.saving')
      : identity.favorite
        ? i18n.t('wallet.identity.favorite.remove')
        : i18n.t('wallet.identity.favorite.add')
  );
  let rowElement: HTMLDivElement;
  onMount(() => {
    if (typeof IntersectionObserver === 'undefined') {
      onProfileVisible();
      return undefined;
    }
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) {
        onProfileVisible();
        observer.disconnect();
      }
    });
    observer.observe(rowElement);
    return () => observer.disconnect();
  });
</script>

<div
  bind:this={rowElement}
  data-linked-identity-row
  data-divider={showDivider ? 'between' : 'none'}
  class={`flex w-full items-center gap-3.5 py-[18px] transition-colors hover:bg-muted/25 dark:hover:bg-muted/15 ${showDivider ? 'border-b' : ''}`}
>
  <button
    type="button"
    class="group flex min-w-0 flex-1 items-center gap-3.5 rounded-sm text-left outline-none focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
    onclick={() => onSelect(identity)}
  >
    <IdentityAvatar
      seed={identity.identityAddress}
      label={displayName}
      imageUrl={avatarUrl}
      class="size-12 text-sm"
    />
    <div class="min-w-0 flex-1">
      {#if displayNameIsAddress}
        <IdentifierText
          value={identity.identityAddress}
          mode="compact"
          class="block truncate text-[17px] leading-[21px] font-semibold text-foreground"
        />
      {:else}
        <p class="truncate text-[17px] leading-[21px] font-semibold text-foreground">
          {displayName}
        </p>
      {/if}
      {#if pendingProfile}
        <p class="mt-1 truncate text-[13px] leading-[21px] text-muted-foreground">
          {i18n.t('wallet.identity.profile.pendingShort')}
        </p>
      {:else if description}
        <p class="mt-1 truncate text-[13px] leading-[21px] text-muted-foreground">
          {description}
        </p>
      {:else if profile?.state === 'unavailable'}
        <p class="mt-1 truncate text-xs leading-[21px] text-muted-foreground">
          {i18n.t('wallet.contacts.profileUnavailable')}
        </p>
      {/if}
    </div>
  </button>

  <button
    type="button"
    class={`inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md transition-colors ${favoriteBusy ? 'text-amber-500' : 'text-muted-foreground enabled:hover:text-foreground disabled:opacity-45'}`}
    onclick={() => onToggleFavorite(identity)}
    disabled={favoriteDisabled}
    aria-busy={favoriteBusy}
    aria-label={favoriteActionLabel}
    title={favoriteActionLabel}
    data-favorite-state={favoriteBusy ? 'saving' : identity.favorite ? 'favorite' : 'not-favorite'}
  >
    {#if favoriteBusy}
      <LoaderCircleIcon class="size-4 animate-spin motion-reduce:animate-none" aria-hidden="true" />
    {:else}
      <StarIcon class={`size-4 ${identity.favorite ? 'fill-current text-amber-500' : ''}`} />
    {/if}
  </button>

  <Button
    variant="ghost"
    size="icon-sm"
    class="shrink-0 text-muted-foreground hover:text-foreground"
    onclick={() => onSelect(identity)}
    aria-label={i18n.t('wallet.identity.list.manage')}
    title={i18n.t('wallet.identity.list.manage')}
    data-linked-identity-manage
  >
    <ChevronRightIcon class="size-4" aria-hidden="true" />
  </Button>
</div>
