# Backend agent map (`src-tauri`)

This file applies to backend work under `src-tauri/`.

## Core backend rules

- Preserve the trust boundary: UI never supplies tx hex or signing data for send
  flows.
- Send operations must execute by backend-owned `preflight_id`.
- Keep preflight records session-scoped, single-use, and clear-on-lock.
- Do not log secret material (seed, WIF, private keys, raw signing payloads).
- Expose safe, user-facing errors through typed `WalletError` mapping.
- Validate inputs at boundaries.
- Fail early on invalid state.
- Favor deterministic code paths for critical logic.
- Keep error messages actionable.
- Keep logs concise, structured, and relevant.
- Avoid broad catch-all exception handling unless necessary.
- Add tests for high-risk paths and edge cases.

## Start points by task

- Command entry points: `src-tauri/src/commands`
- Session and guard state: `src-tauri/src/core/auth`
- Channel routing and preflight store: `src-tauri/src/core/channels/mod.rs`,
  `src-tauri/src/core/channels/store.rs`
- VRPC and BTC send/preflight: `src-tauri/src/core/channels/vrpc`,
  `src-tauri/src/core/channels/btc`
- Type boundaries: `src-tauri/src/types`

## Related docs

- Docs index: [`../docs/index.md`](../docs/index.md)
- Architecture map:
  [`../docs/architecture/index.md`](../docs/architecture/index.md)
- Send flow context pack:
  [`../docs/context-packs/send-flow.md`](../docs/context-packs/send-flow.md)
- Identity + guard context pack:
  [`../docs/context-packs/identity-guard.md`](../docs/context-packs/identity-guard.md)
