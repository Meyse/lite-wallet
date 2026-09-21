<script lang="ts">
  import { i18nStore } from '$lib/i18n';
  import { settingsStore } from '$lib/stores/settings';
  import { profileFeeDisplay } from '$lib/identity/profileImages';
  import type { IdentityProfilePreflightResult } from '$lib/types/wallet';
  import { formatFiatAmount } from '$lib/utils/fiatDisplay';
  let {
    preflight,
    feeRate,
  }: { preflight: IdentityProfilePreflightResult; feeRate: number | null } = $props();
  const i18n = $derived($i18nStore);
  const plan = $derived(preflight.publication);
  const split = $derived(plan.totalSteps === 2 && plan.step === 1);
  const changed = $derived(
    plan.step === 2 &&
      plan.earlierFeeSats !== null &&
      BigInt(plan.earlierFeeSats) !== BigInt(preflight.feeSats)
  );
  const firstLabel = $derived(
    preflight.changedFields
      .map((field) => i18n.t(`wallet.identity.profile.draft.${field}`))
      .join(', ')
  );
  function fiat(sats: string): string | null {
    const amount = feeRate === null ? 0 : (Number(sats) / 100_000_000) * feeRate;
    return amount > 0 && Number.isFinite(amount)
      ? `${amount < 0.01 ? '< ' : '≈ '}${formatFiatAmount(Math.max(0.01, amount), i18n.intlLocale, $settingsStore.displayCurrency)}`
      : null;
  }
</script>

<div class="mt-2 rounded-lg bg-muted px-3.5 py-3 text-[13px]" data-publication-costs>
  {#if split}
    <p class="mb-2 text-xs text-muted-foreground">
      {i18n.t('wallet.identity.profile.sequence.approvals')}
    </p>
    <dl class="space-y-1.5">
      {#each [{ label: firstLabel, sats: preflight.feeSats, accuracy: 'exact' }, { label: i18n.t('wallet.identity.profile.draft.header'), sats: plan.nextFeeSats!, accuracy: 'estimated' }, { label: i18n.t('wallet.identity.profile.sequence.total'), sats: plan.estimatedTotalFeeSats, accuracy: null }] as row}
        <div class="flex items-baseline justify-between gap-3">
          <dt class="text-muted-foreground">{row.label}</dt>
          <dd class="shrink-0 text-right tabular-nums">
            {#if row.accuracy}<span class="mr-2 text-xs text-muted-foreground"
                >{i18n.t(`wallet.identity.profile.sequence.${row.accuracy}`)}</span
              >{/if}<span class:font-semibold={!row.accuracy}
              >{profileFeeDisplay(row.sats)} VRSCTEST</span
            >{#if fiat(row.sats)}<span class="ml-2 text-xs text-muted-foreground"
                >{fiat(row.sats)}</span
              >{/if}
          </dd>
        </div>
      {/each}
    </dl>
  {:else}
    <div class="flex items-center justify-between gap-3">
      <span class="font-medium">{i18n.t('wallet.identity.profile.draft.fee')}</span>
      <div class="text-right">
        <p class="font-semibold tabular-nums">
          {#if plan.step === 2}<span class="mr-2 text-xs font-normal text-muted-foreground"
              >{i18n.t('wallet.identity.profile.sequence.exact')}</span
            >{/if}{profileFeeDisplay(preflight.feeSats)} VRSCTEST
        </p>
        {#if fiat(preflight.feeSats)}<p class="mt-1 text-xs text-muted-foreground">
            {fiat(preflight.feeSats)}
          </p>{/if}
      </div>
    </div>
  {/if}
</div>
{#if changed}<p class="mt-2 text-xs leading-5 text-muted-foreground">
    {i18n.t('wallet.identity.profile.sequence.feeChanged')}
    {i18n.t('wallet.identity.profile.sequence.earlier', {
      fee: profileFeeDisplay(plan.earlierFeeSats!),
    })}
  </p>{/if}
{#if BigInt(plan.availableSats) < BigInt(preflight.feeSats)}<p
    role="alert"
    class="mt-2 text-xs text-destructive"
  >
    {i18n.t('wallet.identity.profile.error.insufficientFunds')}
  </p>{/if}
<details class="mt-2 text-xs text-muted-foreground" data-fee-details>
  <summary class="w-fit rounded-sm focus-visible:ring-2 focus-visible:ring-ring"
    >{i18n.t('wallet.identity.profile.ux.feeDetails')}</summary
  >
  <div class="mt-2 space-y-1">
    <p>
      {i18n.t('wallet.identity.profile.sequence.available', {
        fee: profileFeeDisplay(plan.availableSats),
      })}
    </p>
    {#if plan.earlierFeeSats && !changed}<p>
        {i18n.t('wallet.identity.profile.sequence.earlier', {
          fee: profileFeeDisplay(plan.earlierFeeSats),
        })}
      </p>{/if}
    <p>{i18n.t('wallet.identity.profile.ux.quoteHeight', { height: plan.quoteHeight })}</p>
  </div>
</details>
