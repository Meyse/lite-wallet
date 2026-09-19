<script lang="ts">
  import WalletLayout from '$lib/components/wallet/WalletLayout.svelte';
  const walletData = {
    name: 'Example wallet',
    emoji: '🌱',
    color: 'blue',
    network: 'mainnet' as const,
    sessionId: 'transfer-navigation-fixture',
  };
</script>

<div class="h-screen overflow-hidden"><WalletLayout {walletData} /></div>
