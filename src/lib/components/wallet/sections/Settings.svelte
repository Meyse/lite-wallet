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
    normalizeAutoLockMinutes,
  } from '$lib/security/sessionTimeout.js';
  import { buildLocaleOptions } from '$lib/utils/localeOptions.js';
  import { loadRuntimeAppInfo } from '$lib/utils/appInfo.js';
  import * as walletService from '$lib/services/walletService';
  import DisplayLanguageSettings from '$lib/components/wallet/settings/DisplayLanguageSettings.svelte';
  import PrivateVerusSettings from '$lib/components/wallet/settings/PrivateVerusSettings.svelte';
  import ProfileSecuritySettings from '$lib/components/wallet/settings/ProfileSecuritySettings.svelte';
  import RecoveryKeysSettings from '$lib/components/wallet/settings/RecoveryKeysSettings.svelte';
  import AboutSupportSettings from '$lib/components/wallet/settings/AboutSupportSettings.svelte';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import { withRequestTimeout } from '$lib/utils/requestTimeout.js';

  type SettingsProps = {
    walletNetwork: WalletNetwork;
    walletName: string;
    walletSessionKey: string;
    resetSignal?: number;
  };

  type RecoveryOrigin = 'profile-security' | 'private-verus';

  type SettingsView =
    | 'home'
    | 'display-language'
    | 'profile-security'
    | 'private-verus'
    | 'recovery-keys'
    | 'about-support';

  const { walletNetwork, walletName, walletSessionKey, resetSignal = 0 }: SettingsProps = $props();

  const i18n = $derived($i18nStore);
  const settings = $derived($settingsStore);
  const localeOptions = $derived(buildLocaleOptions(i18n.t));
  const selectedLocaleLabel = $derived(
    localeOptions.find((option) => option.value === i18n.locale)?.label ??
      localeOptions[0]?.label ??
      '—'
  );
  const selectedAppearanceLabel = $derived(
    i18n.t(`wallet.settings.display.appearance.${settings.theme}`)
  );

  let activeView = $state<SettingsView>('home');
  let recoveryOrigin = $state<RecoveryOrigin>('profile-security');
  let lastResetSignal = $state(0);
  let privateStatusLoading = $state(true);
  let privateConfigured = $state(false);
  let privateStatusUnavailable = $state(false);
  let appVersion = $state<string | null>(null);
  let appVersionPending = $state(true);

  const displayLanguageSummary = $derived(
    i18n.t('wallet.settings.home.summary.displayLanguage', {
      appearance: selectedAppearanceLabel,
      currency: settings.displayCurrency,
      language: selectedLocaleLabel,
    })
  );
  const autoLockMinutes = $derived(normalizeAutoLockMinutes(settings.autoLockMinutes));
  const profileSummary = $derived(i18n.t('wallet.settings.home.summary.profileSecurity'));
  const privateSummary = $derived.by(() => {
    if (privateStatusLoading) return '—';
    if (privateStatusUnavailable) return i18n.t('wallet.settings.home.summary.privateUnavailable');
    if (privateConfigured) return i18n.t('wallet.settings.home.summary.privateConfigured');
    return i18n.t('wallet.settings.home.summary.privateNotConfigured');
  });
  const aboutSummary = $derived.by(() => {
    if (appVersionPending) return '—';
    if (appVersion) return i18n.t('wallet.settings.home.summary.version', { version: appVersion });
    return i18n.t('wallet.settings.about.versionUnavailable');
  });

  async function refreshPrivateStatus(): Promise<void> {
    privateStatusLoading = true;
    privateStatusUnavailable = false;
    try {
      const status = await withRequestTimeout(walletService.getDlightSeedStatus());
      privateConfigured = status.configured;
    } catch {
      privateStatusUnavailable = true;
    } finally {
      privateStatusLoading = false;
    }
  }

  async function loadVersionSummary(): Promise<void> {
    try {
      const info = await withRequestTimeout(loadRuntimeAppInfo());
      appVersion = info.version;
    } catch {
      appVersion = null;
    } finally {
      appVersionPending = false;
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
    recoveryOrigin = 'profile-security';
  });

  function openRecovery(origin: RecoveryOrigin): void {
    recoveryOrigin = origin;
    activeView = 'recovery-keys';
  }

  function returnFromRecovery(): void {
    activeView = recoveryOrigin;
  }
</script>

