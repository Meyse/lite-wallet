<!--
  Component: Apps
  Purpose: Placeholder for Apps section
  Last Updated: Added for simplified sidebar navigation
  Security: No sensitive operations - placeholder only
-->

<script lang="ts">
  import WalletEmptyState from '$lib/components/wallet/WalletEmptyState.svelte';
  import { i18nStore } from '$lib/i18n';

  const i18n = $derived($i18nStore);
</script>

<section class="flex h-full min-h-0 flex-col" aria-label={i18n.t('wallet.sidebar.apps')}>
  <WalletEmptyState
    illustration="apps"
    description={i18n.t('wallet.apps.emptyDescription')}
    actionLabel={i18n.t('wallet.placeholder.comingSoon')}
    comingSoon
    testId="apps-empty"
  />
</section>
