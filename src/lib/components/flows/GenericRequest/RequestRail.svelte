<script lang="ts">
  import CheckIcon from '@lucide/svelte/icons/check';
  import ClockIcon from '@lucide/svelte/icons/clock';
  import { i18nStore } from '$lib/i18n';
  import NotSignedChip from './NotSignedChip.svelte';
  import RequesterAvatar from './RequesterAvatar.svelte';
  import RequesterName from './RequesterName.svelte';
  import type { RailStep, RequesterView } from './types';

  /**
   * Who asks, the steps, and the time left. The requester's avatar and name
   * belong to the same identity; the site it answers to sits below. A finished
   * step shows what was chosen in it, such as the VerusID that answers.
   */
  let {
    requester,
    steps,
    timeLeftLabel = '',
    urgent = false,
  }: {
    requester: RequesterView;
    steps: RailStep[];
    timeLeftLabel?: string;
    urgent?: boolean;
  } = $props();

  const i18n = $derived($i18nStore);
  const showSite = $derived(
    requester.signed &&
      !!requester.appLabel &&
      requester.appLabel.toLowerCase() !== requester.identityLabel.toLowerCase()
  );
</script>

<!-- A gift card has no requester: its steps and title already name the card. -->
{#if !requester.giftCard}
  <div class="flex items-center gap-2.5 px-1.5">
    <RequesterAvatar {requester} class="size-9 text-sm" tile="bg-background dark:bg-app-canvas" />
    <div class="flex min-w-0 flex-col gap-0.5">
      <p class="flex min-w-0 text-sm font-semibold text-foreground">
        <RequesterName {requester} />
      </p>
      {#if showSite}
        <p class="truncate text-[12.5px] text-muted-foreground">{requester.appLabel}</p>
      {/if}
      {#if !requester.signed}
        <NotSignedChip />
      {/if}
    </div>
  </div>
{/if}

<ol class="flex flex-col gap-0.5" aria-label={i18n.t('genericRequest.flow.stepsLabel')}>
  {#each steps as step, index (step.id)}
    <li
      class={`flex items-start gap-2.5 rounded-lg px-2 py-2 text-[13px] ${
        step.status === 'current'
          ? 'bg-sidebar-item-active font-semibold text-foreground'
          : step.status === 'done'
            ? 'text-foreground'
            : 'text-muted-foreground'
      }`}
      aria-current={step.status === 'current' ? 'step' : undefined}
    >
      <span
        class={`grid size-[18px] shrink-0 place-items-center rounded-full text-[10.5px] font-semibold ${
          step.status === 'done'
            ? 'bg-foreground text-background'
            : step.status === 'current'
              ? 'border-[1.5px] border-foreground'
              : 'border-[1.5px] border-current'
        }`}
        aria-hidden="true"
      >
        {#if step.status === 'done'}<CheckIcon class="size-3" strokeWidth={3} />{:else}{index +
            1}{/if}
      </span>
      <span class="flex min-w-0 flex-col">
        <span class="truncate leading-[18px]">{step.label}</span>
        {#if step.detail}
          <span class="truncate text-xs font-normal text-muted-foreground">{step.detail}</span>
        {/if}
      </span>
    </li>
  {/each}
</ol>

{#if timeLeftLabel}
  <p
    class={`mt-auto flex items-center gap-1.5 px-1.5 text-xs ${
      urgent ? 'text-amber-700 dark:text-amber-300' : 'text-muted-foreground'
    }`}
  >
    <ClockIcon class="size-3.5 shrink-0" aria-hidden="true" />
    {timeLeftLabel}
  </p>
{/if}
