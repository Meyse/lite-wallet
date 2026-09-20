<script lang="ts">
  import BookOpenIcon from '@lucide/svelte/icons/book-open';
  import SquarePenIcon from '@lucide/svelte/icons/square-pen';
  import NavigationBackButton from '$lib/components/common/NavigationBackButton.svelte';
  import * as Sheet from '$lib/components/ui/sheet';
  import { i18nStore } from '$lib/i18n';
  import type { ImportMethod } from './types';

  const defaultOnSelect = (method: ImportMethod) => {
    void method;
  };

  type ImportMethodListProps = {
    title?: string;
    showHeader?: boolean;
    onSelect?: typeof defaultOnSelect;
    onBack?: (() => void) | null;
  };

  let {
    title = '',
    showHeader = true,
    onSelect = defaultOnSelect,
    onBack = null,
  }: ImportMethodListProps = $props();

  const i18n = $derived($i18nStore);
  const resolvedTitle = $derived(title || i18n.t('unlock.importMethods.title'));
</script>

<div>
  {#if showHeader && onBack}
    <NavigationBackButton
      label={i18n.t('unlock.importMethods.back')}
      class="mb-3"
      onclick={() => onBack?.()}
    />
  {/if}

  {#if showHeader}
    <Sheet.Header class="gap-1 p-0 pt-4 pr-8">
      <Sheet.Title class="text-base">{resolvedTitle}</Sheet.Title>
    </Sheet.Header>
  {/if}

  <div class="{showHeader ? 'mt-5' : ''} space-y-3">
    <button
      type="button"
      class="group selection-card-button selection-card-button--neutral"
      onclick={() => onSelect('seed24')}
    >
      <div class="flex items-start gap-3">
        <BookOpenIcon
          class="selection-card-icon"
          absoluteStrokeWidth
          stroke-linecap="butt"
          aria-hidden="true"
        />
        <div class="min-w-0">
          <p class="text-sm font-semibold text-foreground">
            {i18n.t('unlock.importMethods.seed24Title')}
          </p>
          <p class="mt-1 text-xs text-muted-foreground">
            {i18n.t('unlock.importMethods.seed24Description')}
          </p>
        </div>
      </div>
    </button>

    <button
      type="button"
      class="group selection-card-button selection-card-button--neutral"
      onclick={() => onSelect('text')}
    >
      <div class="flex items-start gap-3">
        <SquarePenIcon
          class="selection-card-icon"
          absoluteStrokeWidth
          stroke-linecap="butt"
          aria-hidden="true"
        />
        <div class="min-w-0">
          <p class="text-sm font-semibold text-foreground">
            {i18n.t('unlock.importMethods.textTitle')}
          </p>
          <p class="mt-1 text-xs text-muted-foreground">
            {i18n.t('unlock.importMethods.textDescription')}
          </p>
        </div>
      </div>
    </button>
  </div>
</div>
