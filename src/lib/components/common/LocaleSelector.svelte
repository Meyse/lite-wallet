<!--
  Component: LocaleSelector
  Purpose: Shared locale picker used by onboarding and settings surfaces.
-->

<script lang="ts">
  import DropdownSelectTrigger from '$lib/components/common/DropdownSelectTrigger.svelte';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { i18nStore, setLocale } from '$lib/i18n';
  import type { Locale } from '$lib/i18n';
  import { buildLocaleOptions } from '$lib/utils/localeOptions.js';

  type LocaleSelectorProps = {
    triggerId?: string;
    triggerAriaLabel: string;
    size?: 'compact' | 'regular';
    buttonClass?: string;
    contentClass?: string;
  };

  const {
    triggerId = 'locale-selector-trigger',
    triggerAriaLabel,
    size = 'regular',
    buttonClass = '',
    contentClass = '',
  }: LocaleSelectorProps = $props();

  const i18n = $derived($i18nStore);
  const localeOptions = $derived(buildLocaleOptions(i18n.t));
  const selectedOption = $derived(
    localeOptions.find((option) => option.value === i18n.locale) ?? localeOptions[0]
  );

  function chooseLocale(locale: Locale): void {
    setLocale(locale);
  }
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger id={triggerId} aria-label={triggerAriaLabel}>
    {#snippet child({ props })}
      <DropdownSelectTrigger {...props} value={selectedOption.label} {size} class={buttonClass} />
    {/snippet}
  </DropdownMenu.Trigger>

  <DropdownMenu.Content
    align={size === 'compact' ? 'end' : 'start'}
    class={`${size === 'compact' ? 'w-[168px]' : 'w-[var(--bits-dropdown-menu-anchor-width)]'} overflow-y-auto ${contentClass}`}
    style="max-height: min(16rem, var(--bits-dropdown-menu-content-available-height));"
  >
    <DropdownMenu.RadioGroup value={i18n.locale}>
      {#each localeOptions as option (option.value)}
        <DropdownMenu.RadioItem
          value={option.value}
          class={size === 'regular' ? 'h-9 text-sm leading-5' : ''}
          onclick={() => chooseLocale(option.value)}
        >
          <span class="min-w-0 truncate">{option.label}</span>
        </DropdownMenu.RadioItem>
      {/each}
    </DropdownMenu.RadioGroup>
  </DropdownMenu.Content>
</DropdownMenu.Root>
