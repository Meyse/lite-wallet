---
owner: lite-wallet-team
last_reviewed: 2026-10-11
---

# Plan: Requests in the guided flow

- Status: active (phase 0 done; phases 1 to 6 in progress)
- Owner: lite-wallet-team
- Last updated: 2026-10-11

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

**Progress on 2026-10-10.**

- **Done:**
  - Step 7, without the module contract: the shell has the variation B layout.
    The per-detail logic still lives in the shell. The module contract comes
    with Pay invoice in phase 2, when a third detail type tests it.
  - Step 8, without the auto-lock pause and the lock handling: the update is
    broadcast after Approval, and only a failed stage retries.
  - Step 9: Sign in, Update identity (Changes and Network fee), New VerusID, and
    the private address risk class.
  - Step 10, in part: the new copy in four languages.
  - Step 11: the `requests.html` fixture.
- **Verified:**
  - Unit tests for the flow rules.
  - A mounted test: nothing is broadcast before Approval, a delivery retry never
    broadcasts again, and Decline sends nothing.
  - Native testnet runs: sign in alone, and sign in plus update
    (`78e2cabf86df99cfa44f3fd48cb48337a07a84ff64ea80364eed5ea7e68b2b1c`). Deep
    Link Sim verified both responses.
- **Not done:** steps 1 to 6, the module contract, the auto-lock pause, keeping
  a request across a lock, warning-code copy, the locale test, and Help.
- **Found:**
  - Desktop echoes the sign-in detail ID; mobile echoes the envelope ID (step
    4).
  - The update builder returns `IdentityBuildFailed` for `namespace::name`
    content keys, which mobile accepts (step 3).
  - The Changes step shows content values as raw JSON.
  - "Connected to" stays hidden: no website-connection check exists yet.

**Progress on 2026-10-11.**

