<script lang="ts">
  import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
  import ShieldXIcon from '@lucide/svelte/icons/shield-x';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { i18nStore } from '$lib/i18n';
  import type { GuardFlowMode } from '$lib/components/flows/VerusIdGuard/types';

  const defaultHandler = (mode: GuardFlowMode) => {
    void mode;
  };

  type VerusIdGuardSheetProps = {
    isOpen?: boolean;
    onSelectMode?: typeof defaultHandler;
  };

  let { isOpen = $bindable(false), onSelectMode = defaultHandler }: VerusIdGuardSheetProps = $props();

  const i18n = $derived($i18nStore);

  function handleOpenAutoFocus(event: Event) {
    event.preventDefault();
  }

  function handleSelect(mode: GuardFlowMode) {
    isOpen = false;
    onSelectMode(mode);
  }
</script>

<StandardRightSheet
  bind:isOpen
  title={i18n.t('guard.sheet.title')}
  onOpenAutoFocus={handleOpenAutoFocus}
>
  <div class="space-y-3">
    <button
      type="button"
      class="selection-card-button selection-card-button--revoke"
      onclick={() => handleSelect('revoke')}
    >
      <div class="flex items-start gap-3">
        <ShieldXIcon
          class="selection-card-icon selection-card-icon--lg selection-card-icon--revoke"
          absoluteStrokeWidth
          stroke-linecap="butt"
          aria-hidden="true"
        />
        <div class="min-w-0">
          <p class="text-sm font-semibold text-foreground">{i18n.t('guard.sheet.revokeTitle')}</p>
          <p class="text-muted-foreground mt-1 text-xs">{i18n.t('guard.sheet.revokeDescription')}</p>
        </div>
      </div>
    </button>

    <button
      type="button"
      class="selection-card-button selection-card-button--recover"
      onclick={() => handleSelect('recover')}
    >
      <div class="flex items-start gap-3">
        <ShieldCheckIcon
          class="selection-card-icon selection-card-icon--lg selection-card-icon--recover"
          absoluteStrokeWidth
          stroke-linecap="butt"
          aria-hidden="true"
        />
        <div class="min-w-0">
          <p class="text-sm font-semibold text-foreground">{i18n.t('guard.sheet.recoverTitle')}</p>
          <p class="text-muted-foreground mt-1 text-xs">{i18n.t('guard.sheet.recoverDescription')}</p>
        </div>
      </div>
    </button>
  </div>
</StandardRightSheet>
