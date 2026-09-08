<!--
  Component: Receive
  Purpose: Show VRSC (and optionally BTC) address with copy. Uses get_addresses.
  Last Updated: Module 9 — walletService.getAddresses, copy button
  Security: Display-only; no sensitive handling beyond showing address
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import * as Card from '$lib/components/ui/card';
  import * as Label from '$lib/components/ui/label';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import CopyIcon from '@lucide/svelte/icons/copy';
  import CheckIcon from '@lucide/svelte/icons/check';
  import { toast } from 'svelte-sonner';
  import * as walletService from '$lib/services/walletService.js';
  import { i18nStore } from '$lib/i18n';
  import type { WalletNetwork } from '$lib/types/wallet.js';
  import { TimedValueState, writeClipboardText } from '$lib/utils/clipboard-feedback.svelte';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';

  let addresses = $state<{ vrsc_address: string; eth_address: string; btc_address: string } | null>(null);
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
        description: i18n.t('wallet.receive.toast.copiedDescription', { ticker })
      });
      return;
    }

    copiedState.clear();
    toast.error(i18n.t('wallet.receive.toast.copyFailed'));
  }
</script>

<div class="flex flex-col gap-6 p-6 max-w-lg mx-auto">
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
        <p class="text-muted-foreground text-sm">{i18n.t('wallet.receive.loading')}</p>
      {:else if error}
        <p class="text-destructive text-sm">{error}</p>
      {:else if addresses}
        <div>
          <Label.Root for="receive-vrsc" class="mb-2 block">
            <span class="inline-flex items-center gap-2">
              <CoinIcon coinId={vrscCoinId} proto="vrsc" size={18} decorative />
              <span>{vrscLabel}</span>
            </span>
          </Label.Root>
          <div class="flex gap-2 items-center">
            <Input
              id="receive-vrsc"
              type="text"
              readonly
              value={addresses.vrsc_address}
              class="flex-1 truncate font-mono text-xs sm:text-sm"
            />
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
          <Label.Root for="receive-eth" class="mb-2 block">
            <span class="inline-flex items-center gap-2">
              <CoinIcon coinId={ethCoinId} proto="eth" size={18} decorative />
              <span>{ethLabel}</span>
            </span>
          </Label.Root>
          <div class="flex gap-2 items-center">
            <Input
              id="receive-eth"
              type="text"
              readonly
              value={addresses.eth_address}
              class="flex-1 truncate font-mono text-xs sm:text-sm"
            />
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
          <Label.Root for="receive-btc" class="mb-2 block">
            <span class="inline-flex items-center gap-2">
              <CoinIcon coinId={btcCoinId} proto="btc" size={18} decorative />
              <span>{btcLabel}</span>
            </span>
          </Label.Root>
          <div class="flex gap-2 items-center">
            <Input
              id="receive-btc"
              type="text"
              readonly
              value={addresses.btc_address}
              class="flex-1 truncate font-mono text-xs sm:text-sm"
            />
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
