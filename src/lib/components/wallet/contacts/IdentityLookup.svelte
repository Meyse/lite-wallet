<script lang="ts">
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
  import BookUserIcon from '@lucide/svelte/icons/book-user';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import RotateCwIcon from '@lucide/svelte/icons/rotate-cw';
  import CheckIcon from '@lucide/svelte/icons/check';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { i18nStore } from '$lib/i18n';
  import { onDestroy } from 'svelte';
  import { addressBookStore } from '$lib/stores/addressBook';
  import { matchingContacts } from '$lib/contacts/identity';
  import { contactSession, contactsLoadState } from '$lib/contacts/session';
  import { addIdentityContact, loadContacts, resolveContactIdentity } from '$lib/contacts/service';
  import type { AddressBookContact, ResolvedContactIdentity } from '$lib/types/addressBook';
  import ContactDetailDialog from './ContactDetailDialog.svelte';
  import PublicProfile from './PublicProfile.svelte';
  let { onSaved }: { onSaved: (contact: AddressBookContact) => void } = $props();
  const i18n = $derived($i18nStore);
  let value = $state('');
  let resolved = $state<ResolvedContactIdentity | null>(null);
  let busy = $state(false);
  let saveState = $state<'idle' | 'saving' | 'saved' | 'error'>('idle');
  const saving = $derived(saveState === 'saving');
  const matches = $derived(resolved ? matchingContacts($addressBookStore, resolved) : []);
  const label = $derived(
    i18n.t(
      saving
        ? 'wallet.addressBook.form.saving'
        : saveState === 'saved'
          ? 'wallet.contacts.saved'
          : saveState === 'error'
            ? 'wallet.contacts.retry'
            : $contactsLoadState !== 'ready'
              ? 'wallet.identity.publicProfile.checkingContacts'
              : matches.length > 1
                ? 'wallet.contacts.viewMany'
                : matches.length
                  ? 'wallet.contacts.view'
                  : 'wallet.contacts.add'
    )
  );
  let timer: ReturnType<typeof setTimeout> | undefined;
  let saved = $state<AddressBookContact | null>(null);
  let error = $state('');
  let generation = 0;
  let detailOpen = $state(false);
  $effect(() => {
    $contactSession;
    invalidate();
    detailOpen = false;
  });
  onDestroy(() => {
    generation++;
    clearTimeout(timer);
  });
  function invalidate() {
    generation++;
    clearTimeout(timer);
    resolved = null;
    saved = null;
    error = '';
    busy = false;
    saveState = 'idle';
  }
  async function lookup() {
    invalidate();
    const request = generation;
    busy = true;
    error = '';
    resolved = null;
    try {
      const [result] = await Promise.all([resolveContactIdentity(value.trim()), loadContacts()]);
      if (request === generation) resolved = result;
    } catch {
      if (request === generation) error = i18n.t('wallet.contacts.lookupFailed');
    } finally {
      if (request === generation) busy = false;
    }
  }
  async function save() {
    if (!resolved || saving || saveState === 'saved') return;
    if (matches.length > 1) {
      detailOpen = true;
      return;
    }
    if (matches.length) {
      onSaved(matches[0]);
      return;
    }
    if ($contactsLoadState !== 'ready') {
      await loadContacts(true).catch(() => {});
      return;
    }
    const request = generation;
    saveState = 'saving';
    error = '';
    try {
      const contact = await addIdentityContact(resolved);
      if (request === generation) {
        saved = contact;
        saveState = 'saved';
        timer = setTimeout(() => {
          saveState = 'idle';
        }, 1200);
      }
    } catch {
      if (request === generation) {
        saveState = 'error';
        error = i18n.t('wallet.contacts.saveFailed');
      }
    }
  }
</script>

<div class="space-y-4">
  <div class="flex items-center gap-2">
    <Input
      bind:value
      oninput={invalidate}
      aria-label={i18n.t('wallet.contacts.findIdentity')}
      placeholder={i18n.t('wallet.contacts.identityPlaceholder')}
      class="h-10"
      onkeydown={(event) => {
        if (event.key === 'Enter') {
          event.preventDefault();
          void lookup();
        }
      }}
    />
    <Button
      size="lg"
      variant="default"
      class="shrink-0 px-4"
      disabled={busy || !value.trim()}
      onclick={lookup}
      >{busy ? i18n.t('wallet.contacts.lookingUp') : i18n.t('wallet.contacts.findIdentity')}</Button
    >
  </div>
  {#if resolved}
    <div class="rounded-xl border bg-popover p-5">
      <PublicProfile identity={resolved} compact />
      <Button
        variant="secondary"
        class="mt-3.5 w-full gap-2 rounded-[7px] text-[13px] {saveState === 'saved'
          ? 'bg-contact-saved text-contact-saved-foreground hover:bg-contact-saved'
          : ''}"
        aria-busy={saving}
        aria-disabled={saving || saveState === 'saved' || $contactsLoadState !== 'ready'}
        onclick={save}
      >
        {#if saving}<LoaderCircleIcon
            class="size-3.5 animate-spin motion-reduce:animate-none"
            aria-hidden="true"
          />
        {:else if saveState === 'saved'}<span class="saved-check"
            ><CheckIcon class="size-3.5" aria-hidden="true" /></span
          >
        {:else if saveState === 'error'}<RotateCwIcon class="size-3.5" aria-hidden="true" />
        {:else if saved || matches.length}<BookUserIcon class="size-3.5" aria-hidden="true" />
        {:else}<PlusIcon class="size-3.5" aria-hidden="true" />{/if}
        {label}
      </Button>
      <span class="sr-only" aria-live="polite">{saveState === 'saved' || saving ? label : ''}</span>
    </div>
  {/if}
  {#if error}<p class="text-[13px] text-destructive" role="alert">{error}</p>{/if}
</div>

{#if resolved}<ContactDetailDialog
    identity={resolved}
    bind:open={detailOpen}
    onSelected={onSaved}
  />{/if}

<style>
  .saved-check :global(polyline),
  .saved-check :global(path) {
    stroke-dasharray: 24;
    animation: check 160ms ease-out;
  }
  @keyframes check {
    from {
      stroke-dashoffset: 24;
    }
    to {
      stroke-dashoffset: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .saved-check :global(polyline),
    .saved-check :global(path) {
      animation: none;
    }
  }
</style>
