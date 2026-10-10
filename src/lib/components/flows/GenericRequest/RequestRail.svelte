<script lang="ts">
  import CheckIcon from '@lucide/svelte/icons/check';
  import ClockIcon from '@lucide/svelte/icons/clock';
  import ContactAvatar from '$lib/components/wallet/contacts/ContactAvatar.svelte';
  import { i18nStore } from '$lib/i18n';
  import type { ContactIdentity } from '$lib/types/addressBook';
  import RequesterTrust from './RequesterTrust.svelte';
  import type { RailStep, RequesterView } from './types';

  let {
    requester,
    responder = null,
    steps,
    timeLeftLabel = '',
    expired = false,
  }: {
    requester: RequesterView;
    responder?: { name: string; identity: ContactIdentity | null } | null;
    steps: RailStep[];
    timeLeftLabel?: string;
    expired?: boolean;
  } = $props();

  const i18n = $derived($i18nStore);
</script>

<div class="flex items-start gap-2.5">
  {#if requester.identity}
    <ContactAvatar
      identity={requester.identity}
      name={requester.identityLabel}
      class="size-10 shrink-0"
    />
  {:else}
    <span
      class="grid size-10 shrink-0 place-items-center rounded-[10px] bg-background text-base font-semibold text-foreground uppercase dark:bg-app-canvas"
      aria-hidden="true"
    >
      {requester.appLabel.charAt(0)}
    </span>
  {/if}
  <div class="min-w-0">
    <p class="truncate text-sm font-semibold text-foreground">{requester.appLabel}</p>
    <RequesterTrust {requester} compact />
  </div>
</div>

{#if responder}
  <div
    class="flex items-center gap-2 rounded-[9px] bg-background px-2.5 py-2 text-[13px] dark:bg-app-canvas"
  >
    <ContactAvatar identity={responder.identity} name={responder.name} class="size-[22px]" />
    <span class="min-w-0 truncate font-medium text-foreground">{responder.name}</span>
    <span class="shrink-0 text-muted-foreground">{i18n.t('genericRequest.flow.responds')}</span>
  </div>
{/if}

<ol class="flex flex-col gap-0.5" aria-label={i18n.t('genericRequest.flow.stepsLabel')}>
  {#each steps as step, index (step.id)}
    <li
      class={`flex h-[34px] items-center gap-2.5 rounded-lg px-2 text-[13px] ${
        step.status === 'current'
          ? 'bg-sidebar-item-active font-medium text-foreground'
          : step.status === 'done'
            ? 'text-foreground'
            : 'text-muted-foreground'
      }`}
      aria-current={step.status === 'current' ? 'step' : undefined}
    >
      <span
        class={`grid size-5 shrink-0 place-items-center rounded-full border-[1.5px] text-[11px] font-semibold ${
          step.status === 'done'
            ? 'border-primary bg-primary text-primary-foreground'
            : step.status === 'current'
              ? 'border-primary text-primary'
              : 'border-current'
        }`}
        aria-hidden="true"
      >
        {#if step.status === 'done'}<CheckIcon class="size-3" strokeWidth={3} />{:else}{index +
            1}{/if}
      </span>
      <span class="min-w-0 truncate">{step.label}</span>
    </li>
  {/each}
</ol>

{#if timeLeftLabel}
  <p
    class={`mt-auto flex items-center gap-1.5 text-xs ${
      expired ? 'text-amber-700 dark:text-amber-300' : 'text-muted-foreground'
    }`}
  >
    <ClockIcon class="size-3.5 shrink-0" />
    {timeLeftLabel}
  </p>
{/if}
