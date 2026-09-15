<script lang="ts">
  import CircleCheckBigIcon from '@lucide/svelte/icons/circle-check-big';
  import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
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
    onCopyTxid = defaultHandler,
  }: GuardResultStepProps = $props();

  const i18n = $derived($i18nStore);
  const hasSuccess = $derived(!!sendResult && !errorMessage);
  const actionLabel = $derived(
    mode === 'revoke' ? i18n.t('guard.mode.revoke') : i18n.t('guard.mode.recover')
  );
</script>

<div class="mx-auto w-full max-w-[560px] space-y-6 py-6 text-center">
  <div class="space-y-3">
    <div
      class={`mx-auto flex h-20 w-20 items-center justify-center rounded-full ${
        hasSuccess
          ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400'
          : 'bg-destructive/15 text-destructive'
      }`}
    >
      {#if hasSuccess}
        <CircleCheckBigIcon class="h-10 w-10" />
      {:else}
        <TriangleAlertIcon class="h-10 w-10" />
      {/if}
    </div>

    <h1 class="text-2xl leading-tight font-semibold tracking-tight text-foreground">
      {#if hasSuccess}
        {i18n.t('guard.flow.result.successTitle', { action: actionLabel })}
      {:else}
        {i18n.t('guard.flow.result.errorTitle', { action: actionLabel })}
      {/if}
    </h1>

    <p class="text-sm text-muted-foreground">
      {#if hasSuccess}
        {i18n.t('guard.flow.result.successDescription')}
      {:else}
        {errorMessage || i18n.t('guard.error.generic')}
      {/if}
    </p>
  </div>

  {#if hasSuccess && sendResult}
    <div class="space-y-2 rounded-xl border border-border/70 bg-muted/20 p-4 text-left">
      <p class="text-xs text-muted-foreground">{i18n.t('guard.flow.result.txidLabel')}</p>
      <IdentifierText value={sendResult.txid} mode="full" class="block text-sm text-foreground" />
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
