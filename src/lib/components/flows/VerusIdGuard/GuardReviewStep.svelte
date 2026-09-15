<script lang="ts">
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
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
    <h1 class="text-2xl leading-tight font-semibold tracking-tight text-foreground">
      {i18n.t('guard.flow.review.title')}
    </h1>
    <p class="text-sm text-muted-foreground">
      {i18n.t('guard.flow.review.description', { action: actionLabel })}
    </p>
  </div>

  <div class="space-y-4 rounded-xl border border-border/70 bg-muted/20 p-4">
    <div class="grid gap-3 sm:grid-cols-2">
      <div>
        <p class="text-xs text-muted-foreground">{i18n.t('guard.flow.review.network')}</p>
        <p class="text-sm font-medium text-foreground">
          {i18n.t(networkLocaleKey(context.network))}
        </p>
      </div>
      <div>
        <p class="text-xs text-muted-foreground">{i18n.t('guard.flow.review.operation')}</p>
        <p class="text-sm font-medium text-foreground">{actionLabel}</p>
      </div>
      <div class="sm:col-span-2">
        <p class="text-xs text-muted-foreground">{i18n.t('guard.flow.review.targetIdentity')}</p>
        {#if context.targetIdentity.trim().endsWith('@')}
          <p class="text-sm font-medium text-foreground">{context.targetIdentity}</p>
        {:else}
          <IdentifierText
            value={context.targetIdentity}
            mode="full"
            class="block text-sm font-medium text-foreground"
          />
        {/if}
      </div>
      <div class="sm:col-span-2">
        <p class="text-xs text-muted-foreground">{i18n.t('guard.flow.review.authorityAddress')}</p>
        <IdentifierText
          value={context.authorityAddress}
          mode="full"
          class="block text-sm font-medium text-foreground"
        />
      </div>
      <div>
        <p class="text-xs text-muted-foreground">{i18n.t('guard.flow.review.fee')}</p>
        <p class="text-sm font-medium text-foreground">
          {context.preflight.fee}
          {context.preflight.feeCurrency}
        </p>
      </div>
      <div>
        <p class="text-xs text-muted-foreground">{i18n.t('guard.flow.review.preflightId')}</p>
        <IdentifierText
          value={context.preflight.preflightId}
          mode="full"
          class="block text-sm font-medium text-foreground"
        />
      </div>
    </div>

    {#if context.mode === 'recover'}
      <Separator />
      <div class="space-y-2">
        <p class="text-xs font-medium text-foreground">{i18n.t('guard.flow.review.patchTitle')}</p>
        <div class="space-y-1 text-xs text-muted-foreground">
          <div>
            <p>{i18n.t('guard.flow.patch.primaryAddressLabel')}</p>
            <IdentifierText
              value={context.recoverDraft.primaryAddress}
              mode="full"
              class="block text-foreground"
            />
          </div>
          {#if context.recoverDraft.recoveryAuthority.trim()}
            <div>
              <p>{i18n.t('guard.flow.patch.recoveryAuthorityLabel')}</p>
              <IdentifierText
                value={context.recoverDraft.recoveryAuthority}
                mode="full"
                class="block text-foreground"
              />
            </div>
          {/if}
          {#if context.recoverDraft.revocationAuthority.trim()}
            <div>
              <p>{i18n.t('guard.flow.patch.revocationAuthorityLabel')}</p>
              <IdentifierText
                value={context.recoverDraft.revocationAuthority}
                mode="full"
                class="block text-foreground"
              />
            </div>
          {/if}
          {#if context.recoverDraft.privateAddress.trim()}
            <div>
              <p>{i18n.t('guard.flow.patch.privateAddressLabel')}</p>
              <IdentifierText
                value={context.recoverDraft.privateAddress}
                mode="full"
                class="block text-foreground"
              />
            </div>
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
            <li
              class="rounded-md border border-border/60 bg-background/70 px-3 py-2 text-xs text-muted-foreground"
            >
              <p class="font-medium text-foreground">{warning.warningType}</p>
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
            <li
              class="rounded-md border border-border/60 bg-background/70 px-3 py-2 text-xs text-muted-foreground"
            >
              <p class="font-medium text-foreground">{change.changeType}</p>
              {#if change.beforeValue}
                <p>{i18n.t('guard.flow.review.beforeValue')}</p>
                <IdentifierText value={change.beforeValue} mode="review" class="block" />
              {/if}
              {#if change.afterValue}
                <p class="mt-1">{i18n.t('guard.flow.review.afterValue')}</p>
                <IdentifierText value={change.afterValue} mode="review" class="block" />
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>
</div>
