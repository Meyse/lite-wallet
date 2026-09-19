<script lang="ts">
  import XIcon from '@lucide/svelte/icons/x';
  import { Button } from '$lib/components/ui/button';
  import * as Dialog from '$lib/components/ui/dialog';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { addressBookStore } from '$lib/stores/addressBook';
  import { contactSession } from '$lib/contacts/session';
  import { contactName, matchingContacts } from '$lib/contacts/identity';
  import { i18nStore } from '$lib/i18n';
  import type { AddressBookContact, ContactIdentity } from '$lib/types/addressBook';
  import ContactDetail from './ContactDetail.svelte';
  let {
    identity,
    open = $bindable(false),
    onClosed,
    onSelected,
  }: {
    identity: ContactIdentity;
    open?: boolean;
    onClosed?: () => void;
    onSelected?: (contact: AddressBookContact) => void;
  } = $props();
  const i18n = $derived($i18nStore);
  const matches = $derived(matchingContacts($addressBookStore, identity));
  let selectedId = $state<string | null>(null);
  const selected = $derived(
    matches.length === 1 ? matches[0] : matches.find((contact) => contact.id === selectedId)
  );
  $effect(() => {
    if (!open) selectedId = null;
    if (!$contactSession || !matches.length) open = false;
  });
  function choose(contact: AddressBookContact) {
    if (onSelected) {
      open = false;
      onSelected(contact);
    } else selectedId = contact.id;
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content
    showCloseButton={false}
    class="max-w-md"
    onkeydown={(event) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        event.stopPropagation();
        open = false;
      }
    }}
    onEscapeKeydown={(event) => event.stopPropagation()}
    onCloseAutoFocus={(event) => {
      if (onClosed) {
        event.preventDefault();
        onClosed();
      }
    }}
  >
    <Dialog.Title class="sr-only">{i18n.t('wallet.addressBook.title')}</Dialog.Title>
    <Dialog.Description class="sr-only">{identity.fullyQualifiedName}</Dialog.Description>
    <div class="flex justify-end">
      <Button
        variant="ghost"
        size="icon-sm"
        onclick={() => (open = false)}
        aria-label={i18n.t('common.close')}><XIcon class="size-4" /></Button
      >
    </div>
    <ScrollArea.Root class="max-h-[calc(100vh-180px)]"
      ><ScrollArea.Viewport class="max-h-[calc(100vh-180px)]">
        {#if selected}<ContactDetail contact={selected} />
        {:else}<div class="space-y-2">
            {#each matches as contact (contact.id)}<Button
                variant="secondary"
                class="h-auto w-full justify-start py-3 text-left whitespace-normal"
                onclick={() => choose(contact)}
                ><span
                  >{contactName(contact)}<span
                    class="mt-1 block text-xs break-all text-settings-muted-foreground"
                    >{contact.endpoints.map((endpoint) => endpoint.address).join(' · ')}</span
                  ></span
                ></Button
              >{/each}
          </div>{/if}
      </ScrollArea.Viewport><ScrollArea.Scrollbar orientation="vertical" /></ScrollArea.Root
    >
  </Dialog.Content>
</Dialog.Root>
