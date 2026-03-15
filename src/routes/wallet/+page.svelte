<!--
  Route: /wallet
  Purpose: Main wallet dashboard; guarded - redirects to / if locked. Sets up event bridge, loads coins, starts update engine polling.
  Last Updated: Module 10 — startup polling owned by backend update engine
  Security: No sensitive operations - display only
-->

<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { goto } from '$app/navigation';
  import WalletLayout from '$lib/components/wallet/WalletLayout.svelte';
  import { startWalletActivityMonitor } from '$lib/services/walletActivityMonitor.js';
  import { forceWalletToUnlock, isForcedWalletLockError, walletUnlockRedirectingStore } from '$lib/services/walletLockCoordinator.js';
  import * as walletService from '$lib/services/walletService.js';
  import * as coinsService from '$lib/services/coinsService.js';
  import * as addressBookService from '$lib/services/addressBookService.js';
  import { setupWalletEventBridge } from '$lib/services/eventBridge.js';
  import { balanceStore } from '$lib/stores/balances.js';
  import { ratesStore } from '$lib/stores/rates.js';
  import { transactionStore } from '$lib/stores/transactions.js';
  import { walletBootstrapStore } from '$lib/stores/walletBootstrap.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { buildWalletChannels, resetWalletChannels, walletChannelsStore } from '$lib/stores/walletChannels.js';
  import { clearCoinScopes } from '$lib/stores/coinScopes.js';
  import { clearWalletErrors, pushWalletError } from '$lib/stores/walletErrors.js';
  import { setAddressBookContacts } from '$lib/stores/addressBook.js';
  import { settingsStore } from '$lib/stores/settings.js';
  import { isWalletSupportedAsset } from '$lib/coins/supportedAssets.js';
  import { normalizeAutoLockMinutes } from '$lib/security/sessionTimeout.js';
  import { i18nStore } from '$lib/i18n';
  import type { CoinDefinition, WalletNetwork } from '$lib/types/wallet.js';

  const sessionCoinsByWallet = new Map<string, CoinDefinition[]>();

  function activeAssetsCacheKey(walletName: string, network: WalletNetwork): string {
    return `${walletName.trim().toLowerCase()}::${network}`;
  }

  function normalizeCoinId(value: string): string {
    return value.trim().toLowerCase();
  }

  function filterCoinsByActiveIds(coins: CoinDefinition[], activeCoinIds: string[]): CoinDefinition[] {
    const activeSet = new Set(
      activeCoinIds
        .map((coinId) => normalizeCoinId(coinId))
        .filter((coinId) => coinId.length > 0)
    );
    if (activeSet.size === 0) return [];

    return coins.filter((coin) => activeSet.has(normalizeCoinId(coin.id)));
  }

  let loading = $state(true);
  let walletData = $state<{ name: string; emoji: string; color: string; network: WalletNetwork } | null>(null);
  let teardownActivityMonitor: (() => void) | null = null;
  let teardownEventBridge: (() => void) | null = null;
  let handlingSessionExpiry = $state(false);
  const i18n = $derived($i18nStore);
  const redirectingToUnlock = $derived($walletUnlockRedirectingStore);

  function rethrowForcedWalletLock(error: unknown): void {
    if (isForcedWalletLockError(error)) {
      throw error;
    }
  }

  onMount(async () => {
    walletBootstrapStore.set(true);
    clearWalletErrors();
    balanceStore.set({});
    ratesStore.set({});
    transactionStore.set({});
    clearCoinScopes();
    try {
      const unlocked = await walletService.isUnlocked();
      if (!unlocked) {
        walletBootstrapStore.set(false);
        await goto('/', { replaceState: true });
        return;
      }

      const resolvedAutoLockMinutes = normalizeAutoLockMinutes(get(settingsStore).autoLockMinutes);
      await walletService.setSessionTimeoutMinutes(resolvedAutoLockMinutes).catch((error) => {
        console.error('[WALLET_ROUTE] Failed to apply session timeout', error);
      });
      teardownActivityMonitor = startWalletActivityMonitor();

      const active = await walletService.getActiveWallet().catch((error) => {
        console.error('[WALLET_ROUTE] Failed to resolve active wallet', error);
        return null;
      });
      const walletNetwork: WalletNetwork = active?.network ?? 'mainnet';
      walletData = active
        ? {
            name: active.wallet_name,
            emoji: active.emoji || '💰',
            color: active.color || 'blue',
            network: walletNetwork
          }
        : { name: i18n.t('wallet.overview.mainWallet'), emoji: '💰', color: 'blue', network: walletNetwork };
      const cacheKey = activeAssetsCacheKey(walletData.name, walletNetwork);

      const addresses = await walletService.getAddresses().catch((error) => {
        rethrowForcedWalletLock(error);
        console.error('[WALLET_ROUTE] Failed to load wallet addresses', error);
        return null;
      });
      if (!addresses) {
        pushWalletError(i18n.t('wallet.receive.errorLoad'));
      }

      const allCoins = await coinsService.getCoinRegistry().catch((error) => {
        console.error('[WALLET_ROUTE] Failed to load coin registry', error);
        return [];
      });
      const supportedCoins = allCoins.filter((coin) => isWalletSupportedAsset(coin, walletNetwork));

      let coins: CoinDefinition[] = [];
      try {
        const activeAssets = await walletService.getActiveAssets();
        coins = filterCoinsByActiveIds(supportedCoins, activeAssets.coinIds);
        sessionCoinsByWallet.set(cacheKey, coins);
      } catch (error) {
        rethrowForcedWalletLock(error);
        console.error('[WALLET_ROUTE] Failed to load active assets state', error);
        const previousSessionCoins = sessionCoinsByWallet.get(cacheKey) ?? get(coinsStore);
        const fallbackCoins = previousSessionCoins.length > 0 ? previousSessionCoins : [];
        coins = fallbackCoins;
        sessionCoinsByWallet.set(cacheKey, fallbackCoins);
        pushWalletError(i18n.t('wallet.overview.errorActiveAssetsFallback'));
      }

      coinsStore.set(coins);

      const channels = buildWalletChannels(coins, addresses?.vrsc_address ?? null);
      walletChannelsStore.set(channels);

      const contacts = await addressBookService.listAddressBookContacts().catch((error) => {
        rethrowForcedWalletLock(error);
        console.error('[WALLET_ROUTE] Failed to load address book contacts', error);
        return [];
      });
      setAddressBookContacts(contacts);

      teardownEventBridge = await setupWalletEventBridge({
        onSessionExpired: async () => {
          if (handlingSessionExpiry) return;
          handlingSessionExpiry = true;
          walletBootstrapStore.set(false);
          await forceWalletToUnlock();
        }
      }).catch((error) => {
        console.error('[WALLET_ROUTE] Failed to setup wallet event bridge', error);
        walletBootstrapStore.set(false);
        pushWalletError(error instanceof Error ? error.message : i18n.t('common.unknownError'));
        return null;
      });

      await walletService.startUpdateEngine({
        includeTransactions: false,
        priorityCoinIds: coins.map((coin) => coin.id),
        priorityChannelIds: channels.channels
      }).catch((error) => {
        rethrowForcedWalletLock(error);
        console.error('[WALLET_ROUTE] Failed to start update engine', error);
        walletBootstrapStore.set(false);
        pushWalletError(error instanceof Error ? error.message : i18n.t('common.unknownError'));
      });
    } catch (error) {
      if (isForcedWalletLockError(error)) {
        return;
      }

      console.error('[WALLET_ROUTE] Startup failed', error);
      walletBootstrapStore.set(false);
      const message = error instanceof Error ? error.message : i18n.t('common.unknownError');
      pushWalletError(message);
      if (!walletData) {
        walletData = {
          name: i18n.t('wallet.overview.mainWallet'),
          emoji: '💰',
          color: 'blue',
          network: 'mainnet'
        };
      }
    } finally {
      loading = false;
    }
  });

  onDestroy(() => {
    teardownActivityMonitor?.();
    teardownEventBridge?.();
    walletBootstrapStore.set(false);
    balanceStore.set({});
    ratesStore.set({});
    transactionStore.set({});
    clearCoinScopes();
    resetWalletChannels();
    setAddressBookContacts([]);
  });
</script>

{#if loading || redirectingToUnlock}
  <main class="bg-background flex min-h-screen items-center justify-center">
    <div class="text-muted-foreground">{i18n.t('common.loading')}</div>
  </main>
{:else if walletData}
  <WalletLayout walletData={walletData} />
{/if}
