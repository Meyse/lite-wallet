<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import CheckIcon from '@lucide/svelte/icons/check';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import RotateCwIcon from '@lucide/svelte/icons/rotate-cw';
  import SendIcon from '@lucide/svelte/icons/send';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import { Button } from '$lib/components/ui/button';
  import { CopyButton } from '$lib/components/ui/copy-button';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { identityKey, matchingContacts } from '$lib/contacts/identity';
  import { identityProfiles, loadIdentityProfile, profileImage } from '$lib/contacts/profiles';
  import { addIdentityContact, loadContacts } from '$lib/contacts/service';
  import { contactsLoadState } from '$lib/contacts/session';
  import { i18nStore } from '$lib/i18n';
  import { addressBookStore } from '$lib/stores/addressBook';
  import type { ResolvedContactIdentity } from '$lib/types/addressBook';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import IdentityAvatar from './IdentityAvatar.svelte';

  const noop = (): void => {};
  const noopIdentity = (_identity: ResolvedContactIdentity): void => {};

  let {
    identity,
    navigationDisabled = false,
    onBack = noop,
    onSend = noopIdentity,
  }: {
    identity: ResolvedContactIdentity;
    navigationDisabled?: boolean;
    onBack?: () => void;
    onSend?: (identity: ResolvedContactIdentity) => void;
  } = $props();

  const i18n = $derived($i18nStore);
  const profileEntry = $derived($identityProfiles[identityKey(identity)]);
  const profile = $derived(profileEntry?.profile ?? null);
  const description = $derived(profile?.description?.value?.trim() ?? '');
  const avatarUrl = $derived(profileImage(profile));
  const matches = $derived(matchingContacts($addressBookStore, identity));
  const inContacts = $derived(matches.length > 0);
  const normalizedStatus = $derived(identity.status?.trim().toLowerCase() ?? '');
  const sendBlocked = $derived(normalizedStatus.length > 0 && normalizedStatus !== 'active');
  const networkLabel = $derived(
    i18n.t(
      identity.network === 'testnet'
        ? 'wallet.identity.lookup.network.testnet'
        : 'wallet.identity.lookup.network.mainnet'
    )
  );
  const statusLabel = $derived(
    normalizedStatus === 'active'
      ? i18n.t('wallet.identity.publicProfile.status.active')
      : normalizedStatus === 'revoked'
        ? i18n.t('wallet.identity.publicProfile.status.revoked')
        : normalizedStatus === 'inactive'
          ? i18n.t('wallet.identity.publicProfile.status.inactive')
          : identity.status?.trim() || i18n.t('wallet.identity.publicProfile.status.unknown')
  );

  let detailsOpen = $state(false);
  let saveState = $state<'idle' | 'saving' | 'saved' | 'error'>('idle');
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let alive = true;
  const copied = new TimedValueState<'name' | 'address' | 'chain'>();

  const contactActionLabel = $derived(
    saveState === 'saving'
      ? i18n.t('wallet.addressBook.form.saving')
      : saveState === 'saved'
        ? i18n.t('wallet.contacts.saved')
        : inContacts
          ? i18n.t('wallet.identity.publicProfile.inContacts')
          : saveState === 'error' || $contactsLoadState === 'error'
            ? i18n.t('wallet.contacts.retry')
            : $contactsLoadState === 'ready'
              ? i18n.t('wallet.contacts.add')
              : i18n.t('wallet.identity.publicProfile.checkingContacts')
  );

  async function copy(value: string, key: 'name' | 'address' | 'chain'): Promise<void> {
    if (await writeClipboardText(value)) copied.set(key);
  }

  async function activateContactAction(): Promise<void> {
    if (saveState === 'saving' || saveState === 'saved' || inContacts) return;

    if ($contactsLoadState !== 'ready') {
      saveState = 'idle';
      await loadContacts(true).catch(() => {});
      return;
    }

    saveState = 'saving';
    try {
      await addIdentityContact(identity);
      if (!alive) return;
      saveState = 'saved';
      if (saveTimer) clearTimeout(saveTimer);
      saveTimer = setTimeout(() => {
        saveState = 'idle';
      }, 1200);
    } catch {
      if (alive) saveState = 'error';
    }
  }

  function closeDetailsOnEscape(event: KeyboardEvent): void {
    if (event.key !== 'Escape' || !detailsOpen) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    detailsOpen = false;
  }

  onMount(() => {
    window.addEventListener('keydown', closeDetailsOnEscape, true);
    void loadIdentityProfile(identity, false, true).catch(() => {});
    void loadContacts().catch(() => {});
  });

  onDestroy(() => {
    alive = false;
    if (saveTimer) clearTimeout(saveTimer);
    window.removeEventListener('keydown', closeDetailsOnEscape, true);
  });
</script>

