<script lang="ts">
  import { onMount } from 'svelte';
  import { mode } from 'mode-watcher';

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

  <div class="absolute inset-0 flex flex-col items-center pt-24">
    <img
      src="/images/verus-logo-white.svg"
      alt="Verus"
      class="h-8 w-auto cursor-default select-none"
    />
  </div>
</div>
