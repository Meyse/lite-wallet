<script lang="ts">
  import LockIcon from '@lucide/svelte/icons/lock-keyhole';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import { Button } from '$lib/components/ui/button';

  type WalletEmptyStateProps = {
    title: string;
    actionLabel: string;
    onAction: () => void;
    eyebrow?: string;
    illustration?: 'address-book' | 'watch-list' | 'verus-id';
    testId: string;
  };

  let { title, actionLabel, onAction, eyebrow, illustration, testId }: WalletEmptyStateProps =
    $props();
</script>

<!-- Empty sections omit their 58px page header; the extra top padding preserves this group's position. -->
<div
  class="flex min-h-0 flex-1 flex-col items-center px-8 pt-[114px] pb-10 text-center"
  data-testid={testId}
>
  {#if illustration}
    <div
      class="mb-4 flex h-[132px] w-[156px] shrink-0 items-center justify-center"
      aria-hidden="true"
    >
      <img
        src={`/images/empty-states/${illustration}.png`}
        alt=""
        width="156"
        height="156"
        draggable="false"
        class="pointer-events-none size-[156px] shrink-0 object-contain opacity-[0.48] brightness-[0.86] saturate-[0.32] select-none dark:opacity-[0.28] dark:brightness-[0.75]"
      />
    </div>
  {/if}
  <div
    class="flex h-4 shrink-0 items-center justify-center"
    data-testid="wallet-empty-eyebrow-slot"
  >
    {#if eyebrow}
      <div
        class="flex items-center gap-1.5 text-[10px] leading-4 font-semibold tracking-[0.18em] whitespace-nowrap text-settings-muted-foreground/60 uppercase"
        data-testid="wallet-empty-eyebrow"
      >
        <LockIcon class="size-2.5" strokeWidth={1.75} aria-hidden="true" />
        <span>{eyebrow}</span>
      </div>
    {/if}
  </div>
  <h3 class="mt-2 text-lg font-semibold tracking-tight" data-testid="wallet-empty-title">
    {title}
  </h3>
  <Button size="sm" class="mt-5" onclick={onAction}>
    <PlusIcon class="size-3.5" aria-hidden="true" />
    {actionLabel}
  </Button>
</div>
