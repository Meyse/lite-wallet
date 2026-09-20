<script lang="ts">
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';

  const skeletonRows = [0, 1, 2];

  type IdentityListSkeletonProps = {
    revealed?: boolean;
  };

  let { revealed = true }: IdentityListSkeletonProps = $props();
</script>

{#if revealed}
  <div>
    {#each skeletonRows as row, index (row)}
      <div
        class={`flex items-center gap-3.5 py-[18px] ${index < skeletonRows.length - 1 ? 'border-b' : ''}`}
      >
        <Skeleton class="size-12 shrink-0 rounded-full" />
        <div class="min-w-0 flex-1">
          <Skeleton class="h-[17px] w-40 max-w-full rounded-sm" />
          <Skeleton class="mt-2 h-3 w-56 max-w-[75%] rounded-sm" />
        </div>
        <Skeleton class="size-8 shrink-0 rounded-md" />
        <Skeleton class="h-[34px] w-[98px] shrink-0 rounded-md" />
      </div>
    {/each}
  </div>
{:else}
  <div class="h-[254px]" aria-hidden="true"></div>
{/if}
