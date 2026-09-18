<script lang="ts">
  import BookUserIcon from '@lucide/svelte/icons/book-user';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import MinusIcon from '@lucide/svelte/icons/minus';
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
  import { tick } from 'svelte';
  import SearchInput from '$lib/components/common/SearchInput.svelte';
  import InlineTextActionButton from '$lib/components/common/InlineTextActionButton.svelte';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import { CopyButton } from '$lib/components/ui/copy-button';
  import * as Dialog from '$lib/components/ui/dialog';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { i18nStore } from '$lib/i18n';
  import { isForcedWalletLockError } from '$lib/services/walletLockCoordinator.js';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import { extractWalletErrorType } from '$lib/utils/walletErrors.js';
  import {
    addressBookStore,
    removeAddressBookContact,
    upsertAddressBookContact,
  } from '$lib/stores/addressBook';
  import * as addressBookService from '$lib/services/addressBookService';
  import type { AddressBookContact, AddressEndpointKind } from '$lib/types/addressBook';

  type EndpointDraft = {
    id?: string;
    address: string;
    kind: AddressEndpointKind | null;
  };

  type FormMode = 'create' | 'edit' | null;

  const i18n = $derived($i18nStore);
  const contacts = $derived($addressBookStore);

  let searchTerm = $state('');
  let selectedContactId = $state<string | null>(null);

  let formMode = $state<FormMode>(null);
  let formContactId = $state<string | null>(null);
  let formDisplayName = $state('');
  let formNote = $state('');
  let showNote = $state(false);
  let noteInputEl = $state<HTMLInputElement | null>(null);
  let formEndpoints = $state<EndpointDraft[]>([]);
  let nameError = $state('');
  let endpointsError = $state('');
  let formError = $state('');
  let saving = $state(false);
  let nameInputEl = $state<HTMLInputElement | null>(null);

  let showDeleteDialog = $state(false);
  let deleting = $state(false);
  let deleteError = $state('');
  const copiedEndpointState = new TimedValueState<string>();
  const copiedEndpointId = $derived(copiedEndpointState.current);

  const ETH_ADDRESS_PATTERN = /^0x[a-fA-F0-9]{40}$/;
  const ZS_MAINNET_ADDRESS_PATTERN = /^zs[0-9a-z]{60,140}$/i;
  const VRPC_HANDLE_PATTERN = /^[A-Za-z0-9._-]+@$/;
  const VRPC_ADDRESS_PATTERN = /^[Ri][1-9A-HJ-NP-Za-km-z]{24,60}$/;
  const BTC_BECH32_ADDRESS_PATTERN = /^(bc1|tb1)[ac-hj-np-z02-9]{11,71}$/i;
  const BTC_BASE58_ADDRESS_PATTERN = /^[13mn2][a-km-zA-HJ-NP-Z1-9]{25,39}$/;

  const filteredContacts = $derived(
    (() => {
      const query = searchTerm.trim().toLowerCase();
      const sorted = [...contacts].sort((a, b) => {
        if (a.updatedAt !== b.updatedAt) return b.updatedAt - a.updatedAt;
        return a.displayName.localeCompare(b.displayName);
      });
      if (!query) return sorted;
      return sorted.filter((contact) => {
        if (contact.displayName.toLowerCase().includes(query)) return true;
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

    selectedContactId = contacts[0]?.id ?? null;
  });

  function newEndpointDraft(): EndpointDraft {
    return {
      address: '',
      kind: null,
    };
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

  function contactNetworks(contact: AddressBookContact): string {
    return [
      ...new Set(contact.endpoints.map((endpoint) => endpointNetworkLabel(endpoint.kind))),
    ].join(', ');
  }

  async function revealNote() {
    showNote = true;
    await tick();
    noteInputEl?.focus();
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
    formMode = 'edit';
    formContactId = contact.id;
    formDisplayName = contact.displayName;
    formNote = contact.note ?? '';
    showNote = Boolean(contact.note);
    formEndpoints = contact.endpoints.map((endpoint) => ({
      id: endpoint.id,
      address: endpoint.address,
      kind: endpoint.kind,
    }));
    nameError = '';
    endpointsError = '';
    formError = '';
  }

  function cancelForm() {
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
    });
    endpointsError = '';
    formError = '';
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

    const displayName = formDisplayName.trim();
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
        const resolvedKind = await resolveEndpointKind(trimmedAddress, inferredKind);
        if (!resolvedKind) {
          endpointsError = i18n.t('wallet.addressBook.error.invalidEndpoint');
          saving = false;
          return;
        }

        saveEndpoints.push({
          id: endpoint.id,
          kind: resolvedKind,
          label: buildEndpointLabel(saveEndpoints.length),
          address: trimmedAddress,
        });
      }

      const savedContact = await addressBookService.saveAddressBookContact({
        id: formContactId ?? undefined,
        displayName,
        note: formNote.trim() ? formNote.trim() : null,
        endpoints: saveEndpoints,
      });

      upsertAddressBookContact(savedContact);
      selectedContactId = savedContact.id;
      cancelForm();
    } catch (error) {
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

  async function copyAddress(address: string, endpointId: string) {
    if (await writeClipboardText(address)) {
      copiedEndpointState.set(endpointId, 1800);
      return;
    }
    copiedEndpointState.clear();
  }
</script>

<div class="flex h-full min-h-0 min-w-0 flex-1 flex-col">
  <!-- The wallet shell supplies the 24px titlebar above this 58px toolbar. -->
  <header class="flex h-[58px] shrink-0 items-center justify-between gap-4 px-7 pb-6">
    <h2 class="text-xl leading-7 font-semibold tracking-tight">
      {i18n.t('wallet.addressBook.title')}
    </h2>
    {#if !formMode}
      <Button variant="secondary" size="sm" onclick={startCreateContact}>
        <PlusIcon class="size-4" />
        {i18n.t('wallet.addressBook.addContact')}
      </Button>
    {/if}
  </header>

  <div class="flex min-h-0 flex-1">
    <aside
      class="flex min-h-0 w-[216px] shrink-0 flex-col border-r border-border/50 pr-3 pb-5 pl-4"
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
                    class="flex min-h-[58px] w-full flex-col justify-center gap-0.5 rounded-lg px-3 py-2 text-left transition-colors outline-none focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:ring-inset disabled:cursor-default
                      {selectedContactId === contact.id
                      ? 'bg-settings-selection-surface'
                      : 'hover:bg-muted/60'}"
                    aria-current={selectedContactId === contact.id ? 'true' : undefined}
                    disabled={formMode !== null}
                    onclick={() => (selectedContactId = contact.id)}
                  >
                    <span
                      class="w-full truncate text-sm leading-5 {selectedContactId === contact.id
                        ? 'font-semibold text-primary dark:text-settings-focus-ring'
                        : 'font-medium'}">{contact.displayName}</span
                    >
                    <span
                      class="w-full truncate text-xs leading-[17px] text-settings-muted-foreground"
                    >
                      {contactNetworks(contact)}
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

    <section class="flex min-h-0 min-w-0 flex-1 flex-col">
      {#if formMode}
        <form
          class="flex min-h-0 flex-1 flex-col"
          onsubmit={(event) => {
            event.preventDefault();
            void submitContactForm();
          }}
        >
          <ScrollArea.Root class="min-h-0 flex-1">
            <ScrollArea.Viewport>
              <div class="mx-auto w-full max-w-2xl px-7 pt-[18px] pb-5">
                <h3 class="text-xl leading-7 font-semibold tracking-tight">
                  {i18n.t(
                    formMode === 'edit'
                      ? 'wallet.addressBook.editContact'
                      : 'wallet.addressBook.addContact'
                  )}
                </h3>
                <fieldset disabled={saving || deleting} class="mt-6 min-w-0 space-y-6">
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
                      <p id="address-book-name-error" class="text-xs text-destructive" role="alert">
                        {nameError}
                      </p>
                    {/if}
                  </div>

                  <div class="space-y-[18px]">
                    {#each formEndpoints as endpoint, index}
                      <div class="space-y-2">
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
                    class="min-h-9 gap-1.5 text-[13px] text-primary hover:text-primary/80 dark:text-settings-focus-ring"
                    onclick={addEndpointDraft}
                    disabled={saving || deleting}
                  >
                    <PlusIcon class="size-3.5" />
                    {i18n.t('wallet.addressBook.form.addEndpoint')}
                  </InlineTextActionButton>
                  {#if !showNote}
                    <InlineTextActionButton
                      class="min-h-9 gap-1.5 text-[13px] text-primary hover:text-primary/80 dark:text-settings-focus-ring"
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
              </div>
            </ScrollArea.Viewport>
            <ScrollArea.Scrollbar orientation="vertical" />
          </ScrollArea.Root>

          <footer
            class="mx-auto flex w-full max-w-2xl shrink-0 flex-wrap items-center justify-between gap-3 px-7 pt-3 pb-6"
          >
            <div>
              {#if formMode === 'edit'}
                <InlineTextActionButton
                  class="min-h-8 text-[13px] text-destructive hover:text-destructive/80"
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
              <Button
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
                <span class="absolute inset-0 flex items-center justify-center" aria-live="polite">
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
            </div>
          </footer>
        </form>
      {:else if selectedContact}
        <ScrollArea.Root class="min-h-0 flex-1">
          <ScrollArea.Viewport>
            <div class="mx-auto w-full max-w-2xl px-7 py-5">
              <div class="flex min-h-[52px] items-center gap-3">
                <div
                  class="flex size-12 shrink-0 items-center justify-center rounded-full bg-muted text-[22px] font-medium text-settings-muted-foreground"
                  aria-hidden="true"
                >
                  {Array.from(selectedContact.displayName.trim())[0]?.toLocaleUpperCase()}
                </div>
                <h3
                  class="min-w-0 flex-1 text-2xl leading-[30px] font-semibold tracking-tight break-words"
                >
                  {selectedContact.displayName}
                </h3>
                <Button
                  variant="secondary"
                  size="sm"
                  onclick={() => startEditContact(selectedContact)}
                >
                  {i18n.t('wallet.addressBook.edit')}
                </Button>
              </div>
              {#if selectedContact.note}
                <p
                  class="mt-4 text-[13px] leading-5 break-words whitespace-pre-wrap text-settings-muted-foreground"
                >
                  {selectedContact.note}
                </p>
              {/if}
              <div class="mt-9 space-y-7">
                {#each selectedContact.endpoints as endpoint (endpoint.id)}
                  <div class="flex min-w-0 items-center gap-3">
                    <div class="min-w-0 flex-1 space-y-[7px]">
                      <p
                        class="text-[13px] leading-[18px] font-medium text-settings-muted-foreground"
                      >
                        {endpointNetworkLabel(endpoint.kind)}
                      </p>
                      <IdentifierText
                        value={endpoint.address}
                        mode="full"
                        class="block text-[13px] leading-5"
                      />
                    </div>
                    <CopyButton
                      copied={copiedEndpointId === endpoint.id}
                      onclick={() => copyAddress(endpoint.address, endpoint.id)}
                      title={i18n.t('wallet.receive.copy')}
                      aria-label={i18n.t('wallet.receive.copy')}
                    />
                  </div>
                {/each}
              </div>
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
