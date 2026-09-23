<script lang="ts">
  import { untrack } from 'svelte';
  import MoreHorizontalIcon from '@lucide/svelte/icons/ellipsis';
  import CheckIcon from '@lucide/svelte/icons/check';
  import * as identityLinkService from '$lib/services/identityLinkService';
  import { contactSession } from '$lib/contacts/session';
  import { contactChainId } from '$lib/contacts/identity';
  import { Button } from '$lib/components/ui/button';
  import { CopyButton } from '$lib/components/ui/copy-button';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { i18nStore } from '$lib/i18n';
  import type {
    IdentityDetails,
    IdentityProfileLoadResult,
    PendingIdentityProfileUpdate,
    ProfilePublicationState,
    WalletNetwork,
  } from '$lib/types/wallet';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay';
  import { isCompleteProfileRemoval } from '$lib/utils/identityProfileUpdate';
  import IdentityProfileEditor from './IdentityProfileEditor.svelte';
  import VerusIdProfilePage from './VerusIdProfilePage.svelte';
  import { emptyPublication, ProfilePublicationController } from '$lib/identity/profilePublication';
  import { clearProfileDraft, PROFILE_FIELDS } from '$lib/identity/profileDrafts';
  let {
    details,
    profile = null,
    profileLoading = false,
    pendingProfile = null,
    network,
    unlinking = false,
    onBack = () => {},
    onUnlink = () => {},
    onProfileSettled = (_txids: string[]) => {},
    onProfileConfirmed = (
      _update: PendingIdentityProfileUpdate,
      _profile: IdentityProfileLoadResult
    ) => {},
    onProfileSubmitted = (_update: PendingIdentityProfileUpdate) => {},
  }: {
    details: IdentityDetails;
    profile?: IdentityProfileLoadResult | null;
    profileLoading?: boolean;
    pendingProfile?: PendingIdentityProfileUpdate | null;
    network?: WalletNetwork;
    unlinking?: boolean;
    onBack?: () => void;
    onUnlink?: () => void;
    onProfileSettled?: (txids: string[]) => void;
    onProfileConfirmed?: (
      update: PendingIdentityProfileUpdate,
      profile: IdentityProfileLoadResult
    ) => void;
    onProfileSubmitted?: (update: PendingIdentityProfileUpdate) => void;
  } = $props();
  const i18n = $derived($i18nStore);
  const displayName = $derived(formatIdentityDisplayName(details));
  const activeNetwork = $derived(network ?? $contactSession?.network ?? 'mainnet');
  let editing = $state(false);
  let publication = $state(emptyPublication());
  let owner = $state<ProfilePublicationController | null>(null);
  let confirmedProfile = $state<IdentityProfileLoadResult | null>(null);
  let updated = $state(false);
  let updatedTimer: ReturnType<typeof setTimeout> | null = null;
  let confirmDiscardHeader = $state(false);
  let discardingHeader = $state(false);
  let headerDiscardError = $state(false);
  const continuation = $derived(publication.plan);
  const visibleProfile = $derived(confirmedProfile ?? profile);
  const visibleReceipt = $derived(
    publication.completed ? null : (publication.receipt ?? pendingProfile)
  );
  $effect(() => {
    if (confirmedProfile && profile?.revisionTxid === confirmedProfile.revisionTxid)
      confirmedProfile = null;
  });
  function refreshPublication() {
    return owner?.refresh();
  }
  function clearUpdated() {
    if (updatedTimer) clearTimeout(updatedTimer);
    updatedTimer = null;
    updated = false;
  }
  function showUpdated() {
    clearUpdated();
    updated = true;
    updatedTimer = setTimeout(() => {
      updated = false;
      updatedTimer = null;
    }, 5_000);
  }
  $effect(() => {
    const session = $contactSession;
    const address = details.identityAddress;
    editing = false;
    confirmedProfile = null;
    clearUpdated();
    confirmDiscardHeader = false;
    headerDiscardError = false;
    publication = emptyPublication();
    const controller = new ProfilePublicationController(address, {
      current: () => $contactSession === session && details.identityAddress === address,
      read: () =>
        session?.network === 'testnet'
          ? identityLinkService.getIdentityProfilePublication(address)
          : Promise.resolve(null),
      confirm: (txid) => identityLinkService.confirmIdentityProfileUpdate(address, txid),
      submitted: (receipt) => onProfileSubmitted(receipt),
      settled: (txids) => onProfileSettled(txids),
      confirmed: (receipt, canonical, final) => {
        confirmedProfile = canonical;
        onProfileConfirmed(receipt, canonical);
        if (final) {
          editing = false;
          showUpdated();
        }
      },
    });
    owner = controller;
    const unsubscribe = controller.subscribe((value) => {
      publication = value;
    });
    untrack(() => {
      controller.remember(pendingProfile);
      void controller.refresh();
    });
    let timer: ReturnType<typeof setTimeout>;
    let polling = true;
    const poll = async () => {
      await controller.refresh();
      if (polling) {
        // Retry transport failures with a cap; normal confirmation checks stay near 10 s.
        const failures = publication.readFailures;
        timer = setTimeout(poll, Math.min(60_000, 10_000 * 2 ** Math.min(failures, 3)));
      }
    };
    const wake = () => void controller.refresh();
    const visible = () => {
      if (document.visibilityState === 'visible') wake();
    };
    timer = setTimeout(poll, 10_000);
    window.addEventListener('online', wake);
    window.addEventListener('focus', wake);
    window.addEventListener('pageshow', wake);
    document.addEventListener('visibilitychange', visible);
    return () => {
      polling = false;
      clearTimeout(timer);
      if (updatedTimer) clearTimeout(updatedTimer);
      updatedTimer = null;
      window.removeEventListener('online', wake);
      window.removeEventListener('focus', wake);
      window.removeEventListener('pageshow', wake);
      document.removeEventListener('visibilitychange', visible);
      controller.dispose();
      unsubscribe();
    };
  });
  $effect(() => {
    owner?.remember(pendingProfile);
  });
  const copied = new TimedValueState<boolean>();
  const removal = $derived(
    visibleReceipt &&
      isCompleteProfileRemoval(visibleReceipt.previousProfile, visibleReceipt.proposedProfile)
  );
  const changedFields = $derived(
    PROFILE_FIELDS.filter((field) => {
      if (!visibleReceipt) return false;
      const key = field === 'description' ? field : (`${field}Base64` as const);
      return visibleReceipt.previousProfile[key] !== visibleReceipt.proposedProfile[key];
    })
  );
  async function copyTxid() {
    if (visibleReceipt && (await writeClipboardText(visibleReceipt.txid))) copied.set(true);
  }
  async function discardHeader() {
    const plan = continuation;
    const session = $contactSession;
    if (!plan || plan.status !== 'ready' || plan.step !== 2 || !session || discardingHeader) return;
    discardingHeader = true;
    headerDiscardError = false;
    try {
      await identityLinkService.discardIdentityProfilePublication(plan.planId);
      if ($contactSession === session && details.identityAddress === plan.identityAddress) {
        clearProfileDraft(session, plan.identityAddress);
        owner?.discarded();
        confirmDiscardHeader = false;
      }
    } catch {
      headerDiscardError = true;
    } finally {
      discardingHeader = false;
    }
  }
