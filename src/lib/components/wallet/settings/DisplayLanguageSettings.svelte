<!--
  Component: DisplayLanguageSettings
  Purpose: Focused settings detail page for appearance, display currency, and app language.
-->

<script lang="ts">
  import CheckIcon from '@lucide/svelte/icons/check';
  import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import AppearanceSelector from '$lib/components/wallet/settings/AppearanceSelector.svelte';
  import LocaleSelector from '$lib/components/common/LocaleSelector.svelte';
  import SearchInput from '$lib/components/common/SearchInput.svelte';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { i18nStore } from '$lib/i18n';
  import {
    setAppearancePreference,
    setDisplayCurrency,
    settingsStore,
  } from '$lib/stores/settings.js';
  import type { AppearancePreference } from '$lib/utils/appearance.js';
  import {
    filterFiatCurrencyOptions,
    getFiatCurrencyOptions,
    QUICK_PICK_DISPLAY_CURRENCIES,
  } from '$lib/utils/fiatDisplay.js';

  type DisplayLanguageSettingsProps = {
    onBack: () => void;
  };

  const { onBack }: DisplayLanguageSettingsProps = $props();

  const i18n = $derived($i18nStore);
  const settings = $derived($settingsStore);
  const displayCurrency = $derived(settings.displayCurrency);
  const appearance = $derived(settings.theme);
  const fiatOptions = getFiatCurrencyOptions();
  const quickPickOptions = QUICK_PICK_DISPLAY_CURRENCIES.map(
    (code) => fiatOptions.find((option) => option.code === code) ?? null
  ).filter((option): option is NonNullable<(typeof fiatOptions)[number]> => Boolean(option));
  const selectedFiatOption = $derived(
    fiatOptions.find((option) => option.code === displayCurrency) ?? fiatOptions[0]
  );
  const appearanceOptions = $derived(
    (['light', 'dark', 'system'] as const).map((value) => ({
      value,
      label: i18n.t(`wallet.settings.display.appearance.${value}`),
    }))
  );

  let showAllCurrenciesSheet = $state(false);
  let currencySearchTerm = $state('');

  const filteredFiatOptions = $derived(filterFiatCurrencyOptions(fiatOptions, currencySearchTerm));
  const hasCurrencySearch = $derived(currencySearchTerm.trim().length > 0);

  function chooseAppearance(value: AppearancePreference): void {
    setAppearancePreference(value);
  }

  function chooseDisplayCurrency(code: string): void {
    setDisplayCurrency(code);
    showAllCurrenciesSheet = false;
  }

  function openAllCurrenciesSheet(): void {
    currencySearchTerm = '';
    showAllCurrenciesSheet = true;
  }
</script>

<div
  class="mx-auto flex h-full min-h-0 w-full max-w-5xl flex-col bg-app-canvas px-6 pt-0 pb-6 sm:px-8"
>
  <section class="flex min-h-0 flex-1 flex-col overflow-auto pt-2">
    <header class="flex h-[60px] shrink-0 flex-col gap-3">
      <button
        type="button"
        class="inline-flex h-5 w-fit items-center gap-1 text-[13px] leading-5 text-settings-muted-foreground outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
        onclick={onBack}
      >
        <ChevronLeftIcon class="size-4" />
        {i18n.t('wallet.settings.backLabel')}
      </button>
      <h2 class="text-xl leading-7 font-semibold tracking-[-0.015em]">
        {i18n.t('wallet.settings.display.title')}
      </h2>
    </header>

    <div class="mt-5 flex h-14 shrink-0 items-center rounded-lg bg-settings-surface px-4">
      <p class="min-w-0 flex-1 text-[13px] leading-[18px] font-medium">
        {i18n.t('wallet.settings.display.appearance.label')}
      </p>
      <AppearanceSelector
        value={appearance}
        label={i18n.t('wallet.settings.display.appearance.label')}
        systemDescription={i18n.t('wallet.settings.display.appearance.systemDescription')}
        options={appearanceOptions}
        onChange={chooseAppearance}
      />
    </div>

    <div class="mt-3 overflow-visible rounded-lg bg-settings-surface">
      <button
        type="button"
        class="flex h-12 w-full items-center gap-4 rounded-t-lg px-4 text-left outline-none hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:ring-inset"
        onclick={openAllCurrenciesSheet}
      >
        <span class="min-w-0 flex-1 text-[13px] leading-4 font-medium">
          {i18n.t('wallet.settings.display.currency.label')}
        </span>
        <span class="text-[13px] leading-4 text-settings-muted-foreground">
          {selectedFiatOption?.code ?? displayCurrency} · {selectedFiatOption?.name ??
            displayCurrency}
        </span>
        <ChevronRightIcon class="size-4 shrink-0 text-settings-muted-foreground" />
      </button>

      <div class="flex h-12 items-center gap-4 px-4">
        <p class="min-w-0 flex-1 text-[13px] leading-4 font-medium">
          {i18n.t('wallet.settings.display.language.label')}
        </p>
        <LocaleSelector
          size="compact"
          triggerAriaLabel={i18n.t('wallet.settings.display.language.label')}
        />
      </div>
    </div>
  </section>
