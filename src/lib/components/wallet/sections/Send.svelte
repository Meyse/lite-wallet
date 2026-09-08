<!--
  Component: Send
  Purpose: Entry wrapper for unified transfer wizard (send intent).
-->

<script lang="ts">
  import TransferWizard from './TransferWizard.svelte';
  import type { TransferEntryContext } from './transfer-wizard/types';

  type SendProps = {
    entryContext?: TransferEntryContext | null;
    onClose?: () => void;
  };

  const defaultCloseHandler = () => {};

   
  let { entryContext = null, onClose = defaultCloseHandler }: SendProps = $props();
   
</script>

<TransferWizard entryIntent="send" {entryContext} {onClose} />
