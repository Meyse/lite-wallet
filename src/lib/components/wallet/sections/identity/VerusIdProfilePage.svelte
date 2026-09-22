<script lang="ts">
  import { profileMediaUrl } from '$lib/identity/profileImages';
  import { onDestroy, onMount, type Snippet, tick } from 'svelte';
  import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
  import AtSignIcon from '@lucide/svelte/icons/at-sign';
  import CheckIcon from '@lucide/svelte/icons/check';
  import CircleCheckIcon from '@lucide/svelte/icons/circle-check';
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
  import GlobeIcon from '@lucide/svelte/icons/globe';
  import { Spinner } from '$lib/components/ui/spinner';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import RotateCwIcon from '@lucide/svelte/icons/rotate-cw';
  import WalletIcon from '@lucide/svelte/icons/wallet';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import InlineTextActionButton from '$lib/components/common/InlineTextActionButton.svelte';
  import NavigationBackButton from '$lib/components/common/NavigationBackButton.svelte';
  import { Button } from '$lib/components/ui/button';
  import { CopyButton } from '$lib/components/ui/copy-button';
  import * as Dialog from '$lib/components/ui/dialog';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import * as Tabs from '$lib/components/ui/tabs';
  import { identityKey, matchingContacts } from '$lib/contacts/identity';
  import { getContactNavigation } from '$lib/contacts/navigation';
  import { identityProfiles, loadIdentityProfile, profileImage } from '$lib/contacts/profiles';
  import { addIdentityContact, loadContacts } from '$lib/contacts/service';
  import { contactSession, contactsLoadState } from '$lib/contacts/session';
  import { i18nStore } from '$lib/i18n';
  import { getIdentityDetails } from '$lib/services/identityLinkService';
  import { addressBookStore } from '$lib/stores/addressBook';
  import type { ResolvedContactIdentity } from '$lib/types/addressBook';
  import type {
    IdentityDetails,
    IdentityProfileLoadResult,
    PendingIdentityProfileUpdate,
  } from '$lib/types/wallet';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import { openTrustedExternalUrl } from '$lib/utils/externalLinks';
  import IdentityAvatar from './IdentityAvatar.svelte';
  import ProfileSocialLink from './ProfileSocialLink.svelte';
  import {
    profileExternalUrl,
    profileHeaderImage,
    type PublicProfileContent,
  } from './publicProfileContent';

  let {
    identity,
    owner,
    ownerNotice,
    ownerSafety,
    linked = false,
    linkedUnavailable = false,
    onRetryLinked = () => {},
    content = {},
    navigationDisabled = false,
    backLabel,
    onBack = () => {},
    onSend = (_identity: ResolvedContactIdentity) => {},
  }: {
    identity: ResolvedContactIdentity;
    owner?: {
      details: IdentityDetails | null;
      loading?: boolean;
      profile: IdentityProfileLoadResult | null;
      profileLoading: boolean;
      pending: PendingIdentityProfileUpdate | null;
      unlinking: boolean;
      onEdit: () => void;
      onUnlink: () => void;
    };
    ownerNotice?: Snippet;
    ownerSafety?: Snippet;
    linked?: boolean | null;
    linkedUnavailable?: boolean;
    onRetryLinked?: () => void;
    content?: PublicProfileContent;
    navigationDisabled?: boolean;
    backLabel?: string;
    onBack?: () => void;
    onSend?: (identity: ResolvedContactIdentity) => void;
  } = $props();

  let details = $state<IdentityDetails | null>(null);
  let detailsState = $state<'loading' | 'ready' | 'error'>('loading');
  let detailsRefresh = $state(0);
  let activeTab = $state('websites');
  let saveState = $state<'idle' | 'saving' | 'saved' | 'error'>('idle');
  let alive = true;
  let contactButton = $state<HTMLButtonElement | null>(null);
  let panelViewport = $state<HTMLDivElement | null>(null);
  let canScrollDown = $state(false);
  let unlinkDialogOpen = $state(false);
  const copied = new TimedValueState<string>();

  const openContact = getContactNavigation();
  const i18n = $derived($i18nStore);
  const profileEntry = $derived($identityProfiles[identityKey(identity)]);
  const profile = $derived(owner ? owner.profile : (profileEntry?.profile ?? null));
  const previous = $derived(
    owner?.pending && (!profile || profile.state === 'unavailable')
      ? owner.pending.previousProfile
      : null
  );
  const description = $derived(previous?.description ?? profile?.description?.value?.trim() ?? '');
  const avatarUrl = $derived(
    previous?.avatarBase64
      ? profileMediaUrl(previous.avatarBase64, previous.avatarMimeType)
      : profileImage(profile)
  );
  const headerUrl = $derived(
    previous?.headerBase64
      ? profileMediaUrl(previous.headerBase64, previous.headerMimeType)
      : profileHeaderImage(profile?.header?.value ?? content.header)
  );
  let failedHeader = $state<string | null>(null);
  const websites = $derived(
    (content.websites ?? []).flatMap((site) => {
      const url = profileExternalUrl(site.url);
      return url ? [{ ...site, url, domain: new URL(url).hostname }] : [];
    })
  );
  const socials = $derived(
    (content.socials ?? []).filter((social) => profileExternalUrl(social.profileUrl))
  );
  const matches = $derived(matchingContacts($addressBookStore, identity));
  const inContacts = $derived(matches.length > 0 || saveState === 'saved');
  const normalizedStatus = $derived(
    (details?.status || identity.status)?.trim().toLowerCase() ?? ''
  );
  const sendBlocked = $derived(normalizedStatus.length > 0 && normalizedStatus !== 'active');
  const networkLabel = $derived(
    i18n.t(
      identity.network === 'testnet'
        ? 'wallet.identity.lookup.network.testnet'
        : 'wallet.identity.lookup.network.mainnet'
    )
  );
  const statusLabel = $derived(
    ['active', 'revoked', 'inactive'].includes(normalizedStatus)
      ? i18n.t(`wallet.identity.publicProfile.status.${normalizedStatus}`)
      : normalizedStatus || i18n.t('wallet.identity.publicProfile.status.unknown')
  );
  const addresses = $derived([
    { network: 'VerusID', address: identity.identityAddress, verification: null },
    ...(content.addresses ?? []),
  ]);

  const contactActionLabel = $derived(
    saveState === 'saving'
      ? i18n.t('wallet.identity.publicProfile.adding')
      : inContacts
        ? i18n.t('wallet.identity.publicProfile.inContacts')
        : saveState === 'error' || $contactsLoadState === 'error'
          ? i18n.t('wallet.contacts.retry')
          : $contactsLoadState === 'ready'
            ? i18n.t('wallet.contacts.add')
            : i18n.t('wallet.identity.publicProfile.checkingContacts')
  );

  const detailRows: {
    label: string;
    value?: string | null;
    identifier?: boolean;
    copy?: boolean;
  }[] = $derived([
    { label: i18n.t('wallet.identity.publicProfile.status'), value: statusLabel },
    {
      label: i18n.t('wallet.identity.publicProfile.system'),
      value: details?.systemDisplayName || networkLabel,
    },
    ...(details
      ? [
          {
            label: i18n.t('wallet.identity.publicProfile.signingThreshold'),
            value: i18n.t('wallet.identity.publicProfile.signatures', {
              count: details.minimumSignatures,
              total: details.primaryAddresses.length,
            }),
          },
          {
            label: i18n.t('wallet.identity.publicProfile.revocationAuthority'),
            value: details.revocationAuthorityName || details.revocationAuthority,
            identifier: !details.revocationAuthorityName,
          },
          {
            label: i18n.t('wallet.identity.publicProfile.recoveryAuthority'),
            value: details.recoveryAuthorityName || details.recoveryAuthority,
            identifier: !details.recoveryAuthorityName,
          },
          ...details.primaryAddresses.map((value) => ({
            label: i18n.t('wallet.identity.publicProfile.primaryAddress'),
            value,
            identifier: true,
            copy: true,
          })),
        ]
      : []),
  ]);

  $effect(() => {
    if (owner) {
      details = owner.details;
      detailsState = owner.loading ? 'loading' : 'ready';
      return undefined;
    }
    const currentIdentity = identity;
    const session = $contactSession;
    detailsRefresh;
    let current = true;
    details = null;
    detailsState = 'loading';
    if (!session) {
      detailsState = 'error';
      return undefined;
    }
    void getIdentityDetails(currentIdentity.identityAddress)
      .then((result) => {
        // A lookup can change or a wallet can lock while the existing RPC is in flight.
        if (!current || $contactSession !== session) return;
        if (!result || result.identityAddress !== currentIdentity.identityAddress) {
          detailsState = 'error';
          return;
        }
        details = result;
        detailsState = 'ready';
      })
      .catch(() => {
        if (current) detailsState = 'error';
      });
    return () => {
      current = false;
    };
  });

  function updateFade(): void {
    canScrollDown =
      !!panelViewport &&
      panelViewport.scrollHeight - panelViewport.clientHeight - panelViewport.scrollTop > 1;
  }
  $effect(() => {
    activeTab;
    details;
    detailsState;
    content;
    const viewport = panelViewport;
    if (!viewport) return undefined;
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(updateFade);
    observer?.observe(viewport);
    if (viewport.firstElementChild) observer?.observe(viewport.firstElementChild);
    void tick().then(updateFade);
    return () => observer?.disconnect();
  });

  $effect(() => {
    activeTab;
    if (panelViewport) panelViewport.scrollTop = 0;
  });

  async function copy(value: string, key: string): Promise<void> {
    if (await writeClipboardText(value)) copied.set(key);
  }
  async function activateContactAction(): Promise<void> {
    if (linked !== false || navigationDisabled || saveState === 'saving') return;
    if (inContacts) {
      openContact?.(identity, contactButton);
      return;
    }
    if ($contactsLoadState !== 'ready') {
      await loadContacts(true).catch(() => {});
      return;
    }
    const savingIdentity = identity;
    const session = $contactSession;
    saveState = 'saving';
    try {
      await addIdentityContact(savingIdentity);
      if (alive && identity === savingIdentity && $contactSession === session) saveState = 'saved';
    } catch {
      if (alive && identity === savingIdentity && $contactSession === session) saveState = 'error';
    }
  }
  onMount(() => {
    if (owner) return;
    void loadIdentityProfile(identity, false, true).catch(() => {});
    void loadContacts().catch(() => {});
  });
  onDestroy(() => {
    alive = false;
  });
