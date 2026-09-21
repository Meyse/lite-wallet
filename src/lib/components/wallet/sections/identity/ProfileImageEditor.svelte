<script lang="ts">
  import { onDestroy } from 'svelte';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Spinner } from '$lib/components/ui/spinner';
  import { i18nStore } from '$lib/i18n';
  import {
    cropRectangle,
    encodeProfileImage,
    PROFILE_IMAGE_BACKGROUND,
    type ProfileImageCandidates,
    type ProfileImageKind,
    profileImageSize,
  } from '$lib/identity/profileImages';

  let {
    kind,
    bitmap,
    onStage,
    onCancel,
  }: {
    kind: ProfileImageKind;
    bitmap: ImageBitmap;
    onStage: (value: ProfileImageCandidates) => void;
    onCancel: () => void;
  } = $props();
  const i18n = $derived($i18nStore);
  let canvas = $state<HTMLCanvasElement>();
  let zoom = $state(1);
  let x = $state(0.5);
  let y = $state(0.5);
  let busy = $state(false);
  let error = $state('');
  let alive = true;
  let drag: { x: number; y: number } | null = null;

  $effect(() => {
    if (!canvas || !bitmap) return;
    const [width, height] = profileImageSize(kind);
    canvas.width = width;
    canvas.height = height;
    const context = canvas.getContext('2d');
    if (!context) return;
    const crop = cropRectangle(bitmap.width, bitmap.height, kind, zoom, x, y);
    context.fillStyle = PROFILE_IMAGE_BACKGROUND;
    context.fillRect(0, 0, width, height);
    context.drawImage(bitmap, crop.x, crop.y, crop.width, crop.height, 0, 0, width, height);
  });

  async function stage() {
    if (busy) return;
    busy = true;
    error = '';
    try {
      const value = await encodeProfileImage(bitmap, kind, zoom, x, y);
      if (alive) onStage(value);
    } catch (cause) {
      if (alive)
        error = i18n.t(
          cause instanceof Error && cause.message === 'webp_unsupported'
            ? 'wallet.identity.profile.sequence.webpUnsupported'
            : cause instanceof Error && cause.message === 'too_large'
              ? 'wallet.identity.profile.draft.imageTooLarge'
              : 'wallet.identity.profile.sequence.encodeFailed'
        );
    } finally {
      if (alive) busy = false;
    }
  }
  function move(event: PointerEvent) {
    if (!drag || !bitmap || busy) return;
    const rect =
      event.currentTarget instanceof HTMLElement
        ? event.currentTarget.getBoundingClientRect()
        : null;
    if (!rect) return;
    const crop = cropRectangle(bitmap.width, bitmap.height, kind, zoom, x, y);
    if (bitmap.width > crop.width)
      x = Math.max(
        0,
        Math.min(
          1,
          x - ((event.clientX - drag.x) * crop.width) / rect.width / (bitmap.width - crop.width)
        )
      );
    if (bitmap.height > crop.height)
      y = Math.max(
        0,
        Math.min(
          1,
          y - ((event.clientY - drag.y) * crop.height) / rect.height / (bitmap.height - crop.height)
        )
      );
    drag = { x: event.clientX, y: event.clientY };
  }
  function keyboard(event: KeyboardEvent) {
    if (!bitmap || busy || !['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key))
      return;
    event.preventDefault();
    x = Math.max(
      0,
      Math.min(1, x + (event.key === 'ArrowLeft' ? -0.03 : event.key === 'ArrowRight' ? 0.03 : 0))
    );
    y = Math.max(
      0,
      Math.min(1, y + (event.key === 'ArrowUp' ? -0.03 : event.key === 'ArrowDown' ? 0.03 : 0))
    );
  }
  onDestroy(() => {
    alive = false;
  });
</script>

<div class="flex min-h-0 flex-1 flex-col gap-4">
  <div class="flex min-h-0 flex-1 items-center justify-center rounded-[10px] bg-muted p-4">
    <button
      type="button"
      class="touch-none overflow-hidden border-2 border-background focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none {kind ===
      'avatar'
        ? 'aspect-square h-full max-h-60 rounded-full'
        : 'aspect-[6/1] w-full max-w-[600px]'}"
      aria-label={i18n.t('wallet.identity.profile.draft.cropAccessible')}
      disabled={busy}
      onkeydown={keyboard}
      onpointerdown={(event) => {
        drag = { x: event.clientX, y: event.clientY };
        event.currentTarget.setPointerCapture(event.pointerId);
      }}
      onpointermove={move}
      onpointerup={() => (drag = null)}
      onpointercancel={() => (drag = null)}
    >
      <canvas bind:this={canvas} class="block size-full"></canvas>
    </button>
  </div>
  <div class="flex h-9 shrink-0 items-center gap-3.5">
    <Label for="profile-image-zoom" class="text-[13px] font-normal text-muted-foreground"
      >{i18n.t('wallet.identity.profile.editor.cropZoom')}</Label
    >
    <Input
      id="profile-image-zoom"
      type="range"
      min="1"
      max="3"
      step="0.01"
      bind:value={zoom}
      disabled={busy}
      class="h-6 w-[200px] rounded-none border-0 bg-transparent p-0 accent-primary dark:bg-transparent"
    />
    <span class="ml-auto text-xs text-muted-foreground"
      >{i18n.t('wallet.identity.profile.draft.cropHelp')}</span
    >
  </div>
  <details class="text-xs leading-5 text-muted-foreground">
    <summary class="w-fit rounded-sm focus-visible:ring-2 focus-visible:ring-ring"
      >{i18n.t('wallet.identity.profile.ux.imageRequirements')}</summary
    >
    <p class="mt-2">{i18n.t('wallet.identity.profile.draft.sourceHelp')}</p>
    <p class="mt-1">{i18n.t(`wallet.identity.profile.draft.${kind}Help`)}</p>
  </details>
  {#if error}<p role="alert" class="text-sm text-destructive">{error}</p>{/if}
</div>
<div class="flex shrink-0 justify-end gap-2 pt-1" data-profile-footer>
  <Button variant="secondary" onclick={onCancel} disabled={busy}>{i18n.t('common.cancel')}</Button>
  <Button onclick={stage} disabled={busy}
    >{#if busy}<Spinner class="size-4" />{/if}{i18n.t(
      'wallet.identity.profile.ux.useImage'
    )}</Button
  >
</div>
