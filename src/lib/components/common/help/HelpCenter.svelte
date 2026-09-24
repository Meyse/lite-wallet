<script lang="ts">
  import { onMount, tick } from 'svelte';
  import SearchIcon from '@lucide/svelte/icons/search';
  import XIcon from '@lucide/svelte/icons/x';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
  import CompassIcon from '@lucide/svelte/icons/compass';
  import ArrowUpDownIcon from '@lucide/svelte/icons/arrow-up-down';
  import RepeatIcon from '@lucide/svelte/icons/repeat';
  import VerusIdAtIcon from '$lib/components/icons/VerusIdAtIcon.svelte';
  import ShieldIcon from '@lucide/svelte/icons/shield';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { Input } from '$lib/components/ui/input';
  import NavigationBackButton from '$lib/components/common/NavigationBackButton.svelte';
  import { i18nStore } from '$lib/i18n';
  import { openCommunityHangout, openTrustedExternalUrl } from '$lib/utils/externalLinks';
  import {
    buildHelpArticles,
    type HelpArticleId,
    helpCategories,
    type HelpCategory,
    searchHelpArticles,
  } from '$lib/help/catalog';

  import { type HelpDestination, parseHelpText } from '$lib/help/links';
  import { createHelpReaderState, type HelpReaderState } from '$lib/help/readerState';

  let {
    onClose,
    backLabel,
    initialArticleId = null,
    readerState = $bindable(createHelpReaderState(initialArticleId)),
    onNavigate,
  }: {
    onClose: () => void;
    backLabel: string;
    initialArticleId?: HelpArticleId | null;
    readerState?: HelpReaderState;
    onNavigate?: (destination: HelpDestination) => void;
  } = $props();
  const i18n = $derived($i18nStore);
  const articles = $derived(buildHelpArticles(i18n.t));
  let unlockNotice = $state(false);
  let heading = $state<HTMLHeadingElement | null>(null);
  let homeHeading = $state<HTMLHeadingElement | null>(null);
  let viewport = $state<HTMLDivElement | null>(null);
  let searchInput = $state<HTMLInputElement | null>(null);
  const article = $derived(articles.find((item) => item.id === readerState.articleId));
  const searching = $derived(readerState.query.trim().length > 0);
  const showSearch = $derived(!article && !readerState.category);
  const categoryIcons = {
    basics: CompassIcon,
    payments: ArrowUpDownIcon,
    conversions: RepeatIcon,
    identity: VerusIdAtIcon,
    security: ShieldIcon,
    data: SettingsIcon,
  };
  const visibleArticles = $derived(
    searching
      ? searchHelpArticles(articles, readerState.query)
      : readerState.category
        ? articles.filter((item) => item.category === readerState.category)
        : []
  );
  const listTitle = $derived(
    searching
      ? i18n.t('helpCenter.results')
      : readerState.category
        ? i18n.t(`helpCenter.category.${readerState.category}`)
        : ''
  );
  const previous = $derived(readerState.history.at(-1));
  const backCategory = $derived(previous?.category ?? article?.category);
  const articleBackLabel = $derived(
    previous?.articleId
      ? i18n.t('helpCenter.backArticle')
      : previous?.query.trim()
        ? i18n.t('helpCenter.backResults')
        : backCategory
          ? i18n.t(`helpCenter.backCategory.${backCategory}`)
          : i18n.t('helpCenter.backHelp')
  );

  async function focusContent(scroll = 0): Promise<void> {
    await tick();
    readerState.focusDestination = null;
    unlockNotice = false;
    (showSearch && !searching ? homeHeading : heading)?.focus({ preventScroll: true });
    readerState.scroll = scroll;
    if (viewport) viewport.scrollTop = scroll;
  }

  function openArticle(id: HelpArticleId): void {
    readerState.history = [
      ...readerState.history,
      {
        articleId: readerState.articleId,
        category: readerState.category,
        query: readerState.query,
        scroll: viewport?.scrollTop ?? 0,
      },
    ];
    readerState.articleId = id;
    void focusContent();
  }

  function back(): void {
    const visit = readerState.history.at(-1);
    // Contextual links may open an article directly, without a previous visit.
    const category = visit ? visit.category : (article?.category ?? null);
    readerState.history = readerState.history.slice(0, -1);
    readerState.articleId = visit?.articleId ?? null;
    readerState.category = category;
    readerState.query = visit?.query ?? '';
    void focusContent(visit?.scroll ?? 0);
  }

  function selectCategory(value: HelpCategory | null): void {
    readerState.articleId = null;
    readerState.category = value;
    readerState.query = '';
    readerState.history = [];
    void focusContent();
  }

  function search(): void {
    readerState.articleId = null;
    readerState.category = null;
    readerState.history = [];
    readerState.scroll = 0;
    readerState.focusDestination = null;
    unlockNotice = false;
    if (viewport) viewport.scrollTop = 0;
  }
  onMount(() => {
    if (viewport) viewport.scrollTop = readerState.scroll;
  });

  function openScreen(destination: HelpDestination): void {
    readerState.scroll = viewport?.scrollTop ?? 0;
    readerState.focusDestination = destination;
    if (!onNavigate) {
      unlockNotice = true;
      return;
    }
    onNavigate(destination);
  }
