<script lang="ts">
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import StarIcon from '@lucide/svelte/icons/star';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import { i18nStore } from '$lib/i18n';
  import type { LinkedIdentity } from '$lib/types/wallet.js';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay';
  import IdentityAvatar from './IdentityAvatar.svelte';

  const noop = (identity: LinkedIdentity): void => {
    void identity;
  };

  type LinkedIdentityRowProps = {
    identity: LinkedIdentity;
    onSelect?: typeof noop;
    onToggleFavorite?: typeof noop;
  };

  let { identity, onSelect = noop, onToggleFavorite = noop }: LinkedIdentityRowProps = $props();

  const i18n = $derived($i18nStore);
  const displayName = $derived(formatIdentityDisplayName(identity));
  const displayNameIsAddress = $derived(displayName === identity.identityAddress);
</script>

<div
  class="flex w-full items-center gap-2 rounded-lg bg-muted/20 px-3 py-2.5 transition-colors hover:bg-muted/45 dark:hover:bg-muted/35"
>
  <button
    type="button"
    class="group flex min-w-0 flex-1 items-center gap-3 text-left"
    onclick={() => onSelect(identity)}
  >
    <IdentityAvatar
      seed={identity.identityAddress}
      label={displayName}
      class="size-8 text-[10px]"
    />
    <div class="min-w-0">
      {#if displayNameIsAddress}
        <IdentifierText
          value={identity.identityAddress}
          mode="compact"
          class="block truncate text-sm font-semibold text-foreground"
        />
      {:else}
        <p class="truncate text-sm font-semibold text-foreground">{displayName}</p>
      {/if}
      {#if identity.status}
        <p
          class="mt-0.5 truncate text-[11px] font-semibold tracking-wide text-muted-foreground uppercase"
        >
          {identity.status}
        </p>
      {/if}
    </div>
    <ChevronRightIcon
      class="size-4 shrink-0 text-muted-foreground/80 transition-colors group-hover:text-foreground"
    />
  </button>

  <button
    type="button"
    class="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:text-foreground"
    onclick={() => onToggleFavorite(identity)}
    aria-label={identity.favorite
      ? i18n.t('wallet.identity.favorite.remove')
      : i18n.t('wallet.identity.favorite.add')}
    title={identity.favorite
      ? i18n.t('wallet.identity.favorite.remove')
      : i18n.t('wallet.identity.favorite.add')}
  >
    <StarIcon class={`size-4 ${identity.favorite ? 'fill-current text-amber-500' : ''}`} />
  </button>
</div>
