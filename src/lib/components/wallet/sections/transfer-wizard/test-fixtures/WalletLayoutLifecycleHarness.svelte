<script lang="ts">
  import WalletLayout from '../../../WalletLayout.svelte';

  let walletData = $state({
    name: 'Wallet A',
    emoji: 'A',
    color: '',
    network: 'mainnet' as 'mainnet' | 'testnet',
    sessionId: 'session-1',
  });

  function replaceContext(kind: 'wallet' | 'network' | 'session'): void {
    walletData = {
      ...walletData,
      name: kind === 'wallet' ? 'Wallet B' : walletData.name,
      network: kind === 'network' ? 'testnet' : walletData.network,
      sessionId: kind === 'session' ? 'session-2' : walletData.sessionId,
    };
  }
</script>

<button data-replace-wallet type="button" onclick={() => replaceContext('wallet')}>Wallet</button>
<button data-replace-network type="button" onclick={() => replaceContext('network')}>Network</button
>
<button data-replace-session type="button" onclick={() => replaceContext('session')}>Session</button
>
<WalletLayout {walletData} />