</script>

{#snippet pendingNotice()}
  {#if continuation?.status === 'ready' && continuation.step === 2 && !publication.uncertain && !visibleReceipt}
    <div class="mx-5 mt-3 flex items-center gap-3 rounded-lg bg-muted px-3 py-2 text-[13px]">
      <p class="min-w-0 flex-1">{i18n.t('wallet.identity.profile.sequence.ready')}</p>
      <div class="flex shrink-0 items-center gap-2">
        <Button size="sm" onclick={() => (editing = true)}
          >{i18n.t('wallet.identity.profile.ux.reviewHeader')}</Button
        >
        <DropdownMenu.Root>
          <DropdownMenu.Trigger
            aria-label={i18n.t('wallet.identity.profile.ux.planActions')}
            class="flex size-8 shrink-0 items-center justify-center rounded-md focus-visible:ring-2 focus-visible:ring-ring"
          >
            <MoreHorizontalIcon class="size-4" />
          </DropdownMenu.Trigger>
          <DropdownMenu.Content align="end">
            <DropdownMenu.Item onclick={() => (confirmDiscardHeader = true)}
              >{i18n.t('wallet.identity.profile.ux.discardHeader')}</DropdownMenu.Item
            >
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </div>
    </div>
    {#if confirmDiscardHeader}<div
        class="mx-5 mt-2 rounded-lg border border-border p-3 text-[13px]"
      >
        <p>{i18n.t('wallet.identity.profile.sequence.discardHelp')}</p>
        <div class="mt-3 flex justify-end gap-2">
          <Button
            variant="secondary"
            size="sm"
            disabled={discardingHeader}
            onclick={() => (confirmDiscardHeader = false)}>{i18n.t('common.cancel')}</Button
          >
          <Button
            variant="destructive"
            size="sm"
            disabled={discardingHeader}
            onclick={discardHeader}>{i18n.t('wallet.identity.profile.ux.discardHeader')}</Button
          >
        </div>
        {#if headerDiscardError}<p role="alert" class="mt-2 text-destructive">
            {i18n.t('wallet.identity.profile.ux.discardHeaderFailed')}
          </p>{/if}
      </div>{/if}
  {:else if visibleReceipt || publication.uncertain || continuation?.status === 'waiting'}
    <div class="mx-5 mt-3 rounded-lg bg-muted px-3 py-2 text-[13px]">
      <p class="font-medium">
        {i18n.t(
          publication.uncertain
            ? 'wallet.identity.profile.ux.submissionUncertain'
            : removal
              ? 'wallet.identity.profile.pending.removalTitle'
              : changedFields.length === 1 && changedFields[0] === 'description'
                ? 'wallet.identity.profile.pending.descriptionSubmitted'
                : 'wallet.identity.profile.pending.title'
        )}
      </p>
      {#if visibleReceipt}<div class="mt-2 flex min-w-0 items-center gap-2 border-t pt-2">
          <span class="shrink-0 text-muted-foreground"
            >{i18n.t('wallet.identity.profile.pending.transaction')}</span
          >
          <span class="min-w-0 flex-1 truncate font-mono text-xs">{visibleReceipt.txid}</span>
          <CopyButton
            size="xs"
            copied={Boolean(copied.current)}
            aria-label={i18n.t('wallet.identity.profile.pending.copyTransaction')}
            title={i18n.t('wallet.identity.profile.pending.copyTransaction')}
            onclick={copyTxid}
          />
        </div>{/if}
    </div>
  {:else if continuation && continuation.status !== 'complete'}
    <div
      class="mx-5 mt-3 flex items-center justify-between gap-3 rounded-lg bg-muted px-3 py-2 text-[13px]"
    >
      <p class="text-muted-foreground">
        {i18n.t(
          continuation.status === 'stale'
            ? 'wallet.identity.profile.sequence.stale'
            : 'wallet.identity.profile.sequence.saved'
        )}
      </p>
      <Button variant="ghost" size="sm" onclick={() => (editing = true)}
        >{i18n.t('wallet.identity.profile.sequence.continue')}</Button
      >
    </div>
  {/if}
  {#if publication.readFailures >= 3 && publication.failureSince !== null && Date.now() - publication.failureSince >= 20_000}<p
      role="status"
      class="mx-5 mt-2 text-xs text-muted-foreground"
    >
      {i18n.t('wallet.identity.profile.ux.connectionRetry')}
    </p>{/if}
{/snippet}

{#snippet safety()}
  <div class="mt-3 space-y-2 text-xs text-muted-foreground">
    {#if !details.ownedByPrimaryAddress || details.primaryAddresses.length > 1 || details.minimumSignatures !== 1}<p
      >
        {i18n.t('wallet.identity.detail.warningCards.spendAndSign.warning')}
      </p>{/if}
    {#if details.revocationAuthority && details.revocationAuthority !== details.identityAddress}<p>
        {i18n.t('wallet.identity.detail.warningCards.revoke.warning')}
      </p>{/if}
    {#if details.recoveryAuthority && details.recoveryAuthority !== details.identityAddress}<p>
        {i18n.t('wallet.identity.detail.warningCards.recover.warning')}
      </p>{/if}
  </div>
{/snippet}

{#if editing && owner}
  <IdentityProfileEditor
    {details}
    profile={visibleProfile}
    publication={owner}
    onCancel={() => {
      editing = false;
      void refreshPublication();
    }}
  />
{:else}
  <VerusIdProfilePage
    identity={{
      identityAddress: details.identityAddress,
      fullyQualifiedName: displayName,
      network: activeNetwork,
      chainId: contactChainId(activeNetwork),
      status: details.status,
    }}
    linked={true}
    owner={{
      details,
      profile: visibleProfile,
      profileLoading,
      pending: pendingProfile,
      unlinking,
      onEdit: () => {
        clearUpdated();
        editing = true;
      },
      onUnlink,
    }}
    ownerNotice={pendingNotice}
    ownerSafety={safety}
    {onBack}
  />
  {#if updated}<div
      role="status"
      class="fixed right-5 bottom-5 z-50 flex items-center gap-2 rounded-lg border border-border bg-background px-4 py-3 text-sm font-medium shadow-sm"
    >
      <CheckIcon class="size-4" />
      {i18n.t('wallet.identity.profile.confirmed.title')}
    </div>{/if}
{/if}
