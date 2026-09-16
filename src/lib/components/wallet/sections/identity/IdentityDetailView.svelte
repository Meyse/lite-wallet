<script lang="ts">
  import AlertTriangleIcon from '@lucide/svelte/icons/alert-triangle';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import Clock3Icon from '@lucide/svelte/icons/clock-3';
  import Link2OffIcon from '@lucide/svelte/icons/link-2-off';
  import { toast } from 'svelte-sonner';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import { Button } from '$lib/components/ui/button';
  import { CopyButton } from '$lib/components/ui/copy-button';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { i18nStore } from '$lib/i18n';
  import type {
    IdentityDetails,
    IdentityProfileLoadResult,
    PendingIdentityProfileUpdate,
  } from '$lib/types/wallet.js';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay';
  import { isCompleteProfileRemoval } from '$lib/utils/identityProfileUpdate';
  import IdentityAvatar from './IdentityAvatar.svelte';
  import IdentityProfileEditor from './IdentityProfileEditor.svelte';

  const noop = (): void => {};
  const noopSubmitted = (_update: PendingIdentityProfileUpdate): void => {};

  type IdentityDetailViewProps = {
    details: IdentityDetails;
    profile?: IdentityProfileLoadResult | null;
    profileLoading?: boolean;
    pendingProfile?: PendingIdentityProfileUpdate | null;
    unlinking?: boolean;
    onBack?: () => void;
    onUnlink?: () => void;
    onProfileSubmitted?: (update: PendingIdentityProfileUpdate) => void;
  };

  let {
    details,
    profile = null,
    profileLoading = false,
    pendingProfile = null,
    unlinking = false,
    onBack = noop,
    onUnlink = noop,
    onProfileSubmitted = noopSubmitted,
  }: IdentityDetailViewProps = $props();

  const i18n = $derived($i18nStore);
  const copiedKeyState = new TimedValueState<string>();
  const copiedKey = $derived(copiedKeyState.current);
  const displayName = $derived(formatIdentityDisplayName(details));
  const displayNameIsAddress = $derived(displayName === details.identityAddress);
  const usePendingProfileFallback = $derived(!profile || profile.state === 'unavailable');
  const displayedAvatarBase64 = $derived(
    usePendingProfileFallback
      ? (pendingProfile?.previousProfile.avatarBase64 ?? null)
      : (profile?.avatar?.value.base64 ?? null)
  );
  const displayedAvatarMime = $derived(
    usePendingProfileFallback ? 'image/jpeg' : (profile?.avatar?.value.mimeType ?? 'image/jpeg')
  );
  const avatarUrl = $derived(
    displayedAvatarBase64 ? `data:${displayedAvatarMime};base64,${displayedAvatarBase64}` : null
  );
  const description = $derived(
    usePendingProfileFallback
      ? (pendingProfile?.previousProfile.description?.trim() ?? '')
      : (profile?.description?.value?.trim() ?? '')
  );
  const blockchainName = $derived(details.systemDisplayName?.trim() ?? '');
  const revocationAuthorityName = $derived(
    details.revocationAuthorityName?.trim() ||
      (details.revocationAuthority?.toLowerCase() === details.identityAddress.toLowerCase() &&
      !displayNameIsAddress
        ? displayName
        : '')
  );
  const recoveryAuthorityName = $derived(
    details.recoveryAuthorityName?.trim() ||
      (details.recoveryAuthority?.toLowerCase() === details.identityAddress.toLowerCase() &&
      !displayNameIsAddress
        ? displayName
        : '')
  );
  const revokeAuthorityExternal = $derived(
    details.revocationAuthority
      ? details.revocationAuthority.toLowerCase() !== details.identityAddress.toLowerCase()
      : false
  );
  const recoveryAuthorityExternal = $derived(
    details.recoveryAuthority
      ? details.recoveryAuthority.toLowerCase() !== details.identityAddress.toLowerCase()
      : false
  );
  const spendAndSignWarning = $derived(
    !details.ownedByPrimaryAddress ||
      details.primaryAddresses.length > 1 ||
      details.minimumSignatures !== 1
  );
  const authorityWarningCount = $derived(
    Number(spendAndSignWarning) +
      Number(revokeAuthorityExternal) +
      Number(recoveryAuthorityExternal)
  );
  const authorityWarningSummary = $derived(
    authorityWarningCount === 0
      ? i18n.t('wallet.identity.detail.noWarnings')
      : i18n.t(
          authorityWarningCount === 1
            ? 'wallet.identity.detail.warningCount.one'
            : 'wallet.identity.detail.warningCount.other',
          { count: authorityWarningCount }
        )
  );
  const pendingAvatarChanged = $derived(
    Boolean(pendingProfile) &&
      (pendingProfile?.previousProfile.avatarDigest !==
        pendingProfile?.proposedProfile.avatarDigest ||
        pendingProfile?.previousProfile.avatarBase64 !==
          pendingProfile?.proposedProfile.avatarBase64)
  );
  const pendingDescriptionChanged = $derived(
    Boolean(pendingProfile) &&
      (pendingProfile?.previousProfile.descriptionDigest !==
        pendingProfile?.proposedProfile.descriptionDigest ||
        pendingProfile?.previousProfile.description !== pendingProfile?.proposedProfile.description)
  );
  const pendingRemovesEntireProfile = $derived(
    Boolean(pendingProfile) &&
      isCompleteProfileRemoval(
        pendingProfile?.previousProfile ?? {},
        pendingProfile?.proposedProfile ?? {}
      )
  );

  let editingProfile = $state(false);
  let showSubmittedChanges = $state(false);

  function showValue(value: string | null | undefined): string {
    return value?.trim() || i18n.t('wallet.identity.detail.notAvailable');
  }

  function profileReadOnlyReason(): string {
    const reason = details.profileEditabilityReason ?? 'unsupported_control';
    return i18n.t(`wallet.identity.profile.readOnly.${reason}`);
  }

  async function copyValue(value: string | null | undefined, key: string): Promise<void> {
    if (!value?.trim()) return;
    if (await writeClipboardText(value)) {
      copiedKeyState.set(key, 1500);
      toast.success(i18n.t('wallet.identity.detail.copySuccess'));
    } else {
      copiedKeyState.clear();
      toast.error(i18n.t('wallet.identity.detail.copyFailed'));
    }
  }