</script>

<div
  class="flex h-full min-h-0 w-full min-w-0 flex-col px-5 pt-5 pb-5"
  aria-busy={owner?.loading ? 'true' : undefined}
  data-verusid-profile-page
  data-owner-profile-state={owner?.loading ? 'loading' : owner ? 'ready' : 'public'}
>
  {#if owner?.loading}
    <p class="sr-only" role="status">{i18n.t('wallet.identity.detail.loading')}</p>
  {/if}
  <div class="flex h-9 shrink-0 items-start justify-between">
    <NavigationBackButton
      label={owner
        ? i18n.t('wallet.identity.detail.back')
        : (backLabel ?? i18n.t('wallet.identity.publicProfile.backToSearch'))}
      onclick={onBack}
    />
    {#if owner}<InlineTextActionButton
        tone="destructive"
        class="h-8 px-1 text-[13px]"
        disabled={owner.unlinking}
        onclick={() => (unlinkDialogOpen = true)}
        data-unlink-identity-action
        >{i18n.t(
          owner.unlinking ? 'wallet.identity.detail.unlinking' : 'wallet.identity.detail.unlink'
        )}</InlineTextActionButton
      >{/if}
  </div>
  <div class="mx-auto mt-4 flex min-h-0 w-full max-w-[800px] flex-1 flex-col">
    <div class="flex shrink-0 flex-col pb-7 {socials.length ? 'min-h-[302px]' : ''}">
      {#if headerUrl && headerUrl !== failedHeader}
        <img
          src={headerUrl}
          alt=""
          class="aspect-[6/1] w-full rounded-[14px] object-cover"
          onerror={() => (failedHeader = headerUrl)}
        />
      {:else}
        <div class="h-[84px] shrink-0 rounded-[14px] bg-muted" aria-hidden="true"></div>
      {/if}
      <div
        class="relative mt-3.5 flex min-h-[34px] flex-wrap items-center justify-end gap-2 pl-[120px]"
      >
        <div
          class="absolute bottom-1 left-5 rounded-full border-4 border-background bg-background dark:border-app-canvas dark:bg-app-canvas [&>div]:text-[30px] [&>div]:font-medium"
        >
          <IdentityAvatar
            seed={identity.identityAddress}
            label={identity.fullyQualifiedName}
            imageUrl={avatarUrl}
            class="size-20 text-2xl"
          />
        </div>
        {#if owner}<Button
            class="h-[34px]"
            onclick={owner.onEdit}
            disabled={owner.loading ||
              !owner.details?.profileEditable ||
              owner.profileLoading ||
              !profile ||
              profile.state === 'unavailable' ||
              Boolean(profile.issues.length) ||
              Boolean(owner.pending)}>{i18n.t('wallet.identity.profile.edit')}</Button
          >{/if}
        {#if linked}
          <span
            class="inline-flex min-h-[34px] items-center gap-[7px] rounded-md bg-contact-saved px-3 text-sm font-medium text-contact-saved-foreground select-none"
            ><CheckIcon class="size-4" aria-hidden="true" />{i18n.t(
              'wallet.identity.publicProfile.linked'
            )}</span
          >
        {:else if linked === null}
          <Button
            variant="secondary"
            class="h-[34px] gap-[7px] px-3 text-sm"
            disabled={!linkedUnavailable || navigationDisabled}
            onclick={onRetryLinked}
          >
            {#if linkedUnavailable}<RotateCwIcon class="size-4" aria-hidden="true" />{:else}<Spinner
                class="size-4"
              />{/if}
            {i18n.t(linkedUnavailable ? 'common.retry' : 'common.loading')}
          </Button>
        {:else}
          <Button
            variant="secondary"
            class="h-[34px] gap-[7px] px-3 text-sm"
            disabled={navigationDisabled || sendBlocked}
            onclick={() => onSend(identity)}
            ><ArrowUpIcon class="size-4" aria-hidden="true" />{i18n.t(
              'wallet.identity.publicProfile.send'
            )}</Button
          >
          <Button
            bind:ref={contactButton}
            variant="secondary"
            class="h-[34px] min-w-[143px] gap-[7px] px-3 text-sm select-none {inContacts
              ? 'bg-contact-saved text-contact-saved-foreground hover:bg-contact-saved'
              : ''}"
            disabled={navigationDisabled ||
              saveState === 'saving' ||
              $contactsLoadState === 'loading' ||
              $contactsLoadState === 'idle' ||
              (inContacts && !openContact)}
            aria-busy={saveState === 'saving'}
            onclick={() => void activateContactAction()}
          >
            {#if saveState === 'saving' || $contactsLoadState === 'loading'}<Spinner
                class="size-4"
              />
            {:else if inContacts}<CheckIcon class="size-4" aria-hidden="true" />
            {:else if saveState === 'error' || $contactsLoadState === 'error'}<RotateCwIcon
                class="size-4"
                aria-hidden="true"
              />
            {:else}<PlusIcon class="size-4" aria-hidden="true" />{/if}
            {contactActionLabel}
          </Button>
        {/if}
      </div>
      <div class="mx-5 mt-3 flex min-w-0 items-start gap-2.5">
        <h2 class="min-w-0 text-[28px] leading-[34px] font-semibold tracking-[-0.02em] break-words">
          {identity.fullyQualifiedName}
        </h2>
        <CopyButton
          class="mt-0.5 shrink-0"
          size="sm"
          copied={copied.current === 'name'}
          aria-label={i18n.t('wallet.contacts.copyIdentity')}
          title={i18n.t('wallet.contacts.copyIdentity')}
          onclick={() => void copy(identity.fullyQualifiedName, 'name')}
        />
      </div>
      {#if description}<p class="mx-5 mt-2 text-sm leading-[21px] break-words whitespace-pre-wrap">
          {description}
        </p>{/if}
      {#if socials.length}<div class="mx-5 mt-4 flex flex-wrap items-center gap-x-[22px] gap-y-1">
          {#each socials as social}<ProfileSocialLink {social} />{/each}
        </div>{/if}
      {#if sendBlocked && !linked}<p class="mx-5 mt-3 text-[13px] text-destructive" role="status">
          {i18n.t('wallet.identity.publicProfile.sendUnavailable', { status: statusLabel })}
        </p>{/if}
      {#if saveState === 'error'}<p class="mx-5 mt-3 text-[13px] text-destructive" role="alert">
          {i18n.t('wallet.identity.publicProfile.saveFailed')}
        </p>{/if}
      <span class="sr-only" aria-live="polite"
        >{saveState === 'saving' || saveState === 'saved' ? contactActionLabel : ''}</span
      >
      {#if owner?.details && !owner.details.profileEditable}<p
          class="mx-5 mt-3 text-[13px] text-muted-foreground"
        >
          {i18n.t(
            `wallet.identity.profile.readOnly.${owner.details.profileEditabilityReason ?? 'unsupported_control'}`
          )}
        </p>{/if}
      {#if ownerNotice}{@render ownerNotice()}{/if}
      {#if !owner && profileEntry?.unavailable}
        <div
          class="mx-5 mt-3 flex items-center gap-3 text-[13px] text-muted-foreground"
          role="status"
        >
          <span>{i18n.t('wallet.identity.publicProfile.profileUnavailable')}</span><Button
            variant="ghost"
            size="sm"
            onclick={() => void loadIdentityProfile(identity, true, true).catch(() => {})}
            >{i18n.t('common.retry')}</Button
          >
        </div>
      {/if}
    </div>
    <Tabs.Root bind:value={activeTab} class="flex min-h-0 flex-1 flex-col">
      <Tabs.List
        class="flex h-9 shrink-0 justify-start gap-6 rounded-none border-b bg-transparent p-0"
        aria-label={i18n.t('wallet.identity.publicProfile.sections')}
      >
        {#each ['websites', 'addresses', 'details'] as tab}
          <Tabs.Trigger
            value={tab}
            class="h-9 shrink-0 gap-1.5 rounded-none border-b-2 border-transparent px-0 text-sm font-normal data-[state=active]:border-text-action data-[state=active]:bg-transparent data-[state=active]:text-text-action data-[state=active]:shadow-none"
            >{i18n.t(`wallet.identity.publicProfile.${tab}`)}{#if tab === 'addresses'}<span
                class="text-xs text-muted-foreground">{addresses.length}</span
              >{/if}</Tabs.Trigger
          >
        {/each}
      </Tabs.List>
      <ScrollArea.Root class="relative mt-3 min-h-0 flex-1" type="scroll">
        <ScrollArea.Viewport
          bind:ref={panelViewport}
          onscroll={updateFade}
          tabindex={0}
          aria-label={i18n.t(`wallet.identity.publicProfile.${activeTab}`)}
        >
          <div class="pb-1">
            <Tabs.Content value="websites" class="space-y-2">
              {#each websites as website}
                <a
                  href={website.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  class="flex min-h-[34px] items-center gap-2 rounded-sm text-sm outline-none focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
                  onclick={(event) => {
                    event.preventDefault();
                    void openTrustedExternalUrl(website.url);
                  }}
                >
                  <span class="flex w-[22px] shrink-0 justify-center text-muted-foreground"
                    ><GlobeIcon class="size-4" aria-hidden="true" /></span
                  ><span class="min-w-0 flex-1 truncate text-text-action">{website.domain}</span>
                  {#if website.verification === 'verified'}<span
                      class="flex shrink-0 items-center gap-1 text-xs text-contact-saved-foreground"
                      ><CircleCheckIcon class="size-3.5" aria-hidden="true" />{i18n.t(
                        'wallet.identity.publicProfile.verified'
                      )}</span
                    >{/if}<ExternalLinkIcon
                    class="mx-1 size-3.5 shrink-0 text-muted-foreground"
                    aria-hidden="true"
                  />
                </a>
              {:else}<p class="py-3 text-sm text-muted-foreground">
                  {i18n.t('wallet.identity.publicProfile.noWebsites')}
                </p>{/each}
            </Tabs.Content>
            <Tabs.Content value="addresses">
              {#each addresses as address, index}
                <div class="flex min-h-[70px] items-center gap-3 border-b last:border-0">
                  <span class="flex w-6 shrink-0 justify-center text-muted-foreground"
                    >{#if index === 0}<AtSignIcon
                        class="size-5"
                        aria-hidden="true"
                      />{:else}<WalletIcon class="size-5" aria-hidden="true" />{/if}</span
                  >
                  <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-2 text-sm font-medium">
                      {address.network}{#if address.verification === 'verified'}<CircleCheckIcon
                          class="size-3.5 text-contact-saved-foreground"
                          aria-label={i18n.t('wallet.identity.publicProfile.verified')}
                        />{/if}
                    </div>
                    <IdentifierText
                      value={address.address}
                      mode="full"
                      class="mt-1 text-[13px] text-muted-foreground"
                    />
                  </div>
                  <CopyButton
                    class="shrink-0"
                    size="sm"
                    copied={copied.current === `address-${index}`}
                    aria-label={i18n.t('wallet.identity.publicProfile.copyPublicAddress', {
                      network: address.network,
                    })}
                    onclick={() => void copy(address.address, `address-${index}`)}
                  />
                </div>
              {/each}
            </Tabs.Content>
            <Tabs.Content value="details">
              <dl class="text-[13px]">
                {#each detailRows as row, index}
                  <div
                    class="grid min-h-[38px] grid-cols-[minmax(120px,180px)_minmax(0,1fr)] items-start gap-5 border-b py-2.5 last:border-0"
                  >
                    <dt class="text-muted-foreground">{row.label}</dt>
                    <dd class="flex min-w-0 items-start gap-2">
                      {#if row.identifier && row.value}
                        <IdentifierText value={row.value} mode="full" class="min-w-0 flex-1" />
                      {:else}
                        <span class="min-w-0 flex-1 break-words"
                          >{row.value || i18n.t('wallet.identity.detail.notAvailable')}</span
                        >
                      {/if}
                      {#if row.copy && row.value}<CopyButton
                          size="xs"
                          copied={copied.current === `detail-${index}`}
                          aria-label={i18n.t('wallet.identity.publicProfile.copyDetail', {
                            label: row.label,
                          })}
                          onclick={() => void copy(row.value ?? '', `detail-${index}`)}
                        />{/if}
                    </dd>
                  </div>
                {/each}
              </dl>
              {#if ownerSafety}{@render ownerSafety()}{/if}
              {#if detailsState === 'loading'}<p
                  class="flex items-center gap-2 py-3 text-[13px] text-muted-foreground"
                  role="status"
                >
                  <Spinner class="size-3.5" />{i18n.t('common.loading')}
                </p>
              {:else if detailsState === 'error'}<div
                  class="flex items-center gap-2 py-3 text-[13px] text-muted-foreground"
                  role="status"
                >
                  <span>{i18n.t('wallet.identity.publicProfile.detailsUnavailable')}</span><Button
                    variant="ghost"
                    size="sm"
                    onclick={() => detailsRefresh++}>{i18n.t('common.retry')}</Button
                  >
                </div>{/if}
            </Tabs.Content>
          </div>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
        {#if canScrollDown}
          <div
            data-profile-scroll-fade
            class="pointer-events-none absolute inset-x-0 bottom-0 h-10 bg-gradient-to-t from-background to-transparent dark:from-app-canvas"
            aria-hidden="true"
          ></div>
        {/if}
      </ScrollArea.Root>
    </Tabs.Root>
  </div>
</div>

{#if owner}
  <Dialog.Root
    open={unlinkDialogOpen}
    onOpenChange={(open) => {
      if (!open && !owner.unlinking) unlinkDialogOpen = false;
    }}
  >
    <Dialog.Content class="max-w-md" showCloseButton={!owner.unlinking}>
      <Dialog.Header>
        <Dialog.Title
          >{i18n.t('wallet.identity.detail.unlinkConfirmTitle', {
            identity: identity.fullyQualifiedName,
          })}</Dialog.Title
        >
        <Dialog.Description
          >{i18n.t('wallet.identity.detail.unlinkConfirmDescription')}</Dialog.Description
        >
      </Dialog.Header>
      <Dialog.Footer class="flex justify-end gap-3">
        <Button
          variant="secondary"
          disabled={owner.unlinking}
          onclick={() => (unlinkDialogOpen = false)}
        >
          {i18n.t('common.cancel')}
        </Button>
        <Button
          variant="destructive"
          disabled={owner.unlinking}
          onclick={owner.onUnlink}
          data-unlink-identity-confirm
        >
          {i18n.t(
            owner.unlinking ? 'wallet.identity.detail.unlinking' : 'wallet.identity.detail.unlink'
          )}
        </Button>
      </Dialog.Footer>
    </Dialog.Content>
  </Dialog.Root>
{/if}
