<script lang="ts">
  import { type ComponentProps, untrack } from 'svelte';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import Overview from '../Overview.svelte';
  let {
    initialWallet,
    onOpenAssetDetails,
  }: {
    initialWallet: ComponentProps<typeof Overview>['walletData'];
    onOpenAssetDetails?: ComponentProps<typeof Overview>['onOpenAssetDetails'];
  } = $props();
  let wallet = $state(untrack(() => initialWallet));
  export function switchWallet(value: typeof wallet): void {
    wallet = value;
  }
</script>

<Tooltip.Provider>
  <Overview walletData={wallet} {onOpenAssetDetails} />
</Tooltip.Provider>