- **Done:**
  - Step 1: the four libraries use the commits that mobile locks, including
    `@bitgo/utxo-lib` `2d324042`. See the
    [dependency plan](./verus-dependency-upgrade.md#pin-update-on-2026-10-11).
  - Step 2: `src-tauri/src/core/generic_request` parses every request, signed or
    unsigned, and applies the library's order rules, the unsigned rule, the
    network rule (flag and signature system), the delegated-signer rule, and
    expiry when the request opens. Signer names resolve as the library resolves
    them.
  - Step 3, for Sign in and Update identity: Rust holds the one open request. It
    reads the request IDs, the target, the expiry, and every control field of
    the update from the verified bytes. The UI's identity JSON supplies only
    content values under the keys that the request names.
  - Step 4: the response carries the request ID, request hash, creation time,
    handled-by `0`, and the testnet flag. The sign-in response echoes the
    request ID when present. Rust builds it byte for byte as mobile does (vector
    test).
  - Step 5: delivery goes only to a response address the request names. POST
    needs HTTPS in release builds and times out after 30 seconds.
  - Step 6, in part: the waiting request is in memory only, and the old
    `localStorage` key is removed once. Paste accepts the raw QR text. Links of
    2046 characters or more are refused. A second link replaces the request
    before Approval and is ignored after it.
  - The unused `sign_generic_response` command, which signed any response the UI
    passed, is removed.
- **Fixed:** `namespace::name` content keys. The daemon resolves them
  (`getvdxfid`), and the library's own hash differs from the daemon's. The
  Changes step shows the key label and the text of a string value.
- **Verified on testnet, 2026-10-11:** sign in alone, and sign in plus a
  `namespace::name` content update
  (`68ad35c6c9241375b5f90381071a22bd4fd77f3e4ee9884622f8d82ac17c751c`). Deep
  Link Sim verified both responses, with the envelope ID echoed. All six
  negative cases are refused with their own message.
- **New VerusID parity, 2026-10-11:**
  - The picker offers New VerusID next to the linked VerusIDs.
  - A request without a name lets the user choose a free one.
  - The challenge ID is the request ID first, then the sign-in detail's ID.
  - Rust sends the provisioning call (`post_provisioning_request`). It accepts
    only the request's provisioning address and stops after 30 seconds. The
    webview `fetch` sent only the CORS preflight.
  - Fixed: the provisioning signature is now the bare 65-byte compact signature
    that `verifyVerusIdProvisioningRequest` reads. Before, the wallet sent a
    serialized identity signature, and the service refused it.
- **Verified on testnet, 2026-10-11:** the fake provisioning webhook accepted
  the signed request for `lwqa1011a.gamesession5@`. The wallet verified the
  signed response and saved the pending job. The service's name commitment
  confirmed
  (`c8d3616ef240ca83823c78b369823357d505bb9c66c79ad0bc3b3f2ab30821fb`). The
  service could not register the name: a subID registration under a currency
  needs a reserve transfer, and testnet has DeFi functions disabled. Linking a
  ready provisioned VerusID was not verified.
- **Copy, 2026-10-11:** `src/lib/i18n/requestLocales.test.ts` fails when a
  request string is untranslated, extra, or loses a placeholder in Dutch,
  German, or Spanish. It found 117 English request strings in each language
  (older Sign in, Update identity, error, and provisioning copy). They are now
  translated. 21 unused request keys are removed.
- **Warning copy, 2026-10-11:** the Changes step shows translated copy for each
  Rust warning code (`src/lib/genericRequest/warnings.ts`), not Rust's English
  message. An unknown code shows a general caution.
- **Auto-lock pause, 2026-10-11:** the host runs the stages inside
  `pause_auto_lock_for_request` and `resume_auto_lock`. Rust holds the
  inactivity timer only while a request is open, for at most five minutes. The
  end of the hold counts as activity. A manual lock still locks. Verified with a
  Deep Link Sim sign in on testnet (verified once).
- **Lock before Approval, 2026-10-11:** when a lock unmounts the wallet with a
  request open and not approved, the request waits in memory (source `lock`) and
  opens again after unlock. Rust checks it again. After Approval it is dropped.
  Verified natively: a Deep Link Sim sign in survived a simulated auto-lock and
  was verified after unlock.
- **Not done yet:** Help.

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

**Progress on 2026-10-11.**

- **Done:**
  - Rust reads the invoice from the verified request: amount, destination,
    currency, expiry, and flags. It refuses tagged invoices, private-address
    destinations, invoices that exclude the Verus chain, missing destinations,
    unknown currencies, expired invoices, preconvert invoices after launch, and
    invoices for the other network. Burn invoices and preconvert before launch
    wait for the conversion release.
  - The exact-amount preflight adds the fee on top and fails rather than
    lowering the amount. The payment command checks that the prepared
    transaction pays the invoice's currency, amount, and destination, and pays
    one request once per wallet session.
  - The module contract: `modules/types.ts`, with Pay invoice as the first
    module. Sign in plus invoice, invoice alone (no response, as on mobile), and
    invoices that accept any amount.
  - **Found and fixed (all sends):** funding ignored outputs that an unconfirmed
    transaction already spends. A second identical payment before the first
    confirmed produced the same transaction ID, and the wallet reported a
    payment it did not make. `funding_utxos` now skips mempool spends.
- **Verified on testnet, 2026-10-11:** sign in plus a 0.01 VRSCTEST invoice to
  `TestBob.antafri@`
  (`0600eba8a139873d6c3dba0a2dd6ebb8589ffb44a5ac9386102c123b776773f7`, Deep Link
  Sim received the response), an invoice alone to an R-address
  (`b639110edeb442d72249b8629a485faf2fb63b83a848168a84e6433d0a1a5d2c`), and two
  identical invoices paid back to back as two transactions (`652d7aa0…db35a9`,
  `57580d48…dfd02b`). An invoice opened while the wallet's only output was
  unconfirmed showed the funds message and paid nothing.
- **Not done:** conversion (second release), Help, and the spec status.

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

**Progress on 2026-10-11.**

- **Done:** Rust reads each object and statement from the verified data packet,
  refuses transmittal packets and packets that ask for no signature, hashes each
  signed item with SHA-256, and signs the height-bound identity hash of each
  one. The data response is byte for byte what mobile builds (vector test), and
  each signature verifies for the responder (unit test). The Sign data module
  asks for consent per item and per statement; a data packet alone is a single
  screen with the responder picker. One data packet per request is shown; a
  second one is refused as not supported yet.
- **Verified on testnet, 2026-10-11:** sign in plus a data packet, and a data
  packet alone, from Deep Link Sim's data lab. Deep Link Sim verified four
  signatures each time.
- **Not done:** Help and the spec status.

1. **Rust command.** It builds each item's and each statement's hash from the
   verified request. It then signs the height-bound identity hash, reusing
   `verus_id_signature.rs:262-315`. It never accepts a hash from the UI.
2. **Module.** Per-item consent, with transmittal rejected, and the data
   response as mobile builds it.
3. **Help and spec status.** Update both.

### Phase 4: private-address crypto, encryption key, credentials

**Progress on 2026-10-11.**

- **Done:**
  - `core/generic_request/private_crypto.rs` ports `verus_zfunc`'s channel keys,
    encryption, and decryption to the wallet's Sapling crates. Its vectors come
    from `verus_zfunc` itself for a fixed public test seed, instead of a Deep
    Link Sim helper mode. The daemon's `z_getencryptionaddress` gave the same
    addresses, and its IVK is the curve's own byte order.
  - Encryption key: Rust derives the keys from the private account, encrypts a
    detail to the address it names, and encrypts the whole response to the
    request's address. The response bytes match mobile's (vector test).
  - Credentials written by an update: Rust encrypts each credential to the
    identity's credential address before the preflight reaches the daemon, and
    stores it under the hashed key. The key matches mobile's `getConditionID`
    and the daemon's `getvdxfid` with the same viewing key.
  - Share data: Rust reads the responder's content under that key, decrypts it,
    and keeps the valid credentials whose key was requested and whose scopes
    include the app. Each credential has its own consent. The response bytes
    match mobile's (vector test). Sign data inside an update is refused as not
    supported yet.
- **Verified on testnet, 2026-10-11:**
  - Encryption key, encrypted and plain: Deep Link Sim decrypted and verified
    both.
  - Credentials written by an update to a new disposable identity,
    `dlscredqa1@`: txid
    `ea9e4cf13280d48a3b6bd015bc47098f21d4d1d383c1b316f9ef5c3ba8254e0c`. The
    chain holds only encrypted descriptors under the hashed key.
  - Sign in plus a credential request with no match: Deep Link Sim verified an
    encrypted response with zero credentials and the missing key.
- **Not done:** sharing the stored credentials, which waits for the update to be
  mined.

1. **Key derivation in Rust.** Implement the channel-key derivation from the
   extended spending key on the existing upstream crates. Promote
   `zcash_note_encryption` to a normal dependency.
2. **Parity gate.** Add a vector mode to Deep Link Sim's encryptor helper: seed,
   from-ID, to-ID, and index in; address, IVK, and viewing key out. Cross-check
   it once against the daemon's `z_getencryptionaddress`. Add decrypt vectors
   for encryption. Nothing in this phase ships before both pass. The IVK is in
   the curve's own byte order, as the daemon and `react-native-verus` return it
   (corrected on 2026-10-11; this step first said display order).
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

**Progress on 2026-10-11.**

- **Done:**
  - `core/generic_request/gift_card.rs` reads the spendable key detail and
    decrypts a password card (PBKDF2-HMAC-SHA256, AES-256-GCM: salt, IV,
    ciphertext, tag). The 24 words become the key as Verus Mobile derives it
    (`derive_keys_v1`). Vectors come from the library and Node's crypto.
  - `core/channels/vrpc/claim.rs` builds the claim. Each VerusID the key
    controls moves to the wallet's primary address, paid from a plain native
    output of the key, with the change to the wallet. A sweep then moves every
    other output, every currency included, to the wallet; a check refuses any
    output that pays anyone else. The claim key signs; a retry broadcasts the
    same transactions.
  - The Gift card module asks for the password, shows what the card holds, warns
    about kept authorities, and blocks a VerusID it cannot move.
- **Verified on testnet, 2026-10-11:** a password card built like mobile's,
  holding 0.06 VRSCTEST in two outputs, 1 AUD, and a disposable VerusID
  `dlsgiftqa1@`. Deep Link Sim has no gift card generator, so a scratch script
  built it. The claim moved the VerusID to the wallet (txid
  `b4ac47621c1cadf9dc14b6dc4dfea125bf39ded15ed3666865dce34864b2875c`) and swept
  0.0298 VRSCTEST and 1 AUD to it (txid
  `a41471025603019efd6542b71252cb9df3317a50f9a1021b3da438450be63c32`).
- **Not done:**
  - Saving the request encrypted in Stronghold until the claim finishes (step
    3). Opening the card again claims what is left.
  - Claims on PBaaS chains other than the wallet's network.
  - Paying an identity fee from the VerusID's own funds when the key holds no
    plain native output, as mobile can.

1. **Design.** Read mobile's gift card handler and screen at `dev-asf-parent`,
   and design the step.
2. **Rust.** Sweep the funds from the key the request carries, including
   password-protected claims.
3. **Storage.** Save the request encrypted in Stronghold until the claim
   finishes, and add it to the
   [storage policy](../../references/storage-policy.md).

### Phase 6: legacy adapters

**Progress on 2026-10-11.**

- **Done:** `core/generic_request/legacy.rs` reads legacy links
  (`<wallet scheme>://x-callback-url/...`). A login consent request is checked
  as mobile checks it: identity view only, no context, the open wallet's
  network, and the requester's signature over its challenge. Its response is
  byte for byte what mobile builds (vector test) and goes to the redirect
  address first, else to the webhook as JSON. A v3 or v4 invoice is held as a
  request's invoice detail, so the invoice commands pay it; a signed invoice
  must verify. The wallet registers the legacy scheme for OS links.
- **Verified on testnet, 2026-10-11:** links built with the libraries and signed
  by the daemon's `signdata`, since Deep Link Sim has no legacy generator.
  - Legacy login answered by webhook as `player7@`: the receiver rebuilt the
    response from the JSON and the daemon verified the signature.
  - v3 invoice, unsigned, 0.01 VRSCTEST: txid
    `a8f3acd53d0fea8687708427ad82b2e5423084089526280942a4bd48dab44d68`.
  - v4 invoice, signed, 0.011 VRSCTEST: txid
    `f4d447d22fec241ee2da38c1e55f79cfb2425d4be8320c769c7dae6eb2f78485`.
- **Not done:** a legacy login that offers provisioning instead of a redirect
  address (mobile's "Request VerusID") is answered as a plain sign in.

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
