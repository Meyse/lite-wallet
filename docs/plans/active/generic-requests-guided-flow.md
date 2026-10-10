---
owner: lite-wallet-team
last_reviewed: 2026-10-10
---

# Plan: Requests in the guided flow

- Status: active (phase 0 done)
- Owner: lite-wallet-team
- Last updated: 2026-10-10

## Goal

Answer every request type that Verus Mobile `dev-asf-parent` answers, in the
guided flow defined by the
[requests product spec](../../product-specs/generic-requests.md).

## Constraints

- **Sources of truth:** the spec, the "Requests" section of
  [`CONTEXT.md`](../../../CONTEXT.md), and ADRs
  [0001](../../adr/0001-approval-before-any-action.md),
  [0002](../../adr/0002-request-parsing-boundary.md), and
  [0003](../../adr/0003-mobile-parity-scope.md).
- **Parity reference:** read `upstream/dev-asf-parent` in
  `/Users/maxtheyse/dev/Verus-Mobile` with `git show`. Do not switch that
  checkout's branch. Behaviour that exists only in `redesign` is not a target,
  and known mobile bugs are not copied (GR3-B2, GR3-B5, GR3-B8).
- **Security:** follow the [send-flow](../../context-packs/send-flow.md) and
  [identity-guard](../../context-packs/identity-guard.md) context packs.
  - Every transaction goes through backend preflight and a `preflight_id`.
  - No UI path supplies transaction hex or signing material.
  - Funding never comes from watched addresses.
  - The fee shows only after the user picks the source.
  - Rust re-checks every value it acts on (ADR 0002).
  - No raw signing payloads or secrets in logs.
- **Live tests:** testnet only. The owner approves each broadcast and identity
  registration run at the time. The disposable "mijn app" wallet is used only
  through the [Keychain helper](../../references/test-wallet-keychain.md), and
  no secret screen is automated.
- **Shipping:** each phase ships on its own. A detail type keeps "not supported
  on desktop yet" until its phase ships. That message is not an experimental
  gate.
- **Copy:** English, Dutch, German, and Spanish for every string. Help articles
  in English and Dutch.
- **UI:** desktop first, both themes, checked at the 920×620 minimum window.
- **Dependencies:** library pins are coordinated with the
  [dependency upgrade plan](./verus-dependency-upgrade.md), so both plans pin
  the same commits.

## Steps

### Phase 0: test readiness

1. **Generators (done).** Use Deep Link Sim on `codex/gr3-generators` in
   `/Users/maxtheyse/dev/deeplink-sim-deps-update`. The GR3 work is commit
   `5566227`; the sign-in generators are uncommitted on that branch. Merging and
   pushing are the owner's call. Generator coverage:

   | Request                              | Source                                                 |
   | ------------------------------------ | ------------------------------------------------------ |
   | Sign in alone, with local receiver   | `create:sign-in`                                       |
   | Sign in plus update identity         | `create:identity-update --sign-in`                     |
   | New VerusID                          | `/Users/maxtheyse/dev/verus-fake-provisioning-webhook` |
   | Pay invoice, with or without sign in | Deep Link Sim invoice lab                              |
   | Sign data, Share data, combinations  | Deep Link Sim data lab                                 |
   | Encryption key                       | GR3 request lab                                        |
   | Gift card claim                      | Add in phase 5                                         |
   | Legacy sign in and invoice links     | Add in phase 6                                         |

