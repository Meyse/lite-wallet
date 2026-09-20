<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { AddressBookContact } from '$lib/types/addressBook';
  import {
    contactEndpointName,
    contactName,
    contactProfile,
    endpointIdentity,
    identityKey,
  } from '$lib/contacts/identity';
  import { i18nStore } from '$lib/i18n';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import { CopyButton } from '$lib/components/ui/copy-button';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import PublicProfile from './PublicProfile.svelte';
  import ContactAvatar from './ContactAvatar.svelte';
  let { contact, actions }: { contact: AddressBookContact; actions?: Snippet } = $props();
  const i18n = $derived($i18nStore);
  const identity = $derived(contactProfile(contact));
  const name = $derived(contactName(contact));
  const endpoints = $derived(
    contact.endpoints.filter((endpoint) => {
      const associated = endpointIdentity(contact, endpoint);
      return !identity || !associated || identityKey(identity) !== identityKey(associated);
    })
  );
  const copied = new TimedValueState<string>();
  async function copy(address: string, id: string) {
    if (await writeClipboardText(address)) copied.set(id, 1800);
  }
</script>

<div class="min-w-0 space-y-6" data-contact-detail>
  <div class="relative min-w-0">
    {#if actions}<div class="absolute top-0 right-0 z-10">{@render actions()}</div>{/if}
    {#if identity}
      <PublicProfile {identity} showDescription={false}>
        {#snippet nameAction()}
          <CopyButton
            copied={copied.current === 'identity'}
            onclick={() => copy(identity!.fullyQualifiedName, 'identity')}
            title={i18n.t('wallet.contacts.copyIdentity')}
            aria-label={i18n.t('wallet.contacts.copyIdentity')}
          />
        {/snippet}
      </PublicProfile>
    {:else}
      <div class="space-y-6">
        <ContactAvatar {name} class="size-16 text-[22px]" />
        <h3 class="text-2xl leading-[30px] font-semibold tracking-tight break-words">{name}</h3>
      </div>
    {/if}
  </div>
  {#if identity || endpoints.length}<div
      class="divide-y divide-border/70 border-y border-border/70"
    >
      {#if identity}<div
          class="flex min-w-0 items-center gap-3 py-[18px]"
          data-contact-identity-identifier
        >
          <div class="min-w-0 flex-1 space-y-1.5">
            <p class="text-xs leading-4 text-settings-muted-foreground">
              {i18n.t('wallet.contacts.identityIdentifier')}
            </p>
            <IdentifierText
              value={identity.identityAddress}
              mode="full"
              class="block text-[13px] leading-5"
            />
          </div>
          <CopyButton
            copied={copied.current === 'identity-address'}
            onclick={() => copy(identity!.identityAddress, 'identity-address')}
            title={i18n.t('wallet.contacts.copyIdentityIdentifier')}
            aria-label={i18n.t('wallet.contacts.copyIdentityIdentifier')}
          />
        </div>{/if}
      {#each endpoints as endpoint (endpoint.id)}
        {@const address = contactEndpointName(contact, endpoint)}
        <div class="flex min-w-0 items-center gap-3 py-[18px]">
          <div class="min-w-0 flex-1 space-y-1.5">
            <p class="text-xs leading-4 text-settings-muted-foreground">
              {i18n.t(`wallet.addressBook.network.${endpoint.kind}`)}
            </p>
            <IdentifierText value={address} mode="full" class="block text-[13px] leading-5" />
          </div>
          <CopyButton
            copied={copied.current === endpoint.id}
            onclick={() => copy(address, endpoint.id)}
            title={i18n.t('wallet.receive.copy')}
            aria-label={i18n.t('wallet.receive.copy')}
          />
        </div>
      {/each}
    </div>{/if}
  {#if contact.note}<p
      class="text-[13px] leading-5 break-words whitespace-pre-wrap text-settings-muted-foreground"
    >
      {contact.note}
    </p>{/if}
</div>
