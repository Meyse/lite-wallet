<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
  import AppSidebar from '$lib/components/wallet/AppSidebar.svelte';
  import Watchlist from '$lib/components/wallet/sections/Watchlist.svelte';

  let activeSection = $state('watchlist' as const);
  const walletData = {
    name: 'Example wallet',
    emoji: 'E',
    color: 'blue',
    network: 'mainnet' as const,
  };
</script>

<div class="h-screen overflow-hidden">
  <Sidebar.Provider class="h-full min-h-0 overflow-hidden" style="--sidebar-width: 244px;">
    <AppSidebar bind:activeSection {walletData} />
    <Sidebar.Inset class="h-full min-h-0 min-w-0 dark:bg-app-canvas">
      <main class="flex min-h-0 flex-1 overflow-hidden">
        <Watchlist walletNetwork="mainnet" />
      </main>
    </Sidebar.Inset>
  </Sidebar.Provider>
</div>
