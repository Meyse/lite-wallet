<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import * as Popover from '$lib/components/ui/popover';
  import InfoIcon from '@lucide/svelte/icons/info';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import CheckIcon from '@lucide/svelte/icons/check';
  import AtSignIcon from '@lucide/svelte/icons/at-sign';
  import BookUserIcon from '@lucide/svelte/icons/book-user';
  import RotateCwIcon from '@lucide/svelte/icons/rotate-cw';
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
  import { Button } from '$lib/components/ui/button';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import type { ContactIdentity } from '$lib/types/addressBook';
  import { i18nStore } from '$lib/i18n';
  import { addressBookStore } from '$lib/stores/addressBook';
  import { matchingContacts } from '$lib/contacts/identity';
  import { activeProfilePreview, contactSession, contactsLoadState } from '$lib/contacts/session';
  import { addIdentityContact, loadContacts } from '$lib/contacts/service';
  import ContactAvatar from './ContactAvatar.svelte';
  import PublicProfile from './PublicProfile.svelte';
  import { getContactNavigation } from '$lib/contacts/navigation';

  let { identity, iconOnly = false }: { identity: ContactIdentity; iconOnly?: boolean } = $props();
  const instance = $props.id();
  const openContact = getContactNavigation();
  const i18n = $derived($i18nStore);
  const matches = $derived(matchingContacts($addressBookStore, identity));
  const saved = $derived(matches.length > 0);
  let open = $state(false);
  let trigger = $state<HTMLButtonElement | null>(null);
  let content = $state<HTMLDivElement | null>(null);
  let action = $state<HTMLButtonElement | null>(null);
  let profileViewport = $state<HTMLDivElement | null>(null);
  let saveState = $state<'idle' | 'saving' | 'saved' | 'error'>('idle');
  let suppressed = false;
  let alive = true;
  let opening = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const actionLabel = $derived(
    saveState === 'saving'
      ? i18n.t('wallet.addressBook.form.saving')
      : saveState === 'saved'
        ? i18n.t('wallet.contacts.saved')
        : saveState === 'error' || $contactsLoadState === 'error'
          ? i18n.t('wallet.contacts.retry')
          : $contactsLoadState !== 'ready'
            ? i18n.t('common.loading')
            : saved
              ? i18n.t(matches.length > 1 ? 'wallet.contacts.viewMany' : 'wallet.contacts.view')
              : i18n.t('wallet.contacts.add')
  );

  function changeOpen(next: boolean) {
    if (next && suppressed) {
      open = false;
      return;
    }
    open = next;
    opening++;
    if (next) {
      activeProfilePreview.set(instance);
      void loadContacts().catch(() => {});
    } else {
      suppressed = true;
      clearTimeout(timer);
      if (saveState === 'saved') saveState = 'idle';
      if ($activeProfilePreview === instance) activeProfilePreview.set(null);
    }
  }
  $effect(() => {
    if (open && ($activeProfilePreview !== instance || !$contactSession)) changeOpen(false);
  });
  $effect(() => {
    if (!open) return undefined;
    // Hover leaves focus in the recipient field. Capture Escape before Send's
    // window shortcut, even when the event originates outside the popover.
    const dismiss = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      event.stopImmediatePropagation();
      changeOpen(false);
      trigger?.focus();
    };
    window.addEventListener('keydown', dismiss, true);
    return () => window.removeEventListener('keydown', dismiss, true);
  });
  onDestroy(() => {
    alive = false;
    clearTimeout(timer);
    if ($activeProfilePreview === instance) activeProfilePreview.set(null);
  });

  async function activate() {
    if (saveState === 'saving' || saveState === 'saved') return;
    if ($contactsLoadState !== 'ready') {
      await loadContacts(true).catch(() => {});
      return;
    }
    if (saved) {
      changeOpen(false);
      openContact?.(identity, trigger);
      return;
    }
    const opened = opening;
    saveState = 'saving';
    try {
      await addIdentityContact(identity);
      if (!alive) return;
      saveState = open && opened === opening ? 'saved' : 'idle';
      if (saveState === 'saved')
        timer = setTimeout(() => {
          saveState = 'idle';
        }, 1200);
    } catch {
      if (alive) saveState = 'error';
    }
  }
  function focusProfileOrAction() {
    if (profileViewport && profileViewport.scrollHeight > profileViewport.clientHeight)
      profileViewport.focus();
    else action?.focus();
  }

  function focusOut(event: FocusEvent) {
    const next = event.relatedTarget;
    if (next instanceof Node && (trigger?.contains(next) || content?.contains(next))) return;
    suppressed = false;
    if (open) changeOpen(false);
  }
</script>

