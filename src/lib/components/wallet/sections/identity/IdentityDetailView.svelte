<script lang="ts">
  import { untrack } from 'svelte';
  import * as identityLinkService from '$lib/services/identityLinkService';
  import { contactSession } from '$lib/contacts/session';
  import { contactChainId } from '$lib/contacts/identity';
  import { Button } from '$lib/components/ui/button';
  import { CopyButton } from '$lib/components/ui/copy-button';
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
  import { PROFILE_FIELDS } from '$lib/identity/profileDrafts';
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
  const continuation = $derived(publication.plan);
  const continuationError = $derived(publication.error);
  const visibleProfile = $derived(confirmedProfile ?? profile);
  $effect(() => {
    if (confirmedProfile && profile?.revisionTxid === confirmedProfile.revisionTxid)
      confirmedProfile = null;
  });
  function refreshPublication() {
    return owner?.refresh();
  }
  $effect(() => {
    const session = $contactSession;
    const address = details.identityAddress;
    editing = false;
    confirmedProfile = null;
    updated = false;
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
          updated = true;
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
    const timer = setInterval(() => void controller.refresh(), 15_000);
    return () => {
      clearInterval(timer);
      controller.dispose();
      unsubscribe();
    };
  });
  $effect(() => {
    owner?.remember(pendingProfile);
  });
  let showChanges = $state(false);
  const copied = new TimedValueState<boolean>();
  const removal = $derived(
    pendingProfile &&
      isCompleteProfileRemoval(pendingProfile.previousProfile, pendingProfile.proposedProfile)
  );
  const changedFields = $derived(
    PROFILE_FIELDS.filter((field) => {
      if (!pendingProfile) return false;
      const key = field === 'description' ? field : (`${field}Base64` as const);
      return pendingProfile.previousProfile[key] !== pendingProfile.proposedProfile[key];
    })
  );
  async function copyTxid() {
    if (pendingProfile && (await writeClipboardText(pendingProfile.txid))) copied.set(true);
  }
</script>

{#snippet pendingNotice()}
  {#if updated}<p role="status" class="mx-5 mt-3 text-[13px] text-muted-foreground">
      {i18n.t('wallet.identity.profile.confirmed.title')}
    </p>{/if}
  {#if continuation && continuation.status !== 'complete'}
    <div
      class="mx-5 mt-3 flex items-center justify-between gap-3 rounded-lg bg-muted px-3 py-2 text-[13px]"
    >
      <p class="text-muted-foreground">
        {i18n.t(
          continuation.status === 'waiting'
            ? 'wallet.identity.profile.sequence.waiting'
            : continuation.status === 'stale'
              ? 'wallet.identity.profile.sequence.stale'
              : continuation.step === 2
                ? 'wallet.identity.profile.sequence.ready'
                : 'wallet.identity.profile.sequence.saved'
        )}
      </p>
      <Button
        variant="ghost"
        size="sm"
        class="shrink-0 text-text-action"
        onclick={() => (editing = true)}
        >{i18n.t('wallet.identity.profile.sequence.continue')}</Button
      >
    </div>
  {:else if continuationError}
    <div class="mx-5 mt-3 flex items-center justify-between gap-3 text-xs text-muted-foreground">
      <p>{i18n.t('wallet.identity.profile.sequence.resumeUnavailable')}</p>
      <Button variant="ghost" size="sm" onclick={refreshPublication}
        >{i18n.t('wallet.identity.profile.sequence.refresh')}</Button
      >
    </div>
  {:else if pendingProfile}
    <div class="mx-5 mt-3 rounded-lg bg-muted px-3 py-2 text-[13px]">
      <div class="flex items-center justify-between gap-3">
        <span class="font-medium"
          >{i18n.t(
            removal
              ? 'wallet.identity.profile.pending.removalTitle'
              : 'wallet.identity.profile.pending.title'
          )}</span
        ><Button
          variant="ghost"
          size="sm"
          class="h-7 text-text-action"
          onclick={() => (showChanges = !showChanges)}
          >{i18n.t(
            showChanges
              ? 'wallet.identity.profile.pending.hideChanges'
              : 'wallet.identity.profile.pending.viewChanges'
          )}</Button
        >
      </div>
      <p class="text-xs text-muted-foreground">
        {i18n.t(
          removal
            ? 'wallet.identity.profile.pending.removalDescription'
            : 'wallet.identity.profile.pending.description'
        )}
      </p>
      {#if showChanges}<div class="mt-2 space-y-1.5 border-t pt-2">
          {#each changedFields as field}<div class="flex justify-between gap-4">
              <span class="shrink-0 text-muted-foreground"
                >{i18n.t(`wallet.identity.profile.draft.${field}`)}</span
              ><span class="min-w-0 truncate"
                >{field === 'description'
                  ? pendingProfile.proposedProfile.description ||
                    i18n.t('wallet.identity.profile.review.removed')
                  : pendingProfile.proposedProfile[`${field}Base64`]
                    ? i18n.t('wallet.identity.profile.draft.changing')
                    : i18n.t('wallet.identity.profile.review.removed')}</span
              >
            </div>{/each}
          <div class="flex min-w-0 items-center gap-2">
            <span class="shrink-0 text-muted-foreground"
              >{i18n.t('wallet.identity.profile.pending.notConfirmed')}</span
            ><span class="min-w-0 flex-1 truncate font-mono text-xs">{pendingProfile.txid}</span
            ><CopyButton
              copied={Boolean(copied.current)}
              onclick={copyTxid}
              aria-label={i18n.t('wallet.identity.detail.copy')}
            />
          </div>
        </div>{/if}
    </div>
  {/if}
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
        updated = false;
        editing = true;
      },
      onUnlink,
    }}
    ownerNotice={pendingNotice}
    ownerSafety={safety}
    {onBack}
  />
{/if}
