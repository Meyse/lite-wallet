---
owner: lite-wallet-team
last_reviewed: 2026-02-21
---

# Context pack: send flow

Read this before touching transaction send behavior.

## Invariants

- UI sends preflight params only.
- Send uses `preflight_id` only.
- Preflight payload is backend-owned and session-scoped.
- Preflight records are single-use and must expire/clear correctly.
- ETH preflight enforces a minimum gas floor of `1 gwei` and may adjust send
  value downward when balance cannot cover `amount + fee`.
- ERC20 preflight must enforce both token balance and ETH-fee balance checks
  before send.
- ETH/ERC20 send must reject missing/expired preflight and consume preflight
  records exactly once.
- dlight private preflight supports destination policy `zs | R | i` only.
- dlight private `i...` recipient handling resolves to primary `R...`
  server-side before send.
- dlight private preflight uses fixed fee `0.0001` in source coin units.
- dlight private preflight and send must return `DlightSynchronizerNotReady`
  while private sync is incomplete.
- dlight private memo is accepted only for shielded (`zs`) destinations.
- dlight private send must fail safely when Sapling proving params are missing
  or checksum-invalid.

## Open these files first

- `src/lib/components/wallet/sections/Send.svelte`
- `src/lib/components/wallet/sections/TransferWizard.svelte`
- `src/lib/components/wallet/sections/transfer-wizard`
- `src/lib/services/txService.ts`
- `src-tauri/src/commands/transaction.rs`
- `src-tauri/src/core/channels/mod.rs`
- `src-tauri/src/core/channels/store.rs`
- `src-tauri/src/core/channels/vrpc/preflight.rs`
- `src-tauri/src/core/channels/vrpc/send.rs`
- `src-tauri/src/core/channels/btc/preflight.rs`
- `src-tauri/src/core/channels/btc/send.rs`
- `src-tauri/src/core/channels/eth/preflight.rs`
- `src-tauri/src/core/channels/eth/send.rs`
- `src-tauri/src/core/channels/eth/transactions.rs`
- `src-tauri/src/core/channels/dlight_private/preflight.rs`
- `src-tauri/src/core/channels/dlight_private/send.rs`
- `src-tauri/src/core/channels/dlight_private/destination.rs`
- `src-tauri/src/core/channels/dlight_private/spend_engine.rs`
- `src-tauri/src/core/channels/dlight_private/spend_sync.rs`
- `src-tauri/src/core/channels/dlight_private/spend_keys.rs`
- `src-tauri/src/core/channels/dlight_private/spend_params.rs`
- `src-tauri/src/core/channels/dlight_private/recipient_resolution.rs`

## Verification checklist

- Confirm flow transitions still cover
  `idle -> preflighting -> confirming -> sending -> success|error`.
- Confirm no UI pathway can submit tx hex/signing material.
- Confirm lock still clears preflight state.
- Confirm user-facing errors remain safe.
- Confirm ETH/ERC20 route IDs use canonical prefixes: `eth.<coinId>` and
  `erc20.<coinId>`.
- Confirm ETH/ERC20 phase-1 scope is core wallet behavior only (no bridge
  convert/cross-chain or add-token persistence UX).
