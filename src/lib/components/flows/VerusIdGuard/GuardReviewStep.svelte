<script lang="ts">
  import { Separator } from '$lib/components/ui/separator';
  import { i18nStore, networkLocaleKey } from '$lib/i18n';
  import type { GuardReviewContext } from './types';

  type GuardReviewStepProps = {
    context: GuardReviewContext;
  };

   
  let { context }: GuardReviewStepProps = $props();
   

  const i18n = $derived($i18nStore);
  const actionLabel = $derived(
    context.mode === 'revoke' ? i18n.t('guard.mode.revoke') : i18n.t('guard.mode.recover')
  );
</script>

<div class="mx-auto w-full max-w-[560px] space-y-6 py-4">
  <div class="space-y-2 text-center">
    <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
      {i18n.t('guard.flow.review.title')}
    </h1>
    <p class="text-muted-foreground text-sm">
      {i18n.t('guard.flow.review.description', { action: actionLabel })}
    </p>
  </div>

  <div class="bg-muted/20 border-border/70 space-y-4 rounded-xl border p-4">
    <div class="grid gap-3 sm:grid-cols-2">
      <div>
        <p class="text-muted-foreground text-xs">{i18n.t('guard.flow.review.network')}</p>
        <p class="text-sm font-medium text-foreground">{i18n.t(networkLocaleKey(context.network))}</p>
      </div>
      <div>
        <p class="text-muted-foreground text-xs">{i18n.t('guard.flow.review.operation')}</p>
        <p class="text-sm font-medium text-foreground">{actionLabel}</p>
      </div>
      <div class="sm:col-span-2">
        <p class="text-muted-foreground text-xs">{i18n.t('guard.flow.review.targetIdentity')}</p>
        <p class="identifier-text text-sm font-medium text-foreground break-all">{context.targetIdentity}</p>
      </div>
      <div class="sm:col-span-2">
        <p class="text-muted-foreground text-xs">{i18n.t('guard.flow.review.authorityAddress')}</p>
        <p class="identifier-text text-sm font-medium text-foreground break-all">{context.authorityAddress}</p>
      </div>
      <div>
        <p class="text-muted-foreground text-xs">{i18n.t('guard.flow.review.fee')}</p>
        <p class="text-sm font-medium text-foreground">{context.preflight.fee} {context.preflight.feeCurrency}</p>
      </div>
      <div>
        <p class="text-muted-foreground text-xs">{i18n.t('guard.flow.review.preflightId')}</p>
        <p class="identifier-text text-sm font-medium text-foreground break-all">{context.preflight.preflightId}</p>
      </div>
    </div>

    {#if context.mode === 'recover'}
      <Separator />
      <div class="space-y-2">
        <p class="text-xs font-medium text-foreground">{i18n.t('guard.flow.review.patchTitle')}</p>
        <div class="space-y-1 text-xs text-muted-foreground">
          <p>
            {i18n.t('guard.flow.patch.primaryAddressLabel')}: <span class="identifier-text text-foreground break-all">{context.recoverDraft.primaryAddress}</span>
          </p>
          {#if context.recoverDraft.recoveryAuthority.trim()}
            <p>
              {i18n.t('guard.flow.patch.recoveryAuthorityLabel')}: <span class="identifier-text text-foreground break-all">{context.recoverDraft.recoveryAuthority}</span>
            </p>
          {/if}
          {#if context.recoverDraft.revocationAuthority.trim()}
            <p>
              {i18n.t('guard.flow.patch.revocationAuthorityLabel')}: <span class="identifier-text text-foreground break-all">{context.recoverDraft.revocationAuthority}</span>
            </p>
          {/if}
          {#if context.recoverDraft.privateAddress.trim()}
            <p>
              {i18n.t('guard.flow.patch.privateAddressLabel')}: <span class="identifier-text text-foreground break-all">{context.recoverDraft.privateAddress}</span>
            </p>
          {/if}
        </div>
      </div>
    {/if}

    <Separator />

    <div class="space-y-2">
      <p class="text-xs font-medium text-foreground">{i18n.t('guard.flow.review.warningsTitle')}</p>
      {#if context.preflight.warnings.length === 0}
        <p class="text-xs text-muted-foreground">{i18n.t('guard.flow.review.noWarnings')}</p>
      {:else}
        <ul class="space-y-2">
          {#each context.preflight.warnings as warning (warning.warningType + warning.message)}
            <li class="bg-background/70 border-border/60 rounded-md border px-3 py-2 text-xs text-muted-foreground">
              <p class="text-foreground font-medium">{warning.warningType}</p>
              <p>{warning.message}</p>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="space-y-2">
      <p class="text-xs font-medium text-foreground">{i18n.t('guard.flow.review.highRiskTitle')}</p>
      {#if context.preflight.highRiskChanges.length === 0}
        <p class="text-xs text-muted-foreground">{i18n.t('guard.flow.review.noHighRisk')}</p>
      {:else}
        <ul class="space-y-2">
          {#each context.preflight.highRiskChanges as change (change.changeType + (change.afterValue || ''))}
            <li class="bg-background/70 border-border/60 rounded-md border px-3 py-2 text-xs text-muted-foreground">
              <p class="text-foreground font-medium">{change.changeType}</p>
              {#if change.beforeValue}
                <p>{i18n.t('guard.flow.review.beforeValue')}: {change.beforeValue}</p>
              {/if}
              {#if change.afterValue}
                <p>{i18n.t('guard.flow.review.afterValue')}: {change.afterValue}</p>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>

</div>
