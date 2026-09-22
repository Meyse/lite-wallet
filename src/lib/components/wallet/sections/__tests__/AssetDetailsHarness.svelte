<script lang="ts">
  import { untrack } from 'svelte';
  import AssetDetails from '../AssetDetails.svelte';

  let { initialCoinId }: { initialCoinId: string } = $props();
  let coinId = $state(untrack(() => initialCoinId));

  export function switchCoin(value: string): void {
    coinId = value;
  }
</script>

<AssetDetails {coinId} />
