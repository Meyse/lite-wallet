<!--
  Component: AboutSupportSettings
  Purpose: Focused settings detail page for app identity and community support.
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import CommunityHangoutButton from '$lib/components/common/CommunityHangoutButton.svelte';
  import NavigationBackButton from '$lib/components/common/NavigationBackButton.svelte';
  import { i18nStore } from '$lib/i18n';
  import { loadRuntimeAppInfo, type RuntimeAppInfo } from '$lib/utils/appInfo.js';

  type AboutSupportSettingsProps = {
    onBack: () => void;
  };

  const { onBack }: AboutSupportSettingsProps = $props();
  const i18n = $derived($i18nStore);

  let appInfo = $state<RuntimeAppInfo | null>(null);
  let appInfoState = $state<'loading' | 'ready' | 'unavailable'>('loading');

  async function loadAppInfo(): Promise<void> {
    appInfoState = 'loading';
    try {
      appInfo = await loadRuntimeAppInfo();
      appInfoState = 'ready';
    } catch {
      appInfo = null;
      appInfoState = 'unavailable';
    }
  }

  onMount(() => {
    void loadAppInfo();
  });
</script>

<div
  class="mx-auto flex h-full min-h-0 w-full max-w-5xl flex-col bg-app-canvas px-6 pt-0 pb-6 sm:px-8"
>
  <section class="flex min-h-0 flex-1 flex-col overflow-auto pt-2">
    <header class="flex h-[72px] shrink-0 flex-col gap-3">
      <NavigationBackButton
        label={i18n.t('wallet.settings.backLabel')}
        tone="settings"
        onclick={onBack}
      />
      <h2 class="text-xl leading-7 font-semibold tracking-[-0.015em]">
        {i18n.t('wallet.settings.about.title')}
      </h2>
    </header>

    <div class="mt-5 flex h-[82px] shrink-0 items-center gap-4 rounded-lg bg-settings-surface px-4">
      <div class="flex size-10 shrink-0 items-center justify-center">
        <img
          src="/images/verus-express-icon.png"
          alt=""
          class="size-10 rounded-[10px]"
          aria-hidden="true"
        />
      </div>
      <div class="min-w-0">
        <p class="truncate text-base leading-5 font-semibold">
          {appInfoState === 'ready' && appInfo
            ? appInfo.name
            : i18n.t('wallet.settings.about.productName')}
        </p>
        <p class="mt-1 text-[13px] leading-5 text-settings-muted-foreground">
          {#if appInfoState === 'loading'}
            {i18n.t('common.loading')}
          {:else if appInfoState === 'ready' && appInfo}
            {i18n.t('wallet.settings.about.versionValue', { version: appInfo.version })}
          {:else}
            {i18n.t('wallet.settings.about.versionUnavailable')}
          {/if}
        </p>
      </div>
    </div>

    <CommunityHangoutButton
      class="mt-3"
      presentation="settings-row"
      label={i18n.t('wallet.settings.about.community')}
      description={i18n.t('wallet.settings.about.communityDescription')}
    />
  </section>
</div>
