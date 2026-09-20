<script lang="ts">
  import BookUserIcon from '@lucide/svelte/icons/book-user';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import MinusIcon from '@lucide/svelte/icons/minus';
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
  import { onDestroy, tick, untrack } from 'svelte';
  import SearchInput from '$lib/components/common/SearchInput.svelte';
  import InlineTextActionButton from '$lib/components/common/InlineTextActionButton.svelte';
  import WalletEmptyState from '$lib/components/wallet/WalletEmptyState.svelte';
  import * as Dialog from '$lib/components/ui/dialog';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import * as Tabs from '$lib/components/ui/tabs';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { i18nStore } from '$lib/i18n';
  import { isForcedWalletLockError } from '$lib/services/walletLockCoordinator.js';
  import { extractWalletErrorType } from '$lib/utils/walletErrors.js';
  import {
    addressBookStore,
    removeAddressBookContact,
    upsertAddressBookContact,
  } from '$lib/stores/addressBook';
  import * as addressBookService from '$lib/services/addressBookService';
  import type {
    AddressBookContact,
    AddressEndpointKind,
    ContactIdentity,
  } from '$lib/types/addressBook';

  import ContactDetail from '../contacts/ContactDetail.svelte';
  import ContactAvatar from '../contacts/ContactAvatar.svelte';
  import PublicProfile from '../contacts/PublicProfile.svelte';
  import IdentityLookup from '../contacts/IdentityLookup.svelte';
  import {
    contactName,
    contactProfile,
    endpointIdentity,
    identityKey,
    matchingContacts,
  } from '$lib/contacts/identity';
  import { loadContacts, resolveContactIdentity } from '$lib/contacts/service';
  import { contactSession, contactsLoadState } from '$lib/contacts/session';

  type EndpointDraft = {
    id?: string;
    label?: string;
    address: string;
    kind: AddressEndpointKind | null;
    identity?: ContactIdentity;
    resolving?: boolean;
  };

  type FormMode = 'create' | 'edit' | null;

  let {
    requestedIdentity = null,
    onReturn,
    returnLabel = '',
  }: {
    requestedIdentity?: ContactIdentity | null;
    onReturn?: () => void;
    returnLabel?: string;
  } = $props();

  const i18n = $derived($i18nStore);
  const contacts = $derived($addressBookStore);

  let searchTerm = $state('');
  let selectedContactId = $state<string | null>(null);

  let formMode = $state<FormMode>(null);
  let formContactId = $state<string | null>(null);
  let formDisplayName = $state('');
  let formIdentities = $state<ContactIdentity[]>([]);
  let selectedProfileKey = $state<string | null>(null);
  let lookupMode = $state(false);
  let formGeneration = 0;
  let alive = true;
  onDestroy(() => {
    alive = false;
    formGeneration++;
  });
  const formProfile = $derived(
    formIdentities.length === 1
      ? formIdentities[0]
      : (formIdentities.find((identity) => identityKey(identity) === selectedProfileKey) ?? null)
  );
  let formNote = $state('');
  let showNote = $state(false);
  let noteInputEl = $state<HTMLInputElement | null>(null);
  let formEndpoints = $state<EndpointDraft[]>([]);
  const hasEmptyEndpoint = $derived(formEndpoints.some((endpoint) => !endpoint.address.trim()));
  let nameError = $state('');
  let endpointsError = $state('');
  let formError = $state('');
  let saving = $state(false);
  let nameInputEl = $state<HTMLInputElement | null>(null);

  let showDeleteDialog = $state(false);
  let deleting = $state(false);
  let deleteError = $state('');

  $effect(() => {
    $contactSession;
    cancelForm();
    selectedContactId = null;
    searchTerm = '';
    showDeleteDialog = false;
  });

  $effect(() => {
    const identity = requestedIdentity;
    if (!identity) return;
    untrack(() => {
      const matches = matchingContacts(contacts, identity);
      selectedContactId = matches.length === 1 ? matches[0].id : null;
      searchTerm = matches.length > 1 ? identity.fullyQualifiedName : '';
    });
  });

  const ETH_ADDRESS_PATTERN = /^0x[a-fA-F0-9]{40}$/;
  const ZS_MAINNET_ADDRESS_PATTERN = /^zs[0-9a-z]{60,140}$/i;
  const VRPC_HANDLE_PATTERN = /^[A-Za-z0-9._-]+@$/;
  const VRPC_ADDRESS_PATTERN = /^[Ri][1-9A-HJ-NP-Za-km-z]{24,60}$/;
  const BTC_BECH32_ADDRESS_PATTERN = /^(bc1|tb1)[ac-hj-np-z02-9]{11,71}$/i;
  const BTC_BASE58_ADDRESS_PATTERN = /^[13mn2][a-km-zA-HJ-NP-Z1-9]{25,39}$/;

  const filteredContacts = $derived(
    (() => {
      const query = searchTerm.trim().toLowerCase();
      const candidates =
        requestedIdentity && query === requestedIdentity.fullyQualifiedName.toLowerCase()
          ? matchingContacts(contacts, requestedIdentity)
          : contacts;
      const sorted = [...candidates].sort((a, b) => {
        if (a.updatedAt !== b.updatedAt) return b.updatedAt - a.updatedAt;
        return contactName(a).localeCompare(contactName(b));
      });
      if (!query) return sorted;
      return sorted.filter((contact) => {
        if (
          contactName(contact).toLowerCase().includes(query) ||
          contact.identities?.some((identity) =>
            identity.fullyQualifiedName.toLowerCase().includes(query)
          )
        )
          return true;
        return contact.endpoints.some(
          (endpoint) =>
            endpoint.label.toLowerCase().includes(query) ||
            endpoint.address.toLowerCase().includes(query)
        );
      });
    })()
  );

  const selectedContact = $derived(
    selectedContactId
      ? (contacts.find((contact) => contact.id === selectedContactId) ?? null)
      : null
  );

  $effect(() => {
    if (selectedContactId && contacts.some((contact) => contact.id === selectedContactId)) {
      return;
    }

    if (requestedIdentity) return;
    selectedContactId = contacts[0]?.id ?? null;
  });

  function newEndpointDraft(): EndpointDraft {
    return {
      address: '',
      kind: null,
    };
  }

  function isAssociatedEndpoint(endpoint: EndpointDraft): boolean {
    return Boolean(
      endpoint.kind &&
      endpointIdentity(
        { identities: formIdentities },
        { kind: endpoint.kind, address: endpoint.address }
      )
    );
  }

  function inferEndpointKind(address: string): AddressEndpointKind | null {
    const trimmed = address.trim();
    if (!trimmed) return null;
    if (ETH_ADDRESS_PATTERN.test(trimmed)) return 'eth';
    if (ZS_MAINNET_ADDRESS_PATTERN.test(trimmed)) {
      return 'zs';
    }
    if (VRPC_HANDLE_PATTERN.test(trimmed) || VRPC_ADDRESS_PATTERN.test(trimmed)) return 'vrpc';
    if (BTC_BECH32_ADDRESS_PATTERN.test(trimmed) || BTC_BASE58_ADDRESS_PATTERN.test(trimmed))
      return 'btc';
    return null;
  }

  function buildEndpointLabel(index: number): string {
    const baseLabel = i18n.t('wallet.addressBook.endpointDefaultLabel');
    return index === 0 ? baseLabel : `${baseLabel} ${index + 1}`;
  }

  function endpointNetworkLabel(kind: AddressEndpointKind): string {
    return i18n.t(`wallet.addressBook.network.${kind}`);
  }

  async function revealNote() {
    showNote = true;
    await tick();
    noteInputEl?.focus();
  }

  function selectContact(contactId: string) {
    if (formMode === 'create') cancelForm();
    selectedContactId = contactId;
  }

  async function resolveEndpointKind(
    address: string,
    initialKind: AddressEndpointKind | null
  ): Promise<AddressEndpointKind | null> {
    const orderedCandidates: AddressEndpointKind[] = initialKind
      ? [
          initialKind,
          ...(['zs', 'vrpc', 'btc', 'eth'] as AddressEndpointKind[]).filter(
            (kind) => kind !== initialKind
          ),
        ]
      : ['zs', 'vrpc', 'btc', 'eth'];

    for (const kind of orderedCandidates) {
      const validation = await addressBookService.validateDestinationAddress({ kind, address });
      if (validation.valid) {
        return kind;
      }
    }

    return null;
  }

  function startCreateContact() {
    formGeneration++;
    formIdentities = [];
    selectedProfileKey = null;
    lookupMode = true;
    formMode = 'create';
    formContactId = null;
    formDisplayName = '';
    formNote = '';
    showNote = false;
    formEndpoints = [newEndpointDraft()];
    nameError = '';
    endpointsError = '';
    formError = '';
  }

  function startEditContact(contact: AddressBookContact) {
    formGeneration++;
    lookupMode = false;
    formIdentities = [...(contact.identities ?? [])];
    selectedProfileKey = contactProfile(contact) ? identityKey(contactProfile(contact)!) : null;
    formMode = 'edit';
    formContactId = contact.id;
    formDisplayName = contact.displayName;
    formNote = contact.note ?? '';
    showNote = Boolean(contact.note);
    formEndpoints = contact.endpoints.map((endpoint) => ({
      id: endpoint.id,
      label: endpoint.label,
      address: endpoint.address,
      kind: endpoint.kind,
    }));
    nameError = '';
    endpointsError = '';
    formError = '';
  }

  function cancelForm() {
    formGeneration++;
    formIdentities = [];
    selectedProfileKey = null;
    lookupMode = false;
    formMode = null;
    formContactId = null;
    formDisplayName = '';
    formNote = '';
    showNote = false;
    formEndpoints = [];
    nameError = '';
    endpointsError = '';
    formError = '';
    saving = false;
  }

  function addEndpointDraft() {
    formEndpoints = [...formEndpoints, newEndpointDraft()];
    endpointsError = '';
    formError = '';
  }

  function removeEndpointDraft(index: number) {
    if (formEndpoints.length <= 1) return;
    formEndpoints = formEndpoints.filter((_, current) => current !== index);
    endpointsError = '';
    formError = '';
  }

  function updateEndpointDraft(index: number, updates: Partial<EndpointDraft>) {
    formEndpoints = formEndpoints.map((endpoint, current) =>
      current === index ? { ...endpoint, ...updates } : endpoint
    );
  }

  function updateEndpointAddress(index: number, value: string) {
    updateEndpointDraft(index, {
      address: value,
      kind: inferEndpointKind(value),
      identity: undefined,
      resolving: false,
    });
    endpointsError = '';
    formError = '';
  }

  async function resolveEndpointIdentity(index: number) {
    const draft = formEndpoints[index];
    if (!draft || draft.resolving) return;
    const generation = formGeneration;
    const address = draft.address;
    updateEndpointDraft(index, { resolving: true });
    try {
      const identity = await resolveContactIdentity(address);
      if (!alive || generation !== formGeneration || formEndpoints[index]?.address !== address)
        return;
      updateEndpointDraft(index, { identity });
    } catch {
      if (alive && generation === formGeneration && formEndpoints[index]?.address === address)
        endpointsError = i18n.t('wallet.contacts.lookupFailed');
    } finally {
      if (alive && generation === formGeneration && formEndpoints[index]?.address === address)
        updateEndpointDraft(index, { resolving: false });
    }
  }

  function associateIdentity(identity: ContactIdentity) {
    if (!formIdentities.some((id) => identityKey(id) === identityKey(identity)))
      formIdentities = [...formIdentities, identity];
    if (!selectedProfileKey) selectedProfileKey = identityKey(identity);
  }

  function removeIdentity(identity: ContactIdentity) {
    formIdentities = formIdentities.filter((id) => identityKey(id) !== identityKey(identity));
    if (selectedProfileKey === identityKey(identity))
      selectedProfileKey = formIdentities.length === 1 ? identityKey(formIdentities[0]) : null;
    if (!formIdentities.length) formDisplayName = '';
  }

  function updateDisplayName(value: string) {
    formDisplayName = value;
    if (nameError && value.trim()) {
      nameError = '';
    }
    if (formError) {
      formError = '';
    }
  }

  function mapSaveError(error: unknown): string {
    if (isForcedWalletLockError(error)) {
      return '';
    }

    const errorType = extractWalletErrorType(error);
    if (errorType === 'AddressBookDuplicate') return i18n.t('wallet.addressBook.error.duplicate');
    if (errorType === 'AddressBookInvalidInput' || errorType === 'InvalidAddress') {
      return i18n.t('wallet.addressBook.error.invalidInput');
    }
    if (error instanceof Error && error.message.trim()) return error.message;
    return i18n.t('wallet.addressBook.error.saveFailed');
  }

  async function submitContactForm() {
    if (saving || deleting || !formMode) return;
    nameError = '';
    endpointsError = '';
    formError = '';

    const generation = formGeneration;
    const displayName = formProfile?.fullyQualifiedName ?? formDisplayName.trim();
    if (formIdentities.length > 1 && !formProfile) {
      formError = i18n.t('wallet.contacts.chooseProfile');
      return;
    }
    if (!displayName) {
      nameError = i18n.t('wallet.addressBook.error.nameRequired');
      nameInputEl?.focus();
      return;
    }

    if (formEndpoints.length === 0) {
      endpointsError = i18n.t('wallet.addressBook.error.endpointRequired');
      return;
    }

    saving = true;
    try {
      const saveEndpoints: Array<{
        id?: string;
        kind: AddressEndpointKind;
        label: string;
        address: string;
      }> = [];

      for (const endpoint of formEndpoints) {
        const trimmedAddress = endpoint.address.trim();
        if (!trimmedAddress) {
          endpointsError = i18n.t('wallet.addressBook.error.endpointFieldsRequired');
          saving = false;
          return;
        }

        const inferredKind = endpoint.kind ?? inferEndpointKind(trimmedAddress);
        const original = contacts
          .find((contact) => contact.id === formContactId)
          ?.endpoints.find((saved) => saved.id === endpoint.id);
        // Stored addresses can belong to another network. Revalidate only edited destinations.
        const unchanged = original?.address === trimmedAddress && original?.kind === inferredKind;
        const resolvedKind = unchanged
          ? original.kind
          : await resolveEndpointKind(trimmedAddress, inferredKind);
        if (!resolvedKind) {
          endpointsError = i18n.t('wallet.addressBook.error.invalidEndpoint');
          saving = false;
          return;
        }

        saveEndpoints.push({
          id: endpoint.id,
          kind: resolvedKind,
          label: endpoint.label ?? buildEndpointLabel(saveEndpoints.length),
          address: trimmedAddress,
        });
      }

      if (!alive || generation !== formGeneration) return;
      const savedContact = await addressBookService.saveAddressBookContact({
        id: formContactId ?? undefined,
        displayName,
        note: formNote.trim() ? formNote.trim() : null,
        endpoints: saveEndpoints,
        identities: formIdentities,
        profileIdentity: formProfile,
      });

      if (!alive || generation !== formGeneration) return;
      upsertAddressBookContact(savedContact);
      selectedContactId = savedContact.id;
      cancelForm();
    } catch (error) {
      if (!alive || generation !== formGeneration) return;
      formError = mapSaveError(error);
      saving = false;
    }
  }

  async function confirmDeleteSelected() {
    // Deletion belongs to the contact being edited, never a changing list selection.
    const contactId = formContactId;
    if (!contactId || deleting || saving) return;
    deleting = true;
    deleteError = '';

    try {
      const deleted = await addressBookService.deleteAddressBookContact(contactId);
      if (!deleted) {
        deleteError = i18n.t('wallet.addressBook.error.deleteFailed');
        return;
      }
      if (!alive) return;
      removeAddressBookContact(contactId);
      showDeleteDialog = false;
      cancelForm();
    } catch (error) {
      deleteError = isForcedWalletLockError(error)
        ? ''
        : i18n.t('wallet.addressBook.error.deleteFailed');
    } finally {
      deleting = false;
    }
  }
