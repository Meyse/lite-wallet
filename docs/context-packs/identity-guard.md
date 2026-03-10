---
owner: lite-wallet-team
last_reviewed: 2026-02-12
---

# Context pack: identity and guard

Read this before changing identity update/revoke/recover or guard-session
behavior.

## Invariants

- Guard sessions are in-memory only and must be clearable.
- Identity preflight/send follows `preflight_id` trust boundary.
- Generic identity update request review must stay separate from funding: do not
  calculate or display the fee until the user selects a funding source.
- Account/guard ownership checks are required before send.
- Identity signing metadata in preflight must cover all signable inputs, including
  the identity input.
- Warnings and risk summaries must remain user-safe.
- Guard revoke/recover preflight must return `InsufficientFunds` when the authority
  address cannot cover fees, instead of generic identity-build errors.
- Recover/revoke success is defined by successful broadcast (txid). Identity status
  in `getidentity` can lag until confirmation.

## Open these files first

- `src/lib/services/identityService.ts`
- `src/lib/services/guardService.ts`
- `src/lib/components/wallet/VerusIdGuardDock.svelte`
- `src/lib/components/wallet/VerusIdGuardSheet.svelte`
- `src/lib/components/flows/VerusIdGuard/GuardFlowHost.svelte`
- `src/lib/components/flows/VerusIdGuard/GuardSecretStep.svelte`
- `src/lib/components/flows/VerusIdGuard/GuardTargetStep.svelte`
- `src/lib/components/flows/VerusIdGuard/GuardRecoverPatchStep.svelte`
- `src/lib/components/flows/VerusIdGuard/GuardReviewStep.svelte`
- `src/lib/components/flows/VerusIdGuard/GuardResultStep.svelte`
- `src-tauri/src/commands/identity.rs`
- `src-tauri/src/commands/guard.rs`
- `src-tauri/src/core/auth/guard_session.rs`
- `src-tauri/src/core/channels/vrpc/identity/preflight.rs`
- `src-tauri/src/core/channels/vrpc/identity/send.rs`
- `src-tauri/src/core/channels/vrpc/identity/verus_tx/`
- `src-tauri/src/types/identity.rs`
- `src-tauri/src/types/guard.rs`

## Verification checklist

- Confirm guard session start/end path still zeroizes and clears correctly.
- Confirm preflight ownership mismatch returns safe errors.
- Confirm flow still works for both mainnet and testnet.
- Confirm signed-out dock is visible on both `WelcomeScreen` and `UnlockScreen`.
- Confirm recover advanced patch fields are optional and validation-safe.
- For recover scenarios, confirm broadcast tx output includes expected
  `identityprimary` patch fields when immediate `getidentity` state has not yet
  advanced.
