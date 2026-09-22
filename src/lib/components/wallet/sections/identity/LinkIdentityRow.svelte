<script lang="ts">
  import CheckIcon from '@lucide/svelte/icons/check';
  import { Spinner } from '$lib/components/ui/spinner';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import { i18nStore } from '$lib/i18n';
  import type { LinkableIdentity } from '$lib/types/wallet.js';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay';

  const noop = (identity: LinkableIdentity): void => {
    void identity;
  };

  type LinkIdentityRowProps = {
    identity: LinkableIdentity;
    busy?: boolean;
    onLink?: typeof noop;
  };

  let { identity, busy = false, onLink = noop }: LinkIdentityRowProps = $props();

  const i18n = $derived($i18nStore);
  const displayName = $derived(formatIdentityDisplayName(identity));
  const displayNameIsAddress = $derived(identity.identityAddress === displayName);
  const secondaryLine = $derived(displayNameIsAddress ? (identity.status ?? '') : '');
</script>

<li class="flex items-center gap-3 rounded-lg bg-muted/65 px-3.5 py-3 dark:bg-muted/55">
  <div class="min-w-0 flex-1">
    {#if displayNameIsAddress}
      <IdentifierText
        value={identity.identityAddress}
        mode="compact"
        class="block truncate text-sm font-semibold text-foreground"
      />
    {:else}
      <p class="truncate text-sm font-semibold text-foreground">{displayName}</p>
    {/if}
    {#if secondaryLine}
      <p class="truncate text-xs text-muted-foreground">{secondaryLine}</p>
    {:else if !displayNameIsAddress}
      <IdentifierText
        value={identity.identityAddress}
        mode="compact"
        class="block truncate text-xs text-muted-foreground"
      />
    {/if}
  </div>

  <div class="flex shrink-0 items-center gap-2">
    {#if identity.status}
      <span
        class="inline-flex rounded-full bg-background/60 px-2.5 py-0.5 text-[10px] font-semibold tracking-wide text-muted-foreground uppercase dark:bg-background/45"
      >
        {identity.status}
      </span>
    {/if}

    {#if identity.linked}
      <span
        class="inline-flex h-8 w-8 items-center justify-center rounded-md bg-emerald-500/15 text-emerald-700 dark:bg-emerald-500/20 dark:text-emerald-300"
        aria-label={i18n.t('wallet.identity.sheet.alreadyLinked')}
        title={i18n.t('wallet.identity.sheet.alreadyLinked')}
      >
        <CheckIcon class="h-4 w-4" absoluteStrokeWidth />
      </span>
    {:else}
      <button
        type="button"
        class="inline-flex h-8 w-8 items-center justify-center rounded-md bg-primary/12 text-primary transition-colors hover:bg-primary/20 focus-visible:ring-[2px] focus-visible:ring-ring focus-visible:outline-none disabled:opacity-45 dark:bg-primary/20 dark:hover:bg-primary/30"
        onclick={() => onLink(identity)}
        disabled={busy}
        aria-label={busy
          ? i18n.t('wallet.identity.sheet.linking')
          : i18n.t('wallet.identity.sheet.link')}
        title={busy
          ? i18n.t('wallet.identity.sheet.linking')
          : i18n.t('wallet.identity.sheet.link')}
      >
        {#if busy}
          <Spinner class="h-4 w-4" absoluteStrokeWidth />
        {:else}
          <PlusIcon class="h-4 w-4" absoluteStrokeWidth />
        {/if}
      </button>
    {/if}
  </div>
</li>
