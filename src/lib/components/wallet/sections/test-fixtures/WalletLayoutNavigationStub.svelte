<script lang="ts">
  import { getContactNavigation } from '$lib/contacts/navigation';
  import type { AddressContactPrefill, ContactReturnState } from '$lib/contacts/navigation';
  import type { AddressBookContact, ContactIdentity } from '$lib/types/addressBook';
  import type { WatchlistEntry } from '$lib/types/watchlist';
  import type { IdentitySectionSessionState } from '../identity/identitySectionSessionState';

  let {
    onNavigate,
    onNavigateToSend,
    onViewIdentityProfile,
    onCreateAddressContact,
    initialSelectedEntryId,
    requestedContactId,
    createPrefill,
    returnState,
    onReturn,
    returnLabel,
    onContactCreated,
    onViewProfile,
    sessionState,
    onReturnToContacts,
    onReturnToWatchlist,
    onSend,
    onNavigationStateChange,
  }: {
    onNavigate?: (section: 'watchlist' | 'address-book' | 'identity' | 'apps') => void;
    onNavigateToSend?: () => void;
    onViewIdentityProfile?: (entry: WatchlistEntry) => void;
    onCreateAddressContact?: (entry: WatchlistEntry) => void;
    initialSelectedEntryId?: string | null;
    requestedContactId?: string | null;
    createPrefill?: AddressContactPrefill | null;
    returnState?: ContactReturnState | null;
    onReturn?: () => void;
    returnLabel?: string;
    onContactCreated?: (contact: AddressBookContact) => void;
    onViewProfile?: (identity: ContactIdentity, state: ContactReturnState) => void;
    sessionState?: IdentitySectionSessionState;
    onReturnToContacts?: (state: ContactReturnState) => void;
    onReturnToWatchlist?: () => void;
    onSend?: (identity: ContactIdentity) => void;
    onNavigationStateChange?: (state: unknown) => void;
  } = $props();

  const openContact = getContactNavigation();
  const identity: ContactIdentity = {
    identityAddress: `i${'b'.repeat(33)}`,
    fullyQualifiedName: 'Bob@',
    network: 'mainnet',
    chainId: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
  };
  const rawEntry: WatchlistEntry = {
    id: 'watched-raw',
    targetKind: 'address',
    displayName: 'Mum',
    address: `R${'a'.repeat(33)}`,
    createdAt: 1,
    updatedAt: 1,
  };
  const identityEntry: WatchlistEntry = {
    ...rawEntry,
    id: 'watched-identity',
    targetKind: 'identity',
    displayName: 'Bob@',
    address: identity.identityAddress,
  };
  const contact: AddressBookContact = {
    id: 'new-contact',
    displayName: 'Mum',
    note: null,
    endpoints: [],
    createdAt: 1,
    updatedAt: 1,
  };
  const profileReturnState = $derived(
    sessionState?.publicProfile?.origin.kind === 'contacts'
      ? sessionState.publicProfile.origin.returnState
      : null
  );
</script>

{#if onNavigate}
  <button type="button" onclick={() => onNavigate?.('watchlist')}>Watchlist</button>
  <button type="button" onclick={() => onNavigate?.('address-book')}>Contacts</button>
  <button type="button" onclick={() => onNavigate?.('identity')}>VerusID</button>
  <button type="button" onclick={() => onNavigate?.('apps')}>Apps</button>
{:else if onViewIdentityProfile}
  <div data-watchlist-detail={initialSelectedEntryId ?? ''}>
    <button type="button" onclick={() => onViewIdentityProfile?.(identityEntry)}
      >View watched profile</button
    >
    <button type="button" onclick={() => onCreateAddressContact?.(rawEntry)}
      >Open watched address</button
    >
  </div>
{:else if onViewProfile}
  <div
    data-contact-request={requestedContactId ?? ''}
    data-contact-return={returnState?.contactId ?? ''}
    data-contact-prefill={createPrefill?.address ?? ''}
  >
    <button
      type="button"
      onclick={() => onViewProfile?.(identity, { contactId: 'contact-b', searchTerm: 'Bob' })}
      >Go to profile B</button
    >
    {#if onReturn}<button type="button" onclick={onReturn}>{returnLabel}</button>{/if}
    {#if createPrefill}
      <button type="button" onclick={onReturn}>Cancel watched address</button>
      <button type="button" onclick={() => onContactCreated?.(contact)}>Save watched address</button
      >
    {/if}
  </div>
{:else if sessionState}
  {#if profileReturnState}
    <button type="button" onclick={() => onReturnToContacts?.(profileReturnState!)}
      >Back to Contacts</button
    >
  {:else if sessionState.publicProfile?.origin.kind === 'watchlist'}
    <div
      data-profile-kind="watchlist"
      data-profile-address={sessionState.publicProfile.identity.identityAddress}
    >
      <button type="button" onclick={onReturnToWatchlist}>Back to Watchlist</button>
    </div>
  {/if}
  {#if sessionState.publicProfile}<button type="button" onclick={() => onSend?.(identity)}
      >Send from profile</button
    >{/if}
{:else if onNavigationStateChange}
  <div data-transfer-stub>
    <button type="button" onclick={() => openContact?.(identity, null)}>View in contacts</button>
  </div>
{:else if onNavigateToSend}
  <button type="button" onclick={onNavigateToSend}>Start send</button>
{/if}
