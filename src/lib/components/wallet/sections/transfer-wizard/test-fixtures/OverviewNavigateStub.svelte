<script lang="ts">
  import type { TransferEntryContext } from '../types';

  let {
    onNavigateToSend,
  }: {
    onNavigateToSend: (context: TransferEntryContext) => void;
  } = $props();
</script>

<button
  data-open-production-send
  type="button"
  onclick={() =>
    onNavigateToSend({
      coinId: 'ETH',
      channelId: 'eth.ETH',
      scopeKind: 'transparent',
      readOnly: false,
    })}
>
  Open send
</button>
