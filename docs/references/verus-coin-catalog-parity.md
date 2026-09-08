---
owner: lite-wallet-team
last_reviewed: 2026-02-14
---

# Verus coin catalog parity

This document defines how desktop `lite-wallet` imports coin display metadata
and iconography from `valu-mobile` (`newsend3`) without changing backend
operational coin registry scope.

## Source inputs

- Coin metadata source:
  `/Users/maxtheyse/dev/valu-mobile/src/utils/CoinData/CoinsList.js`
- CoinPaprika override source: `coinsList[*].rate_url_params.coin_paprika` from
  the same `CoinsList.js`
- Coin logo mapping source:
  `/Users/maxtheyse/dev/valu-mobile/src/utils/CoinData/CoinData.js`
- Icon family indices:
  - `/Users/maxtheyse/dev/valu-mobile/src/images/cryptologo/default/btc/index.js`
  - `/Users/maxtheyse/dev/valu-mobile/src/images/cryptologo/default/web3/index.js`
  - `/Users/maxtheyse/dev/valu-mobile/src/images/cryptologo/default/pbaas/index.js`
  - `/Users/maxtheyse/dev/valu-mobile/src/images/cryptologo/default/fiat/index.js`

## Generated desktop artifacts

- Catalog JSON: `src/lib/coins/verusCoinCatalog.generated.json`
- Catalog metadata: `src/lib/coins/verusCoinCatalog.meta.json`
- Copied icon assets: `static/images/coin-logos/**`

Catalog entries may include `coinPaprikaId` when Valu defines
`rate_url_params.coin_paprika`.

## Sync commands

- Regenerate:
  - `pnpm sync:verus-coins`
- Validate drift + parity invariants:
  - `pnpm check:verus-coins`

## Runtime fallback rules

Desktop runtime resolver (`src/lib/coins/presentation.ts`) applies these rules:

1. Prefer direct `CoinLogos` mapping from synced catalog.
2. Unknown `erc20` uses Ethereum icon fallback.
3. Unknown `btc` uses Bitcoin icon fallback.
4. Unknown `vrsc`/PBaaS uses deterministic generated identicon.
5. Badge parity rules:
   - `displayTicker` contains `.vETH` OR `displayName` contains `on Verus`,
     except when `displayTicker` contains `Bridge.vETH` -> badge `VRSC`.
   - `displayName` contains `on Ethereum` -> badge `ETH`.

## Chain scope parity

Chain options in desktop asset details now follow wallet-scoped activated assets
for the active account + network:

1. Additional chains appear only when an asset on that chain is activated.
2. The selected coin's root system is always included.
3. Chain labels prefer native system definitions (`currency_id == system_id`)
   when present, avoiding token-derived labels (for example, no `DAI` label for
   the `vETH` system).

## Expected parity invariants

Current baseline (from `valu-mobile` `newsend3`):

- `87` total coin entries.
- `85` direct logo mappings.
- `2` generated fallback icons.

If these counts change, update:

- `scripts/check-verus-coin-catalog.mjs` expected invariants.
- This document's baseline section.
- Any impacted UI behavior assumptions.
