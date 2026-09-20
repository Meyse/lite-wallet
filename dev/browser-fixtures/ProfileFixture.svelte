<script lang="ts">
  import AppSidebar from '$lib/components/wallet/AppSidebar.svelte';
  import * as Sidebar from '$lib/components/ui/sidebar';
  import VerusIdProfilePage from '$lib/components/wallet/sections/identity/VerusIdProfilePage.svelte';
  import { provideContactNavigation } from '$lib/contacts/navigation';
  import type { ResolvedContactIdentity } from '$lib/types/addressBook';
  import type { PublicProfileContent } from '$lib/components/wallet/sections/identity/publicProfileContent';
  let { identity, content }: { identity: ResolvedContactIdentity; content: PublicProfileContent } =
    $props();
  let activeSection = $state('identity');
  const params = new URLSearchParams(location.search);
  const walletData = { name: 'Example wallet', emoji: 'E', color: 'blue', network: 'mainnet' };
  let destination = $state('');
  provideContactNavigation(() => {
    destination = 'Contacts';
  });
</script>

<div class="h-screen overflow-hidden">
  <Sidebar.Provider class="h-full min-h-0 overflow-hidden" style="--sidebar-width: 244px;">
    <AppSidebar bind:activeSection {walletData} />
    <Sidebar.Inset class="h-full min-h-0 min-w-0 dark:bg-app-canvas">
      <main class="flex min-h-0 flex-1 overflow-hidden">
        {#if destination}<div class="p-5">Fixture destination: {destination}</div>{:else}
          <VerusIdProfilePage
            {identity}
            {content}
            linked={params.has('linked')}
            onBack={() => (destination = 'Search')}
            onSend={() => (destination = 'Send')}
          />
        {/if}
      </main>
    </Sidebar.Inset>
  </Sidebar.Provider>
</div>
