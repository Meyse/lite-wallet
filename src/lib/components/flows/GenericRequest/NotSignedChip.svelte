<script lang="ts">
  import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
  import { i18nStore } from '$lib/i18n';

  /** Nothing proves who sent the request, so this is one of the few coloured states. */
  const i18n = $derived($i18nStore);
</script>

<span
  class="inline-flex w-fit shrink-0 items-center gap-1 rounded-md bg-amber-50 px-1.5 py-0.5 text-xs font-medium text-amber-800 ring-1 ring-amber-300/70 dark:bg-amber-950/30 dark:text-amber-200 dark:ring-amber-500/30"
>
  <TriangleAlertIcon class="size-3.5 shrink-0" aria-hidden="true" />
  {i18n.t('genericRequest.flow.notSigned')}
</span>
