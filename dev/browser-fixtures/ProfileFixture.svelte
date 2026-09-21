<script lang="ts">
  import { untrack } from 'svelte';
  import AppSidebar from '$lib/components/wallet/AppSidebar.svelte';
  import * as Sidebar from '$lib/components/ui/sidebar';
  import VerusIdProfilePage from '$lib/components/wallet/sections/identity/VerusIdProfilePage.svelte';
  import IdentityDetailView from '$lib/components/wallet/sections/identity/IdentityDetailView.svelte';
  import type { IdentityProfileLoadResult, PendingIdentityProfileUpdate } from '$lib/types/wallet';
  import { provideContactNavigation } from '$lib/contacts/navigation';
  import type { ResolvedContactIdentity } from '$lib/types/addressBook';
  import type { PublicProfileContent } from '$lib/components/wallet/sections/identity/publicProfileContent';
  let {
    identity,
    content,
    initialProfile,
  }: {
    identity: ResolvedContactIdentity;
    content: PublicProfileContent;
    initialProfile?: IdentityProfileLoadResult;
  } = $props();
  let profile = $state(
    untrack(() => initialProfile ?? { state: 'empty' as const, issues: [], revisionTxid: null })
  );
  let pending = $state<PendingIdentityProfileUpdate | null>(null);
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
        {#if destination}<div class="p-5">Fixture destination: {destination}</div>
        {:else if params.has('editor')}
          <IdentityDetailView
            details={{
              identityAddress: identity.identityAddress,
              name: identity.fullyQualifiedName,
              status: 'active',
              primaryAddresses: ['RFixture'],
              ownedByPrimaryAddress: true,
              minimumSignatures: 1,
              tokenizedControl: false,
              profileEditable: true,
              warnings: [],
            }}
            {profile}
            pendingProfile={pending}
            network="testnet"
            onProfileSubmitted={(receipt) => (pending = receipt)}
            onProfileSettled={() => (pending = null)}
            onProfileConfirmed={(_receipt, canonical) => (profile = canonical)}
            onBack={() => (destination = 'Profile')}
          />
        {:else}
          <VerusIdProfilePage
            {identity}
            {content}
            linked={params.has('linked') || params.has('owner')}
            owner={params.has('owner')
              ? {
                  details: {
                    identityAddress: identity.identityAddress,
                    name: identity.fullyQualifiedName,
                    fullyQualifiedName: identity.fullyQualifiedName,
                    status: 'active',
                    systemDisplayName: 'Verus',
                    primaryAddresses: ['RFixture'],
                    ownedByPrimaryAddress: true,
                    minimumSignatures: 1,
                    tokenizedControl: false,
                    revocationAuthority: identity.identityAddress,
                    recoveryAuthority: identity.identityAddress,
                    profileEditable: true,
                    warnings: [],
                  },
                  profile: {
                    state: 'ready',
                    avatar: null,
                    description: null,
                    issues: [],
                    readHeight: 1,
                    revisionTxid: 'fixture',
                  },
                  profileLoading: false,
                  pending: null,
                  unlinking: false,
                  onEdit: () => (destination = 'Edit profile'),
                  onUnlink: () => (destination = 'Identities'),
                }
              : undefined}
            onBack={() => (destination = 'Search')}
            onSend={() => (destination = 'Send')}
          />
        {/if}
      </main>
    </Sidebar.Inset>
  </Sidebar.Provider>
</div>
