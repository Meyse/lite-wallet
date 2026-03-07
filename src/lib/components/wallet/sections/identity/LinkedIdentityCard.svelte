<script lang="ts">
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import StarIcon from '@lucide/svelte/icons/star';
  import { i18nStore } from '$lib/i18n';
  import type { LinkedIdentity } from '$lib/types/wallet.js';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay';
  import IdentityAvatar from './IdentityAvatar.svelte';

  const noop = (identity: LinkedIdentity): void => {
    void identity;
  };

  type LinkedIdentityCardProps = {
    identity: LinkedIdentity;
    onSelect?: typeof noop;
    onToggleFavorite?: typeof noop;
  };

   
  let { identity, onSelect = noop, onToggleFavorite = noop }: LinkedIdentityCardProps = $props();
   

  const i18n = $derived($i18nStore);
  const displayName = $derived(formatIdentityDisplayName(identity));
</script>

<div class="hover:bg-muted/45 dark:hover:bg-muted/35 flex w-full items-center gap-2 rounded-xl bg-muted/20 px-3 py-3 transition-colors">
  <button
    type="button"
    class="group flex min-w-0 flex-1 items-center gap-3 text-left"
    onclick={() => onSelect(identity)}
  >
    <IdentityAvatar seed={identity.identityAddress} label={displayName} />
    <div class="min-w-0 flex-1 space-y-1">
      <p class="truncate text-sm font-semibold text-foreground">{displayName}</p>
      {#if identity.status}
        <p class="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
          {identity.status}
        </p>
      {/if}
    </div>
    <ChevronRightIcon class="text-muted-foreground/80 group-hover:text-foreground size-4 shrink-0 transition-colors" />
  </button>

  <button
    type="button"
    class="text-muted-foreground hover:text-foreground inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md transition-colors"
    onclick={() => onToggleFavorite(identity)}
    aria-label={identity.favorite ? i18n.t('wallet.identity.favorite.remove') : i18n.t('wallet.identity.favorite.add')}
    title={identity.favorite ? i18n.t('wallet.identity.favorite.remove') : i18n.t('wallet.identity.favorite.add')}
  >
    <StarIcon class={`size-4 ${identity.favorite ? 'fill-current text-amber-500' : ''}`} />
  </button>
</div>