</script>

<div class="flex h-full min-h-0 w-full flex-col bg-app-canvas text-foreground" data-help-center>
  <header class="flex shrink-0 items-center justify-between gap-5 px-5 pt-11 pb-3">
    <NavigationBackButton label={backLabel} onclick={onClose} />
    <button
      type="button"
      class="inline-flex h-8 items-center gap-1.5 rounded-md text-[13px] text-muted-foreground outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
      title={i18n.t('helpCenter.communityHint')}
      onclick={() => void openCommunityHangout()}
    >
      <span>{i18n.t('helpCenter.community')}</span><ExternalLinkIcon
        class="size-3.5 shrink-0"
        aria-hidden="true"
      />
    </button>
  </header>

  <section
    class="flex min-h-0 min-w-0 flex-1 flex-col px-5 pt-4"
    aria-label={i18n.t('helpCenter.title')}
  >
    {#if showSearch}
      <div class="mx-auto w-full max-w-[680px] shrink-0 px-1 pb-8">
        <h1
          bind:this={homeHeading}
          tabindex="-1"
          class="mb-6 text-center text-2xl leading-8 font-semibold tracking-[-0.02em] outline-none"
          data-help-heading
        >
          {i18n.t('helpCenter.homeTitle')}
        </h1>
        <div class="relative">
          <SearchIcon
            class="pointer-events-none absolute top-3.5 left-4 z-10 size-5 text-muted-foreground"
            aria-hidden="true"
          />
          <Input
            bind:ref={searchInput}
            bind:value={readerState.query}
            oninput={search}
            type="search"
            variant="lg"
            aria-label={i18n.t('helpCenter.search')}
            placeholder={i18n.t('helpCenter.searchPlaceholder')}
            autocomplete="off"
            spellcheck="false"
            class="pr-12 pl-12 text-base font-normal [&::-webkit-search-cancel-button]:appearance-none"
            data-help-search
          />
          {#if readerState.query}
            <button
              type="button"
              class="absolute top-2 right-2 flex size-8 items-center justify-center rounded-md text-muted-foreground outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
              aria-label={i18n.t('common.clearSearch')}
              onclick={() => {
                readerState.query = '';
                search();
                searchInput?.focus();
              }}><XIcon class="size-4" aria-hidden="true" /></button
            >
          {/if}
        </div>
      </div>
    {/if}

    <ScrollArea.Root class="-mr-3 min-h-0 flex-1 pr-3">
      <ScrollArea.Viewport
        bind:ref={viewport}
        onscroll={() => {
          if (viewport) readerState.scroll = viewport.scrollTop;
        }}
      >
        <div class="mx-auto w-full max-w-[680px] px-1 pb-8">
          {#if article}
            <NavigationBackButton label={articleBackLabel} onclick={back} class="mb-4" />
            <article class="select-text" data-help-article={article.id}>
              <h2
                bind:this={heading}
                tabindex="-1"
                class="text-2xl leading-8 font-semibold tracking-[-0.02em] outline-none"
                data-help-heading
              >
                {article.title}
              </h2>
              <div class="mt-6 space-y-4 text-sm leading-7 text-muted-foreground">
                {#each [article.summary, ...article.paragraphs] as paragraph}
                  <p>
                    {#each parseHelpText(paragraph) as part}{#if part.destination}<button
                          type="button"
                          class="inline cursor-default rounded-sm text-foreground underline decoration-muted-foreground/50 underline-offset-4 outline-none hover:decoration-foreground focus-visible:ring-2 focus-visible:ring-ring"
                          data-help-destination={part.destination}
                          onclick={() => part.destination && openScreen(part.destination)}
                          >{part.text}</button
                        >{:else}{part.text}{/if}{/each}
                  </p>
                  {#if unlockNotice && parseHelpText(paragraph).some((part) => part.destination === readerState.focusDestination)}
                    <p role="status" class="text-sm leading-6 text-foreground">
                      {i18n.t('helpCenter.unlockRequired')}
                    </p>
                  {/if}
                {/each}
              </div>
              {#if article.source}
                {@const source = article.source}
                <button
                  type="button"
                  class="mt-6 inline-flex items-center gap-1.5 rounded-sm text-[13px] text-muted-foreground outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
                  onclick={() => void openTrustedExternalUrl(source.url)}
                  >{source.label}<ExternalLinkIcon class="size-3.5" aria-hidden="true" /></button
                >
              {/if}
            </article>
            <section
              class="mt-8 border-t border-border/60 pt-5"
              aria-label={i18n.t('helpCenter.related')}
            >
              <h3 class="mb-2 text-[13px] font-medium text-muted-foreground">
                {i18n.t('helpCenter.related')}
              </h3>
              {#each article.related as id}
                {@const related = articles.find((item) => item.id === id)}
                {#if related}
                  <button
                    type="button"
                    class="row-hover-fade help-related group/help-row"
                    onclick={() => openArticle(id)}
                  >
                    <span>{related.title}</span><ChevronRightIcon
                      class="size-4 shrink-0 text-muted-foreground/70 transition-colors group-hover/help-row:text-foreground dark:group-hover/help-row:text-white"
                      aria-hidden="true"
                    />
                  </button>
                {/if}
              {/each}
            </section>
          {:else if readerState.category || searching}
            {#if readerState.category}
              <NavigationBackButton
                label={i18n.t('helpCenter.backHelp')}
                onclick={() => selectCategory(null)}
                class="mb-4"
              />
            {/if}
            <h2
              bind:this={heading}
              tabindex="-1"
              class="mb-5 text-2xl leading-8 font-semibold tracking-[-0.02em] outline-none"
              data-help-heading
            >
              {listTitle}
            </h2>
            <p class="sr-only" aria-live="polite" aria-atomic="true">
              {searching
                ? visibleArticles.length === 1
                  ? i18n.t('helpCenter.resultOne')
                  : i18n.t('helpCenter.resultCount', { count: visibleArticles.length })
                : ''}
            </p>
            {#if visibleArticles.length}
              <div class="divide-y divide-border/60" data-help-results>
                {#each visibleArticles as item (item.id)}
                  <button
                    type="button"
                    class="row-hover-fade group/help-row flex w-full items-center justify-between gap-5 rounded-md px-3 py-4 text-left focus-visible:ring-2 focus-visible:ring-ring/55 focus-visible:outline-none focus-visible:ring-inset"
                    onclick={() => openArticle(item.id)}
                  >
                    <span class="min-w-0 text-sm leading-5 font-medium">{item.title}</span>
                    <ChevronRightIcon
                      class="size-[18px] shrink-0 text-muted-foreground/70 transition-colors group-hover/help-row:text-foreground dark:group-hover/help-row:text-white"
                      aria-hidden="true"
                    />
                  </button>
                {/each}
              </div>
            {:else}
              <div class="py-10" role="status">
                <p class="text-sm font-medium">{i18n.t('helpCenter.noResults')}</p>
                <p class="mt-2 text-sm text-muted-foreground">
                  {i18n.t('helpCenter.noResultsHint')}
                </p>
              </div>
            {/if}
          {:else}
            <nav
              aria-label={i18n.t('helpCenter.topics')}
              class="grid grid-cols-2 gap-3 sm:grid-cols-3"
              data-help-categories
            >
              {#each helpCategories as category}
                {@const Icon = categoryIcons[category]}
                <button
                  type="button"
                  class="flex min-h-36 flex-col items-center justify-center gap-4 rounded-xl border border-border/60 bg-muted/20 px-4 py-5 text-center transition-colors outline-none hover:bg-muted/50 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-app-canvas active:bg-muted/65"
                  onclick={() => selectCategory(category)}
                >
                  <Icon class="size-7 text-muted-foreground" strokeWidth={1.5} aria-hidden="true" />
                  <span
                    class="flex min-h-10 items-center justify-center text-sm leading-5 font-medium"
                  >
                    {i18n.t(`helpCenter.category.${category}`)}
                  </span>
                </button>
              {/each}
            </nav>
          {/if}
        </div>
      </ScrollArea.Viewport>
      <ScrollArea.Scrollbar orientation="vertical"><ScrollArea.Thumb /></ScrollArea.Scrollbar>
    </ScrollArea.Root>
  </section>
</div>

<style>
  .help-related:focus-visible {
    box-shadow: inset 0 0 0 2px var(--ring);
  }
  .help-related {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
    padding: 10px 8px;
    border-radius: 6px;
    font-size: 13px;
    text-align: left;
    outline: none;
  }
</style>
