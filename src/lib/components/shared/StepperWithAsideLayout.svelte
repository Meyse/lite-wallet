<script lang="ts">
  import { onMount } from 'svelte';
  import XIcon from '@lucide/svelte/icons/x';
  import type { Snippet } from 'svelte';
  import { Button } from '$lib/components/ui/button';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { i18nStore } from '$lib/i18n';

  type StepStatus = 'complete' | 'current' | 'upcoming';

  type StepperWithAsideLayoutProps = {
    currentStep: number;
    totalSteps: number;
    steps?: { id: string; label: string; status: StepStatus }[];
    onClose?: () => void;
    closeDisabled?: boolean;
    showCloseButton?: boolean;
    showAside?: boolean;
    mobileAsideLabel?: string;
    mobileAsideTitle?: string;
    children?: Snippet;
    aside?: Snippet;
    footer?: Snippet;
    footerAside?: Snippet;
  };

  const defaultCloseHandler = () => {};

  let {
    currentStep,
    totalSteps,
    steps = [],
    onClose = defaultCloseHandler,
    closeDisabled = false,
    showCloseButton = true,
    showAside = true,
    mobileAsideLabel = '',
    mobileAsideTitle = '',
    children,
    aside,
    footer,
    footerAside
  }: StepperWithAsideLayoutProps = $props();

  const i18n = $derived($i18nStore);
  let showMobileAside = $state(false);

  function statusClasses(status: StepStatus): string {
    if (status === 'complete') return 'bg-primary/10 text-primary';
    if (status === 'current') return 'bg-primary text-primary-foreground';
    return 'bg-muted/60 text-muted-foreground';
  }

  onMount(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      if (showMobileAside) {
        showMobileAside = false;
        return;
      }
      onClose();
    };

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  });

  $effect(() => {
    if (!showAside) {
      showMobileAside = false;
    }
  });
</script>

<section class="relative h-full min-h-0 w-full overflow-hidden">
  {#if showCloseButton}
    <div class="absolute top-0 right-0 z-40 flex h-[50px] items-center pr-4">
      <button
        type="button"
        class="ring-offset-background focus-visible:ring-ring inline-flex h-8 w-8 items-center justify-center rounded-xs opacity-70 transition-opacity hover:opacity-100 focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-hidden disabled:pointer-events-none"
        onclick={onClose}
        disabled={closeDisabled}
        aria-label={i18n.t('common.cancel')}
      >
        <XIcon class="size-5" />
      </button>
    </div>
  {/if}

  <div
    class={`h-full min-h-0 md:grid md:grid-rows-[minmax(0,1fr)_auto]
      ${showAside && aside ? 'md:grid-cols-[minmax(0,1fr)_220px]' : 'md:grid-cols-[minmax(0,1fr)]'}`}
  >
    <div class="relative flex min-h-0 flex-col overflow-hidden">
      <div class="absolute top-0 right-0 left-0 z-30 h-11" data-tauri-drag-region aria-hidden="true"></div>

      <header class="relative z-10 shrink-0 border-b border-border/70">
        <div class="flex h-[50px] items-center justify-center px-4">
          <div class="flex min-w-0 flex-col items-center">
            {#if steps.length > 0}
              <ol class="flex max-w-full items-center gap-1 overflow-x-auto">
                {#each steps as step, index}
                  <li class="flex items-center gap-1">
                    <span class={`rounded-full px-3 py-1 text-xs font-semibold whitespace-nowrap ${statusClasses(step.status)}`}>
                      {step.label}
                    </span>
                    {#if index < steps.length - 1}
                      <span class="bg-border/70 h-px w-2.5" aria-hidden="true"></span>
                    {/if}
                  </li>
                {/each}
              </ol>
            {:else}
              <div class="flex items-center gap-2">
                <span class="text-muted-foreground text-sm font-medium">
                  {i18n.t('shared.stepOf', { current: currentStep, total: totalSteps })}
                </span>
                {#each [...Array(totalSteps).keys()] as stepIndex}
                  {@const stepNumber = stepIndex + 1}
                  <div
                    class="h-2 w-2 rounded-full transition-all duration-200
                      {stepNumber === currentStep
                        ? 'bg-primary scale-125'
                        : stepNumber < currentStep
                          ? 'bg-primary/60'
                          : 'bg-muted-foreground/30'}"
                  ></div>
                {/each}
              </div>
            {/if}
          </div>
        </div>
      </header>

      <div class="relative z-10 min-h-0 flex-1 overflow-y-auto px-4 py-4 sm:px-6 sm:py-5">
        <div class="mx-auto w-full max-w-[1040px]">
          {@render children?.()}
        </div>
      </div>
    </div>

    {#if showAside && aside}
      <aside class="bg-sidebar-surface hidden min-h-0 overflow-y-auto border-l border-border/70 px-3 py-4 md:block">
        {@render aside?.()}
      </aside>
    {/if}

    <footer
      class={`border-black/10 bg-muted/10 dark:border-white/20 relative z-10 shrink-0 border-t ${
        showAside && aside ? 'md:col-span-2' : ''
      }`}
    >
      <div class={showAside && aside ? 'md:grid md:grid-cols-[minmax(0,1fr)_220px]' : ''}>
        <div class="space-y-2 px-4 py-3 sm:px-6">
          {#if showAside && aside && mobileAsideLabel}
            <div class="flex justify-end md:hidden">
              <Button variant="ghost" size="sm" class="px-2" onclick={() => (showMobileAside = true)}>
                {mobileAsideLabel}
              </Button>
            </div>
          {/if}
          {@render footer?.()}
        </div>

        {#if showAside && aside}
          <div
            class="bg-sidebar-surface hidden border-l border-border/70 px-3 py-3 md:flex md:items-center"
          >
            {@render footerAside?.()}
          </div>
        {/if}
      </div>
    </footer>
  </div>
</section>

{#if showAside && aside}
  <StandardRightSheet
    bind:isOpen={showMobileAside}
    title={mobileAsideTitle || i18n.t('wallet.transfer.summary.title')}
  >
    {@render aside?.()}
  </StandardRightSheet>
{/if}
