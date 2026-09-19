<!--
  Component: Send
  Purpose: Entry wrapper for unified transfer wizard (send intent).
-->

<script lang="ts">
  import TransferWizard from './TransferWizard.svelte';
  import type { TransferEntryContext } from './transfer-wizard/types';
  import type { WalletNetwork } from '$lib/types/wallet';

  type SendProps = {
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
  }: SendProps = $props();
</script>

<TransferWizard entryIntent="send" {active} {entryContext} {onClose} {walletNetwork} {walletKey} />
