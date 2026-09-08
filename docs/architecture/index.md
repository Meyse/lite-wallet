---
owner: lite-wallet-team
last_reviewed: 2026-02-12
---

# Architecture map

Read this when you need cross-layer behavior, trust boundaries, or lifecycle
details.

## Layer map

- UI routes and composition: `src/routes`, `src/lib/components`
- Frontend state and orchestration: `src/lib/stores`, `src/lib/machines`,
  `src/lib/services`
- Tauri command boundary: `src-tauri/src/commands`
- Core wallet logic: `src-tauri/src/core`
- Type contracts: `src/lib/types/wallet.ts`, `src-tauri/src/types`

## Core invariants

- UI never sends tx hex or signing data for send flows.
- Send commands consume backend-owned preflight payload by `preflight_id` only.
- Sensitive signing material stays inside backend session/guard managers.
- User-facing errors must be safe and generic across the trust boundary.

## Key code entry points

- Wallet page startup and event bridge setup: `src/routes/wallet/+page.svelte`
- Send flow orchestration:
  `src/lib/components/wallet/sections/TransferWizard.svelte`
- Event bridge: `src/lib/services/eventBridge.ts`
- Transaction commands: `src-tauri/src/commands/transaction.rs`
- Preflight store: `src-tauri/src/core/channels/store.rs`
- Update engine: `src-tauri/src/core/updates/engine.rs`
- Fiat rates domain: `src-tauri/src/core/rates`

## Related docs

- Product specs: [`../product-specs/index.md`](../product-specs/index.md)
- Context packs: [`../context-packs/index.md`](../context-packs/index.md)
- Plans: [`../plans/index.md`](../plans/index.md)
- Verus identity signing architecture:
  [`./verus-identity-signing-rust.md`](./verus-identity-signing-rust.md)
