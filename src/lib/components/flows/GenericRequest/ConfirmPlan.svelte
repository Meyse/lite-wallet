<script lang="ts">
  import BadgePlusIcon from '@lucide/svelte/icons/badge-plus';
  import CoinsIcon from '@lucide/svelte/icons/coins';
  import LogInIcon from '@lucide/svelte/icons/log-in';
  import UserCogIcon from '@lucide/svelte/icons/user-cog';
  import CircleCheckIcon from '@lucide/svelte/icons/circle-check';
  import DeliveryLine from './DeliveryLine.svelte';
  import ImpactChip from './ImpactChip.svelte';
  import RequestFacts from './RequestFacts.svelte';
  import type { PlanLine, RequestFact } from './types';

  /** Confirm: one line per detail with its impact, then the delivery line. */
  let {
    lines,
    facts = [],
    delivery,
  }: {
    lines: PlanLine[];
    facts?: RequestFact[];
    delivery: { text: string; sends: boolean };
  } = $props();

  const icons = {
    signIn: LogInIcon,
    newIdentity: BadgePlusIcon,
    updateChanges: UserCogIcon,
    networkFee: CoinsIcon,
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
      <ImpactChip impact={line.impact} />
    </div>
  {/each}
</div>

{#if facts.length > 0}
  <RequestFacts rows={facts} />
{/if}

<DeliveryLine text={delivery.text} sends={delivery.sends} />
