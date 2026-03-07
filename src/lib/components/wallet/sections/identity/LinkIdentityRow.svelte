<script lang="ts">
  import CheckIcon from '@lucide/svelte/icons/check';
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import { i18nStore } from '$lib/i18n';
  import type { LinkableIdentity } from '$lib/types/wallet.js';
  import { formatIdentityDisplayName, truncateIdentityAddress } from '$lib/utils/identityDisplay';

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
  const abbreviatedAddress = $derived(truncateIdentityAddress(identity.identityAddress, 6, 6));
  const displayNameIsAddress = $derived(identity.identityAddress === displayName);
  const primaryLine = $derived(displayNameIsAddress ? abbreviatedAddress : displayName);
  const secondaryLine = $derived(
    displayNameIsAddress ? (identity.status ?? '') : abbreviatedAddress
  );
  const secondaryLineIsAddress = $derived(!displayNameIsAddress);
</script>

<li class="flex items-center gap-3 rounded-lg bg-muted/65 px-3.5 py-3 dark:bg-muted/55">
  <div class="min-w-0 flex-1">
    <p
      class="truncate text-sm font-semibold text-foreground"
      class:font-mono={displayNameIsAddress}
      title={displayNameIsAddress ? identity.identityAddress : undefined}
    >
      {primaryLine}
    </p>
    {#if secondaryLine}
      <p
        class="truncate text-xs text-muted-foreground"
        class:font-mono={secondaryLineIsAddress}
        title={secondaryLineIsAddress ? identity.identityAddress : undefined}
      >
        {secondaryLine}
      </p>
    {/if}
  </div>

  <div class="flex shrink-0 items-center gap-2">
    {#if identity.status}
      <span
        class="bg-background/60 text-muted-foreground inline-flex rounded-full px-2.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide dark:bg-background/45"
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
        class="text-primary inline-flex h-8 w-8 items-center justify-center rounded-md bg-primary/12 transition-colors hover:bg-primary/20 focus-visible:ring-ring focus-visible:ring-[2px] focus-visible:outline-none disabled:opacity-45 dark:bg-primary/20 dark:hover:bg-primary/30"
        onclick={() => onLink(identity)}
        disabled={busy}
        aria-label={busy ? i18n.t('wallet.identity.sheet.linking') : i18n.t('wallet.identity.sheet.link')}
        title={busy ? i18n.t('wallet.identity.sheet.linking') : i18n.t('wallet.identity.sheet.link')}
      >
        {#if busy}
          <LoaderCircleIcon class="h-4 w-4 animate-spin" absoluteStrokeWidth />
        {:else}
          <PlusIcon class="h-4 w-4" absoluteStrokeWidth />
        {/if}
      </button>
    {/if}
  </div>
</li>
