<script lang="ts">
  import type { Snippet } from 'svelte';
  import KeyRoundIcon from '@lucide/svelte/icons/key-round';
  import ShieldAlertIcon from '@lucide/svelte/icons/shield-alert';
  import { Button } from '$lib/components/ui/button';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Label } from '$lib/components/ui/label';
  import { Spinner } from '$lib/components/ui/spinner';
  import { i18nStore } from '$lib/i18n';
  import type { EncryptionKeyModule } from './modules/EncryptionKeyModule.svelte';
  import RequestFacts from './RequestFacts.svelte';

  /** Encryption key: who answers, what the app gets, and private address setup. */
  let {
    module,
    responder,
    onSetupPrivate,
  }: {
    module: EncryptionKeyModule;
    responder: Snippet;
    onSetupPrivate?: () => void;
  } = $props();

  const i18n = $derived($i18nStore);
</script>

{#if module.privateReady === null}
  <p class="flex items-center gap-2 text-sm text-muted-foreground">
    <Spinner class="size-4" />
    {i18n.t('genericRequest.flow.encryptionKey.checking')}
  </p>
{:else if !module.privateReady}
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
{:else}
  {@render responder()}
  <RequestFacts rows={module.facts()} />

  {#if module.detail.returnsSpendingKey}
    <div
      class="flex gap-2.5 rounded-xl border border-amber-300/70 bg-amber-50/80 px-3.5 py-3 dark:border-amber-500/30 dark:bg-amber-950/20"
    >
      <ShieldAlertIcon class="mt-0.5 size-[18px] shrink-0 text-amber-700 dark:text-amber-300" />
      <div class="min-w-0">
        <p class="text-sm font-semibold text-foreground">
          {i18n.t('genericRequest.flow.encryptionKey.spendingKeyTitle')}
        </p>
        <p class="mt-0.5 text-[13px] text-foreground/80">
          {i18n.t('genericRequest.flow.encryptionKey.spendingKeyBody')}
        </p>
      </div>
    </div>
    <Label
      for="request-spending-key-ack"
      class="flex items-start gap-2.5 text-[13.5px] leading-snug font-normal"
    >
      <Checkbox
        id="request-spending-key-ack"
        checked={module.spendingKeyAcknowledged}
        onCheckedChange={(checked) => (module.spendingKeyAcknowledged = checked === true)}
        class="mt-0.5"
      />
      <span>{i18n.t('genericRequest.flow.encryptionKey.spendingKeyAcknowledge')}</span>
    </Label>
  {/if}
{/if}

{#if module.error}
  <p class="text-sm text-destructive">{module.error}</p>
{/if}
