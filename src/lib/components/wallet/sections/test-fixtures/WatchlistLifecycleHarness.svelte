<script lang="ts">
  import Watchlist from '../Watchlist.svelte';
  import { transferWalletSessionKey } from '../transfer-wizard/preflightRequest.js';
  import type { WalletNetwork } from '$lib/types/wallet.js';

  let walletName = $state('Wallet A');
  let walletNetwork = $state<WalletNetwork>('mainnet');
  let walletSessionId = $state('session-a');
  const walletKey = $derived(transferWalletSessionKey(walletName, walletNetwork, walletSessionId));

  function switchWallet(): void {
    walletName = 'Wallet B';
    walletNetwork = 'testnet';
    walletSessionId = 'session-b';
  }
</script>

<button type="button" data-testid="switch-wallet" onclick={switchWallet}>Switch wallet</button>
<span data-testid="wallet-key">{walletKey}</span>

{#key walletKey}
  <Watchlist {walletNetwork} />
{/key}
