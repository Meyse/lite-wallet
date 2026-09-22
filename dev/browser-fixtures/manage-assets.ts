import '../../src/app.css';
import { mount } from 'svelte';
import { setLocale } from '$lib/i18n';
import type { AssetPreferencesState, CoinDefinition } from '$lib/types/wallet';
import Fixture from './ManageAssetsFixture.svelte';

const params = new URLSearchParams(location.search);
const chain = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
const contract = '0x1f9840a85d5af5bf1d1762f925bdaddc4201f984';
const vrsc: CoinDefinition = {
  id: 'VRSC',
  currencyId: chain,
  systemId: chain,
  displayTicker: 'VRSC',
  displayName: 'Verus',
  proto: 'vrsc',
  compatibleChannels: ['vrpc'],
  decimals: 8,
  vrpcEndpoints: [],
  secondsPerBlock: 60,
  isTestnet: false,
};
const usdc: CoinDefinition = {
  ...vrsc,
  id: 'USDC',
  currencyId: '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48',
  systemId: 'ETH',
  displayTicker: 'USDC',
  displayName: 'USD Coin',
  proto: 'erc20',
  compatibleChannels: ['erc20'],
  decimals: 6,
};
const uni: CoinDefinition = {
  ...usdc,
  id: `erc20_${contract}`,
  currencyId: contract,
  systemId: contract,
  displayTicker: 'UNI',
  displayName: 'Uniswap',
  decimals: 18,
};
let preferences: AssetPreferencesState = {
  network: 'mainnet',
  sessionId: 'manage-assets-fixture',
  portfolioCoinIds: [vrsc.id, usdc.id, uni.id],
  hiddenAssetKeys: [],
};
const discovery = {
  network: 'mainnet',
  scopeMetadataComplete: true,
  sources: [
    {
      systemId: chain,
      systemTicker: 'VRSC',
      systemDisplayName: 'Verus',
      status: 'available',
      checkedScopeCount: 1,
      uncheckedScopeCount: 0,
      includesReadOnly: false,
      privateScopeChecked: false,
    },
  ],
  holdings: [
    {
      assetKey: `vrsc:${chain.toLowerCase()}`,
      currencyId: chain,
      systemId: chain,
      systemTicker: 'VRSC',
      systemDisplayName: 'Verus',
      balance: '24.5',
      balanceStatus: 'available',
      includesReadOnly: false,
      coin: vrsc,
    },
  ],
};
let resolveUni: () => void = () => {};
let resolveDiscovery: () => void = () => {};
const uniWait = new Promise<void>((resolve) => (resolveUni = resolve));
const discoveryWait = new Promise<void>((resolve) => (resolveDiscovery = resolve));
let stalledOnce = false;
const invoke = async (command: string, args: Record<string, unknown> = {}) => {
  if (command === 'get_coin_registry') {
    if (params.get('stall') === 'metadata' && !stalledOnce) {
      stalledOnce = true;
      return new Promise(() => {});
    }
    return [vrsc, usdc, uni];
  }
  if (command === 'get_asset_preferences') return preferences;
  if (command === 'discover_vrpc_assets') {
    if (params.get('stall') === 'discovery' && !stalledOnce) {
      stalledOnce = true;
      await discoveryWait;
    }
    return discovery;
  }
  if (command === 'get_coin_scopes')
    return {
      coinId: args.coin_id,
      scopes: [
        {
          channelId: `erc20.${args.coin_id}`,
          coinId: args.coin_id,
          systemId: 'ETH',
          systemTicker: 'ETH',
          systemDisplayName: 'Ethereum',
          address: 'fixture',
          addressLabel: 'fixture',
          isPrimaryAddress: true,
          isReadOnly: false,
          scopeKind: 'transparent',
        },
      ],
    };
  if (command === 'get_balances') {
    if (args.coin_id === uni.id && params.get('stall') === 'uni' && !stalledOnce) {
      stalledOnce = true;
      await uniWait;
    }
    const total = args.coin_id === uni.id ? '2' : '125';
    return { total, confirmed: total, pending: '0' };
  }
  if (command === 'set_asset_preferences') {
    preferences = {
      ...preferences,
      portfolioCoinIds: args.portfolio_coin_ids as string[],
      hiddenAssetKeys: args.hidden_asset_keys as string[],
    };
    return preferences;
  }
  if (command === 'start_update_engine') return;
  if (command.startsWith('plugin:event|')) return 1;
  throw new Error(`Unsupported fixture command: ${command}`);
};
Object.assign(window, {
  __TAURI_INTERNALS__: { invoke, transformCallback: () => 1, unregisterCallback: () => {} },
  __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
  fixture: { resolveUni, resolveDiscovery },
});
document.documentElement.classList.toggle('dark', params.get('theme') === 'dark');
setLocale(params.get('locale') === 'nl' ? 'nl' : 'en');
mount(Fixture, { target: document.getElementById('fixture')! });