<Popover.Root bind:open onOpenChange={changeOpen}>
  <Popover.Trigger
    bind:ref={trigger}
    openOnHover
    openDelay={0}
    closeDelay={200}
    class="inline-flex max-w-full cursor-default items-center gap-2 rounded-sm text-[13px] leading-5 outline-none focus-visible:ring-2 focus-visible:ring-settings-focus-ring focus-visible:ring-offset-2 {saved
      ? 'text-text-action hover:text-text-action data-[state=open]:text-text-action'
      : 'text-settings-muted-foreground hover:text-foreground data-[state=open]:text-foreground'}"
    aria-label={i18n.t(saved ? 'wallet.contacts.mentionSaved' : 'wallet.contacts.mentionUnsaved', {
      name: identity.fullyQualifiedName,
    })}
    onfocus={() => {
      if (!suppressed && trigger?.matches(':focus-visible')) changeOpen(true);
    }}
    onblur={focusOut}
    onclick={() => {
      suppressed = false;
    }}
    onpointerdown={() => {
      suppressed = false;
    }}
    onpointerenter={() => {
      // A delayed close can suppress reopening after pointerleave has already
      // run. A fresh entry must re-arm hover before the primitive opens it.
      suppressed = false;
    }}
    onpointerleave={() => {
      suppressed = false;
    }}
    onkeydown={(event) => {
      if (event.key === 'Enter' || event.key === ' ') suppressed = false;
      if (event.key === 'Tab' && !event.shiftKey && open) {
        event.preventDefault();
        void tick().then(focusProfileOrAction);
      }
      if (event.key === 'Escape') {
        event.stopPropagation();
        event.preventDefault();
        changeOpen(false);
      }
    }}
  >
    {#if iconOnly}<span class="inline-flex size-8 items-center justify-center"
        ><InfoIcon class="size-4" aria-hidden="true" /></span
      >
    {:else}
      <span class="inline-flex size-[22px] shrink-0 items-center justify-center">
        {#if saved}<span class="recognition"
            ><ContactAvatar
              {identity}
              name={identity.fullyQualifiedName}
              class="size-[22px] text-[8px]"
            /></span
          >
        {:else}<span
            class="inline-flex size-[22px] items-center justify-center rounded-full bg-settings-muted-foreground/20 text-settings-muted-foreground"
            ><AtSignIcon class="size-3.5" aria-hidden="true" /></span
          >{/if}
      </span>
      <span
        class="min-w-0 truncate underline decoration-settings-muted-foreground/60 decoration-dotted underline-offset-[5px]"
        >{identity.fullyQualifiedName}</span
      >
    {/if}
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content
      bind:ref={content}
      sideOffset={10}
      align="start"
      collisionPadding={12}
      trapFocus={false}
      role="dialog"
      aria-label={identity.fullyQualifiedName}
      class="z-50 w-[316px] max-w-[calc(100vw-24px)] rounded-xl border bg-popover p-5 text-popover-foreground shadow-[0_6px_20px_#00000010,0_1px_4px_#00000008] outline-none dark:shadow-[0_4px_16px_#00000014]"
      onkeydown={(event) => {
        if (event.key === 'Escape') {
          event.preventDefault();
          event.stopPropagation();
          changeOpen(false);
          trigger?.focus();
        }
      }}
      onOpenAutoFocus={(event) => event.preventDefault()}
      onCloseAutoFocus={(event) => event.preventDefault()}
      onEscapeKeydown={(event) => {
        event.stopPropagation();
        suppressed = true;
        if (content?.contains(document.activeElement)) trigger?.focus();
      }}
      style={`--preview-reserve: ${saveState === 'error' || $contactsLoadState === 'error' ? '122px' : '90px'}`}
      onfocusout={focusOut}
    >
      <ScrollArea.Root
        class="max-h-[min(432px,calc(var(--bits-popover-content-available-height,100vh)-var(--preview-reserve)))]"
      >
        <ScrollArea.Viewport
          bind:ref={profileViewport}
          tabindex={0}
          aria-label={identity.fullyQualifiedName}
          class="max-h-[min(432px,calc(var(--bits-popover-content-available-height,100vh)-var(--preview-reserve)))]"
        >
          <PublicProfile {identity} compact />
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>
      <Button
        bind:ref={action}
        variant="secondary"
        class="mt-3.5 h-[34px] w-full gap-2 rounded-[7px] text-[13px] font-medium {saveState ===
        'saved'
          ? 'bg-contact-saved text-contact-saved-foreground hover:bg-contact-saved'
          : ''}"
        aria-busy={saveState === 'saving'}
        aria-disabled={saveState === 'saving' ||
          saveState === 'saved' ||
          $contactsLoadState === 'loading' ||
          $contactsLoadState === 'idle'}
        onclick={activate}
        onkeydown={(event) => {
          if (event.key === 'Tab') {
            // Resume the document's native tab order at the anchor, outside the portal.
            suppressed = true;
            if (
              event.shiftKey &&
              profileViewport &&
              profileViewport.scrollHeight > profileViewport.clientHeight
            ) {
              event.preventDefault();
              profileViewport.focus();
            } else {
              trigger?.focus();
              if (event.shiftKey) event.preventDefault();
              else changeOpen(false);
            }
          }
        }}
      >
        {#if saveState === 'saving' || $contactsLoadState === 'loading'}<LoaderCircleIcon
            class="size-3.5 animate-spin motion-reduce:animate-none"
            aria-hidden="true"
          />
        {:else if saveState === 'saved'}<span class="saved-check"
            ><CheckIcon class="size-3.5" aria-hidden="true" /></span
          >
        {:else if saveState === 'error' || $contactsLoadState === 'error'}<RotateCwIcon
            class="size-3.5"
            aria-hidden="true"
          />
        {:else if saved}<BookUserIcon class="size-3.5" aria-hidden="true" />
        {:else}<PlusIcon class="size-3.5" aria-hidden="true" />{/if}
        {actionLabel}
      </Button>
      <span class="sr-only" aria-live="polite"
        >{saveState === 'saving' || saveState === 'saved' ? actionLabel : ''}</span
      >
      {#if saveState === 'error' || $contactsLoadState === 'error'}<p
          class="mt-3 text-[13px] leading-5"
          role="alert"
        >
          {i18n.t(
            saveState === 'error' ? 'wallet.contacts.saveFailed' : 'wallet.contacts.loadFailed'
          )}
        </p>{/if}
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>

<style>
  .recognition {
    animation: recognize 180ms ease-out;
  }
  .saved-check :global(polyline),
  .saved-check :global(path) {
    stroke-dasharray: 24;
    animation: check 160ms ease-out;
  }
  @keyframes recognize {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
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
    .recognition,
    .saved-check :global(polyline),
    .saved-check :global(path) {
      animation: none;
    }
  }
</style>
