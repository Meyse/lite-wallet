<script lang="ts">
  import NotSignedChip from './NotSignedChip.svelte';
  import RequesterAvatar from './RequesterAvatar.svelte';
  import RequesterName from './RequesterName.svelte';
  import type { RequesterView } from './types';

  /**
   * The header of a single-screen request: what it asks for, then who asks and
   * the time left on one quiet line. Only a missing signature or an almost
   * expired request gets colour.
   */
  let {
    requester,
    title,
    timeLeftLabel = '',
    urgent = false,
  }: {
    requester: RequesterView;
    title: string;
    timeLeftLabel?: string;
    urgent?: boolean;
  } = $props();
</script>

<div class="flex items-center gap-3.5">
  <RequesterAvatar {requester} class="size-12 text-lg" />
  <div class="min-w-0 flex-1">
    <h1 class="text-2xl leading-tight font-semibold tracking-tight text-balance text-foreground">
      {title}
    </h1>
    <p
      class="mt-1.5 flex min-w-0 flex-wrap items-center gap-x-1.5 gap-y-1 text-[13px] text-muted-foreground"
    >
      {#if !requester.signed}
        <NotSignedChip />
      {:else}
        <RequesterName {requester} />
      {/if}
      {#if timeLeftLabel}
        <span aria-hidden="true">·</span>
        <span class={urgent ? 'text-amber-700 dark:text-amber-300' : ''}>{timeLeftLabel}</span>
      {/if}
    </p>
  </div>
</div>