{#if activeView === 'home'}
  <div class="mx-auto flex h-full min-h-0 w-full max-w-5xl flex-col bg-app-canvas px-5 pt-5 pb-6">
    <section class="flex min-h-0 flex-1 flex-col overflow-auto">
      <div>
        <h2 class="text-xl leading-7 font-semibold tracking-[-0.015em]">
          {i18n.t('wallet.settings.home.title')}
        </h2>

        <div class="mt-5 overflow-hidden rounded-lg bg-settings-surface">
          <button
            type="button"
            class="flex h-[60px] w-full items-center gap-3 px-4 text-left hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:outline-none focus-visible:ring-inset"
            onclick={() => {
              activeView = 'display-language';
            }}
          >
            <span class="flex size-6 shrink-0 items-center justify-center">
              <GlobeIcon class="size-[18px] text-settings-muted-foreground" />
            </span>
            <span class="min-w-0 flex-1">
              <span class="block text-sm leading-5 font-medium">
                {i18n.t('wallet.settings.home.category.displayLanguage')}
              </span>
              <span class="block truncate text-xs leading-4 text-settings-muted-foreground">
                {displayLanguageSummary}
              </span>
            </span>
            <ChevronRightIcon class="size-4 shrink-0 text-settings-muted-foreground" />
          </button>

          <button
            type="button"
            class="flex h-[60px] w-full items-center gap-3 px-4 text-left hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:outline-none focus-visible:ring-inset"
            onclick={() => {
              activeView = 'profile-security';
            }}
          >
            <span class="flex size-6 shrink-0 items-center justify-center">
              <KeyIcon class="size-[18px] text-settings-muted-foreground" />
            </span>
            <span class="min-w-0 flex-1">
              <span class="block text-sm leading-5 font-medium">
                {i18n.t('wallet.settings.home.category.profileSecurity')}
              </span>
              <span class="block truncate text-xs leading-4 text-settings-muted-foreground">
                {profileSummary}
              </span>
            </span>
            <ChevronRightIcon class="size-4 shrink-0 text-settings-muted-foreground" />
          </button>

          <button
            type="button"
            class="flex h-[60px] w-full items-center gap-3 px-4 text-left hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:outline-none focus-visible:ring-inset"
            onclick={() => {
              activeView = 'private-verus';
            }}
          >
            <span class="flex size-6 shrink-0 items-center justify-center">
              <ShieldIcon class="size-[18px] text-settings-muted-foreground" />
            </span>
            <span class="min-w-0 flex-1">
              <span class="block text-sm leading-5 font-medium">
                {i18n.t('wallet.settings.home.category.privateVerus')}
              </span>
              <span class="block truncate text-xs leading-4 text-settings-muted-foreground">
                {#if privateStatusLoading}
                  <Skeleton
                    class="my-0.5 h-3 w-24 rounded-sm bg-settings-muted-foreground/20 motion-reduce:animate-none"
                    role="img"
                    aria-label={i18n.t('wallet.loading.checkingSetup')}
                  />
                {:else}
                  {privateSummary}
                {/if}
              </span>
            </span>
            <ChevronRightIcon class="size-4 shrink-0 text-settings-muted-foreground" />
          </button>

          <button
            type="button"
            class="flex h-[60px] w-full items-center gap-3 px-4 text-left hover:bg-settings-control-surface focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:outline-none focus-visible:ring-inset"
            onclick={() => {
              activeView = 'about-support';
            }}
          >
            <span class="flex size-6 shrink-0 items-center justify-center">
              <InfoIcon class="size-[18px] text-settings-muted-foreground" />
            </span>
            <span class="min-w-0 flex-1">
              <span class="block text-sm leading-5 font-medium">
                {i18n.t('wallet.settings.home.category.aboutSupport')}
              </span>
              <span class="block truncate text-xs leading-4 text-settings-muted-foreground">
                {#if appVersionPending}
                  <Skeleton
                    class="my-0.5 h-3 w-24 rounded-sm bg-settings-muted-foreground/20 motion-reduce:animate-none"
                    role="img"
                    aria-label={i18n.t('wallet.loading.readingVersion')}
                  />
                {:else}
                  {aboutSummary}
                {/if}
              </span>
            </span>
            <ChevronRightIcon class="size-4 shrink-0 text-settings-muted-foreground" />
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
    {autoLockMinutes}
    autoLockOptions={ALLOWED_AUTO_LOCK_MINUTES}
    onSetAutoLockMinutes={handleSetAutoLockMinutes}
    onOpenRecovery={() => {
      openRecovery('profile-security');
    }}
    onBack={() => {
      activeView = 'home';
    }}
  />
{:else if activeView === 'private-verus'}
  <PrivateVerusSettings
    {walletNetwork}
    {walletSessionKey}
    onOpenRecovery={() => {
      openRecovery('private-verus');
    }}
    onBack={() => {
      activeView = 'home';
      void refreshPrivateStatus();
    }}
  />
{:else if activeView === 'recovery-keys'}
  <RecoveryKeysSettings
    {walletNetwork}
    {walletName}
    backLabel={i18n.t(
      recoveryOrigin === 'private-verus'
        ? 'wallet.settings.recovery.backToPrivateVerus'
        : 'wallet.settings.recovery.backToProfileSecurity'
    )}
    onBack={returnFromRecovery}
  />
{:else if activeView === 'about-support'}
  <AboutSupportSettings
    onBack={() => {
      activeView = 'home';
    }}
  />
{/if}
