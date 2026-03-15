<script lang="ts">
  import PlusIcon from '@lucide/svelte/icons/plus';
  import { Button } from '$lib/components/ui/button';
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';
  import { i18nStore } from '$lib/i18n';

  const skeletonCards = [0, 1, 2, 3];

  const noop = (): void => {};

  type IdentityListSkeletonProps = {
    revealed?: boolean;
    onOpenLinkSheet?: () => void;
  };

  let { revealed = true, onOpenLinkSheet = noop }: IdentityListSkeletonProps = $props();

  const i18n = $derived($i18nStore);
</script>

<div>
  <div class="flex min-w-0 items-center gap-3">
    <div class="min-w-0 flex-[3]">
      {#if revealed}
        <Skeleton class="h-10 w-full rounded-md" />
      {:else}
        <div class="h-10 w-full" aria-hidden="true"></div>
      {/if}
    </div>

    <Button
      variant="secondary"
      size="lg"
      class="h-10 min-w-[12rem] flex-1 justify-center gap-1.5 rounded-md px-3"
      onclick={onOpenLinkSheet}
    >
      <PlusIcon class="size-4" />
      {i18n.t('wallet.identity.list.linkButton')}
    </Button>
  </div>

  {#if revealed}
    <div class="mt-4 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
      {#each skeletonCards as skeletonCard (skeletonCard)}
        <div class="flex w-full items-center gap-2 rounded-xl bg-muted/20 px-3 py-3 dark:bg-muted/15">
          <Skeleton class="size-10 shrink-0 rounded-full" />

          <div class="min-w-0 flex-1">
            <Skeleton class="h-4 w-32 max-w-full rounded-sm" />
            <Skeleton class="mt-2 h-3 w-16 rounded-sm" />
          </div>

          <Skeleton class="h-4 w-4 shrink-0 rounded-sm" />
          <Skeleton class="h-8 w-8 shrink-0 rounded-md" />
        </div>
      {/each}
    </div>
  {/if}
</div>
