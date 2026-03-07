<script lang="ts">
  import MinusIcon from '@lucide/svelte/icons/minus';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import { i18nStore } from '$lib/i18n';
  import type { AddAssetEntry } from '$lib/stores/addAssetCatalog.js';

  const noop = (asset: AddAssetEntry): void => {
    void asset;
  };

  type AddAssetRowProps = {
    entry: AddAssetEntry;
    busy?: boolean;
    onAction?: typeof noop;
  };

   
  let { entry, busy = false, onAction = noop }: AddAssetRowProps = $props();
   

  const i18n = $derived($i18nStore);

  const protocolLabel = $derived(entry.proto.toUpperCase());
</script>

<li class="flex items-center gap-3 rounded-lg bg-muted/65 px-3.5 py-3 dark:bg-muted/55">
  <CoinIcon coinId={entry.id} coinName={entry.displayName} size={28} showBadge decorative />

  <div class="min-w-0 flex-1">
    <p class="truncate text-sm font-semibold text-foreground">{entry.displayName}</p>
    <p class="truncate text-xs text-muted-foreground">{entry.displayTicker}</p>
  </div>

  <div class="flex shrink-0 items-center gap-2">
    <span
      class="bg-background/60 text-muted-foreground inline-flex rounded-full px-2.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide dark:bg-background/45"
    >
      {protocolLabel}
    </span>
    {#if entry.status === 'added'}
      <button
        type="button"
        class="text-destructive inline-flex h-8 w-8 items-center justify-center rounded-md bg-destructive/10 transition-colors hover:bg-destructive/15 focus-visible:ring-ring focus-visible:ring-[2px] focus-visible:outline-none disabled:opacity-45 dark:bg-destructive/20 dark:hover:bg-destructive/30"
        onclick={() => onAction(entry)}
        disabled={busy}
        aria-label={busy ? i18n.t('wallet.addAsset.disabling') : i18n.t('wallet.addAsset.disable')}
        title={busy ? i18n.t('wallet.addAsset.disabling') : i18n.t('wallet.addAsset.disable')}
      >
        <MinusIcon class="h-4 w-4" absoluteStrokeWidth />
      </button>
    {:else}
      <button
        type="button"
        class="text-primary inline-flex h-8 w-8 items-center justify-center rounded-md bg-primary/12 transition-colors hover:bg-primary/20 focus-visible:ring-ring focus-visible:ring-[2px] focus-visible:outline-none disabled:opacity-45 dark:bg-primary/20 dark:hover:bg-primary/30"
        onclick={() => onAction(entry)}
        disabled={busy}
        aria-label={busy ? i18n.t('wallet.addAsset.adding') : i18n.t('wallet.addAsset.add')}
      >
        <PlusIcon class="h-4 w-4" absoluteStrokeWidth />
      </button>
    {/if}
  </div>

</li>
