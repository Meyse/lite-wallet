import verusCoinCatalog from '$lib/coins/verusCoinCatalog.generated.json';
import type { CoinDefinition, Protocol } from '$lib/types/wallet.js';

export type CoinIconKind = 'asset' | 'fiat-symbol' | 'generated';

export interface CoinAssetIcon {
  kind: 'asset';
  light: string;
  dark: string;
  logoMapped: boolean;
  family?: string;
  symbolKey?: string;
}

export interface CoinFiatSymbolIcon {
  kind: 'fiat-symbol';
  symbol: string;
  logoMapped: boolean;
  family?: string;
  symbolKey?: string;
}

export interface CoinGeneratedIcon {
  kind: 'generated';
  logoMapped: boolean;
  family?: string;
  symbolKey?: string;
  seed?: string;
}

export type CoinIcon = CoinAssetIcon | CoinFiatSymbolIcon | CoinGeneratedIcon;

export interface CoinPresentation {
  id: string;
  currencyId: string;
  systemId: string;
  displayTicker: string;
  displayName: string;
  proto: string;
  mappedTo: string | null;
  isTestnet: boolean;
  icon: CoinIcon;
  badgeCoinId: string | null;
  source: 'catalog' | 'fallback';
}

interface CatalogCoin {
  id: string;
  currencyId: string;
  systemId: string;
  displayTicker: string;
  displayName: string;
  proto: string;
  mappedTo: string | null;
  isTestnet: boolean;
  icon: CoinIcon;
}

const catalogCoins = verusCoinCatalog as CatalogCoin[];
const catalogById = new Map<string, CatalogCoin>(catalogCoins.map((coin) => [coin.id, coin]));
const catalogByCurrencyId = new Map<string, CatalogCoin>();
for (const coin of catalogCoins) {
  const key = coin.currencyId.trim().toLowerCase();
  if (!key || catalogByCurrencyId.has(key)) continue;
  catalogByCurrencyId.set(key, coin);
}

function cloneIcon(icon: CoinIcon): CoinIcon {
  if (icon.kind === 'asset') {
    return {
      kind: 'asset',
      light: icon.light,
      dark: icon.dark,
      logoMapped: icon.logoMapped,
      family: icon.family,
      symbolKey: icon.symbolKey,
    };
  }

  if (icon.kind === 'fiat-symbol') {
    return {
      kind: 'fiat-symbol',
      symbol: icon.symbol,
      logoMapped: icon.logoMapped,
      family: icon.family,
      symbolKey: icon.symbolKey,
    };
  }

  return {
    kind: 'generated',
    logoMapped: icon.logoMapped,
    family: icon.family,
    symbolKey: icon.symbolKey,
    seed: icon.seed,
  };
}

function resolveBadgeCoinId(displayTicker: string, displayName: string): string | null {
  const hasVerusBridgeMarker = displayTicker.includes('.vETH') || displayName.includes('on Verus');
  if (hasVerusBridgeMarker && !displayTicker.includes('Bridge.vETH')) {
    return 'VRSC';
  }

  if (displayName.includes('on Ethereum')) {
    return 'ETH';
  }

  return null;
}

function buildGeneratedIcon(seed: string, logoMapped = false): CoinGeneratedIcon {
  return {
    kind: 'generated',
    logoMapped,
    seed,
  };
}

function applyCompatibilityPresentation(presentation: CoinPresentation): CoinPresentation {
  if (
    presentation.id.toUpperCase() === 'GETH' &&
    presentation.proto === 'eth' &&
    presentation.isTestnet
  ) {
    presentation.displayTicker = 'ETH';
    presentation.displayName = 'Sepolia ETH';
  }

  return presentation;
}

function fallbackIconForProto(coinId: string, proto?: string): CoinIcon {
  if (proto === 'btc') {
    const btc = catalogById.get('BTC');
    if (btc?.icon.kind === 'asset') {
      return {
        kind: 'asset',
        light: btc.icon.light,
        dark: btc.icon.dark,
        logoMapped: false,
        family: btc.icon.family,
        symbolKey: btc.icon.symbolKey,
      };
    }
  }

  if (proto === 'erc20' || proto === 'eth') {
    const eth = catalogById.get('ETH');
    if (eth?.icon.kind === 'asset') {
      return {
        kind: 'asset',
        light: eth.icon.light,
        dark: eth.icon.dark,
        logoMapped: false,
        family: eth.icon.family,
        symbolKey: eth.icon.symbolKey,
      };
    }
  }

  if (proto === 'fiat') {
    return {
      kind: 'fiat-symbol',
      symbol: coinId.toUpperCase().startsWith('USD') ? '$' : coinId.slice(0, 1).toUpperCase(),
      logoMapped: false,
    };
  }

  return buildGeneratedIcon(coinId, false);
}

function fromCatalog(coin: CatalogCoin): CoinPresentation {
  return applyCompatibilityPresentation({
    id: coin.id,
    currencyId: coin.currencyId,
    systemId: coin.systemId,
    displayTicker: coin.displayTicker,
    displayName: coin.displayName,
    proto: coin.proto,
    mappedTo: coin.mappedTo,
    isTestnet: coin.isTestnet,
    icon: cloneIcon(coin.icon),
    badgeCoinId: resolveBadgeCoinId(coin.displayTicker, coin.displayName),
    source: 'catalog',
  });
}

function buildFallbackPresentation(
  coinId: string,
  proto: string,
  displayName: string,
  displayTicker: string,
  mappedTo: string | null,
  isTestnet: boolean
): CoinPresentation {
  return applyCompatibilityPresentation({
    id: coinId,
    currencyId: coinId,
    systemId: coinId,
    displayTicker,
    displayName,
    proto,
    mappedTo,
    isTestnet,
    icon: fallbackIconForProto(coinId, proto),
    badgeCoinId: resolveBadgeCoinId(displayTicker, displayName),
    source: 'fallback',
  });
}

export function resolveCoinPresentationById(
  coinId: string,
  proto?: Protocol | 'fiat'
): CoinPresentation | null {
  const normalizedCoinId = coinId.trim();
  const fromCatalogCoin =
    catalogById.get(normalizedCoinId) ?? catalogByCurrencyId.get(normalizedCoinId.toLowerCase());
  if (fromCatalogCoin) {
    return fromCatalog(fromCatalogCoin);
  }

  if (!proto) {
    return null;
  }

  return buildFallbackPresentation(
    normalizedCoinId,
    proto,
    normalizedCoinId,
    normalizedCoinId,
    null,
    false
  );
}

export function resolveCoinPresentation(coin: CoinDefinition): CoinPresentation {
  const fromCatalogCoin = catalogById.get(coin.id);
  if (fromCatalogCoin) {
    const presentation = fromCatalog(fromCatalogCoin);

    // Keep runtime network/mapping authoritative if it differs.
    presentation.proto = coin.proto;
    presentation.mappedTo = coin.mappedTo ?? presentation.mappedTo;
    presentation.isTestnet = coin.isTestnet;
    if (!presentation.displayName) presentation.displayName = coin.displayName;
    if (!presentation.displayTicker) presentation.displayTicker = coin.displayTicker;

    return presentation;
  }

  return buildFallbackPresentation(
    coin.id,
    coin.proto,
    coin.displayName,
    coin.displayTicker,
    coin.mappedTo ?? null,
    coin.isTestnet
  );
}
