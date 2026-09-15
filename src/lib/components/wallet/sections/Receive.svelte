<!--
  Component: Receive
  Purpose: Show VRSC (and optionally BTC) address with copy. Uses get_addresses.
  Last Updated: Module 9 — walletService.getAddresses, copy button
  Security: Display-only; no sensitive handling beyond showing address
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import * as Card from '$lib/components/ui/card';
  import IdentifierText from '$lib/components/common/IdentifierText.svelte';
  import { Button } from '$lib/components/ui/button';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import CopyIcon from '@lucide/svelte/icons/copy';
  import CheckIcon from '@lucide/svelte/icons/check';
  import { toast } from 'svelte-sonner';
  import * as walletService from '$lib/services/walletService.js';
  import { i18nStore } from '$lib/i18n';
  import type { WalletNetwork } from '$lib/types/wallet.js';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';

  let addresses = $state<{ vrsc_address: string; eth_address: string; btc_address: string } | null>(
    null
  );
  let network = $state<WalletNetwork>('mainnet');
  let loading = $state(true);
  let error = $state('');
  const copiedState = new TimedValueState<'vrsc' | 'eth' | 'btc'>();
  const copied = $derived(copiedState.current);
  const i18n = $derived($i18nStore);
  const vrscLabel = $derived(
    network === 'testnet'
      ? i18n.t('wallet.receive.vrscAddressTestnet')
      : i18n.t('wallet.receive.vrscAddress')
  );
  const btcLabel = $derived(
    network === 'testnet'
      ? i18n.t('wallet.receive.btcAddressTestnet')
      : i18n.t('wallet.receive.btcAddress')
  );
  const vrscCoinId = $derived(network === 'testnet' ? 'VRSCTEST' : 'VRSC');
  const btcCoinId = $derived(network === 'testnet' ? 'BTCTEST' : 'BTC');
  const ethLabel = $derived(
    network === 'testnet'
      ? i18n.t('wallet.receive.ethAddressTestnet')
      : i18n.t('wallet.receive.ethAddress')
  );
  const ethCoinId = $derived(network === 'testnet' ? 'GETH' : 'ETH');

  onMount(async () => {
    try {
      addresses = await walletService.getAddresses();
      try {
        const active = await walletService.getActiveWallet();
        network = active?.network ?? 'mainnet';
      } catch {
        // keep default network
      }
    } catch {
      error = i18n.t('wallet.receive.errorLoad');
    } finally {
      loading = false;
    }
  });

  async function copyAddress(addr: string, which: 'vrsc' | 'eth' | 'btc') {
    if (await writeClipboardText(addr)) {
      copiedState.set(which, 2000);
      const ticker = which.toUpperCase();
      toast.success(i18n.t('wallet.receive.toast.copiedTitle'), {
        description: i18n.t('wallet.receive.toast.copiedDescription', { ticker }),
      });
      return;
    }

    copiedState.clear();
    toast.error(i18n.t('wallet.receive.toast.copyFailed'));
  }
</script>

<div class="mx-auto flex max-w-lg flex-col gap-6 p-6">
  <Card.Root>
    <Card.Header>
      <Card.Title class="flex items-center gap-2">
        <DownloadIcon class="h-5 w-5" />
        {i18n.t('wallet.receive.title')}
      </Card.Title>
      <Card.Description>{i18n.t('wallet.receive.description')}</Card.Description>
    </Card.Header>
    <Card.Content class="space-y-6">
      {#if loading}
        <p class="text-sm text-muted-foreground">{i18n.t('wallet.receive.loading')}</p>
      {:else if error}
        <p class="text-sm text-destructive">{error}</p>
      {:else if addresses}
        <div>
          <p id="receive-vrsc-label" class="mb-2 block text-sm font-medium">
            <span class="inline-flex items-center gap-2">
              <CoinIcon coinId={vrscCoinId} proto="vrsc" size={18} decorative />
              <span>{vrscLabel}</span>
            </span>
          </p>
          <div class="flex items-center gap-2">
            <div
              id="receive-vrsc"
              aria-labelledby="receive-vrsc-label"
              class="flex min-h-11 min-w-0 flex-1 items-center rounded-md bg-muted/90 px-4 py-2 dark:bg-muted/65"
            >
              <IdentifierText
                value={addresses.vrsc_address}
                mode="full"
                class="block min-w-0 text-xs text-foreground sm:text-sm"
              />
            </div>
            <Button
              variant="outline"
              size="icon"
              onclick={() => copyAddress(addresses!.vrsc_address, 'vrsc')}
              title={i18n.t('wallet.receive.copy')}
            >
              {#if copied === 'vrsc'}
                <CheckIcon class="h-4 w-4 text-green-600" />
              {:else}
                <CopyIcon class="h-4 w-4" />
              {/if}
            </Button>
          </div>
        </div>
        <div>
          <p id="receive-eth-label" class="mb-2 block text-sm font-medium">
            <span class="inline-flex items-center gap-2">
              <CoinIcon coinId={ethCoinId} proto="eth" size={18} decorative />
              <span>{ethLabel}</span>
            </span>
          </p>
          <div class="flex items-center gap-2">
            <div
              id="receive-eth"
              aria-labelledby="receive-eth-label"
              class="flex min-h-11 min-w-0 flex-1 items-center rounded-md bg-muted/90 px-4 py-2 dark:bg-muted/65"
            >
              <IdentifierText
                value={addresses.eth_address}
                mode="full"
                class="block min-w-0 text-xs text-foreground sm:text-sm"
              />
            </div>
            <Button
              variant="outline"
              size="icon"
              onclick={() => copyAddress(addresses!.eth_address, 'eth')}
              title={i18n.t('wallet.receive.copy')}
            >
              {#if copied === 'eth'}
                <CheckIcon class="h-4 w-4 text-green-600" />
              {:else}
                <CopyIcon class="h-4 w-4" />
              {/if}
            </Button>
          </div>
        </div>
        <div>
          <p id="receive-btc-label" class="mb-2 block text-sm font-medium">
            <span class="inline-flex items-center gap-2">
              <CoinIcon coinId={btcCoinId} proto="btc" size={18} decorative />
              <span>{btcLabel}</span>
            </span>
          </p>
          <div class="flex items-center gap-2">
            <div
              id="receive-btc"
              aria-labelledby="receive-btc-label"
              class="flex min-h-11 min-w-0 flex-1 items-center rounded-md bg-muted/90 px-4 py-2 dark:bg-muted/65"
            >
              <IdentifierText
                value={addresses.btc_address}
                mode="full"
                class="block min-w-0 text-xs text-foreground sm:text-sm"
              />
            </div>
            <Button
              variant="outline"
              size="icon"
              onclick={() => copyAddress(addresses!.btc_address, 'btc')}
              title={i18n.t('wallet.receive.copy')}
            >
              {#if copied === 'btc'}
                <CheckIcon class="h-4 w-4 text-green-600" />
              {:else}
                <CopyIcon class="h-4 w-4" />
              {/if}
            </Button>
          </div>
        </div>
      {/if}
    </Card.Content>
  </Card.Root>
</div>
