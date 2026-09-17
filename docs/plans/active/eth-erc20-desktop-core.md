---
owner: lite-wallet-team
last_reviewed: 2026-09-17
---

# Plan: ETH and ERC20 desktop core

- Status: active
- Owner: lite-wallet-team
- Last updated: 2026-09-17

## Goal

Deliver desktop ETH and ERC20 core parity with `valu-mobile` for balances,
history, preflight, and send on both mainnet and testnet.

## Constraints

- Runtime secrets come from environment variables (`INFURA_PROJECT_ID`,
  `ETHERSCAN_API_KEY`), with desktop debug builds loading local
  `.env`/`.env.local` files into process env.
- Keep frontend/backend trust boundary unchanged: frontend submits preflight
  params and later `preflight_id` only.
- Preserve single-use, session-scoped preflight semantics.
- Phase-1 excludes bridge convert/cross-chain and add-token persistence UX.

## Implemented scope (phase-1)

1. Added Rust-native ETH channel under `src-tauri/src/core/channels/eth/`:
   `provider`, `config`, `balance`, `transactions`, `preflight`, `send`.
2. Added provider/config bootstrap in `src-tauri/src/lib.rs` with startup
   diagnostics for disabled ETH providers.
3. Routed `eth.<coinId>` and `erc20.<coinId>` through channel router for:
   preflight, send, balances, and transaction history.
4. Added coin registry support: `find_by_id(...)`, ETH zero-address metadata
   parity, and testnet `GETH`.
5. Extended update engine polling to include ETH/ERC20 channels when ETH runtime
   config is enabled.
6. Updated frontend channel ID mapping, wallet channel selection, send coin
   filtering, and receive screen ETH address row.
7. Added i18n keys for ETH/GETH receive labels in English and Dutch locales.

## Runtime config

- Required:
  - `INFURA_PROJECT_ID`
  - `ETHERSCAN_API_KEY`
- Optional overrides:
  - `ETH_MAINNET_RPC_URL`
  - `ETH_TESTNET_RPC_URL`
  - `ETHERSCAN_MAINNET_URL`
  - `ETHERSCAN_TESTNET_URL`
- Startup behavior:
  - If required env vars are missing or invalid, ETH providers are disabled.
  - Release builds do not auto-load local `.env*` files.
  - ETH/ERC20 routes return deterministic `EthNotConfigured` errors without
    panics.

## Decisions

- Use Rust-native Ethereum integration (`ethers` crate) rather than a JS sidecar
  to preserve backend signing boundary and avoid extra runtime complexity.
- Keep Sepolia as default ETH testnet target, with env override support.
- Use Etherscan for normalized ETH/ERC20 history parity and Infura-compatible
  RPC for balances/fees/send.

## Verification

- `cargo check` in `src-tauri` passes.
- `yarn check` passes.
- Added unit coverage for:
  - ETH/ ERC20 channel id parsing helpers.
  - ETH fee-from-amount adjustment logic.
  - ETH invalid destination parsing.
  - ERC20 fee-drift cap guard.
  - Update engine channel activation with ETH enabled/disabled and testnet
    filtering.

## Deferred (phase-2)

1. Bridge convert and cross-chain parity (delegator contract,
   map-to/via/export-to behavior, approval edge cases).
2. Add-token ERC20 UX and persistent contract definition lifecycle.
3. User-configurable minimum gas floor in desktop settings (currently fixed to
   mobile default parity, 1 gwei).

## Sepolia alignment update (2026-09-17)

The active [Sepolia testnet alignment plan](./sepolia-testnet-alignment.md)
supersedes the historical `GETH` display wording while retaining that value as
an internal compatibility ID. ETH/ERC20 reads and execution now validate live
chain identity, testnet ERC20 discovery is scoped to Sepolia, and visible native
asset labels use `ETH` / `Sepolia ETH`.
