<!--
  Component: ProfileSecuritySettings
  Purpose: Focused security settings detail page with recovery and key access.
-->

<script lang="ts">
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import Clock3Icon from '@lucide/svelte/icons/clock-3';
  import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
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

  const { onBack, autoLockMinutes, autoLockOptions, onSetAutoLockMinutes, onOpenRecovery }: ProfileSecuritySettingsProps = $props();
  const i18n = $derived($i18nStore);
</script>

<div class="mx-auto flex h-full min-h-0 w-full max-w-5xl flex-col px-6 pb-6 pt-0 sm:px-8">
  <section class="flex min-h-0 flex-1 flex-col overflow-auto pt-3">
    <div class="space-y-4">
      <button
        type="button"
        class="text-muted-foreground hover:text-foreground inline-flex w-fit items-center gap-1.5 text-sm"
        onclick={onBack}
      >
        <ArrowLeftIcon class="size-4" />
        {i18n.t('common.back')}
      </button>

      <section class="space-y-4">
        <div class="space-y-1">
          <h2 class="text-xl font-semibold">{i18n.t('wallet.settings.profile.title')}</h2>
          <p class="text-muted-foreground text-sm">{i18n.t('wallet.settings.profile.description')}</p>
        </div>

        <div class="space-y-2">
          <DropdownMenu.Root>
            <DropdownMenu.Trigger aria-label={i18n.t('wallet.settings.profile.autoLock.title')}>
              {#snippet child({ props })}
                <button
                  type="button"
                  {...props}
                  class="hover:bg-muted/45 flex w-full items-center gap-3 rounded-md px-3 py-2 text-left"
                >
                  <Clock3Icon class="text-muted-foreground size-4 shrink-0" />
                  <span class="min-w-0 flex-1">
                    <span class="text-sm font-medium">{i18n.t('wallet.settings.profile.autoLock.title')}</span>
                  </span>
                  <span class="text-muted-foreground text-xs">
                    {i18n.t('wallet.settings.profile.autoLock.option', { minutes: autoLockMinutes })}
                  </span>
                  <ChevronDownIcon class="text-muted-foreground size-4 shrink-0" />
                </button>
              {/snippet}
            </DropdownMenu.Trigger>
            <DropdownMenu.Content align="end" class="min-w-[10rem]">
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
          <p class="text-muted-foreground px-3 text-xs">{i18n.t('wallet.settings.profile.autoLock.description')}</p>
        </div>

        <div>
          <button
            type="button"
            class="hover:bg-muted/45 flex w-full items-center gap-3 rounded-md px-3 py-2 text-left"
            onclick={onOpenRecovery}
          >
            <ShieldCheckIcon class="text-muted-foreground size-4 shrink-0" />
            <span class="min-w-0 flex-1">
              <span class="text-sm font-medium">{i18n.t('wallet.settings.profile.recovery.title')}</span>
              <span class="text-muted-foreground mt-0.5 block text-xs">
                {i18n.t('wallet.settings.profile.recovery.description')}
              </span>
            </span>
            <ChevronRightIcon class="text-muted-foreground size-4 shrink-0" />
          </button>
        </div>
      </section>
    </div>
  </section>
</div>
