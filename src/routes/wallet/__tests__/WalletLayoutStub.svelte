<script lang="ts">
  let {
    walletData,
    onRetryWalletSession,
  }: {
    walletData: { name: string; emoji: string; color: string; network?: string; sessionId: string };
    onRetryWalletSession?: () => Promise<void>;
  } = $props();
</script>

<p data-testid="fallback-wallet-session">{walletData.sessionId}</p>
<button
  type="button"
  data-testid="fallback-retry-session"
  onclick={() => void onRetryWalletSession?.()}
>
  Retry session
</button>
