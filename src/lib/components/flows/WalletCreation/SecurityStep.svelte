<!-- 
  Component: SecurityStep
  Purpose: Security guidelines step before showing seed phrase
  Last Updated: Includes acknowledgment control within step content for cleaner footer alignment
  Security: Ensures users read security guidelines before accessing sensitive data
-->

<script lang="ts">
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Label } from '$lib/components/ui/label';
  import { i18nStore } from '$lib/i18n';

  let {
    securityAccepted = $bindable(false),
    setupPrivateVerus = $bindable(true)
  }: {
    securityAccepted: boolean;
    setupPrivateVerus: boolean;
  } = $props();
  const i18n = $derived($i18nStore);
</script>

<!-- Security Guidelines Content -->
<div class="w-full space-y-5">
  <div class="rounded-xl border border-amber-300/70 bg-amber-50 p-6 dark:border-amber-500/35 dark:bg-amber-500/12">
    <p class="mb-4 text-sm font-semibold text-amber-900 dark:text-amber-200">
      {i18n.t('walletCreation.security.warning')}
    </p>

    <ul class="space-y-2 text-sm text-foreground">
      <li class="flex items-start gap-2">
        <span class="text-amber-700 dark:text-amber-400">•</span>
        <span>{i18n.t('walletCreation.security.item1')}</span>
      </li>
      <li class="flex items-start gap-2">
        <span class="text-amber-700 dark:text-amber-400">•</span>
        <span>{i18n.t('walletCreation.security.item2')}</span>
      </li>
      <li class="flex items-start gap-2">
        <span class="text-amber-700 dark:text-amber-400">•</span>
        <span>{i18n.t('walletCreation.security.item3')}</span>
      </li>
      <li class="flex items-start gap-2">
        <span class="text-amber-700 dark:text-amber-400">•</span>
        <span>{i18n.t('walletCreation.security.item4')}</span>
      </li>
    </ul>
  </div>

  <Label
    for="security-acceptance-main"
    class="flex cursor-pointer items-start gap-3 rounded-lg border border-border/60 bg-muted/20 px-4 py-3 font-normal leading-normal"
  >
    <Checkbox id="security-acceptance-main" bind:checked={securityAccepted} class="mt-0.5" />
    <span class="text-sm leading-5 text-foreground">
      {i18n.t('walletCreation.step3.checkbox')}
    </span>
  </Label>

  <Label
    for="security-private-verus"
    class="flex cursor-pointer items-start gap-3 rounded-lg border border-border/60 bg-muted/20 px-4 py-3 font-normal leading-normal"
  >
    <Checkbox id="security-private-verus" bind:checked={setupPrivateVerus} class="mt-0.5" />
    <div class="space-y-1">
      <span class="text-sm leading-5 text-foreground">
        {i18n.t('walletCreation.security.privateVerusToggle')}
      </span>
      <p class="text-muted-foreground text-xs leading-5">
        {i18n.t('walletCreation.security.privateVerusHint')}
      </p>
    </div>
  </Label>
</div>
