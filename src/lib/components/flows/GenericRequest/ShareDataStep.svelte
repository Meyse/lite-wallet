<script lang="ts">
  import type { Snippet } from 'svelte';
  import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';
  import KeyRoundIcon from '@lucide/svelte/icons/key-round';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import { Button } from '$lib/components/ui/button';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Label } from '$lib/components/ui/label';
  import { Spinner } from '$lib/components/ui/spinner';
  import { i18nStore } from '$lib/i18n';
  import type { GenericSharedCredential } from '$lib/types/wallet.js';
  import type { ShareDataModule } from './modules/ShareDataModule.svelte';

  /** Share data: one consent per credential found (spec "Share data"). */
  let {
    module,
    appLabel,
    responderName,
    responder,
    onSetupPrivate,
    onRetry,
  }: {
    module: ShareDataModule;
    appLabel: string;
    responderName: string;
    /** The responder picker, when no sign in before this step chose one. */
    responder?: Snippet;
    onSetupPrivate?: () => void;
    onRetry: () => void;
  } = $props();

  const i18n = $derived($i18nStore);

  function title(credential: GenericSharedCredential): string {
    return credential.label || credential.key;
  }

  /** The credential's fields as label and value, as the app sees them. */
  function fields(credential: GenericSharedCredential): { name: string; value: string }[] {
    const value = credential.credential;
    if (value && typeof value === 'object' && !Array.isArray(value)) {
      return Object.entries(value).map(([name, field]) => ({
        name,
        value: typeof field === 'string' ? field : JSON.stringify(field),
      }));
    }
    return [{ name: '', value: typeof value === 'string' ? value : JSON.stringify(value) }];
  }
</script>

{#if responder}
  {@render responder()}
{/if}

{#if module.privateReady === false}
  <div class="flex flex-col items-start gap-3 rounded-xl bg-muted/60 px-4 py-4">
    <span class="flex items-center gap-2 text-sm font-semibold text-foreground">
      <KeyRoundIcon class="size-4 text-muted-foreground" />
      {i18n.t('genericRequest.flow.encryptionKey.setupTitle')}
    </span>
    <p class="text-[13.5px] text-foreground/80">
      {i18n.t('genericRequest.flow.encryptionKey.setupBody')}
    </p>
    {#if onSetupPrivate}
      <Button variant="secondary" onclick={onSetupPrivate}>
        {i18n.t('genericRequest.flow.encryptionKey.setupAction')}
      </Button>
    {/if}
  </div>
{:else if module.loading || module.privateReady === null}
  <p class="flex items-center gap-2 text-sm text-muted-foreground">
    <Spinner class="size-4" />
    {i18n.t('genericRequest.flow.shareData.loading')}
  </p>
{:else if module.error}
  <div
    class="flex items-start gap-2 rounded-xl border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
  >
    <AlertCircleIcon class="mt-0.5 size-4 shrink-0" />
    <div class="flex min-w-0 flex-col items-start gap-2">
      <p>{module.error}</p>
      <Button variant="secondary" size="sm" onclick={onRetry}>
        {i18n.t('common.retry')}
      </Button>
    </div>
  </div>
{:else if module.review}
  {#if module.count > 0}
    <p class="text-[13.5px] text-foreground/80">
      {i18n.t('genericRequest.flow.shareData.intro', { app: appLabel, identity: responderName })}
    </p>
    <ul class="overflow-hidden rounded-xl border border-border/70">
      {#each module.review.credentials as credential, index (index)}
        <li class="flex items-start gap-3 border-b border-border/70 px-3.5 py-3 last:border-b-0">
          <Checkbox
            id={`request-share-credential-${index}`}
            checked={module.consent[index]}
            onCheckedChange={(checked) => module.setConsent(index, checked === true)}
            class="mt-0.5"
            aria-label={i18n.t('genericRequest.flow.shareData.consent', {
              credential: title(credential),
            })}
          />
          <div class="flex min-w-0 flex-1 flex-col gap-1.5">
            <Label for={`request-share-credential-${index}`} class="leading-snug text-foreground">
              {title(credential)}
            </Label>
            <dl class="flex flex-col gap-0.5 text-[13px]">
              {#each fields(credential) as field, at (at)}
                <div class="flex min-w-0 gap-2">
                  {#if field.name}
                    <dt class="shrink-0 text-muted-foreground">{field.name}</dt>
                  {/if}
                  <dd class="min-w-0 break-words text-foreground/90">{field.value}</dd>
                </div>
              {/each}
            </dl>
            {#if credential.label}
              <IdentifierText
                value={credential.key}
                mode="compact"
                class="text-xs text-muted-foreground"
              />
            {/if}
          </div>
        </li>
      {/each}
    </ul>
    {#if !module.consent.every(Boolean)}
      <p class="text-xs text-muted-foreground">
        {i18n.t('genericRequest.flow.shareData.allNeeded')}
      </p>
    {/if}
  {:else}
    <div class="flex flex-col gap-1 rounded-xl bg-muted/60 px-4 py-3.5">
      <p class="text-sm text-foreground">
        {i18n.t('genericRequest.flow.shareData.none', { identity: responderName })}
      </p>
      {#if !module.answersOtherwise}
        <p class="text-[13px] text-foreground/80">
          {i18n.t('genericRequest.flow.shareData.nothingToSend')}
        </p>
      {/if}
    </div>
  {/if}

  {#if module.count > 0 && module.review.missingKeys.length > 0}
    <div class="flex flex-col gap-1 text-xs text-muted-foreground">
      <p>
        {i18n.t(
          `genericRequest.flow.shareData.missing.${module.review.missingKeys.length === 1 ? 'one' : 'other'}`,
          { count: module.review.missingKeys.length }
        )}
      </p>
      {#each module.review.missingKeys as key (key)}
        <IdentifierText value={key} mode="compact" />
      {/each}
    </div>
  {/if}
{/if}
