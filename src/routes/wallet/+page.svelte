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
  import {
    forceWalletToUnlock,
    isForcedWalletLockError,
    walletUnlockRedirectingStore,
  } from '$lib/services/walletLockCoordinator.js';
  import * as walletService from '$lib/services/walletService.js';
  import * as coinsService from '$lib/services/coinsService.js';
  import * as addressBookService from '$lib/services/addressBookService.js';
  import { setupWalletEventBridge } from '$lib/services/eventBridge.js';
  import { resetWalletDisplaySession } from '$lib/services/walletDisplayService.js';
  import { balanceStore } from '$lib/stores/balances.js';
  import { ratesStore } from '$lib/stores/rates.js';
  import { networkStore } from '$lib/stores/network.js';
  import { transactionStore } from '$lib/stores/transactions.js';
  import { walletBootstrapStore } from '$lib/stores/walletBootstrap.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import {
    buildWalletChannels,
    resetWalletChannels,
    walletChannelsStore,
  } from '$lib/stores/walletChannels.js';
  import { clearCoinScopes } from '$lib/stores/coinScopes.js';
  import {
    clearWalletErrors,
    pushWalletBackgroundError,
    pushWalletError,
  } from '$lib/stores/walletErrors.js';
  import { setAddressBookContacts } from '$lib/stores/addressBook.js';
  import { settingsStore } from '$lib/stores/settings.js';
  import { isWalletSupportedAsset } from '$lib/coins/supportedAssets.js';
  import { normalizeAutoLockMinutes } from '$lib/security/sessionTimeout.js';
  import { DisposableScope } from '$lib/utils/disposableScope.js';
  import { i18nStore } from '$lib/i18n';
  import type { CoinDefinition, WalletNetwork } from '$lib/types/wallet.js';

  const sessionCoinsByWallet = new Map<string, CoinDefinition[]>();

  function activeAssetsCacheKey(walletName: string, network: WalletNetwork): string {
    return `${walletName.trim().toLowerCase()}::${network}`;
  }

  function normalizeCoinId(value: string): string {
    return value.trim().toLowerCase();
  }

  function filterCoinsByActiveIds(
    coins: CoinDefinition[],
    activeCoinIds: string[]
  ): CoinDefinition[] {
    const activeSet = new Set(
      activeCoinIds.map((coinId) => normalizeCoinId(coinId)).filter((coinId) => coinId.length > 0)
    );
    if (activeSet.size === 0) return [];

    return coins.filter((coin) => activeSet.has(normalizeCoinId(coin.id)));
  }

  let loading = $state(true);
  let walletData = $state<{
    name: string;
    emoji: string;
    color: string;
    network: WalletNetwork;
  } | null>(null);
  let routeScope: DisposableScope | null = null;
  let handlingSessionExpiry = $state(false);
  const i18n = $derived($i18nStore);
  const redirectingToUnlock = $derived($walletUnlockRedirectingStore);

  function rethrowForcedWalletLock(error: unknown): void {
    if (isForcedWalletLockError(error)) {
      throw error;
    }
  }

  onMount(async () => {
    const scope = new DisposableScope();
    const dashboardStartedAt = performance.now();
    routeScope = scope;
    resetWalletDisplaySession();
    walletBootstrapStore.set(true);
    clearWalletErrors();
    balanceStore.set({});
    networkStore.set({});
    ratesStore.set({});
    transactionStore.set({});
    clearCoinScopes();
    try {
      const unlocked = await walletService.isUnlocked();
      if (!scope.active) return;
      if (!unlocked) {
        walletBootstrapStore.set(false);
        await goto('/', { replaceState: true });
        return;
      }

      const handleSessionExpired = async () => {
        if (!scope.active || handlingSessionExpiry) return;
        handlingSessionExpiry = true;
        walletBootstrapStore.set(false);
        await forceWalletToUnlock();
      };

      const eventBridgePromise = setupWalletEventBridge({
        onFirstBalance: () => {
          console.info(
            `[WALLET_PERF] dashboard phase=first_balance_event_received elapsed_ms=${Math.round(performance.now() - dashboardStartedAt)}`
          );
        },
        onFirstRates: () => {
          console.info(
            `[WALLET_PERF] dashboard phase=first_non_empty_rate_event_received elapsed_ms=${Math.round(performance.now() - dashboardStartedAt)}`
          );
        },
        onSessionExpired: handleSessionExpired,
      })
        .then((teardown) => (scope.add(teardown) ? teardown : null))
        .catch((error) => {
          if (!scope.active) return null;
          console.error('[WALLET_ROUTE] Failed to setup wallet event bridge', error);
          walletBootstrapStore.set(false);
          pushWalletBackgroundError(
            error instanceof Error ? error.message : i18n.t('common.unknownError')
          );
          return null;
        });

      const resolvedAutoLockMinutes = normalizeAutoLockMinutes(get(settingsStore).autoLockMinutes);
      void walletService.setSessionTimeoutMinutes(resolvedAutoLockMinutes).catch((error) => {
        if (!scope.active) return;
        console.error('[WALLET_ROUTE] Failed to apply session timeout', error);
      });
      scope.add(startWalletActivityMonitor());

      const [active, addresses, allCoins, activeAssets] = await Promise.all([
        walletService.getActiveWallet().catch((error) => {
          if (!scope.active) return null;
          console.error('[WALLET_ROUTE] Failed to resolve active wallet', error);
          return null;
        }),
        walletService.getAddresses().catch((error) => {
          if (!scope.active) return null;
          rethrowForcedWalletLock(error);
          console.error('[WALLET_ROUTE] Failed to load wallet addresses', error);
          return null;
        }),
        coinsService.getCoinRegistry().catch((error) => {
          if (!scope.active) return [];
          console.error('[WALLET_ROUTE] Failed to load coin registry', error);
          return [];
        }),
        walletService.getActiveAssets().catch((error) => {
          if (!scope.active) return null;
          rethrowForcedWalletLock(error);
          console.error('[WALLET_ROUTE] Failed to load active assets state', error);
          return null;
        }),
      ]);
      if (!scope.active) return;
      const walletNetwork: WalletNetwork = active?.network ?? 'mainnet';
      walletData = active
        ? {
            name: active.wallet_name,
            emoji: active.emoji || '💰',
            color: active.color || 'blue',
            network: walletNetwork,
          }
        : {
            name: i18n.t('wallet.overview.mainWallet'),
            emoji: '💰',
            color: 'blue',
            network: walletNetwork,
          };
      const cacheKey = activeAssetsCacheKey(walletData.name, walletNetwork);

      if (!addresses) {
        pushWalletError(i18n.t('wallet.receive.errorLoad'));
      }

      const supportedCoins = allCoins.filter((coin) => isWalletSupportedAsset(coin, walletNetwork));

      let coins: CoinDefinition[] = [];
      if (activeAssets) {
        coins = filterCoinsByActiveIds(supportedCoins, activeAssets.coinIds);
        sessionCoinsByWallet.set(cacheKey, coins);
      } else {
        const previousSessionCoins = sessionCoinsByWallet.get(cacheKey) ?? get(coinsStore);
        const fallbackCoins = previousSessionCoins.length > 0 ? previousSessionCoins : [];
        coins = fallbackCoins;
        sessionCoinsByWallet.set(cacheKey, fallbackCoins);
        pushWalletError(i18n.t('wallet.overview.errorActiveAssetsFallback'));
      }

      coinsStore.set(coins);

      const channels = buildWalletChannels(coins, addresses?.vrsc_address ?? null);
      walletChannelsStore.set(channels);
      loading = false;
      console.info(
        `[WALLET_PERF] dashboard phase=essential_metadata elapsed_ms=${Math.round(performance.now() - dashboardStartedAt)}`
      );

      void addressBookService
        .listAddressBookContacts()
        .then((contacts) => {
          if (scope.active) setAddressBookContacts(contacts);
        })
        .catch(async (error) => {
          if (!scope.active) return;
          if (isForcedWalletLockError(error)) {
            await handleSessionExpired();
            return;
          }
          console.error('[WALLET_ROUTE] Failed to load address book contacts', error);
        });

      const teardownEventBridge = await eventBridgePromise;
      if (!scope.active || !teardownEventBridge) return;

      await walletService
        .startUpdateEngine({
          includeTransactions: false,
          priorityCoinIds: coins.map((coin) => coin.id),
          priorityChannelIds: channels.channels,
        })
        .catch((error) => {
          if (!scope.active) return;
          rethrowForcedWalletLock(error);
          console.error('[WALLET_ROUTE] Failed to start update engine', error);
          walletBootstrapStore.set(false);
          pushWalletBackgroundError(
            error instanceof Error ? error.message : i18n.t('common.unknownError')
          );
        });
    } catch (error) {
      if (!scope.active) return;
      if (isForcedWalletLockError(error)) {
        return;
      }

      console.error('[WALLET_ROUTE] Startup failed', error);
      walletBootstrapStore.set(false);
      const message = error instanceof Error ? error.message : i18n.t('common.unknownError');
      pushWalletBackgroundError(message);
      if (!walletData) {
        walletData = {
          name: i18n.t('wallet.overview.mainWallet'),
          emoji: '💰',
          color: 'blue',
          network: 'mainnet',
        };
      }
    } finally {
      if (scope.active) {
        loading = false;
      }
    }
  });

  onDestroy(() => {
    routeScope?.dispose();
    routeScope = null;
    resetWalletDisplaySession();
    walletBootstrapStore.set(false);
    balanceStore.set({});
    networkStore.set({});
    ratesStore.set({});
    transactionStore.set({});
    clearCoinScopes();
    resetWalletChannels();
    setAddressBookContacts([]);
  });
</script>

{#if loading || redirectingToUnlock}
  <main class="flex min-h-screen items-center justify-center bg-background">
    <div class="text-muted-foreground">{i18n.t('common.loading')}</div>
  </main>
{:else if walletData}
  <WalletLayout {walletData} />
{/if}
