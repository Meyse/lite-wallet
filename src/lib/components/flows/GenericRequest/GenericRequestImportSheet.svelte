<script lang="ts">
  import ClipboardPasteIcon from '@lucide/svelte/icons/clipboard-paste';
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { Button } from '$lib/components/ui/button';
  import { Textarea } from '$lib/components/ui/textarea';
  import { i18nStore } from '$lib/i18n';
  import * as walletService from '$lib/services/walletService.js';

  type GenericRequestImportSheetProps = {
    isOpen?: boolean;
    value?: string;
    submitting?: boolean;
    errorMessage?: string;
    onSubmit?: (value: string) => void;
  };

  const defaultSubmit = (value: string): void => {
    void value;
  };

  let {
    isOpen = $bindable(false),
    value = $bindable(''),
    submitting = false,
    errorMessage = '',
    onSubmit = defaultSubmit
  }: GenericRequestImportSheetProps = $props();

  const i18n = $derived($i18nStore);
  let pasteBusy = $state(false);

  function handleOpenAutoFocus(event: Event) {
    event.preventDefault();
  }

  async function handlePaste(): Promise<void> {
    if (pasteBusy || submitting) return;

    pasteBusy = true;
    try {
      const clipboardText = await walletService.readClipboardText();
      if (clipboardText?.trim()) {
        value = clipboardText.trim();
      }
    } finally {
      pasteBusy = false;
    }
  }

  function handleSubmit(): void {
    if (!value.trim() || submitting) return;
    onSubmit(value.trim());
  }
</script>

<StandardRightSheet
  bind:isOpen
  title={i18n.t('genericRequest.import.title')}
  onOpenAutoFocus={handleOpenAutoFocus}
>
  <div class="flex h-full min-h-0 flex-col">
    <div class="min-h-0 flex-1">
      <label for="generic-request-input" class="text-sm font-medium text-foreground">
        {i18n.t('genericRequest.import.label')}
      </label>

      <div class="relative mt-2">
        <Button
          variant="ghost"
          size="icon-sm"
          class="text-muted-foreground hover:text-foreground focus-visible:ring-ring/50 absolute top-2 right-2 z-10 rounded-sm"
          aria-label={i18n.t('genericRequest.import.pasteTooltip')}
          title={i18n.t('genericRequest.import.pasteTooltip')}
          disabled={pasteBusy || submitting}
          onclick={() => void handlePaste()}
        >
          {#if pasteBusy}
            <LoaderCircleIcon class="size-4 animate-spin" />
          {:else}
            <ClipboardPasteIcon class="size-4" />
          {/if}
        </Button>

        <Textarea
          id="generic-request-input"
          bind:value
          class="min-h-[220px] resize-none pr-12 text-base md:text-sm"
          placeholder={i18n.t('genericRequest.import.placeholder')}
          spellcheck="false"
          autocapitalize="off"
          autocomplete="off"
          disabled={submitting}
          onkeydown={(event) => {
            if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
              event.preventDefault();
              handleSubmit();
            }
          }}
        />
      </div>

      <p class="mt-2 text-xs text-muted-foreground">{i18n.t('genericRequest.import.help')}</p>

      {#if errorMessage}
        <p class="mt-3 rounded-md bg-destructive/12 px-3 py-2 text-sm text-destructive">{errorMessage}</p>
      {/if}
    </div>

    <div class="mt-4 flex items-center justify-end gap-3">
      <Button class="gap-2" disabled={!value.trim() || submitting} onclick={handleSubmit}>
        {#if submitting}
          <LoaderCircleIcon class="size-4 animate-spin" />
        {/if}
        {i18n.t('genericRequest.import.submit')}
      </Button>
    </div>
  </div>
</StandardRightSheet>
