<!-- 
  Root layout component for lite-wallet app
  Imports global styles, sets up theme detection, and renders page content
  Updated: Added mode-watcher for automatic light/dark theme detection
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrent, onOpenUrl } from '@tauri-apps/plugin-deep-link';
  import '../app.css';
  import '$lib/shims/node-globals';
  import { ModeWatcher } from 'mode-watcher';
  import { Toaster } from 'svelte-sonner';
  import { initI18n } from '$lib/i18n';
  import { hydrateQueuedGenericRequest, queueGenericRequest } from '$lib/stores/genericRequest.js';

  const { children } = $props();

  function findLatestVerusDeepLink(urls: string[] | null | undefined): string | null {
    if (!urls?.length) {
      return null;
    }

    for (let index = urls.length - 1; index >= 0; index -= 1) {
      const candidate = urls[index]?.trim();
      if (!candidate) continue;

      if (candidate.toLowerCase().startsWith('verus:')) {
        return candidate;
      }

      try {
        if (new URL(candidate).protocol === 'verus:') {
          return candidate;
        }
      } catch {
        continue;
      }
    }

    return null;
  }

  onMount(() => {
    initI18n();
    hydrateQueuedGenericRequest();

    let disposed = false;
    let lastQueuedDeepLink: string | null = null;
    let removeDeepLinkListener: (() => void) | null = null;

    const queueDeepLinks = (urls: string[] | null | undefined): void => {
      const nextDeepLink = findLatestVerusDeepLink(urls);
      if (!nextDeepLink || nextDeepLink === lastQueuedDeepLink) {
        return;
      }

      lastQueuedDeepLink = nextDeepLink;
      queueGenericRequest({
        input: nextDeepLink,
        passthroughAutoLinkFqn: null,
        source: 'deep-link'
      });
    };

    void (async () => {
      try {
        queueDeepLinks(await getCurrent());
        if (disposed) return;

        removeDeepLinkListener = await onOpenUrl((urls) => {
          queueDeepLinks(urls);
        });
      } catch (error) {
        console.error('[DEEP_LINK] Failed to initialize deep-link handling', error);
      }
    })();

    return () => {
      disposed = true;
      removeDeepLinkListener?.();
    };
  });
</script>

<!-- Theme detection and system preference monitoring -->
<ModeWatcher />
<Toaster theme="system" richColors position="bottom-right" />

<!-- Render page content -->
{@render children?.()}
