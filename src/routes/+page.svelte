<!--
  Root Page Route
  Branches on list_wallets + is_unlocked: WelcomeScreen (no wallets), UnlockScreen (locked), or redirect to /wallet.
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { invoke } from '@tauri-apps/api/core';
  import { i18nStore } from '$lib/i18n';
  import LanguageGate from '$lib/components/wallet/LanguageGate.svelte';
  import WelcomeScreen from '$lib/components/wallet/WelcomeScreen.svelte';
  import UnlockScreen from '$lib/components/wallet/UnlockScreen.svelte';

  type WalletListItem = {
    account_id: string;
    wallet_name: string;
    network?: 'mainnet' | 'testnet';
    emoji?: string;
    color?: string;
  };

  let loading = $state(true);
  let wallets = $state<WalletListItem[]>([]);
  let unlocked = $state(false);
  let showLanguageGate = $state(false);
  const i18n = $derived($i18nStore);
  const LANGUAGE_GATE_SEEN_KEY = 'lite_wallet_language_gate_seen_v1';

  function hasSeenLanguageGate(): boolean {
    if (typeof globalThis.localStorage === 'undefined') return false;
    try {
      return globalThis.localStorage.getItem(LANGUAGE_GATE_SEEN_KEY) === '1';
    } catch {
      return false;
    }
  }

  function markLanguageGateSeen(): void {
    if (typeof globalThis.localStorage === 'undefined') return;
    try {
      globalThis.localStorage.setItem(LANGUAGE_GATE_SEEN_KEY, '1');
    } catch {
      // Ignore local storage failures.
    }
  }

  function handleLanguageGateContinue(): void {
    markLanguageGateSeen();
    showLanguageGate = false;
  }

  onMount(async () => {
    try {
      const [listResult, unlockedResult] = await Promise.all([
        invoke<WalletListItem[]>('list_wallets').catch(() => []),
        invoke<boolean>('is_unlocked').catch(() => false)
      ]);
      wallets = Array.isArray(listResult) ? listResult : [];
      unlocked = !!unlockedResult;

      if (wallets.length > 0 && unlocked) {
        goto('/wallet');
        return;
      }
      showLanguageGate = wallets.length === 0 && !hasSeenLanguageGate();
    } catch {
      wallets = [];
      unlocked = false;
      showLanguageGate = !hasSeenLanguageGate();
    } finally {
      loading = false;
    }
  });
</script>

{#if loading || (wallets.length > 0 && unlocked)}
  <main class="bg-background flex min-h-screen flex-col items-center justify-center">
    <div class="bg-app-canvas absolute inset-0"></div>
    <div class="text-muted-foreground relative z-10">{i18n.t('common.loading')}</div>
  </main>
{:else if wallets.length === 0}
  {#if showLanguageGate}
    <LanguageGate onContinue={handleLanguageGateContinue} />
  {:else}
    <WelcomeScreen />
  {/if}
{:else}
  <UnlockScreen {wallets} />
{/if}
