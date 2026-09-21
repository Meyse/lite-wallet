<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
  import AppSidebar from '$lib/components/wallet/AppSidebar.svelte';
  import AddAssetSheet from '$lib/components/wallet/AddAssetSheet.svelte';

  let activeSection = $state('overview');
  const walletData = { name: 'Example wallet', emoji: 'E', color: 'blue', network: 'mainnet' };
</script>

<div class="h-[620px] w-[920px] overflow-hidden">
  <Sidebar.Provider class="h-full min-h-0 overflow-hidden" style="--sidebar-width: 244px;">
    <AppSidebar bind:activeSection {walletData} />
    <Sidebar.Inset class="h-full min-h-0 min-w-0 dark:bg-app-canvas">
      <main class="min-h-0 flex-1 overflow-hidden p-5">
        <AddAssetSheet isOpen network="mainnet" />
      </main>
    </Sidebar.Inset>
  </Sidebar.Provider>
</div>
