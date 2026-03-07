<!--
  Component: Conversions
  Purpose: Entry wrapper for unified transfer wizard (convert intent).
-->

<script lang="ts">
  import TransferWizard from './TransferWizard.svelte';
  import type { TransferEntryContext } from './transfer-wizard/types';

  type ConversionsProps = {
    entryContext?: TransferEntryContext | null;
    onClose?: () => void;
  };

  const defaultCloseHandler = () => {};

   
  let { entryContext = null, onClose = defaultCloseHandler }: ConversionsProps = $props();
   
</script>

<TransferWizard entryIntent="convert" {entryContext} {onClose} />
