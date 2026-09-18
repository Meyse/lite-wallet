<!--
  Component: AppearanceSelector
  Purpose: Compact light, dark, and system appearance radio control.
-->

<script lang="ts">
  import CheckIcon from '@lucide/svelte/icons/check';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import type { AppearancePreference } from '$lib/utils/appearance.js';
  import { cn } from '$lib/utils.js';

  type AppearanceOption = {
    value: AppearancePreference;
    label: string;
  };

  type AppearanceSelectorProps = {
    value: AppearancePreference;
    label: string;
    systemDescription: string;
    options: readonly AppearanceOption[];
    onChange: (value: AppearancePreference) => void;
  };

  const { value, label, systemDescription, options, onChange }: AppearanceSelectorProps = $props();
  let controls = $state<HTMLButtonElement[]>([]);

  function choose(option: AppearanceOption, index: number): void {
    onChange(option.value);
    controls[index]?.focus();
  }

  function handleKeydown(event: KeyboardEvent, index: number): void {
    let nextIndex: number | null = null;

    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
      nextIndex = (index + 1) % options.length;
    } else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
      nextIndex = (index - 1 + options.length) % options.length;
    } else if (event.key === 'Home') {
      nextIndex = 0;
    } else if (event.key === 'End') {
      nextIndex = options.length - 1;
    }

    if (nextIndex === null) return;
    event.preventDefault();
    const nextOption = options[nextIndex];
    if (nextOption) choose(nextOption, nextIndex);
  }

  function controlClass(selected: boolean): string {
    return cn(
      'inline-flex h-[26px] w-[78px] shrink-0 items-center justify-center gap-1 rounded-sm border-0 text-[13px] leading-4 outline-none focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-settings-focus-ring',
      selected ? 'bg-primary font-medium text-white' : 'text-foreground'
    );
  }
</script>

<Tooltip.Provider delayDuration={0}>
  <div
    class="flex h-[30px] w-[240px] shrink-0 items-center gap-px rounded-md bg-settings-segment-surface p-0.5"
    role="radiogroup"
    aria-label={label}
  >
    {#each options as option, index (option.value)}
      {@const selected = value === option.value}
      {#if option.value === 'system'}
        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              <button
                {...props}
                bind:this={controls[index]}
                type="button"
                role="radio"
                aria-checked={selected}
                tabindex={selected ? 0 : -1}
                class={controlClass(selected)}
                onclick={() => choose(option, index)}
                onkeydown={(event) => handleKeydown(event, index)}
              >
                {option.label}
                <span
                  class="flex size-[13px] shrink-0 items-center justify-center"
                  aria-hidden="true"
                >
                  {#if selected}<CheckIcon class="size-[13px]" />{/if}
                </span>
              </button>
            {/snippet}
          </Tooltip.Trigger>
          <Tooltip.Content
            side="bottom"
            align="end"
            sideOffset={6}
            class="settings-menu-popover w-[228px] rounded-md bg-settings-menu-surface px-3 py-2.5 text-[13px] leading-[18px] text-wrap text-foreground"
            arrowClasses="hidden"
          >
            {systemDescription}
          </Tooltip.Content>
        </Tooltip.Root>
      {:else}
        <button
          bind:this={controls[index]}
          type="button"
          role="radio"
          aria-checked={selected}
          tabindex={selected ? 0 : -1}
          class={controlClass(selected)}
          onclick={() => choose(option, index)}
          onkeydown={(event) => handleKeydown(event, index)}
        >
          {option.label}
          <span class="flex size-[13px] shrink-0 items-center justify-center" aria-hidden="true">
            {#if selected}<CheckIcon class="size-[13px]" />{/if}
          </span>
        </button>
      {/if}
    {/each}
  </div>
</Tooltip.Provider>
