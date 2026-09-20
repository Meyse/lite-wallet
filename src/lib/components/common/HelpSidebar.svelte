<!--
  Component: HelpSidebar
  Purpose: Shows topic-based help in a right sheet with list and detail views.
  Security: Display-only help content, no sensitive data.
-->

<script lang="ts">
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import * as Accordion from '$lib/components/ui/accordion';
  import { Separator } from '$lib/components/ui/separator';
  import CommunityHangoutButton from '$lib/components/common/CommunityHangoutButton.svelte';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { i18nStore } from '$lib/i18n';

  type HelpQa = {
    id: string;
    question: string;
    answer: string;
  };

  type HelpTopic = {
    id: string;
    label: string;
    title: string;
    qas: Array<HelpQa>;
  };

  type HelpContent = {
    topics: Array<HelpTopic>;
  };

  let {
    isOpen = $bindable(false),
    title,
    content,
  }: {
    isOpen?: boolean;
    title: string;
    content: HelpContent;
  } = $props();

  const topics = $derived(content.topics);
  const i18n = $derived($i18nStore);

  let view = $state<'topics' | 'detail'>('topics');
  let activeTopicId = $state<string | null>(null);
  let activeAccordionItem = $state('');

  const activeTopic = $derived(
    activeTopicId ? (topics.find((topic) => topic.id === activeTopicId) ?? null) : null
  );

  $effect(() => {
    if (isOpen) return;
    view = 'topics';
    activeTopicId = null;
    activeAccordionItem = '';
  });

  $effect(() => {
    if (view !== 'detail' || !activeTopic || activeTopic.qas.length === 0) return;
    if (activeTopic.qas.some((qa) => qa.id === activeAccordionItem)) return;
    activeAccordionItem = activeTopic.qas[0].id;
  });

  function openTopic(topicId: string) {
    const topic = topics.find((item) => item.id === topicId);
    if (!topic) return;
    activeTopicId = topic.id;
    activeAccordionItem = topic.qas[0]?.id ?? '';
    view = 'detail';
  }

  function backToTopics() {
    view = 'topics';
    activeTopicId = null;
    activeAccordionItem = '';
  }
</script>

<StandardRightSheet
  bind:isOpen
  {title}
  hideTitle={view === 'detail'}
  bodyClass={view === 'detail' ? 'mt-2' : ''}
  backLabel={view === 'detail' ? i18n.t('help.backToTopics') : undefined}
  onBack={view === 'detail' ? backToTopics : undefined}
>
  {#if view === 'topics'}
    <div class="flex h-full min-h-0 flex-col">
      <div class="flex-1 overflow-y-auto pr-1">
        <div class="divide-y divide-border/70 overflow-hidden rounded-md border border-border/70">
          {#each topics as topic (topic.id)}
            <button
              type="button"
              class="group flex w-full items-center justify-between gap-3 px-3 py-3 text-left text-sm font-medium text-foreground transition-colors outline-none hover:bg-muted/45 focus-visible:ring-[2px] focus-visible:ring-ring/50"
              onclick={() => openTopic(topic.id)}
            >
              <span>{topic.label}</span>
              <ChevronRightIcon
                class="size-4 shrink-0 text-muted-foreground transition-transform duration-150 group-hover:translate-x-0.5"
                aria-hidden="true"
              />
            </button>
          {/each}
        </div>
      </div>

      <div class="mt-6 space-y-4">
        <Separator />
        <CommunityHangoutButton label={i18n.t('help.communityHangout')} />
      </div>
    </div>
  {:else if activeTopic}
    <div class="flex h-full min-h-0 flex-col">
      <Accordion.Root
        type="single"
        class="mt-1 flex-1 overflow-y-auto pr-1 text-left"
        value={activeAccordionItem}
        onValueChange={(value) => {
          activeAccordionItem = value || '';
        }}
      >
        {#each activeTopic.qas as qa (qa.id)}
          <Accordion.Item value={qa.id}>
            <Accordion.Trigger>{qa.question}</Accordion.Trigger>
            <Accordion.Content>
              <p>{qa.answer}</p>
            </Accordion.Content>
          </Accordion.Item>
        {/each}
      </Accordion.Root>
    </div>
  {/if}
</StandardRightSheet>
