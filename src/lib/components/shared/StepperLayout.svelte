<script lang="ts">
  import XIcon from '@lucide/svelte/icons/x';
  import type { Snippet } from 'svelte';
  import { i18nStore } from '$lib/i18n';
  import type { WalletNetwork } from '$lib/types/wallet.js';
  import { cn } from '$lib/utils.js';

  type StepperLayoutProps = {
    currentStep: number;
    totalSteps: number;
    children?: Snippet;
    footer?: Snippet;
    backgroundClass?: string;
    contentClass?: string;
    contentInnerClass?: string;
    footerClass?: string;
    onClose?: () => void;
    closeDisabled?: boolean;
    closeAriaLabel?: string;
    showNetworkToggle?: boolean;
    network?: WalletNetwork;
    networkLabel?: string;
    networkToggleDisabled?: boolean;
    onNetworkChange?: (value: WalletNetwork) => void;
  };

  const defaultNetworkHandler = (value: WalletNetwork) => {
    void value;
  };

  let {
    currentStep,
    totalSteps,
    children,
    footer,
    backgroundClass = 'bg-app-canvas',
    contentClass = 'flex-1 overflow-y-auto px-6 py-10 sm:px-8',
    contentInnerClass = 'mx-auto w-full max-w-[620px] space-y-6',
    footerClass = 'px-6 py-4 sm:px-8',
    onClose = undefined,
    closeDisabled = false,
    closeAriaLabel = '',
    showNetworkToggle = false,
    network = undefined,
    networkLabel = '',
    networkToggleDisabled = false,
    onNetworkChange = defaultNetworkHandler
  }: StepperLayoutProps = $props();

  const i18n = $derived($i18nStore);
  const canRenderClose = $derived(typeof onClose === 'function');
  const closeLabel = $derived(closeAriaLabel || i18n.t('common.cancel'));
  const canRenderNetworkToggle = $derived(showNetworkToggle && !!network);
</script>

<main class="flex h-screen flex-col overflow-hidden">
  <div class={cn('absolute inset-0', backgroundClass)}></div>
  <div class="absolute top-0 right-0 left-0 z-30 h-11" data-tauri-drag-region aria-hidden="true"></div>
  {#if canRenderClose}
    <div class="absolute top-0 right-0 z-40 flex h-[50px] items-center pr-4">
      <button
        type="button"
        class="ring-offset-background focus-visible:ring-ring inline-flex h-8 w-8 items-center justify-center rounded-xs opacity-70 transition-opacity hover:opacity-100 focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-hidden disabled:pointer-events-none"
        onclick={() => onClose?.()}
        disabled={closeDisabled}
        aria-label={closeLabel}
      >
        <XIcon class="size-5" />
      </button>
    </div>
  {/if}

  <div class="relative z-10 flex min-h-0 flex-1 w-full">
    <section class="flex min-w-0 flex-1">
      <div class="flex min-h-0 flex-1 flex-col">
        <div class="border-border/80 shrink-0 border-b">
          <div class="flex h-[50px] items-center justify-center px-6">
            <div class="flex items-center gap-4">
              <span class="text-muted-foreground text-sm font-medium">
                {i18n.t('shared.stepOf', { current: currentStep, total: totalSteps })}
              </span>

              <div class="flex items-center gap-2">
                {#each [...Array(totalSteps).keys()] as stepIndex}
                  {@const stepNum = stepIndex + 1}
                  <div
                    class="h-2 w-2 rounded-full transition-all duration-200
                      {stepNum === currentStep
                        ? 'bg-primary scale-125'
                        : stepNum < currentStep
                          ? 'bg-primary/60'
                          : 'bg-muted-foreground/30'}"
                  ></div>
                {/each}
              </div>
            </div>
          </div>
        </div>

        {#if canRenderNetworkToggle}
          <div class="absolute top-[58px] right-3 z-20 sm:right-4">
            <div class="flex min-w-[150px] justify-end">
              <div class="flex items-center gap-1 opacity-70 transition-opacity hover:opacity-100">
              {#if networkLabel}
                <span class="sr-only">{networkLabel}</span>
              {/if}
              <button
                type="button"
                onclick={() => onNetworkChange('mainnet')}
                disabled={networkToggleDisabled}
                class="h-5 w-[70px] rounded border px-2 text-[10px] font-medium transition-colors {network === 'mainnet'
                  ? 'border-border bg-muted/70 text-foreground'
                  : 'border-transparent bg-transparent text-muted-foreground hover:border-border/60 hover:bg-muted/40'}"
              >
                {i18n.t('common.network.mainnet')}
              </button>
              <button
                type="button"
                onclick={() => onNetworkChange('testnet')}
                disabled={networkToggleDisabled}
                class="h-5 w-[70px] rounded border px-2 text-[10px] font-medium transition-colors {network === 'testnet'
                  ? 'border-border bg-muted/70 text-foreground'
                  : 'border-transparent bg-transparent text-muted-foreground hover:border-border/60 hover:bg-muted/40'}"
              >
                {i18n.t('common.network.testnet')}
              </button>
              </div>
            </div>
          </div>
        {/if}

        <div class={contentClass}>
          <div class={contentInnerClass}>
            {@render children?.()}
          </div>
        </div>

        <div class="border-black/10 bg-muted/10 dark:border-white/20 shrink-0 border-t">
          <div class={footerClass}>
            {@render footer?.()}
          </div>
        </div>
      </div>
    </section>
  </div>
</main>
