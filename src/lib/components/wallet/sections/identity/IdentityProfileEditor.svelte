<script lang="ts">
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import Clock3Icon from '@lucide/svelte/icons/clock-3';
  import ImagePlusIcon from '@lucide/svelte/icons/image-plus';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import { untrack } from 'svelte';
  import { toast } from 'svelte-sonner';
  import { Button } from '$lib/components/ui/button';
  import { CopyButton } from '$lib/components/ui/copy-button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Spinner } from '$lib/components/ui/spinner';
  import { Textarea } from '$lib/components/ui/textarea';
  import { i18nStore } from '$lib/i18n';
  import * as identityLinkService from '$lib/services/identityLinkService.js';
  import { sendIdentityUpdate } from '$lib/services/identityService.js';
  import * as walletService from '$lib/services/walletService.js';
  import type {
    IdentityDetails,
    IdentityProfileAvatarChange,
    IdentityProfileDescriptionChange,
    IdentityProfileLoadResult,
    IdentityProfilePreflightResult,
    PendingIdentityProfileUpdate,
  } from '$lib/types/wallet.js';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import { extractWalletErrorType } from '$lib/utils/walletErrors.js';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay';
  import {
    isCompleteProfileRemoval,
    profileUpdateRemovesData,
  } from '$lib/utils/identityProfileUpdate';
  import IdentityAvatar from './IdentityAvatar.svelte';

  const VRSCTEST_SYSTEM_ID = 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq';
  const MAX_SOURCE_BYTES = 10 * 1024 * 1024;
  const MAX_SOURCE_PIXELS = 20_000_000;
  const MAX_AVATAR_BYTES = 32 * 1024;
  const TARGET_AVATAR_BYTES = 16 * 1024;
  const DESCRIPTION_LIMIT = 160;
  const DESCRIPTION_BYTE_LIMIT = 1024;

  type EditorStep = 'edit' | 'review' | 'publishing' | 'submitted';
  type AvatarMode = 'keep' | 'set' | 'remove';

  const noop = (): void => {};
  const noopSubmitted = (_update: PendingIdentityProfileUpdate): void => {};

  let {
    details,
    profile,
    onCancel = noop,
    onSubmitted = noopSubmitted,
  }: {
    details: IdentityDetails;
    profile: IdentityProfileLoadResult | null;
    onCancel?: () => void;
    onSubmitted?: (update: PendingIdentityProfileUpdate) => void;
  } = $props();

  const i18n = $derived($i18nStore);
  const displayName = $derived(formatIdentityDisplayName(details));
  const currentAvatarBase64 = $derived(profile?.avatar?.value.base64 ?? null);
  const currentAvatarMime = $derived(profile?.avatar?.value.mimeType ?? 'image/jpeg');
  const currentDescription = $derived(profile?.description?.value ?? '');

  let step = $state<EditorStep>('edit');
  let avatarMode = $state<AvatarMode>('keep');
  let editedAvatarBase64 = $state<string | null>(null);
  let sourceFile = $state<File | null>(null);
  let cropAdjusting = $state(false);
  let cropX = $state(0.5);
  let cropY = $state(0.5);
  let cropZoom = $state(1);
  let description = $state(untrack(() => currentDescription));
  let imageBusy = $state(false);
  let preflighting = $state(false);
  let preflightingRemoval = $state(false);
  let errorMessage = $state('');
  let preflight = $state<IdentityProfilePreflightResult | null>(null);
  let submittedTxid = $state('');
  let fileInput = $state<HTMLInputElement | null>(null);

  const copiedState = new TimedValueState<boolean>();
  const copied = $derived(Boolean(copiedState.current));
  const avatarBase64 = $derived(
    avatarMode === 'remove' ? null : avatarMode === 'set' ? editedAvatarBase64 : currentAvatarBase64
  );
  const avatarMime = $derived(avatarMode === 'set' ? 'image/jpeg' : currentAvatarMime);
  const avatarUrl = $derived(avatarBase64 ? `data:${avatarMime};base64,${avatarBase64}` : null);
  const normalizedDescription = $derived(description.replace(/\r\n?/g, '\n').trim());
  const descriptionCount = $derived(graphemeCount(normalizedDescription));
  const descriptionBytes = $derived(new TextEncoder().encode(normalizedDescription).length);
  const descriptionByteLimitExceeded = $derived(descriptionBytes > DESCRIPTION_BYTE_LIMIT);
  const descriptionTooLong = $derived(
    descriptionCount > DESCRIPTION_LIMIT || descriptionByteLimitExceeded
  );
  const hasChanges = $derived(
    avatarMode !== 'keep' || normalizedDescription !== currentDescription.trim()
  );
  const hasConfirmedProfileData = $derived(
    profile?.state === 'ready' &&
      profile.issues.length === 0 &&
      Boolean(profile.avatar || profile.description)
  );
  const completeRemoval = $derived(
    Boolean(preflight) &&
      isCompleteProfileRemoval(preflight?.currentProfile ?? {}, preflight?.proposedProfile ?? {})
  );
  const removesProfileData = $derived(
    Boolean(preflight) &&
      profileUpdateRemovesData(preflight?.currentProfile ?? {}, preflight?.proposedProfile ?? {})
  );

  function dataUrlBase64(dataUrl: string): string {
    const separator = dataUrl.indexOf(',');
    return separator >= 0 ? dataUrl.slice(separator + 1) : '';
  }

  function graphemeCount(value: string): number {
    return Array.from(new Intl.Segmenter(undefined, { granularity: 'grapheme' }).segment(value))
      .length;
  }

  function blobBase64(blob: Blob): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(dataUrlBase64(String(reader.result ?? '')));
      reader.onerror = () => reject(new Error('invalid_source'));
      reader.readAsDataURL(blob);
    });
  }

  function canvasBlob(canvas: HTMLCanvasElement, quality: number): Promise<Blob | null> {
    return new Promise((resolve) => canvas.toBlob(resolve, 'image/jpeg', quality));
  }

  async function prepareAvatar(
    file: File,
    zoom = 1,
    horizontalPosition = 0.5,
    verticalPosition = 0.5
  ): Promise<string> {
    if (
      !['image/jpeg', 'image/png', 'image/webp'].includes(file.type) ||
      file.size > MAX_SOURCE_BYTES
    ) {
      throw new Error('invalid_source');
    }

    const bitmap = await createImageBitmap(file);
    try {
      if (bitmap.width * bitmap.height > MAX_SOURCE_PIXELS) throw new Error('invalid_source');

      const canvas = document.createElement('canvas');
      canvas.width = 256;
      canvas.height = 256;
      const context = canvas.getContext('2d');
      if (!context) throw new Error('invalid_source');

      const sourceSize = Math.min(bitmap.width, bitmap.height) / zoom;
      const sourceX = Math.floor((bitmap.width - sourceSize) * horizontalPosition);
      const sourceY = Math.floor((bitmap.height - sourceSize) * verticalPosition);
      context.drawImage(bitmap, sourceX, sourceY, sourceSize, sourceSize, 0, 0, 256, 256);
      const pixels = context.getImageData(0, 0, 256, 256);
      for (let offset = 0; offset < pixels.data.length; offset += 4) {
        const alpha = pixels.data[offset + 3] / 255;
        for (let channel = 0; channel < 3; channel += 1) {
          pixels.data[offset + channel] = Math.round(
            pixels.data[offset + channel] * alpha + 255 * (1 - alpha)
          );
        }
        pixels.data[offset + 3] = 255;
      }
      context.putImageData(pixels, 0, 0);

      let fallbackBlob: Blob | null = null;
      for (const quality of [0.86, 0.78, 0.7, 0.62, 0.55]) {
        const blob = await canvasBlob(canvas, quality);
        if (!blob) continue;
        if (blob.size <= MAX_AVATAR_BYTES) {
          fallbackBlob = blob;
          if (blob.size <= TARGET_AVATAR_BYTES) return blobBase64(blob);
        }
      }
      if (fallbackBlob) return blobBase64(fallbackBlob);
    } finally {
      bitmap.close();
    }

    throw new Error('too_large');
  }

  async function handleFileChange(event: Event): Promise<void> {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;

    imageBusy = true;
    errorMessage = '';
    try {
      sourceFile = file;
      cropX = 0.5;
      cropY = 0.5;
      cropZoom = 1;
      editedAvatarBase64 = await prepareAvatar(file, cropZoom, cropX, cropY);
      avatarMode = 'set';
      cropAdjusting = true;
    } catch (errorValue) {
      errorMessage = i18n.t(
        errorValue instanceof Error && errorValue.message === 'too_large'
          ? 'wallet.identity.profile.error.avatarTooLarge'
          : 'wallet.identity.profile.error.avatarInvalid'
      );
    } finally {
      imageBusy = false;
    }
  }

  async function updateCrop(): Promise<void> {
    if (!sourceFile || imageBusy) return;
    imageBusy = true;
    errorMessage = '';
    try {
      editedAvatarBase64 = await prepareAvatar(sourceFile, cropZoom, cropX, cropY);
      avatarMode = 'set';
    } catch {
      errorMessage = i18n.t('wallet.identity.profile.error.avatarTooLarge');
    } finally {
      imageBusy = false;
    }
  }

  function avatarChange(): IdentityProfileAvatarChange {
    if (avatarMode === 'remove') return { action: 'remove' };
    if (avatarMode === 'set' && editedAvatarBase64) {
      return { action: 'set', value: editedAvatarBase64 };
    }
    return { action: 'keep' };
  }

  function descriptionChange(): IdentityProfileDescriptionChange {
    if (normalizedDescription === currentDescription.trim()) return { action: 'keep' };
    if (!normalizedDescription) return { action: 'remove' };
    return { action: 'set', value: normalizedDescription };
  }

  function mapProfileError(errorValue: unknown): string {
    switch (extractWalletErrorType(errorValue)) {
      case 'IdentityProfileNoChanges':
        return i18n.t('wallet.identity.profile.error.noChanges');
      case 'IdentityProfileReadOnly':
      case 'IdentityProfileWriteUnsupported':
        return i18n.t('wallet.identity.profile.error.readOnly');
      case 'IdentityProfileInvalidAvatar':
        return i18n.t('wallet.identity.profile.error.avatarInvalid');
      case 'IdentityProfileInvalidDescription':
        return i18n.t('wallet.identity.profile.error.descriptionInvalid');
      case 'InsufficientFunds':
        return i18n.t('wallet.identity.profile.error.insufficientFunds');
      case 'WalletLocked':
        return i18n.t('wallet.identity.error.walletLocked');
      case 'NetworkError':
        return i18n.t('wallet.identity.error.network');
      default:
        return i18n.t('wallet.identity.profile.error.generic');
    }
  }

  async function prepareReview(
    avatar: IdentityProfileAvatarChange,
    nextDescription: IdentityProfileDescriptionChange,
    removal = false
  ): Promise<void> {
    if ((!removal && descriptionTooLong) || imageBusy || preflighting) return;
    errorMessage = '';
    preflighting = true;
    preflightingRemoval = removal;
    try {
      const addresses = await walletService.getAddresses();
      preflight = await identityLinkService.preflightIdentityProfileUpdate({
        coinId: 'VRSCTEST',
        channelId: `vrpc.${addresses.vrsc_address}.${VRSCTEST_SYSTEM_ID}`,
        identityAddress: details.identityAddress,
        avatar,
        description: nextDescription,
      });
      step = 'review';
    } catch (errorValue) {
      errorMessage = mapProfileError(errorValue);
    } finally {
      preflighting = false;
      preflightingRemoval = false;
    }
  }

  async function reviewPublication(): Promise<void> {
    if (!hasChanges) return;
    await prepareReview(avatarChange(), descriptionChange());
  }

  async function removeProfileData(): Promise<void> {
    if (!hasConfirmedProfileData || preflighting) return;

    // Remove both supported fields so the backend's fresh snapshot, rather
    // than a possibly stale editor snapshot, decides which removals are needed.
    const removeAvatar: IdentityProfileAvatarChange = { action: 'remove' };
    const removeDescription: IdentityProfileDescriptionChange = { action: 'remove' };

    await prepareReview(removeAvatar, removeDescription, true);
  }

  async function publishProfile(): Promise<void> {
    if (!preflight || step === 'publishing') return;

    step = 'publishing';
    errorMessage = '';
    try {
      const result = await sendIdentityUpdate({ preflightId: preflight.preflightId });
      submittedTxid = result.txid;
      const pendingUpdate: PendingIdentityProfileUpdate = result.profileUpdate ?? {
        identityAddress: details.identityAddress,
        txid: result.txid,
        submittedAt: Math.floor(Date.now() / 1000),
        previousProfile: preflight.currentProfile,
        proposedProfile: preflight.proposedProfile,
      };
      onSubmitted(pendingUpdate);
      step = 'submitted';
    } catch (errorValue) {
      errorMessage = mapProfileError(errorValue);
      step = 'review';
    }
  }

  async function copyTxid(): Promise<void> {
    if (!submittedTxid) return;
    if (await writeClipboardText(submittedTxid)) {
      copiedState.set(true, 1500);
      toast.success(i18n.t('wallet.identity.detail.copySuccess'));
    } else {
      toast.error(i18n.t('wallet.identity.detail.copyFailed'));
    }
  }