</div>

<StandardRightSheet
  bind:isOpen={showAllCurrenciesSheet}
  title={i18n.t('wallet.settings.display.currency.sheetTitle')}
  closeLabel={i18n.t('common.close')}
>
  <div class="flex h-full min-h-0 flex-col">
    <SearchInput
      bind:value={currencySearchTerm}
      placeholder={i18n.t('wallet.settings.display.currency.searchPlaceholder')}
      clearLabel={i18n.t('common.clearSearch')}
      showFocusRing
      inputClass="bg-settings-surface dark:bg-settings-control-surface h-9 rounded-md text-[13px] leading-[18px]"
    />

    {#if !hasCurrencySearch}
      <div class="mt-4 flex h-7 shrink-0 gap-2">
        {#each quickPickOptions as option (option.code)}
          {@const selected = displayCurrency === option.code}
          <button
            type="button"
            class={`flex h-7 min-w-0 flex-1 items-center justify-center gap-1.5 rounded-[5px] text-[13px] leading-4 outline-none focus-visible:ring-2 focus-visible:ring-settings-focus-ring ${
              selected
                ? 'bg-settings-selection-surface'
                : 'bg-settings-surface dark:bg-settings-control-surface'
            }`}
            onclick={() => chooseDisplayCurrency(option.code)}
          >
            {option.code}
            {#if selected}<CheckIcon
                class="size-3.5 text-primary dark:text-settings-focus-ring"
              />{/if}
          </button>
        {/each}
      </div>
    {/if}

    <p class="mt-6 shrink-0 text-xs leading-[18px] text-settings-muted-foreground">
      {hasCurrencySearch
        ? i18n.t(
            filteredFiatOptions.length === 1
              ? 'wallet.settings.display.currency.resultCount.one'
              : 'wallet.settings.display.currency.resultCount.other',
            {
              count: filteredFiatOptions.length,
            }
          )
        : i18n.t('wallet.settings.display.currency.allCurrencies')}
    </p>

    {#if filteredFiatOptions.length === 0}
      <p class="mt-2 text-[13px] leading-[18px] text-settings-muted-foreground">
        {i18n.t('wallet.settings.display.currency.noResults')}
      </p>
    {:else}
      <ScrollArea.Root class="mt-2 min-h-0 flex-1">
        <ScrollArea.Viewport class="h-full pr-2">
          <div class="pb-1">
            {#each filteredFiatOptions as option (option.code)}
              <button
                type="button"
                class="flex h-10 w-full items-center gap-3 rounded-sm px-2 text-left outline-none hover:bg-settings-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:ring-inset dark:hover:bg-settings-control-surface"
                onclick={() => chooseDisplayCurrency(option.code)}
              >
                <span class="w-10 shrink-0 text-[13px] leading-[18px] font-medium">
                  {option.code}
                </span>
                <span
                  class="min-w-0 flex-1 truncate text-[13px] leading-[18px] text-settings-muted-foreground"
                >
                  {option.name}
                </span>
                <span class="flex w-4 shrink-0 items-center justify-end">
                  {#if displayCurrency === option.code}
                    <CheckIcon class="size-4 text-primary dark:text-settings-focus-ring" />
                  {/if}
                </span>
              </button>
            {/each}
          </div>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar
          orientation="vertical"
          class="w-[3px] border-0 p-0 [&_[data-slot=scroll-area-thumb]]:bg-settings-scrollbar-thumb"
        />
      </ScrollArea.Root>
    {/if}
  </div>
</StandardRightSheet>
