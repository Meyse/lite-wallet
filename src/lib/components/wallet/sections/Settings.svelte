<!--
  Component: Settings
  Purpose: Wallet settings hub with category drill-down detail pages.
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import GlobeIcon from '@lucide/svelte/icons/globe';
  import InfoIcon from '@lucide/svelte/icons/info';
  import KeyIcon from '@lucide/svelte/icons/key';
  import ShieldIcon from '@lucide/svelte/icons/shield';
  import { i18nStore } from '$lib/i18n';
  import type { WalletNetwork } from '$lib/types/wallet';
  import { setAutoLockMinutes, settingsStore } from '$lib/stores/settings.js';
  import {
    ALLOWED_AUTO_LOCK_MINUTES,
    type AutoLockMinutes,
    normalizeAutoLockMinutes
  } from '$lib/security/sessionTimeout.js';
  import { buildLocaleOptions } from '$lib/utils/localeOptions.js';
  import { loadRuntimeAppInfo } from '$lib/utils/appInfo.js';
  import * as walletService from '$lib/services/walletService';
  import DisplayLanguageSettings from '$lib/components/wallet/settings/DisplayLanguageSettings.svelte';
  import PrivateVerusSettings from '$lib/components/wallet/settings/PrivateVerusSettings.svelte';
  import ProfileSecuritySettings from '$lib/components/wallet/settings/ProfileSecuritySettings.svelte';
  import RecoveryKeysSettings from '$lib/components/wallet/settings/RecoveryKeysSettings.svelte';
  import AboutSupportSettings from '$lib/components/wallet/settings/AboutSupportSettings.svelte';

  type SettingsProps = {
    walletNetwork: WalletNetwork;
    resetSignal?: number;
  };

  type SettingsView =
    | 'home'
    | 'display-language'
    | 'profile-security'
    | 'private-verus'
    | 'recovery-keys'
    | 'about-support';

  const { walletNetwork, resetSignal = 0 }: SettingsProps = $props();

  const i18n = $derived($i18nStore);
  const settings = $derived($settingsStore);
  const localeOptions = $derived(buildLocaleOptions(i18n.t));
  const selectedLocaleLabel = $derived(
    localeOptions.find((option) => option.value === i18n.locale)?.label ?? localeOptions[0]?.label ?? '—'
  );

  let activeView = $state<SettingsView>('home');
  let lastResetSignal = $state(0);
  let privateStatusLoading = $state(true);
  let privateConfigured = $state(false);
  let appVersion = $state<string | null>(null);

  const displayLanguageSummary = $derived(
    i18n.t('wallet.settings.home.summary.displayLanguage', {
      currency: settings.displayCurrency,
      language: selectedLocaleLabel
    })
  );
  const autoLockMinutes = $derived(normalizeAutoLockMinutes(settings.autoLockMinutes));
  const profileSummary = $derived(
    i18n.t('wallet.settings.home.summary.profileSecurity', { minutes: autoLockMinutes })
  );
  const privateSummary = $derived(
    privateStatusLoading
      ? i18n.t('common.loading')
      : privateConfigured
        ? i18n.t('wallet.settings.home.summary.privateConfigured')
        : i18n.t('wallet.settings.home.summary.privateNotConfigured')
  );
  const aboutSummary = $derived(
    appVersion
      ? i18n.t('wallet.settings.home.summary.version', { version: appVersion })
      : i18n.t('common.loading')
  );

  async function refreshPrivateStatus(): Promise<void> {
    privateStatusLoading = true;
    try {
      const status = await walletService.getDlightSeedStatus();
      privateConfigured = status.configured;
    } catch {
      privateConfigured = false;
    } finally {
      privateStatusLoading = false;
    }
  }

  async function loadVersionSummary(): Promise<void> {
    try {
      const info = await loadRuntimeAppInfo();
      appVersion = info.version;
    } catch {
      appVersion = null;
    }
  }

  onMount(() => {
    void refreshPrivateStatus();
    void loadVersionSummary();
  });

  async function handleSetAutoLockMinutes(minutes: AutoLockMinutes): Promise<void> {
    const normalized = normalizeAutoLockMinutes(minutes);
    setAutoLockMinutes(normalized);
    await walletService.setSessionTimeoutMinutes(normalized).catch(() => {});
    await walletService.touchSessionActivity().catch(() => {});
  }

  $effect(() => {
    if (resetSignal === lastResetSignal) return;
    lastResetSignal = resetSignal;
    activeView = 'home';
  });
</script>