</script>

<div class="mx-auto flex h-full w-full max-w-3xl min-w-0 flex-col px-5 pt-5 pb-6">
  {#if step === 'edit'}
    <div class="flex items-center justify-between gap-3">
      <button
        type="button"
        class="inline-flex items-center gap-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
        onclick={onCancel}
      >
        <ArrowLeftIcon class="size-4" />
        {i18n.t('wallet.identity.profile.editor.back')}
      </button>
      <p class="text-xs text-muted-foreground">{displayName}</p>
    </div>

    <div class="mx-auto mt-5 w-full max-w-2xl min-w-0 flex-1">
      <div class="flex flex-wrap items-center justify-between gap-2">
        <h1 class="text-xl font-semibold text-foreground">
          {i18n.t('wallet.identity.profile.editor.title')}
        </h1>
        {#if hasConfirmedProfileData}
          <Button
            variant="ghost"
            size="sm"
            class="h-8 gap-1.5 text-muted-foreground hover:text-destructive"
            onclick={removeProfileData}
            disabled={preflighting}
          >
            {#if preflightingRemoval}<Spinner class="size-4" />{:else}<Trash2Icon
                class="size-4"
              />{/if}
            {preflightingRemoval
              ? i18n.t('wallet.identity.profile.editor.preparingRemoval')
              : i18n.t('wallet.identity.profile.editor.removeProfileData')}
          </Button>
        {/if}
      </div>

      <section class="mt-4 rounded-2xl bg-muted/45 p-4 dark:bg-muted/30">
        <p class="text-sm font-medium text-foreground">
          {i18n.t('wallet.identity.profile.editor.avatarLabel')}
        </p>
        <div class="mt-3 flex items-center gap-4">
          <IdentityAvatar
            seed={details.identityAddress}
            label={displayName}
            imageUrl={avatarUrl}
            class="size-24 text-xl"
          />
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap gap-2">
              <Input
                bind:ref={fileInput}
                type="file"
                accept="image/jpeg,image/png,image/webp"
                class="sr-only !h-px !w-px !border-0 !p-0"
                onchange={handleFileChange}
              />
              <Button variant="secondary" onclick={() => fileInput?.click()} disabled={imageBusy}>
                {#if imageBusy}<Spinner class="size-4" />{:else}<ImagePlusIcon
                    class="size-4"
                  />{/if}
                {avatarBase64
                  ? i18n.t('wallet.identity.profile.editor.replace')
                  : i18n.t('wallet.identity.profile.editor.choose')}
              </Button>
              {#if sourceFile && avatarMode === 'set'}
                <Button variant="link" onclick={() => (cropAdjusting = !cropAdjusting)}>
                  {i18n.t('wallet.identity.profile.editor.adjust')}
                </Button>
              {/if}
              {#if avatarBase64}
                <Button
                  variant="ghost"
                  class="text-destructive hover:text-destructive"
                  onclick={() => {
                    avatarMode = 'remove';
                    editedAvatarBase64 = null;
                    sourceFile = null;
                    cropAdjusting = false;
                  }}
                >
                  <Trash2Icon class="size-4" />
                  {i18n.t('wallet.identity.profile.editor.remove')}
                </Button>
              {/if}
            </div>
            <p class="mt-2 text-xs leading-relaxed text-muted-foreground">
              {i18n.t('wallet.identity.profile.editor.imageHelp')}
            </p>
          </div>
        </div>
      </section>

      {#if cropAdjusting && sourceFile}
        <div class="mt-4 grid gap-4 rounded-xl bg-muted/35 p-4 sm:grid-cols-3">
          <div>
            <Label for="identity-profile-crop-x" class="text-xs text-muted-foreground">
              {i18n.t('wallet.identity.profile.editor.cropHorizontal')}
            </Label>
            <Input
              id="identity-profile-crop-x"
              type="range"
              min="0"
              max="1"
              step="0.05"
              value={cropX}
              oninput={(event) => (cropX = Number(event.currentTarget.value))}
              onchange={updateCrop}
              disabled={imageBusy}
            />
          </div>
          <div>
            <Label for="identity-profile-crop-y" class="text-xs text-muted-foreground">
              {i18n.t('wallet.identity.profile.editor.cropVertical')}
            </Label>
            <Input
              id="identity-profile-crop-y"
              type="range"
              min="0"
              max="1"
              step="0.05"
              value={cropY}
              oninput={(event) => (cropY = Number(event.currentTarget.value))}
              onchange={updateCrop}
              disabled={imageBusy}
            />
          </div>
          <div>
            <Label for="identity-profile-crop-zoom" class="text-xs text-muted-foreground">
              {i18n.t('wallet.identity.profile.editor.cropZoom')}
            </Label>
            <Input
              id="identity-profile-crop-zoom"
              type="range"
              min="1"
              max="2"
              step="0.05"
              value={cropZoom}
              oninput={(event) => (cropZoom = Number(event.currentTarget.value))}
              onchange={updateCrop}
              disabled={imageBusy}
            />
          </div>
        </div>
      {/if}

      <div class="mt-5">
        <div class="mb-2 flex items-center justify-between gap-3">
          <Label for="identity-profile-description" class="text-sm font-medium text-foreground">
            {i18n.t('wallet.identity.profile.editor.descriptionLabel')}
          </Label>
          <span class:text-destructive={descriptionTooLong} class="text-xs text-muted-foreground">
            {descriptionCount}/{DESCRIPTION_LIMIT}
          </span>
        </div>
        <Textarea
          id="identity-profile-description"
          bind:value={description}
          class="min-h-[110px] resize-none"
          placeholder={i18n.t('wallet.identity.profile.editor.descriptionPlaceholder')}
        />
        {#if descriptionByteLimitExceeded}
          <p class="mt-2 text-xs text-destructive">
            {i18n.t('wallet.identity.profile.editor.descriptionSizeHelp')}
          </p>
        {/if}
      </div>

      <div
        class="mt-4 rounded-xl bg-primary/7 px-4 py-3 text-xs leading-relaxed text-muted-foreground dark:bg-primary/10"
      >
        {i18n.t('wallet.identity.profile.editor.publicDisclosure')}
      </div>

      {#if errorMessage}
        <p class="mt-4 rounded-md bg-destructive/12 px-3 py-2 text-sm text-destructive">
          {errorMessage}
        </p>
      {/if}

      <div class="mt-5 flex justify-end">
        <Button
          onclick={reviewPublication}
          disabled={!hasChanges || descriptionTooLong || imageBusy || preflighting}
        >
          {#if preflighting && !preflightingRemoval}<Spinner class="size-4" />{/if}
          {preflighting && !preflightingRemoval
            ? i18n.t('wallet.identity.profile.editor.preparingReview')
            : i18n.t('wallet.identity.profile.editor.review')}
        </Button>
      </div>
    </div>
  {:else if step === 'review' && preflight}
    <div class="flex items-center justify-between gap-3">
      <button
        type="button"
        class="inline-flex items-center gap-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
        onclick={() => {
          step = 'edit';
          preflight = null;
        }}
      >
        <ArrowLeftIcon class="size-4" />
        {i18n.t('wallet.identity.profile.review.back')}
      </button>
      <p class="text-xs text-muted-foreground">{displayName}</p>
    </div>

    <div class="mx-auto mt-8 w-full max-w-2xl">
      <h1 class="text-xl font-semibold text-foreground">
        {i18n.t(
          completeRemoval
            ? 'wallet.identity.profile.review.removalTitle'
            : 'wallet.identity.profile.review.title'
        )}
      </h1>
      <p class="mt-1 text-sm text-muted-foreground">
        {i18n.t(
          completeRemoval
            ? 'wallet.identity.profile.review.removalDescription'
            : 'wallet.identity.profile.review.description'
        )}
      </p>

      <section class="mt-6 rounded-2xl bg-muted/45 p-5 dark:bg-muted/30">
        <p class="text-sm font-semibold text-foreground">
          {i18n.t('wallet.identity.profile.review.changes')}
        </p>
        <div class="mt-4 space-y-3">
          {#if preflight.changedFields.includes('avatar')}
            <div class="flex items-center justify-between gap-4">
              <span class="text-sm text-muted-foreground">
                {i18n.t('wallet.identity.profile.review.avatar')}
              </span>
              <span class="text-sm font-medium text-foreground">
                {preflight.proposedProfile.avatarBase64
                  ? i18n.t('wallet.identity.profile.review.replaced')
                  : i18n.t('wallet.identity.profile.review.removed')}
              </span>
            </div>
          {/if}
          {#if preflight.changedFields.includes('description')}
            <div class="flex items-start justify-between gap-4">
              <span class="text-sm text-muted-foreground">
                {i18n.t('wallet.identity.profile.review.descriptionLabel')}
              </span>
              <span
                class="max-w-sm text-right text-sm font-medium whitespace-pre-line text-foreground"
              >
                {preflight.proposedProfile.description ||
                  i18n.t('wallet.identity.profile.review.removed')}
              </span>
            </div>
          {/if}
        </div>
      </section>

      <section class="mt-4 rounded-2xl bg-muted/45 p-5 dark:bg-muted/30">
        <p class="text-sm font-semibold text-foreground">
          {i18n.t('wallet.identity.profile.review.cost')}
        </p>
        <div class="mt-4 space-y-3 text-sm">
          <div class="flex items-center justify-between gap-4">
            <span class="text-muted-foreground">{i18n.t('wallet.identity.profile.review.fee')}</span
            >
            <span class="font-medium text-foreground">{preflight.feeDisplay} VRSCTEST</span>
          </div>
          <div class="flex items-center justify-between gap-4">
            <span class="text-muted-foreground"
              >{i18n.t('wallet.identity.profile.review.funding')}</span
            >
            <span class="max-w-sm truncate font-mono text-xs text-foreground">
              {preflight.fundingSummary}
            </span>
          </div>
        </div>
      </section>

      <p
        class="mt-4 rounded-xl bg-amber-500/10 px-4 py-3 text-xs leading-relaxed text-amber-800 dark:text-amber-200"
      >
        {i18n.t(
          completeRemoval
            ? 'wallet.identity.profile.review.removalDisclosure'
            : removesProfileData
              ? 'wallet.identity.profile.review.confirmationDisclosureWithHistory'
              : 'wallet.identity.profile.review.confirmationDisclosure'
        )}
      </p>

      {#if errorMessage}
        <p class="mt-4 rounded-md bg-destructive/12 px-3 py-2 text-sm text-destructive">
          {errorMessage}
        </p>
      {/if}

      <div class="mt-6 flex justify-end">
        <Button onclick={publishProfile}>
          {i18n.t(
            completeRemoval
              ? 'wallet.identity.profile.review.publishRemoval'
              : 'wallet.identity.profile.review.publish'
          )}
        </Button>
      </div>
    </div>
  {:else if step === 'publishing'}
    <div class="flex flex-1 flex-col items-center justify-center px-6 text-center">
      <Spinner class="size-10 text-primary" />
      <h1 class="mt-5 text-xl font-semibold text-foreground">
        {i18n.t(
          completeRemoval
            ? 'wallet.identity.profile.publishing.removalTitle'
            : 'wallet.identity.profile.publishing.title'
        )}
      </h1>
      <p class="mt-2 max-w-md text-sm text-muted-foreground">
        {i18n.t(
          completeRemoval
            ? 'wallet.identity.profile.publishing.removalDescription'
            : 'wallet.identity.profile.publishing.description'
        )}
      </p>
      <p class="mt-5 rounded-xl bg-muted/35 px-4 py-3 text-xs text-muted-foreground">
        {i18n.t('wallet.identity.profile.publishing.keepOpen')}
      </p>
    </div>
  {:else if step === 'submitted'}
    <div class="flex flex-1 flex-col items-center justify-center px-6 text-center">
      <div
        class="flex size-14 items-center justify-center rounded-full bg-amber-500/12 text-amber-700 dark:text-amber-300"
      >
        <Clock3Icon class="size-7" />
      </div>
      <h1 class="mt-5 text-xl font-semibold text-foreground">
        {i18n.t(
          completeRemoval
            ? 'wallet.identity.profile.submitted.removalTitle'
            : 'wallet.identity.profile.submitted.title'
        )}
      </h1>
      <p class="mt-2 max-w-md text-sm text-muted-foreground">
        {i18n.t(
          completeRemoval
            ? 'wallet.identity.profile.submitted.removalDescription'
            : 'wallet.identity.profile.submitted.description'
        )}
      </p>
      <div class="mt-5 flex max-w-lg items-center gap-2 rounded-xl bg-muted/35 px-4 py-3">
        <span class="min-w-0 truncate font-mono text-xs text-foreground">{submittedTxid}</span>
        <CopyButton
          {copied}
          onclick={copyTxid}
          aria-label={i18n.t('wallet.identity.detail.copy')}
          copiedIconClass="size-4 text-emerald-600 dark:text-emerald-300"
        />
      </div>
      <Button class="mt-6" onclick={onCancel}>
        {i18n.t('wallet.identity.profile.submitted.done')}
      </Button>
    </div>
  {/if}
</div>
