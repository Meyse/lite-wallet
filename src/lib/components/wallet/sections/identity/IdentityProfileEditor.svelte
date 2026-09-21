<script lang="ts">
  import { onDestroy, onMount, tick, untrack } from 'svelte';
  import ImageIcon from '@lucide/svelte/icons/image';
  import MoreHorizontalIcon from '@lucide/svelte/icons/ellipsis';
  import NavigationBackButton from '$lib/components/common/NavigationBackButton.svelte';
  import InlineTextActionButton from '$lib/components/common/InlineTextActionButton.svelte';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { CopyButton } from '$lib/components/ui/copy-button';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { Spinner } from '$lib/components/ui/spinner';
  import { Textarea } from '$lib/components/ui/textarea';
  import { contactSession } from '$lib/contacts/session';
  import { i18nStore } from '$lib/i18n';
  import {
    clearProfileDraft,
    effectiveProfileMime,
    effectiveProfileValue,
    loadProfileDraft,
    PROFILE_FIELDS,
    type ProfileDraft,
    type ProfileField,
    publishedProfileValue,
    removeProfileValue,
    saveProfileDraft,
    stageProfileValue,
    undoProfileValue,
  } from '$lib/identity/profileDrafts';
  import {
    loadProfileImage,
    profileFeeDisplay,
    type ProfileImageCandidates,
    type ProfileImageKind,
    profileMediaUrl,
  } from '$lib/identity/profileImages';
  import type { ProfilePublicationController } from '$lib/identity/profilePublication';
  import * as identityLinkService from '$lib/services/identityLinkService';
  import { sendIdentityUpdate } from '$lib/services/identityService';
  import * as walletService from '$lib/services/walletService';
  import { settingsStore } from '$lib/stores/settings';
  import { ratesStore } from '$lib/stores/rates';
  import type {
    IdentityDetails,
    IdentityProfileLoadResult,
    IdentityProfilePreflightResult,
    IdentityProfileSnapshot,
  } from '$lib/types/wallet';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import { extractWalletErrorType } from '$lib/utils/walletErrors';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay';
  import { getRateForCurrency } from '$lib/utils/fiatDisplay';
  import ProfileDraftPreview from './ProfileDraftPreview.svelte';
  import ProfilePublicationCosts from './ProfilePublicationCosts.svelte';
  import ProfileImageEditor from './ProfileImageEditor.svelte';

  let {
    details,
    profile,
    publication,
    onCancel = () => {},
  }: {
    details: IdentityDetails;
    profile: IdentityProfileLoadResult | null;
    publication: ProfilePublicationController;
    onCancel?: () => void;
  } = $props();
  const i18n = $derived($i18nStore);
  const session = untrack(() => $contactSession);
  const identityAddress = untrack(() => details.identityAddress);
  const displayName = $derived(formatIdentityDisplayName(details));
  const initial = untrack(() => $publication.plan);
  const saved = $derived($publication.plan);
  let draft = $state<ProfileDraft>(
    initial
      ? { ...initial.request, header: initial.request.header ?? { action: 'keep' } }
      : loadProfileDraft(session, identityAddress)
  );
  let description = $state(
    untrack(
      () => draft.descriptionText ?? effectiveProfileValue(profile, draft, 'description') ?? ''
    )
  );
  let step = $state<'overview' | 'crop' | 'review' | 'progress' | 'compare' | 'discard'>(
    initial ? 'progress' : 'overview'
  );
  let preflight = $state<IdentityProfilePreflightResult | null>(null);
  // Keep the comparison visible after invalidating its approval. Merely comparing never sends.
  let comparison = $state<IdentityProfilePreflightResult | null>(null);
  let preparing = $state(false);
  let errorMessage = $state('');
  let input = $state<HTMLInputElement | null>(null);
  let crop = $state.raw<ImageBitmap | null>(null);
  let imageKind = $state<ProfileImageKind>('avatar');
  let initiatingControl: HTMLElement | null = null;
  let root = $state<HTMLDivElement>();
  let heading = $state<HTMLHeadingElement>();
  let alive = true;
  const copied = new TimedValueState<string>();
  const count = $derived(PROFILE_FIELDS.filter((field) => draft[field].action !== 'keep').length);
  const normalizedDescription = $derived(description.replace(/\r\n?/g, '\n').trim());
  const descriptionCount = $derived(
    Array.from(
      new Intl.Segmenter(undefined, { granularity: 'grapheme' }).segment(normalizedDescription)
    ).length
  );
  const descriptionInvalid = $derived(
    descriptionCount > 160 || new TextEncoder().encode(normalizedDescription).length > 1024
  );
  const editable = $derived(
    details.profileEditable &&
      session?.network === 'testnet' &&
      $contactSession === session &&
      profile !== null &&
      profile.state !== 'unavailable' &&
      !profile.issues.length
  );
  const pending = $derived(
    saved?.status === 'waiting' ||
      saved?.status === 'complete' ||
      $publication.uncertain ||
      Boolean($publication.receipt)
  );
  const feeRate = $derived(
    getRateForCurrency($ratesStore.VRSCTEST?.rates, $settingsStore.displayCurrency)
  );
  const reviewProfile = $derived(
    preflight?.publication.proposedProfile ?? preflight?.proposedProfile
  );
  const split = $derived(preflight?.publication.totalSteps === 2);
  const second = $derived(preflight?.publication.step === 2);
  const alternativeReview = $derived(comparison ?? preflight);
  const optimization = $derived.by(() => {
    const plan = alternativeReview?.publication;
    const option = plan?.optimization;
    return plan?.step === 1 &&
      plan.totalSteps === 2 &&
      option?.totalSteps === 1 &&
      BigInt(option.estimatedTotalFeeSats) < BigInt(plan.estimatedTotalFeeSats)
      ? option
      : null;
  });
  const progressRequest = $derived(saved?.request ?? draft);
  const firstFields = $derived(
    PROFILE_FIELDS.filter((field) => {
      const receipt = saved?.firstReceipt;
      return (
        receipt &&
        snapshotValue(receipt.previousProfile, field) !==
          snapshotValue(receipt.proposedProfile, field)
      );
    })
  );
  const progressFields = $derived(
    PROFILE_FIELDS.filter(
      (field) =>
        (progressRequest[field]?.action ?? 'keep') !== 'keep' || firstFields.includes(field)
    )
  );
  const title = $derived(
    i18n.t(
      `wallet.identity.profile.${$publication.submitting ? 'ux.publishing' : step === 'overview' ? 'editor.title' : step === 'crop' ? 'ux.crop' : step === 'review' ? 'draft.reviewTitle' : step === 'compare' ? 'sequence.previewSmaller' : step === 'discard' ? 'ux.discardTitle' : pending ? 'sequence.waiting' : 'sequence.continue'}`
    )
  );

  function active() {
    return (
      alive &&
      session !== null &&
      $contactSession === session &&
      details.identityAddress === identityAddress
    );
  }
  function label(field: ProfileField) {
    return i18n.t(`wallet.identity.profile.draft.${field}`);
  }
  function snapshotValue(
    snapshot: IdentityProfileSnapshot | null | undefined,
    field: ProfileField
  ): string | null {
    return field === 'description'
      ? (snapshot?.description ?? null)
      : (snapshot?.[`${field}Base64`] ?? null);
  }
  function persist() {
    saveProfileDraft(session, identityAddress, $state.snapshot(draft));
  }
  function invalidate() {
    preflight = null;
    comparison = null;
    errorMessage = '';
  }
  function typeDescription(event: Event) {
    description = (event.currentTarget as HTMLTextAreaElement).value;
    draft = {
      ...stageProfileValue(
        profile,
        $state.snapshot(draft),
        'description',
        normalizedDescription || null
      ),
      descriptionText: description,
    };
    invalidate();
    persist();
  }
  function remove(field: ProfileField) {
    draft = removeProfileValue(profile, $state.snapshot(draft), field);
    invalidate();
    persist();
  }
  function undo(field: ProfileField) {
    draft = undoProfileValue($state.snapshot(draft), field);
    if (field === 'description') description = publishedProfileValue(profile, field) ?? '';
    invalidate();
    persist();
  }
  function chooseImage(field: ProfileImageKind, event: MouseEvent) {
    if (!editable || preparing) return;
    imageKind = field;
    initiatingControl = event.currentTarget as HTMLElement;
    input?.click();
  }
  async function restoreImageFocus() {
    await tick();
    root?.querySelector<HTMLElement>(`[data-image-control="${imageKind}"]`)?.focus();
  }
  async function selectedImage(event: Event) {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    if (input) input.value = '';
    if (!file || preparing) {
      initiatingControl?.focus();
      return;
    }
    preparing = true;
    errorMessage = '';
    try {
      const image = await loadProfileImage(file);
      if (!active()) {
        image.close();
        return;
      }
      crop = image;
      step = 'crop';
    } catch (error) {
      if (active()) {
        const code = error instanceof Error ? error.message : '';
        errorMessage = i18n.t(
          `wallet.identity.profile.ux.${code === 'source_bytes' ? 'sourceBytes' : code === 'source_pixels' ? 'sourcePixels' : code === 'source_type' ? 'sourceType' : 'sourceDecode'}`
        );
        void restoreImageFocus();
      }
    } finally {
      if (active()) preparing = false;
    }
  }
  function closeCrop() {
    crop?.close();
    crop = null;
    step = 'overview';
    void restoreImageFocus();
  }
  function stageImage(candidates: ProfileImageCandidates) {
    if (!active()) return;
    draft = {
      ...stageProfileValue(profile, $state.snapshot(draft), imageKind, candidates.balanced.base64),
      [imageKind === 'avatar' ? 'smallerAvatar' : 'smallerHeader']: candidates.smaller
        ? { action: 'set', value: candidates.smaller.base64, mimeType: candidates.smaller.mimeType }
        : null,
    };
    invalidate();
    persist();
    closeCrop();
  }
  function mapError(error: unknown) {
    const type = extractWalletErrorType(error);
    return i18n.t(
      type === 'IdentityProfilePublicationPending'
        ? 'wallet.identity.profile.sequence.waiting'
        : type === 'IdentityProfilePublicationExists'
          ? 'wallet.identity.profile.sequence.exists'
          : type === 'InsufficientFunds'
            ? 'wallet.identity.profile.error.insufficientFunds'
            : type === 'IdentityProfileInvalidDescription'
              ? 'wallet.identity.profile.error.descriptionInvalid'
              : type === 'IdentityProfileInvalidAvatar' || type === 'IdentityProfileInvalidHeader'
                ? 'wallet.identity.profile.draft.imageInvalid'
                : type === 'WalletLocked'
                  ? 'wallet.identity.error.walletLocked'
                  : type === 'NetworkError'
                    ? 'wallet.identity.error.network'
                    : 'wallet.identity.profile.error.generic'
    );
  }
  function adoptReview(result: IdentityProfilePreflightResult, changes: ProfileDraft = draft) {
    preflight = result;
    comparison = null;
    publication.reviewed(result, {
      coinId: 'VRSCTEST',
      channelId: '',
      identityAddress,
      ...$state.snapshot(changes),
    });
    step = 'review';
  }
  async function review() {
    if (!editable || !active() || !count || descriptionInvalid || preparing || step !== 'overview')
      return;
    preparing = true;
    errorMessage = '';
    const changes = $state.snapshot(draft);
    try {
      const addresses = await walletService.getAddresses();
      if (!active()) return;
      const result = await identityLinkService.preflightIdentityProfileUpdate({
        coinId: 'VRSCTEST',
        channelId: `vrpc.${addresses.vrsc_address}.iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq`,
        identityAddress,
        ...changes,
      });
      if (active()) adoptReview(result, changes);
    } catch (error) {
      if (active()) errorMessage = mapError(error);
    } finally {
      if (active()) preparing = false;
    }
  }
  async function resume(smaller = false) {
    if (!active() || !saved || preparing || pending) return;
    const option = optimization;
    if (smaller && !option) return;
    preparing = true;
    errorMessage = '';
    preflight = null;
    try {
      const result = await identityLinkService.reviewIdentityProfilePublication(
        identityAddress,
        saved.planId,
        smaller ? option!.field : undefined
      );
      if (!active()) return;
      // Adopt only after success; failed replanning retains both original pixels and intent.
      if (smaller) {
        draft = {
          ...draft,
          [option!.field]: option!.image,
          [option!.field === 'avatar' ? 'smallerAvatar' : 'smallerHeader']: null,
        };
        persist();
      }
      adoptReview(result, {
        ...saved.request,
        header: saved.request.header ?? { action: 'keep' },
        ...(smaller ? { [option!.field]: option!.image } : {}),
      });
    } catch (error) {
      if (active()) {
        errorMessage = mapError(error);
        void publication.refresh();
      }
    } finally {
      if (active()) preparing = false;
    }
  }
  async function publish() {
    if (!active() || !editable || !preflight || step !== 'review' || preparing) return;
    const prepared = preflight;
    preflight = null;
    if (Date.now() >= prepared.expiresAt * 1000) {
      step = 'progress';
      errorMessage = i18n.t('wallet.identity.profile.draft.expired');
      return;
    }
    if (!publication.beginSubmission()) {
      step = 'progress';
      return;
    }
    step = 'progress';
    errorMessage = '';
    try {
      const result = await sendIdentityUpdate({ preflightId: prepared.preflightId });
      if (!active()) return;
      publication.submitted(
        result.profileUpdate ?? {
          identityAddress,
          txid: result.txid,
          submittedAt: Math.floor(Date.now() / 1000),
          previousProfile: prepared.currentProfile,
          proposedProfile: prepared.proposedProfile,
        }
      );
      clearProfileDraft(session, identityAddress);
    } catch (error) {
      if (active()) {
        publication.submissionFailed();
        errorMessage = mapError(error);
      }
    }
  }
  function compare() {
    comparison = preflight;
    preflight = null;
    step = 'compare';
  }
  function back() {
    if (step === 'review') {
      const resumed = second;
      invalidate();
      step = resumed ? 'progress' : 'overview';
    } else if (step === 'compare') void resume();
    else if (step === 'discard') {
      errorMessage = '';
      step = 'progress';
    } else onCancel();
  }
  async function discard() {
    if (!active() || !saved || preparing || pending || $publication.submitting) return;
    preparing = true;
    errorMessage = '';
    try {
      await identityLinkService.discardIdentityProfilePublication(saved.planId);
      if (active()) {
        clearProfileDraft(session, identityAddress);
        publication.discarded();
        onCancel();
      }
    } catch (error) {
      if (active()) errorMessage = mapError(error);
    } finally {
      if (active()) preparing = false;
    }
  }
  function progressStatus(field: ProfileField) {
    if (saved?.step === 1 && saved.totalSteps === 2 && field === 'header') return 'saved';
    return $publication.submitting && !(saved?.step === 2 && firstFields.includes(field))
      ? 'publishing'
      : saved?.step === 2 && firstFields.includes(field)
        ? 'published'
        : pending
          ? 'waiting'
          : 'ready';
  }
  const publishLabel = $derived(
    i18n.t(
      second
        ? 'wallet.identity.profile.sequence.publishHeader'
        : split
          ? preflight?.changedFields.includes('description')
            ? 'wallet.identity.profile.ux.publishAvatarDescription'
            : 'wallet.identity.profile.ux.publishAvatar'
          : 'wallet.identity.profile.draft.publish'
    )
  );
  let previousStep = untrack(() => step);
  $effect(() => {
    const next = step;
    if (next !== previousStep) {
      const from = previousStep;
      previousStep = next;
      if (!(next === 'overview' && from === 'crop'))
        void tick().then(() => {
          if (active()) heading?.focus();
        });
    }
  });
  $effect(() => {
    if (step === 'review' && saved && saved.status !== 'ready') {
      preflight = null;
      step = 'progress';
    }
  });
  onMount(() => heading?.focus());
  onDestroy(() => {
    alive = false;
    crop?.close();
  });
