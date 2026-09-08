<script lang="ts">
  import BookUserIcon from '@lucide/svelte/icons/book-user';
  import CirclePlusIcon from '@lucide/svelte/icons/circle-plus';
  import PencilIcon from '@lucide/svelte/icons/pencil';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import SearchInput from '$lib/components/common/SearchInput.svelte';
  import InlineTextActionButton from '$lib/components/common/InlineTextActionButton.svelte';
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
  import { addressBookStore, removeAddressBookContact, upsertAddressBookContact } from '$lib/stores/addressBook';
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
  let formEndpoints = $state<EndpointDraft[]>([]);
  let nameError = $state('');
  let endpointsError = $state('');
  let formError = $state('');
  let saving = $state(false);
  let nameInputEl = $state<HTMLInputElement | null>(null);

  let showDeleteDialog = $state(false);
  let deleting = $state(false);
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
    selectedContactId ? contacts.find((contact) => contact.id === selectedContactId) ?? null : null
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
      kind: null
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
    if (BTC_BECH32_ADDRESS_PATTERN.test(trimmed) || BTC_BASE58_ADDRESS_PATTERN.test(trimmed)) return 'btc';
    return null;
  }

  function buildEndpointLabel(index: number): string {
    const baseLabel = i18n.t('wallet.addressBook.endpointDefaultLabel');
    return index === 0 ? baseLabel : `${baseLabel} ${index + 1}`;
  }

  function endpointBadgeLabel(kind: AddressEndpointKind): string {
    if (kind === 'vrpc') return 'VERUS';
    if (kind === 'zs') return 'ZS';
    return kind.toUpperCase();
  }

  async function resolveEndpointKind(address: string, initialKind: AddressEndpointKind | null): Promise<AddressEndpointKind | null> {
    const orderedCandidates: AddressEndpointKind[] = initialKind
      ? [initialKind, ...(['zs', 'vrpc', 'btc', 'eth'] as AddressEndpointKind[]).filter((kind) => kind !== initialKind)]
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
    formEndpoints = contact.endpoints.map((endpoint) => ({
      id: endpoint.id,
      address: endpoint.address,
      kind: endpoint.kind
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
      kind: inferEndpointKind(value)
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
    if (saving) return;
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
      const saveEndpoints: Array<{ id?: string; kind: AddressEndpointKind; label: string; address: string }> = [];

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
          address: trimmedAddress
        });
      }

      const savedContact = await addressBookService.saveAddressBookContact({
        id: formContactId ?? undefined,
        displayName,
        note: formNote.trim() ? formNote.trim() : null,
        endpoints: saveEndpoints
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
    if (!selectedContact || deleting) return;
    deleting = true;

    try {
      const deleted = await addressBookService.deleteAddressBookContact(selectedContact.id);
      if (deleted) {
        removeAddressBookContact(selectedContact.id);
      }
      showDeleteDialog = false;
    } catch (error) {
      formError = mapSaveError(error);
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

<div class="flex h-full min-h-0 flex-col">
  <header class="border-border/70 flex items-center justify-between border-b px-6 py-5">
    <div>
      <h2 class="text-2xl font-semibold">{i18n.t('wallet.addressBook.title')}</h2>
      <p class="text-muted-foreground mt-1 text-sm">{i18n.t('wallet.addressBook.description')}</p>
    </div>
    {#if !formMode}
      <Button variant="secondary" size="lg" class="h-10 gap-1.5 rounded-md px-3" onclick={startCreateContact}>
        <CirclePlusIcon class="size-4" />
        {i18n.t('wallet.addressBook.addContact')}
      </Button>
    {/if}
  </header>

  <div class="flex min-h-0 flex-1">
    <aside class="border-border/70 w-[256px] shrink-0 border-r">
      <div class="p-4">
        <SearchInput bind:value={searchTerm} placeholder={i18n.t('wallet.addressBook.searchPlaceholder')} />
      </div>

      <ScrollArea.Root class="min-h-0 flex-1">
        <ScrollArea.Viewport class="h-full px-3 pb-3">
          {#if filteredContacts.length === 0}
            <div class="text-muted-foreground rounded-md px-3 py-5 text-sm">
              {i18n.t('wallet.addressBook.empty')}
            </div>
          {:else}
            <ul class="space-y-2">
              {#each filteredContacts as contact}
                <li>
                  <button
                    type="button"
                    class="w-full rounded-md px-3 py-2 text-left transition-colors
                      {selectedContactId === contact.id
                        ? 'bg-primary/10'
                        : 'hover:bg-muted/45'}"
                    onclick={() => (selectedContactId = contact.id)}
                  >
                    <p class="truncate text-sm font-medium">{contact.displayName}</p>
                    <p class="text-muted-foreground mt-1 truncate text-xs">
                      {contact.endpoints.length} {i18n.t('wallet.addressBook.endpointsCount')}
                    </p>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>
    </aside>

    <section class="min-h-0 flex-1 overflow-y-auto p-6">
      {#if formMode}
        <div class="mx-auto max-w-xl space-y-4">
          <div class="space-y-2">
            <Label for="address-book-name">{i18n.t('wallet.addressBook.form.nameLabel')}</Label>
            <Input
              bind:ref={nameInputEl}
              id="address-book-name"
              value={formDisplayName}
              oninput={(event) => updateDisplayName((event.currentTarget as HTMLInputElement).value)}
              class={nameError ? 'border-destructive focus-visible:ring-destructive/40' : ''}
              aria-invalid={nameError ? 'true' : 'false'}
              aria-describedby="address-book-name-error"
              placeholder={i18n.t('wallet.addressBook.form.namePlaceholder')}
            />
            <p id="address-book-name-error" class="text-destructive min-h-5 text-sm">{nameError}</p>
          </div>

          <div class="space-y-2">
            <Label for="address-book-note">{i18n.t('wallet.addressBook.form.noteLabel')}</Label>
            <Input
              id="address-book-note"
              bind:value={formNote}
              placeholder={i18n.t('wallet.addressBook.form.notePlaceholder')}
            />
          </div>

          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <p class="text-sm font-medium">{i18n.t('wallet.addressBook.form.endpointsTitle')}</p>
              <InlineTextActionButton onclick={addEndpointDraft}>
                <CirclePlusIcon class="size-3.5" />
                {i18n.t('wallet.addressBook.form.addEndpoint')}
              </InlineTextActionButton>
            </div>

            {#each formEndpoints as endpoint, index}
              <div class="rounded-md p-3">
                <div class="flex items-start gap-2">
                  <div class="min-w-0 flex-1 space-y-1">
                    <Label for={`endpoint-address-${index}`}>{i18n.t('wallet.addressBook.form.addressLabel')}</Label>
                    <Input
                      id={`endpoint-address-${index}`}
                      value={endpoint.address}
                      oninput={(event) => updateEndpointAddress(index, (event.currentTarget as HTMLInputElement).value)}
                      placeholder={i18n.t('wallet.addressBook.form.addressPlaceholder')}
                      class="identifier-text"
                    />
                  </div>

                  <div class="pt-6">
                    <button
                      type="button"
                      class="text-destructive hover:text-destructive/90 focus-visible:ring-ring/50 h-8 w-8 rounded-sm p-0 transition-colors focus-visible:outline-none focus-visible:ring-2 disabled:opacity-40"
                      disabled={formEndpoints.length <= 1}
                      onclick={() => removeEndpointDraft(index)}
                      title={i18n.t('wallet.addressBook.deleteContact')}
                      aria-label={i18n.t('wallet.addressBook.deleteContact')}
                    >
                      <Trash2Icon class="size-4" />
                    </button>
                  </div>
                </div>
              </div>
            {/each}
          </div>

          <p class="text-destructive min-h-5 text-sm">{endpointsError || formError}</p>

          <div class="flex justify-end gap-2">
            <Button variant="secondary" onclick={cancelForm} disabled={saving}>
              {i18n.t('common.cancel')}
            </Button>
            <Button onclick={submitContactForm} disabled={saving}>
              {saving ? i18n.t('common.loading') : i18n.t('wallet.addressBook.form.save')}
            </Button>
          </div>
        </div>
      {:else if selectedContact}
        <div class="mx-auto max-w-xl space-y-4">
          <div class="flex items-start justify-between gap-3">
            <div>
              <h3 class="text-lg font-semibold">{selectedContact.displayName}</h3>
              {#if selectedContact.note}
                <p class="text-muted-foreground mt-1 text-sm">{selectedContact.note}</p>
              {/if}
            </div>
            <div class="flex items-center gap-1">
              <button
                type="button"
                class="text-muted-foreground hover:text-foreground focus-visible:ring-ring/50 h-8 w-8 rounded-sm p-0 transition-colors focus-visible:outline-none focus-visible:ring-2"
                onclick={() => startEditContact(selectedContact)}
                title={i18n.t('wallet.addressBook.editContact')}
                aria-label={i18n.t('wallet.addressBook.editContact')}
              >
                <PencilIcon class="size-4" />
              </button>
              <button
                type="button"
                class="text-destructive hover:text-destructive/90 focus-visible:ring-ring/50 h-8 w-8 rounded-sm p-0 transition-colors focus-visible:outline-none focus-visible:ring-2"
                onclick={() => (showDeleteDialog = true)}
                title={i18n.t('wallet.addressBook.deleteContact')}
                aria-label={i18n.t('wallet.addressBook.deleteContact')}
              >
                <Trash2Icon class="size-4" />
              </button>
            </div>
          </div>

          <div class="space-y-2">
            {#each selectedContact.endpoints as endpoint}
              <div class="bg-muted/35 rounded-md p-3">
                <div class="flex min-h-8 items-center gap-2">
                  <span
                    class="bg-background/60 text-muted-foreground inline-flex shrink-0 rounded-full px-2.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide dark:bg-background/45"
                  >
                    {endpointBadgeLabel(endpoint.kind)}
                  </span>
                  <p class="identifier-text min-w-0 flex-1 break-all text-sm leading-6">{endpoint.address}</p>
                  <CopyButton
                    copied={copiedEndpointId === endpoint.id}
                    size="sm"
                    class="-mr-3 pl-1"
                    onclick={() => copyAddress(endpoint.address, endpoint.id)}
                    title={i18n.t('wallet.receive.copy')}
                    aria-label={i18n.t('wallet.receive.copy')}
                  />
                </div>
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <div class="text-muted-foreground flex h-full flex-col items-center justify-center text-center">
          <BookUserIcon class="mb-3 h-10 w-10" />
          <p class="text-base font-medium">{i18n.t('wallet.addressBook.noSelectionTitle')}</p>
          <p class="mt-1 text-sm">{i18n.t('wallet.addressBook.noSelectionDescription')}</p>
        </div>
      {/if}
    </section>
  </div>
</div>

<Dialog.Root
  open={showDeleteDialog}
  onOpenChange={(open) => {
    if (!open) showDeleteDialog = false;
  }}
>
  <Dialog.Content class="max-w-md">
    <Dialog.Header>
      <Dialog.Title>{i18n.t('wallet.addressBook.deleteConfirmTitle')}</Dialog.Title>
      <Dialog.Description>{i18n.t('wallet.addressBook.deleteConfirmDescription')}</Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer class="flex justify-end gap-3">
      <Button variant="secondary" onclick={() => (showDeleteDialog = false)} disabled={deleting}>
        {i18n.t('common.cancel')}
      </Button>
      <Button variant="destructive" onclick={confirmDeleteSelected} disabled={deleting}>
        {deleting ? i18n.t('common.loading') : i18n.t('wallet.addressBook.deleteContact')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
