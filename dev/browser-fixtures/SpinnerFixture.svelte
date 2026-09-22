<script lang="ts">
  import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
  import { Button } from '$lib/components/ui/button';
  import { Spinner } from '$lib/components/ui/spinner';
  import DelayedStatus from '$lib/components/common/DelayedStatus.svelte';
  import TransferSourceStatus from '$lib/components/wallet/sections/transfer-wizard/TransferSourceStatus.svelte';
  import VerusIdLookup from '$lib/components/wallet/sections/identity/VerusIdLookup.svelte';
  import Watchlist from '$lib/components/wallet/sections/Watchlist.svelte';
  import { i18nStore } from '$lib/i18n';

  let { screen = 'examples' }: { screen?: string } = $props();
  const i18n = $derived($i18nStore);
</script>

{#if screen === 'watchlist'}
  <main class="flex h-screen flex-col overflow-hidden bg-background dark:bg-app-canvas">
    <Watchlist walletNetwork="mainnet" />
  </main>
{:else}
  <main class="min-h-screen bg-background p-5 text-foreground dark:bg-app-canvas">
    <header class="mb-8">
      <h1 class="text-xl font-semibold">Spinner consistency</h1>
      <p class="mt-1 text-sm text-muted-foreground">
        Production components · Synthetic pending states
      </p>
    </header>

    <section class="grid grid-cols-[160px_1fr] gap-8 border-b border-border py-5">
      <h2 class="text-sm font-medium">Button</h2>
      <div class="h-[60px]" data-example="button">
        <VerusIdLookup
          walletNetwork="mainnet"
          initialState={{
            query: 'alex@',
            submittedQuery: '',
            status: 'idle',
            result: null,
            scrollTop: 0,
          }}
        />
      </div>
    </section>

    <section class="grid grid-cols-[160px_1fr] gap-8 border-b border-border py-6">
      <h2 class="text-sm font-medium">Inline status</h2>
      <div class="space-y-4" data-example="inline">
        <TransferSourceStatus loading failed={false} hasAssets={false} onRetry={() => {}} />
        <DelayedStatus active label={i18n.t('wallet.loading.fetchingAddresses')} />
      </div>
    </section>

    <section class="grid grid-cols-[160px_1fr] items-center gap-8 border-b border-border py-6">
      <h2 class="text-sm font-medium">Standalone status</h2>
      <div class="flex h-14 items-center" data-example="standalone">
        <Spinner
          class="size-5 text-settings-muted-foreground"
          aria-label={i18n.t('common.loading')}
        />
      </div>
    </section>

    <section class="grid grid-cols-[160px_1fr] items-center gap-8 py-6">
      <h2 class="text-sm font-medium">Refresh action</h2>
      <div data-example="refresh">
        <Button variant="link" size="sm" class="gap-2 px-0" disabled aria-busy="true">
          <RefreshCwIcon class="size-4 animate-spin" aria-hidden="true" />
          {i18n.t('wallet.identity.provisioning.refresh')}
        </Button>
      </div>
    </section>

    <p class="mt-3 text-xs text-muted-foreground">
      One open-circle loader. Refresh keeps its arrows. Reduced motion stops both.
    </p>
  </main>
{/if}
