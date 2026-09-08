<!--
  Component: CommunityHangoutButton
  Purpose: Reusable Discord-style community hangout button.
-->

<script lang="ts">
  import DiscordIcon from '$lib/components/icons/DiscordIcon.svelte';
  import { Button } from '$lib/components/ui/button';
  import { i18nStore } from '$lib/i18n';
  import { openCommunityHangout } from '$lib/utils/externalLinks.js';

  type CommunityHangoutButtonProps = {
    label?: string;
    class?: string;
  };

  let { label = '', class: className = '' }: CommunityHangoutButtonProps = $props();
  const i18n = $derived($i18nStore);
  const resolvedLabel = $derived(label || i18n.t('help.communityHangout'));
</script>

<Button
  variant="outline"
  size="lg"
  onclick={() => {
    void openCommunityHangout();
  }}
  class={`cursor-pointer bg-brand-discord/10 text-brand-discord-foreground hover:bg-brand-discord/18 hover:text-brand-discord-foreground-hover dark:bg-brand-discord/20 justify-center border-0 text-sm dark:hover:bg-brand-discord/30 ${className}`.trim()}
>
  <DiscordIcon />
  {resolvedLabel}
</Button>