</script>

<div
  class="relative mx-auto flex h-full min-h-0 w-full max-w-6xl min-w-0 flex-1 flex-col px-5 pt-5 pb-6"
  data-address-book-layout
>
  {#if onReturn}
    <div class="mb-4 shrink-0">
      <button
        type="button"
        class="inline-flex items-center gap-1.5 text-sm text-muted-foreground transition-colors outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50"
        onclick={onReturn}
      >
        <ArrowLeftIcon class="size-4" aria-hidden="true" />{returnLabel}
      </button>
    </div>
  {/if}
  {#if contacts.length > 0 && !formMode}
    <div class="absolute top-5 right-5 z-10 shrink-0">
      <Button size="sm" onclick={startCreateContact}
        ><PlusIcon class="size-3.5" aria-hidden="true" />{i18n.t(
          'wallet.addressBook.addContact'
        )}</Button
      >
    </div>
  {/if}
  <div class="flex min-h-0 flex-1">
    <aside
      class="flex min-h-0 w-[184px] shrink-0 flex-col"
      class:hidden={contacts.length === 0 && !formMode}
      aria-label={i18n.t('wallet.addressBook.title')}
    >
      <SearchInput
        bind:value={searchTerm}
        placeholder={i18n.t('wallet.addressBook.searchPlaceholder')}
        aria-label={i18n.t('wallet.addressBook.searchPlaceholder')}
        clearLabel={i18n.t('common.clearSearch')}
        showFocusRing
        class="shrink-0"
        inputClass="h-8 rounded-md pr-2 pl-8 text-[13px] md:text-[13px]"
        iconClass="left-2.5 size-3.5"
      />
      <ScrollArea.Root class="mt-3 min-h-0 flex-1">
        <ScrollArea.Viewport>
          {#if filteredContacts.length === 0}
            {#if contacts.length > 0}
              <p class="px-3 py-4 text-[13px] text-settings-muted-foreground" role="status">
                {i18n.t('wallet.addressBook.noResults')}
              </p>
            {/if}
          {:else}
            <ul class="space-y-1">
              {#each filteredContacts as contact (contact.id)}
                <li>
                  <button
                    type="button"
                    class="flex min-h-[58px] w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left transition-colors outline-none focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:ring-inset disabled:cursor-default
                      {formMode !== 'create' && selectedContactId === contact.id
                      ? 'bg-settings-selection-surface'
                      : 'hover:bg-muted/60'}"
                    aria-current={formMode !== 'create' && selectedContactId === contact.id
                      ? 'true'
                      : undefined}
                    disabled={formMode === 'edit'}
                    onclick={() => selectContact(contact.id)}
                  >
                    <ContactAvatar
                      identity={contactProfile(contact)}
                      name={contactName(contact)}
                      class="size-[30px] text-xs"
                    />
                    <span class="min-w-0 flex-1">
                      <span class="block truncate text-[13px] leading-[18px] font-medium"
                        >{contactName(contact)}</span
                      >
                    </span>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>
    </aside>

    {#if contacts.length > 0 || formMode}
      <div
        class="relative mx-5 w-px shrink-0 self-stretch"
        data-address-book-divider
        aria-hidden="true"
      >
        <div
          class="absolute inset-x-0 -top-5 -bottom-6 bg-border/50"
          data-address-book-divider-line
        ></div>
      </div>
    {/if}

    <section
      class="flex min-h-0 min-w-0 flex-1 flex-col"
      class:pt-16={contacts.length > 0 && !formMode}
      class:pt-2={contacts.length === 0 && !formMode}
    >
      {#if $contactSession && ($contactsLoadState === 'loading' || $contactsLoadState === 'error') && !contacts.length && !formMode}
        <div class="m-auto space-y-3 px-7 text-sm" role="status">
          <p>
            {i18n.t(
              $contactsLoadState === 'error' ? 'wallet.contacts.loadFailed' : 'common.loading'
            )}
          </p>
          {#if $contactsLoadState === 'error'}<Button
              variant="secondary"
              onclick={() => loadContacts(true).catch(() => {})}
              >{i18n.t('wallet.contacts.retry')}</Button
            >{/if}
        </div>
      {:else if contacts.length === 0 && !formMode}
        <WalletEmptyState
          illustration="address-book"
          eyebrow={i18n.t('wallet.empty.encrypted')}
          title={i18n.t('wallet.addressBook.empty')}
          actionLabel={i18n.t('wallet.addressBook.addContact')}
          onAction={startCreateContact}
          testId="address-book-empty"
        />
      {:else if formMode}
        <form
          class="flex min-h-0 flex-1 flex-col"
          onsubmit={(event) => {
            event.preventDefault();
            void submitContactForm();
          }}
        >
          <ScrollArea.Root class="min-h-0 flex-1">
            <ScrollArea.Viewport>
              <div class="mx-auto w-full max-w-2xl px-1 pb-5" data-address-book-form-content>
                {#if formMode === 'edit'}
                  <h3 class="text-xl leading-7 font-semibold tracking-tight">
                    {i18n.t('wallet.addressBook.editContact')}
                  </h3>
                {/if}
                {#if formMode === 'create'}
                  <div class="mb-5 flex w-full justify-start border-b border-border/70">
                    <Tabs.Root
                      value={lookupMode ? 'verusid' : 'address'}
                      onValueChange={(value) => (lookupMode = value === 'verusid')}
                      class="w-full"
                    >
                      <Tabs.List
                        class="h-10 w-full justify-start gap-6 rounded-none bg-transparent p-0"
                        aria-label={i18n.t('wallet.contacts.contactType')}
                      >
                        <Tabs.Trigger
                          value="verusid"
                          class="h-10 rounded-none border-b-2 border-transparent px-0 text-sm font-normal shadow-none data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:font-medium data-[state=active]:shadow-none"
                          >{i18n.t('wallet.contacts.verusId')}</Tabs.Trigger
                        >
                        <Tabs.Trigger
                          value="address"
                          class="h-10 rounded-none border-b-2 border-transparent px-0 text-sm font-normal shadow-none data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:font-medium data-[state=active]:shadow-none"
                          >{i18n.t('wallet.contacts.addressOnly')}</Tabs.Trigger
                        >
                      </Tabs.List>
                    </Tabs.Root>
                  </div>
                {/if}
                {#if lookupMode}
                  <IdentityLookup
                    onSaved={(contact) => {
                      selectedContactId = contact.id;
                      cancelForm();
                    }}
                  />
                {:else}
                  <fieldset disabled={saving || deleting} class="mt-6 min-w-0 space-y-6">
                    {#if formProfile}
                      <PublicProfile identity={formProfile} compact />
                    {:else if !formIdentities.length}
                      <div class="space-y-2">
                        <Label
                          for="address-book-name"
                          class="block text-[13px] leading-[18px] font-normal text-settings-muted-foreground"
                        >
                          {i18n.t('wallet.addressBook.form.nameLabel')}
                        </Label>
                        <Input
                          bind:ref={nameInputEl}
                          id="address-book-name"
                          value={formDisplayName}
                          oninput={(event) => updateDisplayName(event.currentTarget.value)}
                          class="h-[38px] px-3"
                          aria-invalid={Boolean(nameError)}
                          aria-describedby={nameError ? 'address-book-name-error' : undefined}
                          placeholder={i18n.t('wallet.addressBook.form.namePlaceholder')}
                        />
                        {#if nameError}
                          <p
                            id="address-book-name-error"
                            class="text-xs text-destructive"
                            role="alert"
                          >
                            {nameError}
                          </p>
                        {/if}
                      </div>
                    {/if}
                    {#if formIdentities.length}
                      {#if !formProfile}<p class="text-sm text-settings-muted-foreground">
                          {i18n.t('wallet.contacts.chooseProfile')}
                        </p>{/if}
                      <div class="space-y-2">
                        {#each formIdentities as identity (identityKey(identity))}
                          <div class="flex flex-wrap items-center gap-2 text-xs">
                            <span class="min-w-0 flex-1 break-words"
                              >{identity.fullyQualifiedName}</span
                            >
                            {#if formIdentities.length > 1}<Button
                                variant="link"
                                size="sm"
                                aria-pressed={selectedProfileKey === identityKey(identity)}
                                onclick={() => (selectedProfileKey = identityKey(identity))}
                                >{i18n.t('wallet.contacts.useProfile')}</Button
                              >{/if}
                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onclick={() => removeIdentity(identity)}
                              aria-label={i18n.t('wallet.contacts.removeIdentity', {
                                name: identity.fullyQualifiedName,
                              })}><MinusIcon class="size-4" /></Button
                            >
                          </div>
                        {/each}
                      </div>
                    {/if}
                    <div class="space-y-[18px]">
                      {#each formEndpoints as endpoint, index}
                        <div class="space-y-2" hidden={isAssociatedEndpoint(endpoint)}>
                          <Label
                            for={`endpoint-address-${index}`}
                            class="block text-[13px] leading-[18px] font-normal text-settings-muted-foreground"
                          >
                            {endpoint.kind
                              ? endpointNetworkLabel(endpoint.kind)
                              : i18n.t('wallet.addressBook.form.addressLabel')}
                          </Label>
                          <div class="flex min-w-0 items-center gap-2">
                            <div class="min-w-0 flex-1">
                              <Input
                                id={`endpoint-address-${index}`}
                                value={endpoint.address}
                                oninput={(event) =>
                                  updateEndpointAddress(index, event.currentTarget.value)}
                                placeholder={i18n.t('wallet.addressBook.form.addressPlaceholder')}
                                class="identifier-text h-[38px] px-3 text-xs md:text-xs"
                                aria-invalid={Boolean(endpointsError)}
                                aria-describedby={endpointsError
                                  ? 'address-book-endpoints-error'
                                  : undefined}
                              />
                            </div>
                            <Button
                              variant="ghost"
                              size="icon-sm"
                              class="text-settings-muted-foreground"
                              disabled={formEndpoints.length <= 1 || saving || deleting}
                              onclick={() => removeEndpointDraft(index)}
                              title={i18n.t('wallet.addressBook.form.removeEndpoint')}
                              aria-label={i18n.t('wallet.addressBook.form.removeEndpoint')}
                            >
                              <MinusIcon class="size-4" />
                            </Button>
                          </div>
                          {#if endpoint.kind === 'vrpc' && (endpoint.address
                              .trim()
                              .endsWith('@') || endpoint.address.trim().startsWith('i'))}
                            {#if endpoint.identity && !formIdentities.some((id) => identityKey(id) === identityKey(endpoint.identity!))}
                              <div class="mt-3 rounded-lg border p-4">
                                <PublicProfile identity={endpoint.identity} compact /><Button
                                  variant="secondary"
                                  size="sm"
                                  class="mt-3"
                                  onclick={() => associateIdentity(endpoint.identity!)}
                                  >{i18n.t('wallet.contacts.associate')}</Button
                                >
                              </div>
                            {:else if !endpoint.identity && !formIdentities.some((id) => id.identityAddress === endpoint.address || id.fullyQualifiedName.toLowerCase() === endpoint.address.toLowerCase())}
                              <Button
                                variant="ghost"
                                size="sm"
                                disabled={endpoint.resolving}
                                onclick={() => resolveEndpointIdentity(index)}
                                >{i18n.t(
                                  endpoint.resolving
                                    ? 'wallet.contacts.lookingUp'
                                    : 'wallet.contacts.findIdentity'
                                )}</Button
                              >
                            {/if}
                          {/if}
                        </div>
                      {/each}
                      {#if endpointsError}
                        <p
                          id="address-book-endpoints-error"
                          class="text-xs text-destructive"
                          role="alert"
                        >
                          {endpointsError}
                        </p>
                      {/if}
                    </div>
                  </fieldset>

                  <div class="mt-3 flex flex-col items-start">
                    <InlineTextActionButton
                      class="min-h-9 gap-1.5 text-[13px] disabled:text-settings-muted-foreground disabled:opacity-35 dark:disabled:text-settings-muted-foreground"
                      onclick={addEndpointDraft}
                      disabled={hasEmptyEndpoint || saving || deleting}
                    >
                      <PlusIcon class="size-3.5" />
                      {i18n.t('wallet.addressBook.form.addEndpoint')}
                    </InlineTextActionButton>
                    {#if !showNote}
                      <InlineTextActionButton
                        class="min-h-9 gap-1.5 text-[13px]"
                        onclick={revealNote}
                        disabled={saving || deleting}
                      >
                        <PlusIcon class="size-3.5" />
                        {i18n.t('wallet.addressBook.form.addNote')}
                      </InlineTextActionButton>
                    {/if}
                  </div>
                  {#if showNote}
                    <div class="mt-4 space-y-2">
                      <Label
                        for="address-book-note"
                        class="block text-[13px] leading-[18px] font-normal text-settings-muted-foreground"
                      >
                        {i18n.t('wallet.addressBook.form.noteLabel')}
                      </Label>
                      <Input
                        bind:ref={noteInputEl}
                        id="address-book-note"
                        bind:value={formNote}
                        disabled={saving || deleting}
                        placeholder={i18n.t('wallet.addressBook.form.notePlaceholder')}
                        class="h-[38px] px-3"
                      />
                    </div>
                  {/if}
                  {#if formError}
                    <p class="mt-4 text-xs text-destructive" role="alert">{formError}</p>
                  {/if}
                {/if}
              </div>
            </ScrollArea.Viewport>
            <ScrollArea.Scrollbar orientation="vertical" />
          </ScrollArea.Root>

          <footer
            class="mx-auto flex w-full max-w-2xl shrink-0 flex-wrap items-center justify-between gap-3 px-1 pt-3"
          >
            <div>
              {#if formMode === 'edit'}
                <InlineTextActionButton
                  tone="destructive"
                  class="min-h-8 text-[13px]"
                  disabled={saving || deleting}
                  onclick={() => {
                    deleteError = '';
                    showDeleteDialog = true;
                  }}>{i18n.t('wallet.addressBook.deleteContact')}</InlineTextActionButton
                >
              {/if}
            </div>
            <div class="flex items-center gap-2">
              <Button
                variant="secondary"
                size="sm"
                onclick={cancelForm}
                disabled={saving || deleting}
              >
                {i18n.t('common.cancel')}
              </Button>
              {#if !lookupMode}<Button
                  type="submit"
                  size="sm"
                  class="relative"
                  disabled={saving || deleting}
                  aria-busy={saving}
                  aria-label={i18n.t(
                    saving ? 'wallet.addressBook.form.saving' : 'wallet.addressBook.form.save'
                  )}
                >
                  <!-- Reserve both translated labels and balanced space for the spinner. -->
                  <span class="invisible inline-grid px-4" aria-hidden="true">
                    <span class="col-start-1 row-start-1"
                      >{i18n.t('wallet.addressBook.form.save')}</span
                    >
                    <span class="col-start-1 row-start-1"
                      >{i18n.t('wallet.addressBook.form.saving')}</span
                    >
                  </span>
                  <span
                    class="absolute inset-0 flex items-center justify-center"
                    aria-live="polite"
                  >
                    {#if saving}
                      <LoaderCircleIcon
                        class="absolute left-2.5 size-3.5 animate-spin motion-reduce:animate-none"
                        aria-hidden="true"
                      />
                    {/if}
                    <span
                      >{i18n.t(
                        saving ? 'wallet.addressBook.form.saving' : 'wallet.addressBook.form.save'
                      )}</span
                    >
                  </span>
                </Button>
              {/if}
            </div>
          </footer>
        </form>
      {:else if selectedContact}
        <ScrollArea.Root class="min-h-0 flex-1">
          <ScrollArea.Viewport>
            <div class="mx-auto w-full max-w-2xl pb-5">
              <ContactDetail contact={selectedContact}>
                {#snippet actions()}<Button
                    variant="secondary"
                    size="sm"
                    onclick={() => startEditContact(selectedContact)}
                    >{i18n.t('wallet.addressBook.edit')}</Button
                  >{/snippet}
              </ContactDetail>
            </div>
          </ScrollArea.Viewport>
          <ScrollArea.Scrollbar orientation="vertical" />
        </ScrollArea.Root>
      {:else}
        <div
          class="flex flex-1 flex-col items-center justify-center px-7 text-center text-settings-muted-foreground"
        >
          <BookUserIcon class="mb-3 size-8" aria-hidden="true" />
          <p class="text-sm font-medium">
            {i18n.t(
              contacts.length ? 'wallet.addressBook.noSelectionTitle' : 'wallet.addressBook.empty'
            )}
          </p>
          <p class="mt-1 max-w-64 text-[13px] leading-5">
            {i18n.t(
              contacts.length
                ? 'wallet.addressBook.noSelectionDescription'
                : 'wallet.addressBook.emptyDescription'
            )}
          </p>
        </div>
      {/if}
    </section>
  </div>
</div>

<Dialog.Root
  open={showDeleteDialog}
  onOpenChange={(open) => {
    if (!open && !deleting) showDeleteDialog = false;
  }}
>
  <Dialog.Content class="max-w-md" showCloseButton={!deleting}>
    <Dialog.Header>
      <Dialog.Title>{i18n.t('wallet.addressBook.deleteConfirmTitle')}</Dialog.Title>
      <Dialog.Description
        >{i18n.t('wallet.addressBook.deleteConfirmDescription')}</Dialog.Description
      >
    </Dialog.Header>
    {#if deleteError}
      <p class="text-sm text-destructive" role="alert">{deleteError}</p>
    {/if}
    <Dialog.Footer class="flex justify-end gap-3">
      <Button
        variant="secondary"
        size="sm"
        onclick={() => (showDeleteDialog = false)}
        disabled={deleting}
      >
        {i18n.t('common.cancel')}
      </Button>
      <Button variant="destructive" size="sm" onclick={confirmDeleteSelected} disabled={deleting}>
        {deleting ? i18n.t('common.loading') : i18n.t('wallet.addressBook.deleteContact')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
