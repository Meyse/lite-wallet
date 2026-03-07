<script lang="ts">
  import { Spinner } from '$lib/components/ui/spinner';
  import { i18nStore } from '$lib/i18n';
  import type { ImportMethod } from './types';

  type ImportCompleteStepProps = {
    method?: ImportMethod;
    isOpening?: boolean;
    openError?: string;
  };

   
  let { method = 'seed24', isOpening = false, openError = '' }: ImportCompleteStepProps = $props();
   
  const i18n = $derived($i18nStore);
</script>

<div class="mx-auto flex w-full max-w-[560px] flex-col items-center justify-center space-y-6 py-4 text-center">
  <div class="bg-muted/30 flex h-20 w-20 items-center justify-center rounded-full">
    {#if openError}
      <span class="text-4xl">!</span>
    {:else if isOpening}
      <Spinner class="text-primary size-10" />
    {:else}
      <span class="bg-primary/80 inline-block h-3 w-3 animate-pulse rounded-full"></span>
    {/if}
  </div>

  <div class="bg-muted/20 border-border/70 w-full rounded-xl border p-4 text-left">
    <div class="space-y-2 text-sm">
      <div class="text-foreground flex items-center gap-2">
        <span>✓</span>
        <span>
          {method === 'seed24'
            ? i18n.t('walletImport.complete.statusSeedValidated')
            : i18n.t('walletImport.complete.statusInputValidated')}
        </span>
      </div>
      <div class="text-foreground flex items-center gap-2">
        <span>✓</span>
        <span>{i18n.t('walletImport.complete.statusImported')}</span>
      </div>
      <div class={"flex items-center gap-2 " + (openError ? 'text-destructive' : 'text-muted-foreground')}>
        {#if openError}
          <span>!</span>
          <span>{i18n.t('walletImport.complete.statusOpenFailed')}</span>
        {:else}
          <span class="bg-primary/80 inline-block h-3 w-3 animate-pulse rounded-full"></span>
          <span>{i18n.t('walletImport.complete.statusOpening')}</span>
        {/if}
      </div>
    </div>
  </div>

  <p class="text-muted-foreground text-xs">
    {#if openError}
      {i18n.t('walletImport.complete.loadingHint')}
    {:else if isOpening}
      {i18n.t('walletImport.complete.waitingHint')}
    {:else}
      {i18n.t('walletImport.complete.loadingHint')}
    {/if}
  </p>
</div>
