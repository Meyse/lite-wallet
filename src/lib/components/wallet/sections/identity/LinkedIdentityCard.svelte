<script lang="ts">
  import { onMount } from 'svelte';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import { Spinner } from '$lib/components/ui/spinner';
  import StarIcon from '@lucide/svelte/icons/star';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
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

  type LinkedIdentityCardProps = {
    identity: LinkedIdentity;
    profile?: IdentityProfileLoadResult | null;
    pendingProfile?: PendingIdentityProfileUpdate | null;
    favoriteBusy?: boolean;
    favoriteDisabled?: boolean;
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
    onProfileVisible = () => {},
    onSelect = noop,
    onToggleFavorite = noop,
  }: LinkedIdentityCardProps = $props();

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
  class="flex w-full items-center gap-2 rounded-xl bg-muted/20 px-3 py-3 transition-colors hover:bg-muted/45 dark:hover:bg-muted/35"
>
  <button
    type="button"
    class="group flex min-w-0 flex-1 items-center gap-3 text-left"
    onclick={() => onSelect(identity)}
  >
    <IdentityAvatar
      seed={identity.identityAddress}
      label={displayName}
      imageUrl={avatarUrl}
      class="size-12"
    />
    <div class="min-w-0 flex-1 space-y-1">
      {#if displayNameIsAddress}
        <IdentifierText
          value={identity.identityAddress}
          mode="compact"
          class="block truncate text-sm font-semibold text-foreground"
        />
      {:else}
        <p class="truncate text-sm font-semibold text-foreground">{displayName}</p>
      {/if}
      <p class="line-clamp-2 min-h-8 text-xs leading-4 text-muted-foreground">
        {description || i18n.t('wallet.identity.profile.noProfile')}
      </p>
      {#if pendingProfile}
        <p class="text-[11px] font-medium text-amber-700 dark:text-amber-300">
          {i18n.t('wallet.identity.profile.pendingShort')}
        </p>
      {/if}
    </div>
    <ChevronRightIcon
      class="size-4 shrink-0 text-muted-foreground/80 transition-colors group-hover:text-foreground"
    />
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
      <Spinner class="size-4" />
    {:else}
      <StarIcon class={`size-4 ${identity.favorite ? 'fill-current text-amber-500' : ''}`} />
    {/if}
  </button>
</div>
