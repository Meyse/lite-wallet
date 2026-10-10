<script lang="ts">
  import CoinsIcon from '@lucide/svelte/icons/coins';
  import EyeIcon from '@lucide/svelte/icons/eye';
  import FingerprintIcon from '@lucide/svelte/icons/fingerprint';
  import ShieldAlertIcon from '@lucide/svelte/icons/shield-alert';
  import { i18nStore } from '$lib/i18n';
  import type { RequestImpact } from '$lib/genericRequest/flowPlan';

  let { impact }: { impact: RequestImpact } = $props();

  const i18n = $derived($i18nStore);
  const icons = {
    signs: FingerprintIcon,
    shares: EyeIcon,
    spends: CoinsIcon,
    control: ShieldAlertIcon,
  };
  const Icon = $derived(icons[impact]);
</script>

<span
  class={`inline-flex h-5 shrink-0 items-center gap-1 rounded-full px-2 text-[11px] leading-none font-medium whitespace-nowrap ${
    impact === 'control'
      ? 'bg-amber-50 text-amber-800 ring-1 ring-amber-300/70 dark:bg-amber-950/30 dark:text-amber-200 dark:ring-amber-500/30'
      : 'bg-muted text-foreground/80'
  }`}
>
  <Icon class="size-3" />
  {i18n.t(`genericRequest.flow.impact.${impact}`)}
</span>
