<script lang="ts">
  import { i18nStore } from '$lib/i18n';
  import { settingsStore } from '$lib/stores/settings';
  import { profileFeeDisplay } from '$lib/identity/profileImages';
  import type { IdentityProfilePreflightResult } from '$lib/types/wallet';
  import { formatFiatAmount } from '$lib/utils/fiatDisplay';
  let {
    preflight,
    feeRate,
    plain = false,
  }: {
    preflight: IdentityProfilePreflightResult;
    feeRate: number | null;
    plain?: boolean;
  } = $props();
  const i18n = $derived($i18nStore);
  const plan = $derived(preflight.publication);
  const split = $derived(plan.totalSteps === 2 && plan.step === 1);
  const changed = $derived(
    plan.step === 2 &&
      plan.earlierFeeSats !== null &&
      BigInt(plan.earlierFeeSats) !== BigInt(preflight.feeSats)
  );
  const firstLabel = $derived(
    (() => {
      const fields = preflight.changedFields.map((field) =>
        i18n.t(`wallet.identity.profile.draft.${field}`).toLocaleLowerCase(i18n.intlLocale)
      );
      const text = new Intl.ListFormat(i18n.intlLocale, {
        style: 'long',
        type: 'conjunction',
      }).format(fields);
      return text.charAt(0).toLocaleUpperCase(i18n.intlLocale) + text.slice(1);
    })()
  );
  function fiat(sats: string): string | null {
    const amount = feeRate === null ? 0 : (Number(sats) / 100_000_000) * feeRate;
    return amount > 0 && Number.isFinite(amount)
      ? `${amount < 0.01 ? '< ' : '≈ '}${formatFiatAmount(Math.max(0.01, amount), i18n.intlLocale, $settingsStore.displayCurrency)}`
      : null;
  }
</script>

<div
  class={split
    ? 'mt-3 text-[13px]'
    : plain
      ? 'mt-5 border-t pt-5 text-sm'
      : 'mt-2 rounded-lg bg-muted px-3.5 py-3 text-[13px]'}
  data-publication-costs
>
  {#if split}
    <dl>
      {#each [{ label: firstLabel, sats: preflight.feeSats, accuracy: 'exact' }, { label: i18n.t('wallet.identity.profile.draft.header'), sats: plan.nextFeeSats!, accuracy: 'estimated' }, { label: i18n.t('wallet.identity.profile.sequence.total'), sats: plan.estimatedTotalFeeSats, accuracy: null }] as row, index}
        <div class="flex min-h-11 items-center justify-between gap-3 border-b last:border-b-0">
          <dt class="flex min-w-0 items-center gap-3 text-muted-foreground">
            {#if index < 2}<span
                class="flex size-5 shrink-0 items-center justify-center rounded-full bg-muted text-xs"
                >{index + 1}</span
              >{/if}
            {row.label}
          </dt>
          <dd class="shrink-0 text-right tabular-nums">
            {#if row.accuracy}<span class="mr-2 text-xs text-muted-foreground"
                >{i18n.t(`wallet.identity.profile.sequence.${row.accuracy}`)}</span
              >{/if}<span class:font-semibold={!row.accuracy}
              >{profileFeeDisplay(row.sats)} VRSCTEST</span
            >
          </dd>
        </div>
      {/each}
    </dl>
  {:else}
    <div class="flex items-center justify-between gap-3">
      <span class={plain ? 'text-muted-foreground' : 'font-medium'}
        >{i18n.t('wallet.identity.profile.draft.fee')}</span
      >
      <div class="text-right">
        <p class={plain ? 'text-xl font-semibold tabular-nums' : 'font-semibold tabular-nums'}>
          {profileFeeDisplay(preflight.feeSats)} VRSCTEST
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
