<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { Input } from '$lib/components/ui/input';
  import { i18nStore } from '$lib/i18n';
  import type { GuardFlowMode } from './types';

  type GuardTargetStepProps = {
    mode: GuardFlowMode;
    targetIdentity: string;
    busy?: boolean;
    errorMessage?: string;
    shakeNonce?: number;
    onTargetIdentityChange?: (value: string) => void;
  };

  const defaultStringHandler = (value: string) => {
    void value;
  };

   
  let {
    mode,
    targetIdentity,
    busy = false,
    errorMessage = '',
    shakeNonce = 0,
    onTargetIdentityChange = defaultStringHandler
  }: GuardTargetStepProps = $props();
   

  let shakeTargetField = $state(false);
  let previousShakeNonce = $state(0);
  let shakeResetTimer: ReturnType<typeof setTimeout> | null = null;

  const i18n = $derived($i18nStore);
  const actionLabel = $derived(
    mode === 'recover' ? i18n.t('guard.mode.recoverLower') : i18n.t('guard.mode.revokeLower')
  );

  async function triggerTargetShake() {
    shakeTargetField = false;
    await tick();
    shakeTargetField = true;

    if (shakeResetTimer) {
      clearTimeout(shakeResetTimer);
    }
    shakeResetTimer = setTimeout(() => {
      shakeTargetField = false;
    }, 320);
  }

  $effect(() => {
    if (shakeNonce === previousShakeNonce) return;
    previousShakeNonce = shakeNonce;
    void triggerTargetShake();
  });

  onDestroy(() => {
    if (shakeResetTimer) {
      clearTimeout(shakeResetTimer);
    }
  });
</script>

<div class="mx-auto w-full max-w-[560px] space-y-7 py-4">
  <div class="space-y-2 text-center">
    <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
      {i18n.t('guard.flow.target.title', { action: actionLabel })}
    </h1>
  </div>

  <div class="space-y-2">
    <div class={shakeTargetField ? 'target-error-shake' : ''}>
      <Input
        id="guard-target-id"
        value={targetIdentity}
        oninput={(event) => onTargetIdentityChange((event.target as HTMLInputElement).value)}
        placeholder={i18n.t('guard.flow.target.identityPlaceholder')}
        class={'identifier-text ' + (errorMessage ? 'border-destructive' : '')}
        disabled={busy}
        autocapitalize="off"
        spellcheck="false"
      />
    </div>
    <p class={'min-h-5 text-xs ' + (errorMessage ? 'text-destructive' : 'text-muted-foreground')} aria-live="polite">
      {errorMessage || i18n.t('guard.flow.target.identityHelp')}
    </p>
  </div>
</div>

<style>
  @keyframes target-error-shake {
    0%,
    100% {
      transform: translateX(0);
    }
    20% {
      transform: translateX(-5px);
    }
    40% {
      transform: translateX(5px);
    }
    60% {
      transform: translateX(-4px);
    }
    80% {
      transform: translateX(4px);
    }
  }

  .target-error-shake {
    animation: target-error-shake 320ms cubic-bezier(0.36, 0.07, 0.19, 0.97);
    will-change: transform;
  }

  @media (prefers-reduced-motion: reduce) {
    .target-error-shake {
      animation: none;
    }
  }
</style>
