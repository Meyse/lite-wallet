<!--
  Component: Conversions
  Purpose: Entry wrapper for unified transfer wizard (convert intent).
-->

<script lang="ts">
  import TransferWizard from './TransferWizard.svelte';
  import type { TransferEntryContext } from './transfer-wizard/types';
  import type { WalletNetwork } from '$lib/types/wallet';

  type ConversionsProps = {
    active?: boolean;
    entryContext?: TransferEntryContext | null;
    onClose?: () => void;
    walletNetwork?: WalletNetwork;
    walletKey?: string;
  };

  const defaultCloseHandler = () => {};

  let {
    active = true,
    entryContext = null,
    onClose = defaultCloseHandler,
    walletNetwork = 'mainnet',
    walletKey = '',
  }: ConversionsProps = $props();
</script>

<TransferWizard
  entryIntent="convert"
  {active}
  {entryContext}
  {onClose}
  {walletNetwork}
  {walletKey}
/>
