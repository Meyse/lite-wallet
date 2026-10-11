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
  import { mode, ModeWatcher } from 'mode-watcher';
  import { Toaster } from 'svelte-sonner';
  import { initI18n } from '$lib/i18n';
  import { queueGenericRequest } from '$lib/stores/genericRequest.js';
  import { settingsStore } from '$lib/stores/settings.js';

  const { children } = $props();
  const settings = $derived($settingsStore);

  /**
   * The wallet scheme of legacy links (`WALLET_VDXF_KEY` in lower case), which
   * Verus Mobile also registers. Its logins and invoices open as requests.
   */
  const LEGACY_WALLET_SCHEME = 'i5jtwbp6zymeay9llnraglgjqgdrffsau4:';

  function findLatestVerusDeepLink(urls: string[] | null | undefined): string | null {
    if (!urls?.length) {
      return null;
    }

    for (let index = urls.length - 1; index >= 0; index -= 1) {
      const candidate = urls[index]?.trim();
      if (!candidate) continue;

      const lower = candidate.toLowerCase();
      if (lower.startsWith('verus:') || lower.startsWith(LEGACY_WALLET_SCHEME)) {
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
        source: 'deep-link',
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
<ModeWatcher defaultMode={settings.theme} />
<Toaster theme={mode.current ?? 'system'} richColors position="bottom-right" />

<!-- Render page content -->
{@render children?.()}
