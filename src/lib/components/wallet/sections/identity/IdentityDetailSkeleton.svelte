<script lang="ts">
  import { contactChainId } from '$lib/contacts/identity';
  import { contactSession } from '$lib/contacts/session';
  import type {
    IdentityProfileLoadResult,
    LinkedIdentity,
    PendingIdentityProfileUpdate,
    WalletNetwork,
  } from '$lib/types/wallet.js';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay';
  import VerusIdProfilePage from './VerusIdProfilePage.svelte';

  let {
    identity,
    profile = null,
    profileLoading = false,
    pendingProfile = null,
    network,
    unlinking = false,
    onBack = () => {},
    onUnlink = () => {},
  }: {
    identity: LinkedIdentity | null;
    profile?: IdentityProfileLoadResult | null;
    profileLoading?: boolean;
    pendingProfile?: PendingIdentityProfileUpdate | null;
    network?: WalletNetwork;
    unlinking?: boolean;
    onBack?: () => void;
    onUnlink?: () => void;
  } = $props();

  const activeNetwork = $derived(network ?? $contactSession?.network ?? 'mainnet');
  const displayName = $derived(identity ? formatIdentityDisplayName(identity) : '');
</script>

{#if identity}
  <VerusIdProfilePage
    identity={{
      identityAddress: identity.identityAddress,
      fullyQualifiedName: displayName,
      network: activeNetwork,
      chainId: identity.systemId || contactChainId(activeNetwork),
      status: identity.status,
    }}
    linked={true}
    owner={{
      details: null,
      loading: true,
      profile,
      profileLoading,
      pending: pendingProfile,
      unlinking,
      onEdit: () => {},
      onUnlink,
    }}
    {onBack}
  />
{/if}
