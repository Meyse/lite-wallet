<script lang="ts">
  import { onMount } from 'svelte';
  import { mode } from 'mode-watcher';
  import DiscordIcon from '$lib/components/icons/DiscordIcon.svelte';
  import { Button } from '$lib/components/ui/button';
  import { openCommunityHangout } from '$lib/utils/externalLinks.js';
  import { i18nStore } from '$lib/i18n';

  const i18n = $derived($i18nStore);

  type WalletHeroBackgroundProps = {
    suspended?: boolean;
  };

  // This prop updates while create/import flows cover the hero panel.
  // eslint-disable-next-line prefer-const
  let { suspended = false }: WalletHeroBackgroundProps = $props();
  let isDesktop = $state(false);
  let prefersReducedMotion = $state(false);
  let isDocumentVisible = $state(true);

  const isDark = $derived(mode.current === 'dark');
  const videoSrc = $derived(
    isDark ? '/images/wallet-mesh-dark.mp4' : '/images/wallet-mesh-light.mp4'
  );
  const posterSrc = $derived(
    isDark ? '/images/wallet-mesh-dark.png' : '/images/wallet-mesh-light.png'
  );
  const shouldRenderVideo = $derived(
    isDesktop && !prefersReducedMotion && isDocumentVisible && !suspended
  );

  onMount(() => {
    const desktopQuery = window.matchMedia('(min-width: 768px)');
    const reducedMotionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');

    const updateMediaState = () => {
      isDesktop = desktopQuery.matches;
      prefersReducedMotion = reducedMotionQuery.matches;
    };
    const updateVisibility = () => {
      isDocumentVisible = document.visibilityState === 'visible';
    };

    updateMediaState();
    updateVisibility();
    desktopQuery.addEventListener('change', updateMediaState);
    reducedMotionQuery.addEventListener('change', updateMediaState);
    document.addEventListener('visibilitychange', updateVisibility);

    return () => {
      desktopQuery.removeEventListener('change', updateMediaState);
      reducedMotionQuery.removeEventListener('change', updateMediaState);
      document.removeEventListener('visibilitychange', updateVisibility);
    };
  });
</script>

<div class="absolute inset-0">
  {#if shouldRenderVideo}
    {#key videoSrc}
      <video
        src={videoSrc}
        poster={posterSrc}
        autoplay
        muted
        loop
        playsinline
        aria-hidden="true"
        tabindex="-1"
        class="h-full w-full object-cover"
      ></video>
    {/key}
  {:else}
    <img src={posterSrc} alt="" class="h-full w-full object-cover" />
  {/if}

  <div class="absolute inset-0 flex flex-col items-start px-8 pt-24 pb-8 lg:px-12 lg:pb-12">
    <p class="text-foreground max-w-sm text-left text-3xl leading-tight font-medium tracking-tight">
      {i18n.t('wallet.hero.ownership')}
    </p>
    <Button
      variant="ghost"
      size="icon"
      class="cursor-pointer text-foreground/70 hover:text-foreground hover:bg-transparent dark:hover:bg-transparent mt-auto -ml-2 size-10"
      aria-label={i18n.t('wallet.hero.discord')}
      title={i18n.t('wallet.hero.discord')}
      onclick={() => { void openCommunityHangout(); }}
    >
      <DiscordIcon class="size-5" />
    </Button>
  </div>
</div>