2. **Responder pre-check (done).** Through the Keychain helper, unlock "mijn
   app" and confirm four things:
   - a linked testnet VerusID
   - testnet funds
   - private addresses set up
   - an identity whose private address comes from the wallet's seed (needed for
     credentials)

   Report any gap to the owner instead of assuming.

   Done on 2026-10-10; see the
   [runbook](../../references/request-testing.md#test-wallet). The wallet has
   testnet funds and six linked VerusIDs that it controls. Private Verus is now
   set up with a separate privacy secret, and `player7@` has the wallet's
   shielded address.

3. **Real links (done).** The runbook is
   [`request-testing.md`](../../references/request-testing.md). Daily work
   pastes links. Real `verus://` links need a bundled debug build
   (`pnpm tauri build --debug`), launched once so macOS registers it.

### Phase 1: library versions, shell, existing details, parity fixes

1. **Library pins.** Pin the `dev-asf-parent` commits:
   - `verus-typescript-primitives` `3ffa9d77`
   - `verusid-ts-client` `933e8f8d`
   - `verusd-rpc-ts-client` `e8ea8c21`

   Rerun the generic-request tests and the signature parity vectors.

2. **Acceptance in Rust** (`src-tauri/src/commands/generic_request.rs`,
   `src-tauri/src/core/crypto/verus_id_signature.rs`):
   - Replace the ordinal whitelist and the app grouping list with the library's
     ordering rules.
   - Accept fully qualified compact addresses.
   - Add the delegated-signer rule and the unsigned rule from the spec.
   - Enforce the sign-in expiry when the request opens, before signing, and
     before delivery.
   - Reject links of 2046 characters or more.
3. **Parsing boundary (ADR 0002).** Rust re-derives the requested identity, the
   target, the expiry, and the request IDs from the verified request bytes.
   Today the UI supplies them (`generic_request.rs:1851-1869`,
   `src-tauri/src/types/generic_request.rs:55-61`).
4. **Response.**
   - Set request ID, request hash, creation time, handled-by, and the testnet
     flag as mobile does.
   - Apply the sign-in echo rule.
   - Lock one responder for all details.
5. **Delivery.**
   - HTTPS only in release builds; plain HTTP only in debug builds.
   - A 30-second timeout on the response POST (`generic_request.rs:1366-1386`)
     and on the provisioning call
     (`src/lib/genericRequest/provisioning.ts:120-131`).
   - Return a result per stage so each stage can retry alone.
6. **Intake.**
   - Replace the plain-text `localStorage` queue
     (`src/lib/stores/genericRequest.ts`) with one in-memory request, and clear
     the old `lite_wallet_generic_request_queue_v1` key once.
   - Apply the second-link rule.
   - Accept the raw QR text string in paste
     (`src/lib/genericRequest/session.ts`).
7. **Shell.** Split `GenericRequestFlowHost.svelte` (2,299 lines) into a shell
   and one module per detail type.
   - The shell owns the layout rule, the rail (the `StepperWithAsideLayout`
     aside), and the requester header (`IdentityMention`, website-connection
     check, one expiry format).
   - It also owns the responder picker, Confirm, the stages, Done, the failure
     states, and Decline.
   - Each module provides a label, an icon, an impact, a plan line, a body, a
     ready check, a commit, and receipt lines.
8. **Approval and stages (ADR 0001).**
   - Move the identity-update broadcast from the funding step to after Approval.
   - Run the transaction, then signing, then delivery, and retry only the failed
     stage.
   - Pause auto-lock while the stages run
     (`src-tauri/src/core/auth/expiry_monitor.rs`).
   - Keep the request in memory across a lock before Approval
     (`src/lib/services/walletLockCoordinator.ts`).
9. **Existing details.** Move Sign in, Update identity (a Changes step and a
   Network fee step), and New VerusID (keeping "Link and continue") onto the
   shell. Classify a private address change as high risk, as mobile does
   ("Change private address" in
   `IdentityUpdateRequestInfo/utils/classifyChanges.js`). Desktop now counts it
   as a content change (found on 2026-10-10).
10. **Copy.**
    - Map Rust warning codes to translated copy.
    - Add the strings in all four languages.
    - Add a test that fails when a request string is missing in any language.
    - Update the Help "Requests" article through the
      [help-copy workflow](../../references/help-center.md).
11. **Fixture.** Add `dev/browser-fixtures/requests.html` with synthetic states:
    single screen, rail, high risk, network fee, each stage, failed delivery,
    declined, and expired.

### Phase 2: pay invoice

1. **Rust.**
   - Re-check the invoice fields from the verified bytes.
   - Add a strict-amount preflight that adds the fee instead of lowering the
     amount (`src-tauri/src/core/channels/vrpc/transfer.rs:85-106, 508-527`).
   - Add the rejections from the spec.
   - Guard against paying the same request twice in one session.
2. **Module.** Amount, source picker, fee after the choice, and total. Sign in
   plus invoice, and the "Not signed" header for unsigned invoices.
3. **Second release: conversion.** Add converter discovery (desktop has no
   `getcurrencyconverters` call) and a quote for an exact received amount.
4. **Help and spec status.** Update both.

### Phase 3: sign data

1. **Rust command.** It builds each item's and each statement's hash from the
   verified request. It then signs the height-bound identity hash, reusing
   `verus_id_signature.rs:262-315`. It never accepts a hash from the UI.
2. **Module.** Per-item consent, with transmittal rejected, and the data
   response as mobile builds it.
3. **Help and spec status.** Update both.

### Phase 4: private-address crypto, encryption key, credentials

1. **Key derivation in Rust.** Implement the channel-key derivation from the
   extended spending key on the existing upstream crates. Promote
   `zcash_note_encryption` to a normal dependency.
2. **Parity gate.** Add a vector mode to Deep Link Sim's encryptor helper: seed,
   from-ID, to-ID, and index in; address, IVK, and viewing key out. Cross-check
   it once against the daemon's `z_getencryptionaddress`. Add decrypt vectors
   for encryption. Nothing in this phase ships before both pass. The IVK is in
   display order.
3. **Response encryption.** Encrypt the whole response to the address the
   request names.
4. **Encryption key module.**
   - The responder picker sits on this step, and no sign-in response is sent.
   - A requested spending key gets a strong warning.
   - Without private addresses, show "Set up private addresses first".
5. **Credential writing in Update identity.**
   - Check that the identity's private address comes from the wallet's seed.
   - Encrypt each entry before preflight.
   - Store each entry under the hashed credential key.
6. **Share data module.**
   - Read and decrypt the responder's on-chain content.
   - Filter by requested key and scope, and enforce a required signer.
   - Give partial answers, and make lookup failures retryable.
7. **Help and spec status.** Update both.

### Phase 5: gift card claim

1. **Design.** Read mobile's gift card handler and screen at `dev-asf-parent`,
   and design the step.
2. **Rust.** Sweep the funds from the key the request carries, including
   password-protected claims.
3. **Storage.** Save the request encrypted in Stronghold until the claim
   finishes, and add it to the
   [storage policy](../../references/storage-policy.md).

### Phase 6: legacy adapters

1. **Legacy sign in.** A request with only the identity-view permission goes to
   the Sign in module. Its response follows the legacy delivery rules.
2. **Legacy invoices.** v3 and v4 links go to the Pay invoice module, with
   mobile's checks: payable, signature, expiry, network, and burn-change price.

## Decisions

- The guided flow, variation B, is the chosen presentation.
- **Parity:** protocol and behaviour follow `dev-asf-parent`; the flow is ours
  (ADR 0003).
- **Deliberate deviations:** no experimental setting, and wallet backups are
  rejected.
- **Approval:** nothing happens before it. After it, the stages run in order
  with a retry per stage (ADR 0001).
- **Storage:** requests stay in memory, one at a time, and nothing is stored
  after Approval, as on mobile.
- **Parsing:** TypeScript parses; Rust re-checks what it acts on (ADR 0002).
- **Credentials:** they live on-chain, encrypted to the identity. There is no
  local record store and no credential viewer.
- **Intake:** there is no QR image intake; websites offer a copy-link option.

## Verification

- **Each phase:** `pnpm check`, `pnpm lint`, `pnpm format:check`, `pnpm test`,
  `pnpm test:mounted`, `pnpm docs:check`, and `cargo test`. `cargo test` needs
  unrestricted loopback and the developer directory noted in the
  [dependency upgrade plan](./verus-dependency-upgrade.md).
- **Unit and Rust tests:**
  - ordering rules, the unsigned rule, expiry at each point, delegated signers,
    fully qualified addresses, and the 2046 limit
  - response fields, the sign-in echo rule, and no second broadcast on retry
  - strict invoice amounts, per-item signatures, derivation and decrypt vectors,
    and the credential scope filter
- **Fixture:** every state at 920×620 in light and dark.
- **Native testnet acceptance** (paste path), against Deep Link Sim receivers
  and the provisioning webhook:
  - every detail type and the allowed combinations
  - the delivery modes `post`, `fail-once`, `fail-always`, `redirect`, and
    `none`
  - Decline, expiry, and a lock before Approval
- **Real links:** one `verus://` link per phase through the bundled debug build.

## Exit criteria

- Every detail type in the spec works on testnet against its receiver, with the
  evidence recorded per phase.
- Wallet backups are rejected with the agreed message, and no experimental
  setting exists.
- The spec status, Help, and indexes match what shipped, and the
  locale-completeness test passes.
- Mainnet acceptance runs are recorded as the remaining follow-up.
