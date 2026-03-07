<script lang="ts">
  import CircleCheckBigIcon from '@lucide/svelte/icons/circle-check-big';
  import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
  import { CopyActionButton, type CopyActionState } from '$lib/components/ui/copy-action-button';
  import { i18nStore } from '$lib/i18n';
  import type { GuardFlowMode } from './types';
  import type { GuardSendResult } from '$lib/types/wallet.js';

  type GuardResultStepProps = {
    mode: GuardFlowMode;
    sendResult: GuardSendResult | null;
    errorMessage?: string;
    copyStatus?: CopyActionState | 'idle';
    onCopyTxid?: () => void;
  };

  const defaultHandler = () => {};

   
  let {
    mode,
    sendResult,
    errorMessage = '',
    copyStatus = 'idle',
    onCopyTxid = defaultHandler
  }: GuardResultStepProps = $props();
   

  const i18n = $derived($i18nStore);
  const hasSuccess = $derived(!!sendResult && !errorMessage);
  const actionLabel = $derived(mode === 'revoke' ? i18n.t('guard.mode.revoke') : i18n.t('guard.mode.recover'));
</script>

<div class="mx-auto w-full max-w-[560px] space-y-6 py-6 text-center">
  <div class="space-y-3">
    <div
      class={`mx-auto flex h-20 w-20 items-center justify-center rounded-full ${
        hasSuccess ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400' : 'bg-destructive/15 text-destructive'
      }`}
    >
      {#if hasSuccess}
        <CircleCheckBigIcon class="h-10 w-10" />
      {:else}
        <TriangleAlertIcon class="h-10 w-10" />
      {/if}
    </div>

    <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
      {#if hasSuccess}
        {i18n.t('guard.flow.result.successTitle', { action: actionLabel })}
      {:else}
        {i18n.t('guard.flow.result.errorTitle', { action: actionLabel })}
      {/if}
    </h1>

    <p class="text-muted-foreground text-sm">
      {#if hasSuccess}
        {i18n.t('guard.flow.result.successDescription')}
      {:else}
        {errorMessage || i18n.t('guard.error.generic')}
      {/if}
    </p>
  </div>

  {#if hasSuccess && sendResult}
    <div class="bg-muted/20 border-border/70 space-y-2 rounded-xl border p-4 text-left">
      <p class="text-muted-foreground text-xs">{i18n.t('guard.flow.result.txidLabel')}</p>
      <p class="identifier-text text-sm break-all text-foreground">{sendResult.txid}</p>
      <CopyActionButton
        state={copyStatus}
        variant="outline"
        size="sm"
        onclick={onCopyTxid}
        label={i18n.t('guard.flow.result.copyTxid')}
        copiedLabel={i18n.t('guard.flow.result.copySuccess')}
        failedLabel={i18n.t('guard.flow.result.copyFailed')}
      />
    </div>
  {/if}

</div>