<div class="flex h-full min-h-0 w-full max-w-[676px] flex-col px-7 pb-7">
  <div class="mt-3 flex h-9 shrink-0 items-center">
    <Button
      variant="ghost"
      class="h-9 gap-1.5 px-0 text-[13px] font-normal text-muted-foreground hover:bg-transparent hover:text-foreground"
      onclick={onBack}
    >
      <ArrowLeftIcon class="size-4" aria-hidden="true" />
      {i18n.t('wallet.identity.publicProfile.backToSearch')}
    </Button>
  </div>

  <ScrollArea.Root class="mt-9 min-h-0 flex-1" type="scroll">
    <ScrollArea.Viewport class="h-full pr-1">
      <div class="flex w-full max-w-[620px] flex-col gap-5 pb-6">
        <IdentityAvatar
          seed={identity.identityAddress}
          label={identity.fullyQualifiedName}
          imageUrl={avatarUrl}
          class="size-[72px] text-xl"
        />

        <div class="space-y-1.5">
          <div class="flex min-w-0 items-start gap-2.5">
            <h2 class="min-w-0 text-[28px] leading-8 font-semibold tracking-[-0.02em] break-words">
              {identity.fullyQualifiedName}
            </h2>
            <CopyButton
              class="mt-0.5"
              size="sm"
              copied={copied.current === 'name'}
              aria-label={i18n.t('wallet.contacts.copyIdentity')}
              title={i18n.t('wallet.contacts.copyIdentity')}
              onclick={() => void copy(identity.fullyQualifiedName, 'name')}
            />
          </div>
          <p class="text-xs text-muted-foreground">{networkLabel}</p>
        </div>

        <div class="flex flex-wrap items-center gap-2.5">
          <Button
            class="h-[34px] gap-1.5 px-3 text-[13px]"
            disabled={navigationDisabled || sendBlocked}
            onclick={() => onSend(identity)}
          >
            <SendIcon class="size-3.5" aria-hidden="true" />
            {i18n.t('wallet.identity.publicProfile.send')}
          </Button>
          <Button
            variant="secondary"
            class="h-[34px] min-w-[143px] gap-1.5 px-3 text-[13px]"
            disabled={inContacts || saveState === 'saving' || saveState === 'saved'}
            aria-busy={saveState === 'saving'}
            onclick={() => void activateContactAction()}
          >
            {#if saveState === 'saving' || $contactsLoadState === 'loading'}
              <LoaderCircleIcon
                class="size-3.5 animate-spin motion-reduce:animate-none"
                aria-hidden="true"
              />
            {:else if saveState === 'saved' || inContacts}
              <CheckIcon class="size-3.5" aria-hidden="true" />
            {:else if saveState === 'error' || $contactsLoadState === 'error'}
              <RotateCwIcon class="size-3.5" aria-hidden="true" />
            {:else}
              <PlusIcon class="size-3.5" aria-hidden="true" />
            {/if}
            {contactActionLabel}
          </Button>
        </div>

        {#if sendBlocked}
          <p class="text-[13px] text-destructive" role="status">
            {i18n.t('wallet.identity.publicProfile.sendUnavailable', { status: statusLabel })}
          </p>
        {/if}

        {#if saveState === 'error'}
          <p class="text-[13px] text-destructive" role="alert">
            {i18n.t('wallet.identity.publicProfile.saveFailed')}
          </p>
        {/if}

        {#if description}
          <p class="max-w-[560px] pt-2 text-sm leading-[21px] break-words whitespace-pre-wrap">
            {description}
          </p>
        {/if}

        {#if profileEntry?.unavailable}
          <div class="flex items-center gap-3 text-[13px] text-muted-foreground" role="status">
            <span>{i18n.t('wallet.identity.publicProfile.profileUnavailable')}</span>
            <Button
              variant="ghost"
              size="sm"
              class="h-8 px-2"
              onclick={() => void loadIdentityProfile(identity, true, true).catch(() => {})}
            >
              {i18n.t('common.retry')}
            </Button>
          </div>
        {/if}

        <div class="border-t border-b">
          <button
            type="button"
            class="flex min-h-[58px] w-full items-center justify-between gap-4 rounded-sm text-left text-[13px] font-medium text-muted-foreground outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
            aria-expanded={detailsOpen}
            onclick={() => (detailsOpen = !detailsOpen)}
          >
            <span>{i18n.t('wallet.identity.publicProfile.details')}</span>
            <ChevronDownIcon
              class={`size-4 shrink-0 transition-transform ${detailsOpen ? 'rotate-180' : ''}`}
              aria-hidden="true"
            />
          </button>

          {#if detailsOpen}
            <dl class="space-y-4 border-t py-4 text-[13px]">
              <div class="space-y-1.5">
                <dt class="text-muted-foreground">
                  {i18n.t('wallet.identity.publicProfile.identityAddress')}
                </dt>
                <dd class="flex items-start gap-2">
                  <IdentifierText
                    value={identity.identityAddress}
                    mode="full"
                    class="selectable-text min-w-0 flex-1"
                  />
                  <CopyButton
                    size="xs"
                    copied={copied.current === 'address'}
                    aria-label={i18n.t('wallet.identity.publicProfile.copyAddress')}
                    title={i18n.t('wallet.identity.publicProfile.copyAddress')}
                    onclick={() => void copy(identity.identityAddress, 'address')}
                  />
                </dd>
              </div>
              <div class="space-y-1.5">
                <dt class="text-muted-foreground">
                  {i18n.t('wallet.identity.publicProfile.network')}
                </dt>
                <dd>{networkLabel}</dd>
              </div>
              <div class="space-y-1.5">
                <dt class="text-muted-foreground">
                  {i18n.t('wallet.identity.publicProfile.chainId')}
                </dt>
                <dd class="flex items-start gap-2">
                  <IdentifierText
                    value={identity.chainId}
                    mode="full"
                    class="selectable-text min-w-0 flex-1"
                  />
                  <CopyButton
                    size="xs"
                    copied={copied.current === 'chain'}
                    aria-label={i18n.t('wallet.identity.publicProfile.copyChainId')}
                    title={i18n.t('wallet.identity.publicProfile.copyChainId')}
                    onclick={() => void copy(identity.chainId, 'chain')}
                  />
                </dd>
              </div>
              <div class="space-y-1.5">
                <dt class="text-muted-foreground">
                  {i18n.t('wallet.identity.publicProfile.status')}
                </dt>
                <dd>{statusLabel}</dd>
              </div>
            </dl>
          {/if}
        </div>
      </div>
    </ScrollArea.Viewport>
    <ScrollArea.Scrollbar orientation="vertical" />
  </ScrollArea.Root>
</div>