</script>

{#snippet imageControls(field: ProfileImageKind)}
  {@const value = effectiveProfileValue(profile, draft, field)}
  <div class="flex items-center gap-1">
    <Button
      variant="secondary"
      size="sm"
      data-image-control={field}
      disabled={!editable || preparing}
      aria-label={i18n.t(
        `wallet.identity.profile.ux.${value ? 'change' : 'add'}${field === 'avatar' ? 'Avatar' : 'Header'}`
      )}
      onclick={(event) => chooseImage(field, event)}
      ><ImageIcon class="size-4" />{i18n.t(
        `wallet.identity.profile.ux.${value ? 'change' : 'add'}${field === 'avatar' ? 'Avatar' : 'Header'}`
      )}</Button
    >
    {#if value || draft[field].action !== 'keep'}
      <DropdownMenu.Root>
        <DropdownMenu.Trigger
          class="flex size-8 items-center justify-center rounded-md bg-secondary text-secondary-foreground outline-none hover:bg-secondary/80 focus-visible:ring-2 focus-visible:ring-ring"
          disabled={!editable || preparing}
          aria-label={i18n.t('wallet.identity.profile.ux.imageActions', { field: label(field) })}
          ><MoreHorizontalIcon class="size-4" /></DropdownMenu.Trigger
        >
        <DropdownMenu.Content align="end">
          {#if value}<DropdownMenu.Item onclick={() => remove(field)}
              >{i18n.t('wallet.identity.profile.ux.removeImage')}</DropdownMenu.Item
            >{/if}
          {#if draft[field].action !== 'keep'}<DropdownMenu.Item onclick={() => undo(field)}
              >{i18n.t('wallet.identity.profile.ux.undo')}</DropdownMenu.Item
            >{/if}
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    {/if}
  </div>
{/snippet}
{#snippet descriptionEditor()}
  <Label for="identity-profile-description" class="sr-only">{label('description')}</Label>
  <Textarea
    id="identity-profile-description"
    bind:value={description}
    oninput={typeDescription}
    disabled={!editable || preparing}
    rows={2}
    class="min-h-[72px] resize-none text-sm leading-5"
    placeholder={i18n.t('wallet.identity.profile.editor.descriptionPlaceholder')}
    aria-invalid={descriptionInvalid}
    aria-describedby={descriptionInvalid ? 'profile-description-error' : undefined}
  />
  <div class="mt-2 flex min-h-5 items-center justify-between gap-3 text-xs text-muted-foreground">
    {#if draft.description.action !== 'keep'}<InlineTextActionButton
        class="text-xs"
        onclick={() => undo('description')}
        >{i18n.t('wallet.identity.profile.ux.undoDescription')}</InlineTextActionButton
      >{:else}<span></span>{/if}
    {#if descriptionCount >= 130}<span class:text-destructive={descriptionInvalid}
        >{descriptionCount} / 160</span
      >{/if}
  </div>
  {#if descriptionInvalid}<p
      id="profile-description-error"
      role="alert"
      class="mt-1 text-xs text-destructive"
    >
      {i18n.t('wallet.identity.profile.error.descriptionInvalid')}
    </p>{/if}
{/snippet}
<div
  bind:this={root}
  class="mx-auto flex h-full min-h-0 w-full max-w-[840px] flex-col gap-3 p-5"
  data-profile-editor
>
  <Input
    bind:ref={input}
    class="hidden"
    tabindex={-1}
    aria-hidden="true"
    type="file"
    accept="image/jpeg,image/png,image/webp"
    aria-label={i18n.t('wallet.identity.profile.editor.choose')}
    onchange={selectedImage}
    oncancel={() => initiatingControl?.focus()}
  />
  {#if step !== 'crop' && step !== 'compare' && step !== 'discard' && !$publication.submitting}
    <NavigationBackButton
      label={i18n.t(
        step === 'review'
          ? second
            ? 'wallet.identity.profile.ux.backToProgress'
            : 'wallet.identity.profile.draft.backToEditor'
          : 'wallet.identity.profile.editor.back'
      )}
      onclick={back}
      disabled={preparing}
    />
  {/if}
  <h1
    bind:this={heading}
    tabindex="-1"
    class="shrink-0 text-[22px] leading-7 font-semibold outline-none"
  >
    {title}
  </h1>
  <p class="sr-only" role="status" aria-live="polite">{title}</p>
  {#if step === 'crop' && crop}
    <p class="shrink-0 truncate text-sm text-muted-foreground">{displayName}</p>
    <ProfileImageEditor kind={imageKind} bitmap={crop} onStage={stageImage} onCancel={closeCrop} />
  {:else}
    <ScrollArea.Root class="min-h-0 flex-1" type="scroll">
      <ScrollArea.Viewport class="h-full">
        <div class="p-1">
          {#if step === 'overview'}
            <ProfileDraftPreview
              {identityAddress}
              {displayName}
              avatarUrl={profileMediaUrl(
                effectiveProfileValue(profile, draft, 'avatar'),
                effectiveProfileMime(profile, draft, 'avatar')
              )}
              headerUrl={profileMediaUrl(
                effectiveProfileValue(profile, draft, 'header'),
                effectiveProfileMime(profile, draft, 'header')
              )}
              body={descriptionEditor}
            >
              {#snippet headerControls()}{@render imageControls('header')}{/snippet}
              {#snippet avatarControls()}{@render imageControls('avatar')}{/snippet}
            </ProfileDraftPreview>
            {#if count}<details class="mt-3 text-xs text-muted-foreground">
                <summary class="w-fit rounded-sm focus-visible:ring-2 focus-visible:ring-ring"
                  >{i18n.t('wallet.identity.profile.ux.unpublished')}</summary
                >
                <p class="mt-1.5 max-w-md leading-5">
                  {i18n.t('wallet.identity.profile.ux.draftLifetime')}
                </p>
              </details>{/if}
          {:else if step === 'review' && preflight && reviewProfile}
            <ProfileDraftPreview
              {identityAddress}
              {displayName}
              avatarUrl={profileMediaUrl(reviewProfile.avatarBase64, reviewProfile.avatarMimeType)}
              headerUrl={profileMediaUrl(reviewProfile.headerBase64, reviewProfile.headerMimeType)}
              description={reviewProfile.description}
              compact
            />
            {#if !split || second}
              <p class="mt-3 text-xs text-muted-foreground" data-reviewed-fields>
                {preflight.changedFields
                  .map((field) => label(field as ProfileField))
                  .join(', ')}{#if second}<span class="ml-1"
                    >· {i18n.t('wallet.identity.profile.sequence.secondUpdate')}</span
                  >{/if}
              </p>
            {/if}
            {#each PROFILE_FIELDS.filter((field) => preflight!.publication.changedFields.includes(field) && !snapshotValue(reviewProfile, field)) as field}<p
                class="mt-1 text-xs"
                data-profile-removal
              >
                {i18n.t('wallet.identity.profile.ux.removal', { field: label(field) })}
              </p>{/each}
            {#if optimization}<div
                class="mt-2 flex items-center justify-between gap-3 rounded-lg bg-muted/60 px-3 py-2"
                data-one-update-option
              >
                <div>
                  <p class="text-[13px] font-medium">
                    {i18n.t('wallet.identity.profile.ux.oneUpdate')}
                  </p>
                  <p class="text-xs text-muted-foreground">
                    {i18n.t('wallet.identity.profile.ux.saving', {
                      fee: profileFeeDisplay(optimization.savingSats),
                    })}
                  </p>
                </div>
                <InlineTextActionButton class="shrink-0 text-[13px]" onclick={compare}
                  >{i18n.t(
                    'wallet.identity.profile.sequence.previewSmaller'
                  )}</InlineTextActionButton
                >
              </div>{/if}
            <ProfilePublicationCosts {preflight} {feeRate} />
          {:else if step === 'compare' && optimization && comparison}
            <p class="mb-4 truncate text-sm text-muted-foreground">{displayName}</p>
            <div class="grid grid-cols-2 gap-4">
              {#each [false, true] as smaller}<div>
                  <img
                    class="w-full rounded-lg bg-muted object-contain {optimization.field ===
                    'avatar'
                      ? 'aspect-square max-h-60'
                      : 'aspect-[6/1]'}"
                    src={profileMediaUrl(
                      smaller
                        ? optimization.image.value
                        : comparison.publication.proposedProfile[`${optimization.field}Base64`],
                      smaller
                        ? optimization.image.mimeType
                        : comparison.publication.proposedProfile[`${optimization.field}MimeType`]
                    ) ?? undefined}
                    alt={i18n.t(
                      smaller
                        ? 'wallet.identity.profile.sequence.smallerVersion'
                        : 'wallet.identity.profile.sequence.currentVersion'
                    )}
                  />
                  <p class="mt-3 text-sm">
                    {i18n.t(
                      smaller
                        ? 'wallet.identity.profile.sequence.smallerVersion'
                        : 'wallet.identity.profile.sequence.currentVersion'
                    )}
                  </p>
                </div>{/each}
            </div>
            <p class="mt-5 text-sm">
              {i18n.t('wallet.identity.profile.ux.oneEstimate', {
                fee: profileFeeDisplay(optimization.estimatedTotalFeeSats),
              })}
            </p>
            <p class="mt-1 text-sm text-muted-foreground">
              {i18n.t('wallet.identity.profile.ux.saving', {
                fee: profileFeeDisplay(optimization.savingSats),
              })}
            </p>
            <p class="mt-3 text-xs text-muted-foreground">
              {i18n.t('wallet.identity.profile.sequence.compareQuality')}
            </p>
          {:else if step === 'progress' || step === 'discard'}
            <p class="mb-4 text-lg font-semibold break-words">{displayName}</p>
            {#if saved?.totalSteps === 2}<p class="mb-3 text-xs text-muted-foreground">
                {i18n.t('wallet.identity.profile.ux.updateNumber', {
                  step: saved.step,
                  total: saved.totalSteps,
                })}
              </p>{/if}
            {#if step === 'discard'}<p class="mb-4 text-sm leading-6">
                {i18n.t('wallet.identity.profile.ux.discardExplanation', {
                  fields: PROFILE_FIELDS.filter(
                    (field) =>
                      saved?.request[field]?.action && saved.request[field]?.action !== 'keep'
                  )
                    .map(label)
                    .join(', '),
                })}
              </p>{/if}
            <div class="space-y-1" data-publication-progress>
              {#each progressFields as field}<div
                  class="flex min-h-11 items-center justify-between gap-4 border-b border-border/60 text-sm"
                >
                  <span
                    >{label(field)}{#if progressRequest[field]?.action === 'remove'}
                      · {i18n.t('wallet.identity.profile.review.removed')}{/if}</span
                  ><span class="text-right text-[13px] text-muted-foreground"
                    >{i18n.t(`wallet.identity.profile.ux.status.${progressStatus(field)}`)}</span
                  >
                </div>{/each}
            </div>
            {#if saved?.status === 'stale'}<p class="mt-4 text-sm text-muted-foreground">
                {i18n.t('wallet.identity.profile.sequence.stale')}
              </p>{/if}
            {#if $publication.uncertain}<p class="mt-4 text-sm text-muted-foreground">
                {i18n.t('wallet.identity.profile.ux.checkingSubmission')}
              </p>{/if}
            {#if $publication.error}<p role="alert" class="mt-3 text-sm text-destructive">
                {i18n.t('wallet.identity.profile.ux.confirmationUnavailable')}
              </p>{/if}
            {#if $publication.receipt || saved?.firstReceipt}<details
                class="mt-4 text-xs text-muted-foreground"
              >
                <summary class="w-fit rounded-sm focus-visible:ring-2 focus-visible:ring-ring"
                  >{i18n.t('wallet.identity.profile.ux.transactionDetails')}</summary
                >
                <div class="mt-3 space-y-3">
                  {#each [saved?.firstReceipt, $publication.receipt].filter(Boolean) as receipt}<div
                    >
                      <p class="mb-1">
                        {i18n.t(
                          receipt === saved?.firstReceipt
                            ? 'wallet.identity.profile.ux.previousTransaction'
                            : 'wallet.identity.profile.pending.transaction'
                        )}
                      </p>
                      <div class="flex items-center gap-2">
                        <span class="min-w-0 flex-1 truncate font-mono">{receipt!.txid}</span
                        ><CopyButton
                          copied={copied.current === receipt!.txid}
                          onclick={async () => {
                            if (await writeClipboardText(receipt!.txid)) copied.set(receipt!.txid);
                          }}
                          aria-label={i18n.t('wallet.identity.detail.copy')}
                        />
                      </div>
                    </div>{/each}
                </div>
              </details>{/if}
            {#if step === 'progress' && saved && !pending && !$publication.submitting}<details
                class="mt-4 text-xs text-muted-foreground"
              >
                <summary class="w-fit rounded-sm focus-visible:ring-2 focus-visible:ring-ring"
                  >{i18n.t('wallet.identity.profile.ux.planActions')}</summary
                ><InlineTextActionButton
                  class="mt-2 text-xs"
                  onclick={() => (step = 'discard')}
                  disabled={preparing}
                  >{i18n.t('wallet.identity.profile.sequence.discard')}</InlineTextActionButton
                >
              </details>{/if}
          {/if}
          {#if errorMessage}<p
              role="alert"
              class="mt-3 rounded-md bg-destructive/10 p-3 text-sm text-destructive"
            >
              {errorMessage}
            </p>{/if}
        </div>
      </ScrollArea.Viewport><ScrollArea.Scrollbar orientation="vertical" />
    </ScrollArea.Root>
    <div class="shrink-0 space-y-3 pt-1" data-profile-footer>
      {#if step === 'review'}<p class="text-xs leading-5 text-muted-foreground">
          {i18n.t('wallet.identity.profile.draft.publicDisclosure')}
        </p>{/if}
      <div class="flex min-h-9 items-center justify-end gap-2">
        {#if step === 'overview'}<Button
            onclick={review}
            disabled={!editable || !count || descriptionInvalid || preparing}
            >{#if preparing}<Spinner class="size-4" />{/if}{i18n.t(
              'wallet.identity.profile.draft.reviewTitle'
            )}</Button
          >
        {:else if step === 'review'}<Button
            onclick={publish}
            disabled={!editable ||
              preparing ||
              pending ||
              !preflight ||
              BigInt(preflight.publication.availableSats) < BigInt(preflight.feeSats)}
            >{publishLabel}</Button
          >
        {:else if step === 'compare'}<Button
            variant="secondary"
            onclick={() => resume()}
            disabled={preparing}>{i18n.t('wallet.identity.profile.sequence.keepCurrent')}</Button
          ><Button onclick={() => resume(true)} disabled={preparing}
            >{#if preparing}<Spinner class="size-4" />{/if}{i18n.t(
              'wallet.identity.profile.sequence.useSmaller'
            )}</Button
          >
        {:else if step === 'discard'}<Button variant="secondary" onclick={back} disabled={preparing}
            >{i18n.t('common.cancel')}</Button
          ><Button variant="destructive" onclick={discard} disabled={preparing || pending}
            >{i18n.t('wallet.identity.profile.sequence.confirmDiscard')}</Button
          >
        {:else if $publication.submitting}<Spinner class="size-4" /><span
            class="text-sm text-muted-foreground"
            >{i18n.t('wallet.identity.profile.publishing.keepOpen')}</span
          >
        {:else if pending}<Button variant="secondary" onclick={() => publication.refresh()}
            >{#if $publication.checking}<Spinner class="size-4" />{/if}{i18n.t(
              'wallet.identity.profile.sequence.refresh'
            )}</Button
          >
        {:else}<Button onclick={() => resume()} disabled={preparing || !editable}
            >{#if preparing}<Spinner class="size-4" />{/if}{i18n.t(
              saved?.step === 2
                ? 'wallet.identity.profile.ux.reviewHeader'
                : 'wallet.identity.profile.sequence.reviewNext'
            )}</Button
          >{/if}
      </div>
    </div>
  {/if}
</div>