{#if activeView === 'home'}
  <div class="mx-auto flex h-full min-h-0 w-full max-w-5xl flex-col px-6 pb-6 pt-0 sm:px-8">
    <section class="flex min-h-0 flex-1 flex-col overflow-auto pt-3">
      <div class="space-y-4">
        <div class="space-y-1">
          <h2 class="text-xl font-semibold">{i18n.t('wallet.settings.home.title')}</h2>
          <p class="text-muted-foreground text-sm">{i18n.t('wallet.settings.home.description')}</p>
        </div>

        <div class="space-y-2">
          <button
            type="button"
            class="hover:bg-muted/45 flex w-full items-center justify-between gap-3 rounded-md px-3 py-2 text-left"
            onclick={() => {
              activeView = 'display-language';
            }}
          >
            <span class="min-w-0 flex flex-1 items-start gap-3">
              <GlobeIcon class="text-muted-foreground mt-0.5 size-4 shrink-0" />
              <span class="min-w-0">
                <span class="block text-sm font-medium">{i18n.t('wallet.settings.home.category.displayLanguage')}</span>
                <span class="text-muted-foreground mt-0.5 block truncate text-xs">{displayLanguageSummary}</span>
              </span>
            </span>
            <ChevronRightIcon class="text-muted-foreground size-4 shrink-0" />
          </button>

          <button
            type="button"
            class="hover:bg-muted/45 flex w-full items-center justify-between gap-3 rounded-md px-3 py-2 text-left"
            onclick={() => {
              activeView = 'profile-security';
            }}
          >
            <span class="min-w-0 flex flex-1 items-start gap-3">
              <KeyIcon class="text-muted-foreground mt-0.5 size-4 shrink-0" />
              <span class="min-w-0">
                <span class="block text-sm font-medium">{i18n.t('wallet.settings.home.category.profileSecurity')}</span>
                <span class="text-muted-foreground mt-0.5 block truncate text-xs">{profileSummary}</span>
              </span>
            </span>
            <ChevronRightIcon class="text-muted-foreground size-4 shrink-0" />
          </button>

          <button
            type="button"
            class="hover:bg-muted/45 flex w-full items-center justify-between gap-3 rounded-md px-3 py-2 text-left"
            onclick={() => {
              activeView = 'private-verus';
            }}
          >
            <span class="min-w-0 flex flex-1 items-start gap-3">
              <ShieldIcon class="text-muted-foreground mt-0.5 size-4 shrink-0" />
              <span class="min-w-0">
                <span class="block text-sm font-medium">{i18n.t('wallet.settings.home.category.privateVerus')}</span>
                <span class="text-muted-foreground mt-0.5 block truncate text-xs">{privateSummary}</span>
              </span>
            </span>
            <ChevronRightIcon class="text-muted-foreground size-4 shrink-0" />
          </button>

          <button
            type="button"
            class="hover:bg-muted/45 flex w-full items-center justify-between gap-3 rounded-md px-3 py-2 text-left"
            onclick={() => {
              activeView = 'about-support';
            }}
          >
            <span class="min-w-0 flex flex-1 items-start gap-3">
              <InfoIcon class="text-muted-foreground mt-0.5 size-4 shrink-0" />
              <span class="min-w-0">
                <span class="block text-sm font-medium">{i18n.t('wallet.settings.home.category.aboutSupport')}</span>
                <span class="text-muted-foreground mt-0.5 block truncate text-xs">{aboutSummary}</span>
              </span>
            </span>
            <ChevronRightIcon class="text-muted-foreground size-4 shrink-0" />
          </button>
        </div>
      </div>
    </section>
  </div>
{:else if activeView === 'display-language'}
  <DisplayLanguageSettings
    onBack={() => {
      activeView = 'home';
    }}
  />
{:else if activeView === 'profile-security'}
  <ProfileSecuritySettings
    autoLockMinutes={autoLockMinutes}
    autoLockOptions={ALLOWED_AUTO_LOCK_MINUTES}
    onSetAutoLockMinutes={handleSetAutoLockMinutes}
    onOpenRecovery={() => {
      activeView = 'recovery-keys';
    }}
    onBack={() => {
      activeView = 'home';
    }}
  />
{:else if activeView === 'private-verus'}
  <PrivateVerusSettings
    {walletNetwork}
    onBack={() => {
      activeView = 'home';
      void refreshPrivateStatus();
    }}
  />
{:else if activeView === 'recovery-keys'}
  <RecoveryKeysSettings
    onBack={() => {
      activeView = 'home';
    }}
  />
{:else if activeView === 'about-support'}
  <AboutSupportSettings
    onBack={() => {
      activeView = 'home';
    }}
  />
{/if}
