<script>
  import * as Sidebar from '$lib/components/ui/sidebar';
  import AppSidebar from '$lib/components/wallet/AppSidebar.svelte';
  import AddressBook from '$lib/components/wallet/sections/AddressBook.svelte';
  import TransferWizard from '$lib/components/wallet/sections/TransferWizard.svelte';
  import IdentityMention from '$lib/components/wallet/contacts/IdentityMention.svelte';
  let activeSection = $state(
    new URLSearchParams(location.search).get('screen') === 'send' ? 'send' : 'address-book'
  );
  const walletData = { name: 'Example wallet', emoji: 'E', color: 'blue', network: 'mainnet' };
</script>

<div class="h-screen overflow-hidden">
  <Sidebar.Provider class="h-full min-h-0 overflow-hidden" style="--sidebar-width: 244px;">
    <AppSidebar bind:activeSection {walletData} />
    <Sidebar.Inset class="h-full min-h-0 min-w-0 dark:bg-app-canvas"
      ><main class="flex min-h-0 flex-1 overflow-hidden">
        {#if activeSection === 'address-book'}<AddressBook />{:else}<TransferWizard
            entryIntent="send"
            walletNetwork="mainnet"
            walletKey="fixture-session"
          />{/if}
      </main></Sidebar.Inset
    >
  </Sidebar.Provider>
</div>
