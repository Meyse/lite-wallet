<!--
  Component: HelpDrawerLink
  Purpose: Reusable help link + sidebar pair for contextual guidance.
  Security: Display-only helper content, no sensitive operations.
-->

<script lang="ts">
  import HelpLink from '$lib/components/common/HelpLink.svelte';
  import HelpSidebar from '$lib/components/common/HelpSidebar.svelte';

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
    linkText,
    title,
    content,
    class: className = ''
  }: {
    linkText: string;
    title: string;
    content: HelpContent;
    class?: string;
  } = $props();
   

  let isOpen = $state(false);
</script>

<div class={className}>
  <HelpLink
    text={linkText}
    onclick={() => {
      isOpen = true;
    }}
  />
</div>

<HelpSidebar bind:isOpen={isOpen} {title} {content} />
