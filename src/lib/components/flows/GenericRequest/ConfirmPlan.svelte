<script lang="ts">
  import BadgePlusIcon from '@lucide/svelte/icons/badge-plus';
  import CoinsIcon from '@lucide/svelte/icons/coins';
  import LogInIcon from '@lucide/svelte/icons/log-in';
  import UserCogIcon from '@lucide/svelte/icons/user-cog';
  import CircleCheckIcon from '@lucide/svelte/icons/circle-check';
  import GiftIcon from '@lucide/svelte/icons/gift';
  import KeyRoundIcon from '@lucide/svelte/icons/key-round';
  import ReceiptIcon from '@lucide/svelte/icons/receipt';
  import Share2Icon from '@lucide/svelte/icons/share-2';
  import ShieldAlertIcon from '@lucide/svelte/icons/shield-alert';
  import SignatureIcon from '@lucide/svelte/icons/signature';
  import type { RequestStepId } from '$lib/genericRequest/flowPlan';
  import { i18nStore } from '$lib/i18n';
  import DeliveryLine from './DeliveryLine.svelte';
  import RequestFacts from './RequestFacts.svelte';
  import type { PlanLine, RequestFact } from './types';

  /**
   * Confirm: one line per detail, then the delivery line when it says
   * something the lines do not. Each line already says what it does, so only a
   * high-risk change is marked, and it is the one coloured element.
   */
  let {
    lines,
    facts = [],
    delivery = null,
  }: {
    lines: PlanLine[];
    facts?: RequestFact[];
    delivery?: { text: string; sends: boolean } | null;
  } = $props();

  const i18n = $derived($i18nStore);

  const icons: Record<RequestStepId, typeof LogInIcon> = {
    signIn: LogInIcon,
    newIdentity: BadgePlusIcon,
    updateChanges: UserCogIcon,
    networkFee: CoinsIcon,
    payInvoice: ReceiptIcon,
    signData: SignatureIcon,
    encryptionKey: KeyRoundIcon,
    shareData: Share2Icon,
    giftCard: GiftIcon,
    confirm: CircleCheckIcon,
  };
</script>

<div class="overflow-hidden rounded-xl border border-border/70">
  {#each lines as line (line.id)}
    {@const Icon = icons[line.icon]}
    <div class="flex items-center gap-3 border-b border-border/70 px-3.5 py-3 last:border-b-0">
      <span class="grid size-8 shrink-0 place-items-center rounded-lg bg-muted text-foreground/80">
        <Icon class="size-4" />
      </span>
      <span class="min-w-0 flex-1">
        <span class="block text-sm font-medium text-foreground">{line.text}</span>
        {#if line.detail}<span class="block text-xs text-muted-foreground">{line.detail}</span>{/if}
      </span>
      {#if line.highRisk}
        <span
          class="inline-flex h-5 shrink-0 items-center gap-1 rounded-full bg-amber-50 px-2 text-[11px] leading-none font-medium whitespace-nowrap text-amber-800 ring-1 ring-amber-300/70 dark:bg-amber-950/30 dark:text-amber-200 dark:ring-amber-500/30"
        >
          <ShieldAlertIcon class="size-3" aria-hidden="true" />
          {i18n.t('genericRequest.flow.highRisk')}
        </span>
      {/if}
    </div>
  {/each}
</div>

{#if facts.length > 0}
  <RequestFacts rows={facts} />
{/if}

{#if delivery}
  <DeliveryLine text={delivery.text} sends={delivery.sends} />
{/if}
