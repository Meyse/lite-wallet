<!--
  Component: CommunityHangoutButton
  Purpose: Reusable Discord-style community hangout button.
-->

<script lang="ts">
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
  import DiscordIcon from '$lib/components/icons/DiscordIcon.svelte';
  import { Button } from '$lib/components/ui/button';
  import { i18nStore } from '$lib/i18n';
  import { openCommunityHangout } from '$lib/utils/externalLinks.js';

  type CommunityHangoutButtonProps = {
    label?: string;
    description?: string;
    presentation?: 'button' | 'settings-row';
    class?: string;
  };

  let {
    label = '',
    description = '',
    presentation = 'button',
    class: className = '',
  }: CommunityHangoutButtonProps = $props();
  const i18n = $derived($i18nStore);
  const resolvedLabel = $derived(label || i18n.t('help.communityHangout'));
</script>

{#if presentation === 'settings-row'}
  <button
    type="button"
    class={`flex min-h-16 w-full items-center gap-4 rounded-lg bg-settings-surface px-4 py-3 text-left outline-none hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring ${className}`.trim()}
    onclick={() => {
      void openCommunityHangout();
    }}
  >
    <span class="min-w-0 flex-1">
      <span class="block text-sm leading-5 font-medium">{resolvedLabel}</span>
      {#if description}
        <span class="mt-0.5 block text-xs leading-5 text-settings-muted-foreground">
          {description}
        </span>
      {/if}
    </span>
    <ExternalLinkIcon class="size-4 shrink-0 text-settings-muted-foreground" />
  </button>
{:else}
  <Button
    variant="outline"
    size="lg"
    onclick={() => {
      void openCommunityHangout();
    }}
    class={`cursor-default justify-center border-0 bg-brand-discord/10 text-sm text-brand-discord-foreground hover:bg-brand-discord/18 hover:text-brand-discord-foreground-hover dark:bg-brand-discord/20 dark:hover:bg-brand-discord/30 ${className}`.trim()}
  >
    <DiscordIcon />
    {resolvedLabel}
  </Button>
{/if}