</script>

{#if editingProfile}
  <IdentityProfileEditor
    {details}
    {profile}
    onCancel={() => (editingProfile = false)}
    onSubmitted={onProfileSubmitted}
  />
{:else}
  <div class="mx-auto flex h-full w-full max-w-5xl min-w-0 flex-col px-6 pt-3 pb-6">
    <div class="flex items-center justify-between gap-3">
      <button
        type="button"
        class="inline-flex items-center gap-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
        onclick={onBack}
      >
        <ArrowLeftIcon class="size-4" />
        {i18n.t('wallet.identity.detail.back')}
      </button>

      <Button
        variant="ghost"
        size="sm"
        onclick={onUnlink}
        disabled={unlinking}
        class="h-8 text-destructive hover:text-destructive"
      >
        <Link2OffIcon class="size-4" />
        {unlinking
          ? i18n.t('wallet.identity.detail.unlinking')
          : i18n.t('wallet.identity.detail.unlink')}
      </Button>
    </div>

    <ScrollArea.Root class="mt-3 min-h-0 flex-1" type="scroll">
      <ScrollArea.Viewport class="h-full pr-2">
        {#if pendingProfile}
          <section
            class="mx-auto mb-4 w-full max-w-3xl rounded-xl bg-amber-500/10 px-4 py-3 text-amber-900 dark:text-amber-100"
          >
            <div class="flex items-start gap-3">
              <Clock3Icon class="mt-0.5 size-4 shrink-0" />
              <div class="min-w-0 flex-1">
                <p class="text-sm font-semibold">
                  {i18n.t(
                    pendingRemovesEntireProfile
                      ? 'wallet.identity.profile.pending.removalTitle'
                      : 'wallet.identity.profile.pending.title'
                  )}
                </p>
                <p class="mt-1 text-xs leading-relaxed text-amber-800 dark:text-amber-200">
                  {i18n.t(
                    pendingRemovesEntireProfile
                      ? 'wallet.identity.profile.pending.removalDescription'
                      : 'wallet.identity.profile.pending.description'
                  )}
                </p>
                <button
                  type="button"
                  class="mt-2 text-xs font-semibold underline-offset-4 hover:underline"
                  onclick={() => (showSubmittedChanges = !showSubmittedChanges)}
                >
                  {showSubmittedChanges
                    ? i18n.t('wallet.identity.profile.pending.hideChanges')
                    : i18n.t('wallet.identity.profile.pending.viewChanges')}
                </button>
                {#if showSubmittedChanges}
                  <div
                    class="mt-3 space-y-2 rounded-lg bg-background/65 p-3 text-xs text-foreground dark:bg-background/35"
                  >
                    {#if pendingAvatarChanged}
                      <div class="flex items-center justify-between gap-4">
                        <span class="text-muted-foreground">
                          {i18n.t('wallet.identity.profile.review.avatar')}
                        </span>
                        <span class="font-medium">
                          {pendingProfile.proposedProfile.avatarBase64
                            ? i18n.t('wallet.identity.profile.review.replaced')
                            : i18n.t('wallet.identity.profile.review.removed')}
                        </span>
                      </div>
                    {/if}
                    {#if pendingDescriptionChanged}
                      <div class="flex items-start justify-between gap-4">
                        <span class="text-muted-foreground">
                          {i18n.t('wallet.identity.profile.review.descriptionLabel')}
                        </span>
                        <span class="max-w-md text-right whitespace-pre-line">
                          {pendingProfile.proposedProfile.description ||
                            i18n.t('wallet.identity.profile.review.removed')}
                        </span>
                      </div>
                    {/if}
                    <div class="flex items-center justify-between gap-4">
                      <span class="text-muted-foreground">
                        {i18n.t('wallet.identity.profile.pending.status')}
                      </span>
                      <span class="font-medium">
                        {i18n.t('wallet.identity.profile.pending.notConfirmed')}
                      </span>
                    </div>
                    <div class="flex min-w-0 items-center justify-between gap-4">
                      <span class="shrink-0 text-muted-foreground">
                        {i18n.t('wallet.identity.profile.pending.transaction')}
                      </span>
                      <div class="flex min-w-0 items-center gap-1">
                        <span class="truncate font-mono">{pendingProfile.txid}</span>
                        <CopyButton
                          copied={copiedKey === 'pendingTxid'}
                          onclick={() => copyValue(pendingProfile?.txid, 'pendingTxid')}
                          aria-label={i18n.t('wallet.identity.detail.copy')}
                        />
                      </div>
                    </div>
                  </div>
                {/if}
              </div>
            </div>
          </section>
        {/if}

        <section
          class="mx-auto flex w-full max-w-3xl items-center gap-5 rounded-2xl bg-muted/45 p-5 dark:bg-muted/30"
        >
          {#if profileLoading}
            <div class="size-24 shrink-0 animate-pulse rounded-full bg-muted/60"></div>
          {:else}
            <IdentityAvatar
              seed={details.identityAddress}
              label={displayName}
              imageUrl={avatarUrl}
              class="size-24 text-xl"
            />
          {/if}

          <div class="min-w-0 flex-1 text-left">
            {#if displayNameIsAddress}
              <IdentifierText
                value={details.identityAddress}
                mode="compact"
                class="block text-2xl font-semibold text-foreground"
              />
            {:else}
              <h1 class="text-[28px] leading-tight font-semibold text-foreground">
                {displayName}
              </h1>
            {/if}

            {#if profileLoading}
              <div class="mt-2 h-4 w-64 max-w-full animate-pulse rounded bg-muted/60"></div>
            {:else}
              <p
                class="mt-1.5 max-w-xl text-sm leading-relaxed whitespace-pre-line text-muted-foreground"
              >
                {description || i18n.t('wallet.identity.profile.noProfile')}
              </p>
            {/if}

            <div class="mt-3">
              <Button
                onclick={() => (editingProfile = true)}
                disabled={!details.profileEditable ||
                  profileLoading ||
                  profile?.state === 'unavailable' ||
                  Boolean(pendingProfile)}
              >
                {i18n.t('wallet.identity.profile.edit')}
              </Button>
              {#if !details.profileEditable}
                <p class="mt-2 max-w-lg text-xs text-muted-foreground">
                  {profileReadOnlyReason()}
                </p>
              {:else if profile?.state === 'unavailable'}
                <p class="mt-2 max-w-lg text-xs text-amber-700 dark:text-amber-300">
                  {i18n.t('wallet.identity.profile.unavailable')}
                </p>
              {/if}
            </div>
          </div>
        </section>

        <div class="mx-auto mt-3 w-full max-w-3xl space-y-3 pb-3">
          <details class="group rounded-xl bg-muted/45 dark:bg-muted/30">
            <summary
              class="flex list-none items-center justify-between gap-3 px-4 py-3 text-sm font-semibold text-foreground [&::-webkit-details-marker]:hidden"
            >
              {i18n.t('wallet.identity.detail.sections.identityDetails')}
              <ChevronDownIcon
                class="size-4 text-muted-foreground transition-transform group-open:rotate-180"
              />
            </summary>
            <div class="grid gap-2 px-3 pb-3 text-sm md:grid-cols-2">
              <div class="rounded-lg bg-background/75 px-3 py-2.5 dark:bg-background/35">
                <p class="text-xs text-muted-foreground">
                  {i18n.t('wallet.identity.detail.fields.iAddress')}
                </p>
                <div class="mt-1 flex min-w-0 items-center gap-1">
                  <IdentifierText
                    value={details.identityAddress}
                    mode="compact"
                    class="min-w-0 flex-1 text-foreground"
                  />
                  <CopyButton
                    copied={copiedKey === 'iAddress'}
                    onclick={() => copyValue(details.identityAddress, 'iAddress')}
                    aria-label={i18n.t('wallet.identity.detail.copy')}
                  />
                </div>
              </div>
              <div class="rounded-lg bg-background/75 px-3 py-2.5 dark:bg-background/35">
                <p class="text-xs text-muted-foreground">
                  {i18n.t('wallet.identity.detail.fields.status')}
                </p>
                <p class="mt-1 text-foreground">{showValue(details.status)}</p>
              </div>
              <div class="rounded-lg bg-background/75 px-3 py-2.5 dark:bg-background/35">
                <p class="text-xs text-muted-foreground">
                  {i18n.t('wallet.identity.detail.fields.system')}
                </p>
                <p class="mt-1 break-words text-foreground">{showValue(blockchainName)}</p>
              </div>
            </div>
          </details>

          <details class="group rounded-xl bg-muted/45 dark:bg-muted/30">
            <summary
              class="flex list-none items-center justify-between gap-3 px-4 py-3 text-sm text-foreground [&::-webkit-details-marker]:hidden"
            >
              <span class="flex min-w-0 items-center gap-2.5">
                {#if authorityWarningCount > 0}
                  <AlertTriangleIcon class="size-4 shrink-0 text-amber-700 dark:text-amber-300" />
                {/if}
                <span class="font-semibold">
                  {i18n.t('wallet.identity.detail.sections.authoritiesAndSafety')}
                </span>
                <span
                  class={authorityWarningCount > 0
                    ? 'truncate text-xs text-amber-700 dark:text-amber-300'
                    : 'truncate text-xs text-muted-foreground'}
                >
                  {authorityWarningSummary}
                </span>
              </span>
              <ChevronDownIcon
                class="size-4 shrink-0 text-muted-foreground transition-transform group-open:rotate-180"
              />
            </summary>
            <div class="space-y-2 px-3 pb-3 text-sm">
              <div class="rounded-lg bg-background/75 px-3 py-2.5 dark:bg-background/35">
                <p class="text-xs text-muted-foreground">
                  {i18n.t('wallet.identity.detail.sections.primaryAddresses')}
                </p>
                {#if details.primaryAddresses.length > 0}
                  {#each details.primaryAddresses as address, index (address)}
                    <div class="mt-1 flex min-w-0 items-center gap-1">
                      <IdentifierText
                        value={address}
                        mode="compact"
                        class="min-w-0 flex-1 text-foreground"
                      />
                      <CopyButton
                        copied={copiedKey === `primary-${index}`}
                        onclick={() => copyValue(address, `primary-${index}`)}
                        aria-label={i18n.t('wallet.identity.detail.copy')}
                      />
                    </div>
                  {/each}
                {:else}
                  <p class="mt-1 text-muted-foreground">
                    {i18n.t('wallet.identity.detail.noPrimaryAddresses')}
                  </p>
                {/if}
              </div>

              <div class="grid gap-2 md:grid-cols-2">
                <div class="rounded-lg bg-background/75 px-3 py-2.5 dark:bg-background/35">
                  <p class="text-xs text-muted-foreground">
                    {i18n.t('wallet.identity.detail.fields.revocationAuthority')}
                  </p>
                  <p class="mt-1 break-words text-foreground">
                    {showValue(revocationAuthorityName)}
                  </p>
                </div>
                <div class="rounded-lg bg-background/75 px-3 py-2.5 dark:bg-background/35">
                  <p class="text-xs text-muted-foreground">
                    {i18n.t('wallet.identity.detail.fields.recoveryAuthority')}
                  </p>
                  <p class="mt-1 break-words text-foreground">
                    {showValue(recoveryAuthorityName)}
                  </p>
                </div>
              </div>

              {#if authorityWarningCount > 0}
                {#if spendAndSignWarning}
                  <div
                    class="flex items-start gap-3 rounded-xl bg-amber-500/10 p-3 text-amber-900 dark:text-amber-100"
                  >
                    <AlertTriangleIcon class="mt-0.5 size-4 shrink-0" />
                    <div class="min-w-0 text-xs">
                      <p class="font-semibold">
                        {i18n.t('wallet.identity.detail.warningCards.spendAndSign.title')}
                      </p>
                      {#if details.primaryAddresses.length > 0}
                        <p class="mt-1 text-amber-800 dark:text-amber-200">
                          {i18n.t('wallet.identity.detail.warningCards.spendAndSign.namedWarning')}
                        </p>
                        <div class="mt-1.5 flex flex-wrap gap-x-3 gap-y-1">
                          {#each details.primaryAddresses as address (address)}
                            <IdentifierText
                              value={address}
                              mode="compact"
                              class="text-amber-900 dark:text-amber-100"
                            />
                          {/each}
                        </div>
                      {:else}
                        <p class="mt-1 text-amber-800 dark:text-amber-200">
                          {i18n.t('wallet.identity.detail.warningCards.spendAndSign.warning')}
                        </p>
                      {/if}
                    </div>
                  </div>
                {/if}

                {#if revokeAuthorityExternal}
                  <div
                    class="flex items-start gap-3 rounded-xl bg-amber-500/10 p-3 text-amber-900 dark:text-amber-100"
                  >
                    <AlertTriangleIcon class="mt-0.5 size-4 shrink-0" />
                    <div class="min-w-0 text-xs">
                      <p class="font-semibold">
                        {i18n.t('wallet.identity.detail.warningCards.revoke.title')}
                      </p>
                      <p class="mt-1 break-words text-amber-800 dark:text-amber-200">
                        {revocationAuthorityName
                          ? i18n.t('wallet.identity.detail.warningCards.revoke.namedWarning', {
                              value: revocationAuthorityName,
                            })
                          : i18n.t('wallet.identity.detail.warningCards.revoke.warning')}
                      </p>
                    </div>
                  </div>
                {/if}

                {#if recoveryAuthorityExternal}
                  <div
                    class="flex items-start gap-3 rounded-xl bg-amber-500/10 p-3 text-amber-900 dark:text-amber-100"
                  >
                    <AlertTriangleIcon class="mt-0.5 size-4 shrink-0" />
                    <div class="min-w-0 text-xs">
                      <p class="font-semibold">
                        {i18n.t('wallet.identity.detail.warningCards.recover.title')}
                      </p>
                      <p class="mt-1 break-words text-amber-800 dark:text-amber-200">
                        {recoveryAuthorityName
                          ? i18n.t('wallet.identity.detail.warningCards.recover.namedWarning', {
                              value: recoveryAuthorityName,
                            })
                          : i18n.t('wallet.identity.detail.warningCards.recover.warning')}
                      </p>
                    </div>
                  </div>
                {/if}
              {/if}
            </div>
          </details>
        </div>
      </ScrollArea.Viewport>
      <ScrollArea.Scrollbar orientation="vertical" />
    </ScrollArea.Root>
  </div>
{/if}
