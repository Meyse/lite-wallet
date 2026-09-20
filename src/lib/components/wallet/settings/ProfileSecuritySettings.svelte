<!--
  Component: ProfileSecuritySettings
  Purpose: Focused security settings detail page with recovery and key access.
-->

<script lang="ts">
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import NavigationBackButton from '$lib/components/common/NavigationBackButton.svelte';
  import DropdownSelectTrigger from '$lib/components/common/DropdownSelectTrigger.svelte';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import type { AutoLockMinutes } from '$lib/security/sessionTimeout.js';
  import { i18nStore } from '$lib/i18n';

  type ProfileSecuritySettingsProps = {
    onBack(): void;
    autoLockMinutes: AutoLockMinutes;
    autoLockOptions: readonly AutoLockMinutes[];

    onSetAutoLockMinutes: (minutes: AutoLockMinutes) => void;
    onOpenRecovery(): void;
  };

  const {
    onBack,
    autoLockMinutes,
    autoLockOptions,
    onSetAutoLockMinutes,
    onOpenRecovery,
  }: ProfileSecuritySettingsProps = $props();
  const i18n = $derived($i18nStore);
</script>

<div class="mx-auto flex h-full min-h-0 w-full max-w-5xl flex-col bg-app-canvas px-5 pt-5 pb-6">
  <section class="flex min-h-0 flex-1 flex-col overflow-auto">
    <header class="flex h-[72px] shrink-0 flex-col gap-3">
      <NavigationBackButton
        label={i18n.t('wallet.settings.backLabel')}
        tone="settings"
        onclick={onBack}
      />
      <h2 class="text-xl leading-7 font-semibold tracking-[-0.015em]">
        {i18n.t('wallet.settings.profile.title')}
      </h2>
    </header>

    <div class="mt-5 overflow-visible rounded-lg bg-settings-surface">
      <div class="flex h-12 items-center gap-4 px-4">
        <p class="min-w-0 flex-1 text-[13px] leading-4 font-medium">
          {i18n.t('wallet.settings.profile.autoLock.title')}
        </p>
        <DropdownMenu.Root>
          <DropdownMenu.Trigger aria-label={i18n.t('wallet.settings.profile.autoLock.title')}>
            {#snippet child({ props })}
              <DropdownSelectTrigger
                {...props}
                value={i18n.t('wallet.settings.profile.autoLock.option', {
                  minutes: autoLockMinutes,
                })}
                size="compact"
              />
            {/snippet}
          </DropdownMenu.Trigger>
          <DropdownMenu.Content align="end" class="w-[168px]">
            <DropdownMenu.RadioGroup value={String(autoLockMinutes)}>
              {#each autoLockOptions as option}
                <DropdownMenu.RadioItem
                  value={String(option)}
                  onclick={() => {
                    onSetAutoLockMinutes(option);
                  }}
                >
                  {i18n.t('wallet.settings.profile.autoLock.option', { minutes: option })}
                </DropdownMenu.RadioItem>
              {/each}
            </DropdownMenu.RadioGroup>
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </div>

      <button
        type="button"
        class="flex h-12 w-full items-center gap-4 rounded-b-lg px-4 text-left outline-none hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:ring-inset"
        onclick={onOpenRecovery}
      >
        <span class="min-w-0 flex-1 text-[13px] leading-4 font-medium">
          {i18n.t('wallet.settings.profile.recovery.title')}
        </span>
        <ChevronRightIcon class="size-4 shrink-0 text-settings-muted-foreground" />
      </button>
    </div>
  </section>
</div>
