<!--
  LanguageGate
  Purpose: First-launch language selection shown before first wallet onboarding.
  Uses shadcn-svelte dropdown menu primitives for selection.
-->

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import LocaleSelector from '$lib/components/common/LocaleSelector.svelte';
  import { i18nStore } from '$lib/i18n';
  import WalletHeroBackground from '$lib/components/wallet/WalletHeroBackground.svelte';

  const { onContinue = () => {} }: { onContinue?: () => void } = $props();
  const i18n = $derived($i18nStore);
</script>

<main class="bg-background relative flex min-h-screen overflow-hidden">
  <div class="bg-app-canvas absolute inset-0"></div>
  <div class="absolute top-0 right-0 left-0 z-20 h-11" data-tauri-drag-region aria-hidden="true"></div>

  <div class="relative z-10 flex min-h-screen w-full">
    <section class="relative hidden w-[clamp(320px,38vw,500px)] shrink-0 overflow-hidden md:block">
      <WalletHeroBackground />
    </section>

    <section class="flex min-w-0 flex-1 flex-col">
      <div class="flex-1 flex items-center justify-center px-6 py-10 sm:px-8">
        <div class="w-full max-w-[320px] space-y-6">
          <div>
            <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
              {i18n.t('languageGate.title')}
            </h1>
          </div>

          <div class="w-full">
            <LocaleSelector
              triggerId="language-trigger"
              triggerAriaLabel={i18n.t('languageGate.title')}
            />
          </div>
        </div>
      </div>

      <div class="shrink-0 border-t border-black/10 bg-muted/10 dark:border-white/20">
        <div class="flex w-full items-center justify-end px-4 py-4 sm:px-4">
          <Button onclick={onContinue} class="w-48">
            {i18n.t('languageGate.button')}
          </Button>
        </div>
      </div>
    </section>
  </div>
</main>
