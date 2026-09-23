<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
  import AppSidebar from '$lib/components/wallet/AppSidebar.svelte';
  import HelpCenterDialog from '$lib/components/common/help/HelpCenterDialog.svelte';
  import HelpCenterLink from '$lib/components/common/HelpCenterLink.svelte';
  import { i18nStore } from '$lib/i18n';
  const i18n = $derived($i18nStore);
  let open = $state(false);
  let destination = $state('');
  let draft = $state('An unsent draft');
</script>

<Sidebar.Provider class="h-screen overflow-hidden">
  <AppSidebar
    walletData={{ name: 'Help preview', emoji: 'V', color: '' }}
    onOpenHelp={() => {
      open = true;
    }}
  />
  <Sidebar.Inset class="p-5">
    <label for="fixture-draft">Draft preserved while reading Help</label>
    <input id="fixture-draft" bind:value={draft} class="my-4 rounded-md border p-2" />
    <p data-fixture-destination>{destination}</p>
    <HelpCenterLink
      linkText={i18n.t('help.link.needHelp')}
      backLabel={i18n.t('helpCenter.backWelcome')}
    />
  </Sidebar.Inset>
</Sidebar.Provider>
<HelpCenterDialog
  bind:open
  backLabel={i18n.t('helpCenter.backWallet')}
  onNavigate={(value) => {
    destination = value;
    open = false;
  }}
/>
