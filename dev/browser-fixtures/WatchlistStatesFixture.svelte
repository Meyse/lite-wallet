<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
  import AppSidebar from '$lib/components/wallet/AppSidebar.svelte';
  import Watchlist from '$lib/components/wallet/sections/Watchlist.svelte';
</script>

<div class="h-screen overflow-hidden" data-testid="watchlist-fixture">
  <Sidebar.Provider class="h-full overflow-hidden">
    <AppSidebar
      activeSection="watchlist"
      walletData={{ name: 'Example wallet', emoji: 'E', color: 'blue' }}
    />
    <Sidebar.Inset class="h-full min-h-0 min-w-0 dark:bg-app-canvas">
      <div class="h-6 shrink-0" aria-hidden="true"></div>
      <main class="flex min-h-0 flex-1 overflow-hidden">
        <Watchlist walletNetwork="mainnet" />
      </main>
    </Sidebar.Inset>
  </Sidebar.Provider>
</div>
