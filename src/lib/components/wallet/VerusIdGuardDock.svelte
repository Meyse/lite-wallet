<script lang="ts">
  import ShieldIcon from '@lucide/svelte/icons/shield';
  import GuardFlowHost from '$lib/components/flows/VerusIdGuard/GuardFlowHost.svelte';
  import type { GuardFlowMode } from '$lib/components/flows/VerusIdGuard/types';
  import { i18nStore } from '$lib/i18n';
  import type { WalletNetwork } from '$lib/types/wallet.js';
  import VerusIdGuardSheet from './VerusIdGuardSheet.svelte';

  type VerusIdGuardDockProps = {
    defaultNetwork?: WalletNetwork;
    context: 'welcome' | 'unlock';
  };

   
  let { defaultNetwork = 'mainnet', context }: VerusIdGuardDockProps = $props();
   

  let sheetOpen = $state(false);
  let activeMode = $state<GuardFlowMode | null>(null);

  const i18n = $derived($i18nStore);

  function openFlow(mode: GuardFlowMode) {
    activeMode = mode;
  }

  function closeFlow() {
    activeMode = null;
  }

  const buttonAriaLabel = $derived(
    context === 'welcome'
      ? i18n.t('guard.dock.ariaWelcome')
      : i18n.t('guard.dock.ariaUnlock')
  );
</script>

{#if !activeMode}
  <button
    type="button"
    class="guard-dock-button"
    aria-label={buttonAriaLabel}
    onclick={() => {
      sheetOpen = true;
    }}
  >
    <ShieldIcon class="guard-dock-icon" aria-hidden="true" />
    <span class="guard-dock-label">{i18n.t('guard.dock.label')}</span>
  </button>
{/if}

<VerusIdGuardSheet bind:isOpen={sheetOpen} onSelectMode={openFlow} />

{#if activeMode}
  <div class="fixed inset-0 z-[60]">
    <GuardFlowHost mode={activeMode} {defaultNetwork} onClose={closeFlow} />
  </div>
{/if}
