<script lang="ts">
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
  import { Button } from '$lib/components/ui/button';
  import { i18nStore } from '$lib/i18n';

  let {
    loading,
    failed,
    hasAssets,
    onRetry,
  }: {
    loading: boolean;
    failed: boolean;
    hasAssets: boolean;
    onRetry: () => void;
  } = $props();

  const i18n = $derived($i18nStore);
</script>

{#if loading}
  <div
    class="flex items-center gap-2 text-sm text-muted-foreground"
    data-transfer-source-status="loading"
    aria-live="polite"
  >
    <LoaderCircleIcon class="size-4 shrink-0 animate-spin" aria-hidden="true" />
    <span>{i18n.t('wallet.transfer.sourceLoading')}</span>
  </div>
{:else if failed}
  <div
    class="flex items-center justify-between gap-3 rounded-lg border border-border/70 px-3 py-2"
    data-transfer-source-status={hasAssets ? 'partial' : 'failed'}
    aria-live="polite"
  >
    <p class="text-sm text-muted-foreground">
      {i18n.t(hasAssets ? 'wallet.transfer.sourceLoadPartial' : 'wallet.transfer.sourceLoadFailed')}
    </p>
    <Button variant="outline" size="sm" onclick={onRetry}>
      {i18n.t('common.retry')}
    </Button>
  </div>
{:else if !hasAssets}
  <p class="text-sm text-muted-foreground" data-transfer-source-status="empty" aria-live="polite">
    {i18n.t('wallet.transfer.noAssets')}
  </p>
{/if}
