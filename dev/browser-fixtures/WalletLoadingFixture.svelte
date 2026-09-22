<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
  import AppSidebar from '$lib/components/wallet/AppSidebar.svelte';
  import Overview from '$lib/components/wallet/sections/Overview.svelte';
  import AssetDetails from '$lib/components/wallet/sections/AssetDetails.svelte';
  import Settings from '$lib/components/wallet/sections/Settings.svelte';
  import AddressBook from '$lib/components/wallet/sections/AddressBook.svelte';
  let { screen = 'overview' }: { screen?: string } = $props();
  let activeSection = $state('overview');
  $effect(() => {
    activeSection =
      screen === 'settings' ? 'settings' : screen === 'contacts' ? 'address-book' : 'overview';
  });
  const walletData = {
    name: 'Example wallet',
    emoji: 'E',
    color: 'blue',
    network: 'mainnet' as const,
  };
</script>

<div class="h-[620px] w-[920px] overflow-hidden">
  <Sidebar.Provider class="h-full min-h-0 overflow-hidden" style="--sidebar-width: 244px;">
    <AppSidebar bind:activeSection {walletData} />
    <Sidebar.Inset class="h-full min-h-0 min-w-0 dark:bg-app-canvas">
      <main
        class="min-h-0 flex-1 overflow-hidden"
        class:pt-5={screen === 'details' || screen === 'overview'}
      >
        {#if screen === 'details'}
          <AssetDetails coinId="ETH" />
        {:else if screen === 'settings'}
          <Settings
            walletNetwork="mainnet"
            walletName="Example wallet"
            walletSessionKey="loading-fixture"
          />
        {:else if screen === 'contacts'}
          <AddressBook />
        {:else}
          <Overview {walletData} />
        {/if}
      </main>
    </Sidebar.Inset>
  </Sidebar.Provider>
</div>
