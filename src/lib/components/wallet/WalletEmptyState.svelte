<script lang="ts">
  import PlusIcon from '@lucide/svelte/icons/plus';
  import { Button } from '$lib/components/ui/button';

  type WalletEmptyStateProps = {
    title: string;
    actionLabel: string;
    onAction: () => void;
    eyebrow?: string;
    testId: string;
  };

  let { title, actionLabel, onAction, eyebrow, testId }: WalletEmptyStateProps = $props();
</script>

<div
  class="flex min-h-0 flex-1 flex-col items-center justify-center px-8 pb-10 text-center"
  data-testid={testId}
>
  <div class="relative">
    {#if eyebrow}
      <p
        class="absolute bottom-full left-1/2 mb-2 -translate-x-1/2 text-[10px] leading-4 font-semibold tracking-[0.18em] whitespace-nowrap text-settings-muted-foreground/60 uppercase"
        data-testid="wallet-empty-eyebrow"
      >
        {eyebrow}
      </p>
    {/if}
    <h3 class="text-lg font-semibold tracking-tight" data-testid="wallet-empty-title">
      {title}
    </h3>
  </div>
  <Button size="sm" class="mt-5" onclick={onAction}>
    <PlusIcon class="size-3.5" aria-hidden="true" />
    {actionLabel}
  </Button>
</div>
