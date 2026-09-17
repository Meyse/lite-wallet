---
owner: lite-wallet-team
last_reviewed: 2026-09-17
---

# Plan: ETH/ERC20 bridge backend and command surface

- Status: active
- Owner: lite-wallet-team
- Last updated: 2026-09-17

## Goal

Establish a backend-first bridge foundation for desktop parity by defining a
stable bridge command surface and scaffolding ETH bridge modules in Rust,
without UI wiring.

## Constraints

- Keep the existing send trust boundary unchanged: frontend sends preflight
  params, backend owns payload, and send consumes `preflight_id` only.
- Do not couple this phase to UI flows; all work must be callable headlessly via
  Tauri commands.
- Preserve current ETH/ERC20 direct-send behavior and phase-1 stability.
- Avoid partially implemented runtime behavior that can silently route funds;
  unsupported branches must fail deterministically.

## Steps

1. Define bridge command-surface types under `src-tauri/src/types/bridge.rs`:
   - conversion-path request/response shapes
   - bridge preflight request/response shapes
   - execution metadata for backend route selection
2. Introduce backend command handlers under
   `src-tauri/src/commands/bridge_transfer.rs`:
   - `preflight_bridge_transfer`
   - `get_bridge_conversion_paths`
3. Implement phase-1 command routing behavior:
   - route `vrpc.*` bridge preflight through existing advanced VRPC preflight
     path
   - return deterministic not-implemented errors for ETH/ERC20 bridge paths
4. Scaffold ETH bridge module tree under
   `src-tauri/src/core/channels/eth/bridge/`:
   - `delegator`, `token_mapping`, `reserve_transfer`, `fees`, `paths`,
     `preflight`, `send`
   - start with compile-safe stubs and typed interfaces for later implementation
5. Register commands in app surface (`commands/mod.rs`, `lib.rs`) and keep
   invocation contract stable for later UI integration.

## Decisions

- Introduce a dedicated `BridgeNotImplemented` error variant to distinguish
  deferred bridge behavior from generic operation failures.
- Reuse existing advanced VRPC preflight internals as the first executable
  branch for `preflight_bridge_transfer`.
- Keep bridge path discovery command present but explicitly not implemented in
  this phase to lock the API shape early.
- Prefer typed scaffolding over partial execution logic for ETH/ERC20 bridge
  routes until fee/approval/delegator behavior is validated.

## Verification

- `cargo check` in `src-tauri` passes after type/command/module scaffolding.
- New commands compile and are registered in `tauri::generate_handler!`.
- Existing transaction commands and ETH/ERC20 direct-send paths remain
  unchanged.

## Exit criteria

- Backend has a documented and compiled bridge command surface.
- ETH bridge module skeleton exists with clear boundaries for next
  implementation phase.
- VRPC bridge preflight is reachable through the new command surface.
- No UI code is required to exercise the new backend commands.

## Sepolia alignment update (2026-09-17)

The active [Sepolia testnet alignment plan](./sepolia-testnet-alignment.md)
records the later executable bridge behavior. The verified Sepolia delegator is
now bound to chain `11155111`; missing code, wrong-chain RPCs, unknown chain
IDs, and stale Goerli targets fail closed before signing or broadcast. Live
settlement remains separately deferred.
