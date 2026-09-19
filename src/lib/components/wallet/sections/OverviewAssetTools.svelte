<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
  import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
  import SearchIcon from '@lucide/svelte/icons/search';
  import XIcon from '@lucide/svelte/icons/x';
  import SlidersHorizontalIcon from '@lucide/svelte/icons/sliders-horizontal';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import { i18nStore } from '$lib/i18n';
  import { normalizeWalletOverviewSort, WALLET_OVERVIEW_SORTS } from '$lib/utils/walletOverview.js';
  import type { WalletOverviewPreferences } from '$lib/stores/walletOverviewPreferences.js';

  let {
    query = $bindable(''),
    preferences,
    onPreferencesChange,
  }: {
    query: string;
    preferences: WalletOverviewPreferences;
    onPreferencesChange: (value: WalletOverviewPreferences) => void;
  } = $props();
  const i18n = $derived($i18nStore);
  const sortLabel = $derived(
    i18n.t(
      `wallet.overview.${preferences.reversed ? 'reverseSortTrigger' : 'sortTrigger'}.${preferences.sort}`
    )
  );
  const ascending = $derived(
    preferences.sort === 'name' ? !preferences.reversed : preferences.reversed
  );
  let searchElement = $state<HTMLInputElement | null>(null);

  export function focusSearch(): void {
    searchElement?.focus();
  }

  export function clearSearch(): void {
    query = '';
    focusSearch();
  }
</script>

<div class="mt-4 flex h-9 items-center gap-3" data-testid="overview-asset-tools">
  <div
    class={`flex h-[34px] min-w-0 shrink items-center gap-2 rounded-md bg-muted px-2.5 focus-within:ring-2 focus-within:ring-ring/60 focus-within:ring-inset ${preferences.withBalance ? 'w-64' : 'w-72'}`}
  >
    <SearchIcon class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
    <div class="min-w-0 flex-1">
      <Input
        bind:ref={searchElement}
        bind:value={query}
        type="search"
        aria-label={i18n.t('wallet.overview.searchAssets')}
        placeholder={i18n.t('wallet.overview.searchAssets')}
        autocomplete="off"
        spellcheck={false}
        class="h-[34px] rounded-none border-0 bg-transparent p-0 text-[13px] leading-4 placeholder:text-muted-foreground focus-visible:ring-0 md:text-[13px] dark:bg-transparent dark:placeholder:text-muted-foreground [&::-webkit-search-cancel-button]:appearance-none"
        onkeydown={(event) => {
          if (event.key === 'Escape' && query) {
            event.preventDefault();
            clearSearch();
          }
        }}
      />
    </div>
    {#if query}
      <button
        type="button"
        class="flex size-5 shrink-0 items-center justify-center rounded-sm text-muted-foreground hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
        aria-label={i18n.t('wallet.overview.clearSearch')}
        onclick={clearSearch}
      >
        <XIcon class="size-3" aria-hidden="true" />
      </button>
    {/if}
  </div>
  {#if preferences.withBalance}
    <button
      type="button"
      class="flex h-7 shrink-0 items-center gap-1.5 rounded-[5px] bg-settings-selection-surface px-2 text-xs leading-4 text-settings-focus-ring focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
      aria-label={i18n.t('wallet.overview.clearBalanceFilter')}
      onclick={() => {
        onPreferencesChange({ ...preferences, withBalance: false });
        focusSearch();
      }}
    >
      {i18n.t('wallet.overview.withBalance')}
      <XIcon class="size-3" aria-hidden="true" />
    </button>
  {/if}
  <div class="ml-auto flex shrink-0 items-center gap-1">
    {#if preferences.sort !== 'verus-first'}
      <Button
        variant="ghost"
        class="h-[34px] w-7 rounded-md px-0 has-[>svg]:px-0"
        aria-label={i18n.t('wallet.overview.reverseSort')}
        title={i18n.t('wallet.overview.reverseSort')}
        onclick={() => onPreferencesChange({ ...preferences, reversed: !preferences.reversed })}
      >
        {#if ascending}<ArrowUpIcon class="size-[15px]" aria-hidden="true" />{:else}<ArrowDownIcon
            class="size-[15px]"
            aria-hidden="true"
          />{/if}
      </Button>
    {/if}
    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        {#snippet child({ props })}
          <Button
            {...props}
            variant="ghost"
            class="h-[34px] gap-1.5 rounded-md px-2 text-[13px] leading-4 font-normal has-[>svg]:px-2"
            aria-label={i18n.t('wallet.overview.sortAndFilter', {
              sort: sortLabel,
            })}
          >
            <SlidersHorizontalIcon class="size-[15px]" aria-hidden="true" />
            <span>{sortLabel}</span>
            <ChevronDownIcon class="size-3.5" aria-hidden="true" />
          </Button>
        {/snippet}
      </DropdownMenu.Trigger>
      <DropdownMenu.Content
        align="end"
        sideOffset={7}
        class="overview-sort-menu w-[264px] rounded-[10px] border border-input p-1.5"
      >
        <DropdownMenu.RadioGroup
          value={preferences.sort}
          onValueChange={(sort) =>
            onPreferencesChange({
              ...preferences,
              sort: normalizeWalletOverviewSort(sort),
              reversed: false,
            })}
        >
          <DropdownMenu.GroupHeading
            class="flex h-7 items-center px-2.5 py-0 text-xs leading-4 font-medium text-muted-foreground"
            >{i18n.t('wallet.overview.sortBy')}</DropdownMenu.GroupHeading
          >
          {#each WALLET_OVERVIEW_SORTS as sort}
            <DropdownMenu.RadioItem value={sort} class="h-[34px] rounded-[5px] px-2.5 leading-4">
              <span class="flex-1"
                >{i18n.t(
                  `wallet.overview.${preferences.reversed && preferences.sort === sort ? 'reverseSort' : 'sort'}.${sort}`
                )}</span
              >
            </DropdownMenu.RadioItem>
          {/each}
        </DropdownMenu.RadioGroup>
        <DropdownMenu.CheckboxItem
          checked={preferences.withBalance}
          onCheckedChange={(withBalance) => onPreferencesChange({ ...preferences, withBalance })}
          closeOnSelect={false}
          class="h-12 rounded-b-[5px] border-t border-input py-0 ps-2.5 pe-2.5 text-[13px] leading-4 [&>span:first-child]:hidden"
        >
          <span class="flex-1">{i18n.t('wallet.overview.withBalance')}</span>
          <span
            aria-hidden="true"
            class={`flex h-[18px] w-[30px] shrink-0 rounded-[10px] p-0.5 ${preferences.withBalance ? 'bg-primary' : 'bg-muted-foreground/30 dark:bg-muted-foreground/55'}`}
          >
            <span
              class={`size-3.5 rounded-full bg-white transition-transform ${preferences.withBalance ? 'translate-x-3' : ''}`}
            ></span>
          </span>
        </DropdownMenu.CheckboxItem>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  </div>
</div>

<style>
  :global(.overview-sort-menu) {
    box-shadow: 0 8px 28px var(--settings-menu-shadow);
  }
</style>
