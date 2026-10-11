<script lang="ts">
  import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';
  import ShieldAlertIcon from '@lucide/svelte/icons/shield-alert';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Spinner } from '$lib/components/ui/spinner';
  import { i18nStore } from '$lib/i18n';
  import type { GiftCardModule } from './modules/GiftCardModule.svelte';
  import RequestFacts from './RequestFacts.svelte';

  /** Gift card claim: the password when the card has one, then what it holds. */
  let { module }: { module: GiftCardModule } = $props();

  const i18n = $derived($i18nStore);
  const review = $derived(module.review);
  const blocked = $derived(review?.identities.filter((identity) => !identity.claimable) ?? []);
  const kept = $derived(
    review?.identities.filter((identity) => identity.claimable && identity.externalAuthorities) ??
      []
  );
  const empty = $derived(
    !!review && review.currencies.length === 0 && review.identities.length === 0
  );
</script>

{#if !review}
  {#if module.detail.requiresPassword}
    <form
      class="flex flex-col gap-3"
      onsubmit={(event) => {
        event.preventDefault();
        void module.unlock();
      }}
    >
      <p class="text-[13.5px] text-foreground/80">
        {i18n.t('genericRequest.flow.giftCard.passwordIntro')}
      </p>
      <div class="flex flex-col gap-1.5">
        <Label for="request-gift-card-password" class="text-xs font-medium text-muted-foreground">
          {i18n.t('genericRequest.flow.giftCard.password')}
        </Label>
        <Input
          id="request-gift-card-password"
          type="password"
          autocomplete="off"
          bind:value={module.password}
          disabled={module.reviewLoading}
        />
      </div>
      <Button
        type="submit"
        variant="secondary"
        class="w-fit"
        disabled={!module.password || module.reviewLoading}
      >
        {#if module.reviewLoading}
          <Spinner class="size-4" />
        {/if}
        {i18n.t('genericRequest.flow.giftCard.unlock')}
      </Button>
    </form>
  {:else if module.reviewLoading}
    <p class="flex items-center gap-2 text-sm text-muted-foreground">
      <Spinner class="size-4" />
      {i18n.t('genericRequest.flow.giftCard.loading')}
    </p>
  {/if}
{:else if empty}
  <p class="rounded-xl bg-muted/60 px-4 py-3.5 text-sm text-foreground">
    {i18n.t('genericRequest.flow.giftCard.empty')}
  </p>
{:else}
  <p class="text-[13.5px] text-foreground/80">
    {blocked.length > 0
      ? i18n.t('genericRequest.flow.giftCard.holds')
      : i18n.t('genericRequest.flow.giftCard.intro')}
  </p>
  <ul class="overflow-hidden rounded-xl border border-border/70">
    {#each module.currencyTexts() as text, index (index)}
      <li
        class="border-b border-border/70 px-3.5 py-3 text-sm font-medium text-foreground tabular-nums last:border-b-0"
      >
        {text}
      </li>
    {/each}
    {#each review.identities as identity (identity.identityAddress)}
      <li class="border-b border-border/70 px-3.5 py-3 last:border-b-0">
        <p class="text-sm font-medium text-foreground">{module.identityName(identity)}</p>
        <p class="text-xs text-muted-foreground">
          {identity.claimable
            ? i18n.t('genericRequest.flow.giftCard.moves')
            : i18n.t('genericRequest.flow.giftCard.cannotMove')}
        </p>
      </li>
    {/each}
  </ul>

  {#if blocked.length > 0}
    <p class="text-sm text-destructive">
      {i18n.t('genericRequest.flow.giftCard.blocked', {
        identity: module.identityName(blocked[0]),
      })}
    </p>
  {:else if kept.length > 0}
    <div
      class="flex gap-2.5 rounded-xl border border-amber-300/70 bg-amber-50/80 px-3.5 py-3 dark:border-amber-500/30 dark:bg-amber-950/20"
    >
      <ShieldAlertIcon class="mt-0.5 size-[18px] shrink-0 text-amber-700 dark:text-amber-300" />
      <p class="text-[13px] text-foreground/80">
        {i18n.t('genericRequest.flow.giftCard.authorities', {
          identity: module.identityName(kept[0]),
        })}
      </p>
    </div>
  {/if}

  {#if module.preflightLoading}
    <p class="flex items-center gap-2 text-sm text-muted-foreground">
      <Spinner class="size-4" />
      {i18n.t('genericRequest.flow.giftCard.preparing')}
    </p>
  {:else if module.preflight}
    <RequestFacts
      rows={[{ label: i18n.t('genericRequest.flow.networkFee'), value: module.feeText() }]}
    />
  {/if}
{/if}

{#if module.error}
  <div
    class="flex items-start gap-2 rounded-xl border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
  >
    <AlertCircleIcon class="mt-0.5 size-4 shrink-0" />
    <p>{module.error}</p>
  </div>
{/if}
