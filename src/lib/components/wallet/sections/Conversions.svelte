<!--
  Component: Conversions
  Purpose: Entry wrapper for unified transfer wizard (convert intent).
-->

<script lang="ts">
  import TransferWizard from './TransferWizard.svelte';
  import type { TransferEntryContext } from './transfer-wizard/types';
  import type { WalletNetwork } from '$lib/types/wallet';

  type ConversionsProps = {
    entryContext?: TransferEntryContext | null;
    onClose?: () => void;
    walletNetwork?: WalletNetwork;
  };

  const defaultCloseHandler = () => {};

  let {
    entryContext = null,
    onClose = defaultCloseHandler,
    walletNetwork = 'mainnet',
  }: ConversionsProps = $props();
</script>

<TransferWizard entryIntent="convert" {entryContext} {onClose} {walletNetwork} />
