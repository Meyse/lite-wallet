---
owner: lite-wallet-team
last_reviewed: 2026-09-21
---

# Plan: VerusID profile publishing UX corrections

- Status: complete; local verification and native macOS VRSCTEST acceptance
  passed
- Owner: lite-wallet-team
- Last updated: 2026-09-21
- Product contract:
  [UX specification](../../product-specs/verusid-profile-publishing-ux.md)
- Origin: native macOS VRSCTEST audit, 2026-09-21

## Goal

Resolve all six audited issues so a user can visually edit a profile, approve
each fee and reliably reach the confirmed result with minimal reading.

## Scope and working constraints

Implementation was authorized in a separate isolated task on 2026-09-21. The
implementation task owns local corrections and verification; the originating
task owns fresh native VRSCTEST acceptance and its screenshot report. No live
wallet or transactions were used for the correction work.

The baseline is a dirty working tree containing the existing WebP, sequenced
publishing, fixtures and tests. Inspect current Git state before implementation;
preserve those changes and unrelated work. Do not reset to the audit's base
commit to reproduce it: that commit omits uncommitted feature work. No
dependency upgrade, new state-machine framework, broad profile rewrite or
external service is needed for this plan.

Keep transaction construction, canonical confirmation, generation admission,
single-use preflights and encrypted Stronghold persistence backend-owned. Prefer
a bounded frontend correction. Any IPC change must be justified by a
demonstrated gap in existing completion evidence and include its Rust/TypeScript
contract and regression tests. Do not alter wire formats, fee calculation,
expiry policy, supported networks or signer rules to simplify the UI.

## Baseline source diagnosis (before corrections)

| Area                                                                                                       | What exists                                                                                           | Work required                                                                                       |
| ---------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| [IdentityProfileEditor](../../../src/lib/components/wallet/sections/identity/IdentityProfileEditor.svelte) | Local `step`, copied `saved` continuation and a reactive branch that can replace submitted with ready | One publication presentation owner; use receipt-bound pending and explicit completion events        |
| [IdentityDetailView](../../../src/lib/components/wallet/sections/identity/IdentityDetailView.svelte)       | 15-second refresh; in-flight refreshes skipped; Complete transformed to null before reaching editor   | Queue requested refresh; reject older observations; consume completion explicitly                   |
| [Identity section](../../../src/lib/components/wallet/sections/Identity.svelte)                            | Canonical profile cache, pending receipts and settlement callbacks                                    | Preserve receipt evidence until completion is consumed; update profile and pending state coherently |
| [Backend publication](../../../src-tauri/src/core/channels/vrpc/identity/profile/publication.rs)           | Reconcile returns Complete once, then deletes the plan; settled IDs also include conflict/expiry      | Preserve this evidence distinction; test the existing contract before deciding any API change       |
| [Profile drafts](../../../src/lib/identity/profileDrafts.ts)                                               | Independent memory drafts and existing toggle-style removal                                           | Reuse draft isolation; make Remove and Undo semantics explicit for the visual editor                |
| [Image editor](../../../src/lib/components/wallet/sections/identity/ProfileImageEditor.svelte)             | Crop, arrows, zoom, native encode, empty setup screen                                                 | Direct selection into populated crop; preserve cancel/error behavior; hide routine technical copy   |
| [Costs](../../../src/lib/components/wallet/sections/identity/ProfilePublicationCosts.svelte)               | Separate single/split presentation and redundant details                                              | Shared compact fee hierarchy and conditional disclosure                                             |
| [Image helpers](../../../src/lib/identity/profileImages.ts)                                                | Bounded candidates and integer-based `profileFeeDisplay`                                              | Reuse validation and formatter; do not infer plan count from bytes                                  |

The source paths above were checked on 2026-09-21. Read them again at execution
time; line numbers and local changes may have moved.

## Sequence

### Phase 1 — Reliable publication state (F1, F2, functional part of F3)

- [x] Extend mounted coverage through the real parent/editor composition, not
      only an editor supplied with an already-waiting prop. Reproduce the
      stale-ready response after send and the Complete-to-null handoff failure.
- [x] Define one selected-identity publication owner in the detail flow, with a
      small typed transition helper under `src/lib/identity/` if that makes the
      async ordering testable. Keep local crop/description UI state separate. Do
      not keep competing copies of the publication status in parent/editor.
- [x] Track session, identity, plan, step, submitted txid and a local request
      generation. On send success, adopt its pending receipt before another
      render. A refresh started before admission cannot reinstate ready.
- [x] Coalesce refreshes, queue one follow-up when requested during a refresh,
      and apply only observations matching the current context. Reuse the
      existing polling interval; faster polling is not the fix.
- [x] Handle explicit Complete before clearing the continuation. Preserve the
      known receipt while canonical profile refresh/settlement runs. Handle a
      subsequent null only through confirmed matching evidence, never as a
      success flag. Integrate the parent pending-profile callback without
      prematurely throwing away the receipt needed to finish the editor.
- [x] Distinguish fresh backend stale/conflict/reorg results from old responses.
      Never use status ranking or membership in `settledTxids` as proof of
      successful confirmation.
- [x] Use the same completion path for polling and Check confirmation. Settle
      once, refresh the canonical profile and return to it with Profile updated.
      Guard unmounts and wallet/network/identity changes.
- [x] Derive waiting copy from the real current/total step and changed fields.
      Remove next-update copy for one-update and final-step pending states.

**Exit:** deterministic integration tests cover every observed state defect;
pending never exposes Review/Publish/Discard for the same step, and final
confirmation settles without navigation. Backend transport ambiguity and
recovery behavior remain intact.

### Phase 2 — Visual editor and focused crop (F4)

- [x] Compose the effective profile with existing display geometry. Reuse small
      existing rendering pieces; extract a shared presentational preview only
      where it avoids duplicating the profile geometry. Do not mount the whole
      public-profile page with unrelated tabs/actions inside the editor.
- [x] Add independent image controls and an inline description, one unpublished
      indicator and one Review changes action. Hide inactive Connections and
      routine per-field format/byte metadata.
- [x] Adapt draft helpers/tests for explicit Remove image versus Undo change.
      Keep pending removals visibly reviewable and every field optional.
- [x] Open the real file input from Add/Change; show crop only after selection.
      Preserve the prior field on chooser cancel, crop cancel, invalid source or
      encode failure. Retain drag, arrows, zoom and the existing Rust encoder.
- [x] Use Image requirements disclosure and error-specific explanations. Keep
      file types/dimensions/size/decode validation unchanged.
- [x] Keep in-session navigation persistence and clearing at session boundaries.
      Do not introduce plaintext persistence or promise restart recovery for an
      unreviewed memory draft.
- [x] Update English/Dutch labels, focus behavior and durable browser fixtures.
      Inspect light/dark at 920×620 before progressing.

**Exit:** avatar-only, header-only, description-only, all-three, replacement,
removal and undo work with a combined preview; no inactive future sections or
duplicate draft labels remain. Input cancellation never loses a prior change.

### Phase 3 — Review and retained progress (F3, F6)

- [x] Use the same identity/preview composition for single and split review.
      Show the actual changed fields and explicit removals, with a single
      destination-labelled Back control.
- [x] Unify cost presentation around the current exact fee, and the next
      estimate/estimated total when relevant. Use `profileFeeDisplay` for every
      coin amount; hide missing fiat. Add Fee details for balance and unchanged
      historical estimates. Keep changed-fee and insufficient-funds feedback
      adjacent to approval.
- [x] Use action labels from the real changed-field set. Show the public-history
      consequence once before approval and invalidate old reviews after edits.
- [x] Replace generic continuation pages with stable field/status rows and an
      identity label. Retain update 1/2 context without narrating the protocol.
      Put transaction IDs/copy actions inside Transaction details.
- [x] Make Back to profile preserve the plan without implying a submitted
      transaction needs saving. Keep confirmed profile content separate from
      pending/ready previews. Keep only one concise pending/continue notice on
      the public profile.
- [x] Keep discard secondary, unavailable during pending/ambiguous transport,
      and explicitly confirmed for the named unpublished remainder. Preserve
      both confirmed fields and the remainder if discard fails.
- [x] Add polite, nonrepeating status announcements and stage-aware focus
      handling. Keep the footer visible with local body scrolling and longer
      Dutch copy at the minimum window size.

**Exit:** every row in the specification's progress table has a fixture and
correct action set. Both appearances are inspected. Step two always requires a
fresh exact fee and its own approval; no hidden auto-send path is introduced.

### Phase 4 — Earlier one-update alternative (F5)

- [x] Surface the existing backend-proven candidate before the split fee
      explanation. State the one-update outcome and estimated saving instead of
      leading with byte size.
- [x] Keep balanced/current media selected initially. Show a same-scale visual
      comparison and explicit Use smaller image/Keep current image actions.
      Preserve the original candidate while a new quote is loading or fails.
- [x] Selecting the candidate re-plans and returns to fresh exact review.
      Opening/comparing/cancelling never sends and cannot reuse a consumed or
      invalidated preflight. Display the actual returned plan if it changes.
- [x] Verify the option is absent for an already-single plan, the remaining
      header, a candidate that still needs two updates or one without a lower
      estimated total. Do not change the encoder ladder or invent a byte cutoff.

**Exit:** the audited alternative is discoverable before reading the two-update
explanation. The user can knowingly choose either image and gets the correct
fresh review. Automatic extra quality loss is outside this increment.

### Phase 5 — Native acceptance and documentation closure (all findings)

- [x] Run the existing native Tauri testing workflow on fresh authorized empty
      VRSCTEST identities; do not overwrite player1/player6 or their audit
      evidence just to make the next run repeatable.
- [x] Publish all three fields in one update, and in a real two-update plan.
      Pause, lock, restart and resume between split steps. Check a pending
      screen with automatic refresh and with Check confirmation. End on the
      confirmed profile without needing Back as a workaround.
- [x] Exercise the actual OS chooser through an available native automation
      surface, including cancel. If unavailable, record that step as unverified
      and preserve the real file-input/crop proof boundary; do not report a
      DataTransfer attachment as chooser coverage.
- [x] Capture and inspect every meaningful edit/review/progress/final state in
      light/dark at 920×620. Check normal and wider windows, keyboard/focus,
      long labels and readable contrast. Use fresh run-specific output paths.
- [x] Assert canonical values/digests, expected one/two transaction grouping,
      settled continuation and unchanged identity controls. Preserve screenshots
      and the meaningful assertion log together.
- [x] Record transport/expiry/conflict tests at their actual level of proof. Use
      deterministic fixtures/tests for failure injection; do not claim a
      fabricated broadcast failure as a real network incident or send extra
      transactions merely to manufacture an expiry.
- [x] Update the profile screen and sequenced-publishing documentation to the
      delivered behavior, remove conflicting old UI prescriptions, and record
      remaining platform evidence gaps. Mark this plan complete only after F1–F6
      meet their acceptance checks.

## Regression and acceptance matrix

| ID  | Scenario and required assertion                                                                                                                                             | Level / phase                                                 |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| A1  | Delayed ready response resolves after send success for single, split-first and split-final; pending receipt/action restrictions survive                                     | Parent/editor mounted; 1                                      |
| A2  | Refresh invoked during another refresh runs a queued follow-up; old context results cannot overwrite state after wallet/identity switch or unmount                          | Transition + mounted; 1                                       |
| A3  | Complete is consumed exactly once; canonical profile appears; later null cannot reopen the editor                                                                           | Parent/editor mounted, then native; 1/5                       |
| A4  | Another observer has consumed Complete; known receipt plus confirmed matching canonical content settles; null alone, unavailable data and mismatched revision do not        | Integration with production matching rules; 1                 |
| A5  | Conflict/expiry settled IDs do not produce success; confirmed reorg can legitimately return to recovery; ambiguous send stays pending                                       | Existing Rust recovery + mounted; 1                           |
| A6  | One-update/final-step pending has no remaining/next-update promise; identity and actual field/status labels persist                                                         | Mounted + inspected light/dark; 1/3                           |
| A7  | Independent changes, description limits, chooser/crop cancellation, invalid image, replacement, removal and undo preserve the correct draft                                 | Existing helper/mounted + native chooser where available; 2/5 |
| A8  | Review contains combined result/removals, required fees and one notice; every edit/candidate change needs fresh review; double click consumes one preflight                 | Mounted + native screenshots; 2/3                             |
| A9  | Fee formatting preserves one-satoshi precision and trims zeros; missing fiat disappears; rates use the actual network asset; changed second fee remains clear               | Formatter/component tests; 3                                  |
| A10 | Same plan/header survives restart; first confirmed fields stay public; second requires new review/approval; discard only removes permitted unpublished remainder            | Rust persistence + mounted + native restart; 3/5              |
| A11 | Beneficial alternative is visible before split explanation; compare/cancel never mutate/send; choice re-plans; failure preserves original; ineligible cases hide the option | Mounted + visual comparison; 4                                |
| A12 | Minimum window has visible actions, one Back, one draft status, no inactive Connections/technical rows; keyboard, announcements and long Dutch labels work in light/dark    | Rendered/native and targeted accessibility checks; 2–5        |
| A13 | Final authenticated avatar/header/description match the proposal; tx partition and control invariants hold; completion UI agrees with backend                               | Live native VRSCTEST; 5                                       |

## Verification commands

Use repository-pinned Node 22.23.2 and pnpm 11.24.0; verify the active versions
before commands. Do not run repository-wide formatting over existing dirty work.
Run focused checks as each phase lands, then the applicable broader checks once
at integration. The local correction run used these checks plus the new
publication-owner and real detail/editor composition suites. Native acceptance
subsequently passed as recorded below.

```sh
pnpm exec vitest run src/lib/identity/profilePublication.test.ts src/lib/identity/profileDrafts.test.ts src/lib/identity/profileImages.test.ts src/lib/utils/identityProfileUpdate.test.ts
pnpm exec vitest run --config vitest.mounted.config.ts src/lib/components/wallet/sections/identity/IdentityPublication.mounted.ts src/lib/components/wallet/sections/identity/IdentityProfileEditor.mounted.ts src/lib/components/wallet/sections/identity/IdentityDetailView.mounted.ts src/lib/components/wallet/sections/Identity.mounted.ts --maxWorkers=1 --no-file-parallelism
pnpm check
pnpm lint
pnpm docs:check
pnpm build
git diff --check
```

Include any new transition-helper test file in the focused test command. Run
`cargo test --manifest-path src-tauri/Cargo.toml profile --lib` when Rust/IPC or
publication contracts change, and for the final recovery/persistence acceptance.
For native acceptance, reuse the existing debug `e2e-webdriver` harness and
[Keychain runbook](../../references/test-wallet-keychain.md). Use a test-owned
loopback endpoint, verify it is gone afterward and restore the normal app if
testing replaced it. Keep credentials and credential screens out of artifacts.

Do not use a green static test suite as a substitute for the native completion
and image-quality evidence. Report preexisting unrelated failures separately; do
not expand the implementation into unrelated fixes.

## Local correction verification (2026-09-21)

- A1–A6: unit and real detail/editor mounted coverage exercises delayed ready,
  queued refresh, session changes/unmount, polling/manual completion, consumed
  Complete, mismatched/unavailable evidence, recovery and ambiguous transport. A
  new read-only IPC confirms a receipt's exact revision and canonical block
  after the retained plan has been consumed; it cannot admit or send anything.
- A7–A11: focused draft/image/editor tests cover independent optional fields,
  description limits, decode/encode errors, cancellation, removals/undo,
  single-use approval, fee precision, retained progress/discard and candidate
  eligibility/replanning. Rust profile tests cover existing encrypted
  persistence, admission, confirmation and reorg/expiry rules.
- A12: production components were inspected in synthetic browser fixtures in
  light/dark at 920×620, including 159-character Dutch description and long
  identity, keyboard crop movement, focus restoration and fee/footers. Normal
  1040×680 and wider 1440×900 reviews were checked. Screenshots establish
  browser geometry and transitions, not native rendering or on-chain image
  quality.
- At the local handoff, A13 and native portions of A3/A7/A8/A10/A12 remained
  open: actual OS chooser, native WebP encoding, live one/two-update grouping,
  restart/resume, canonical digests/control invariants and final automatic
  native completion.

The task-local evidence includes command logs, inspected screenshots, a
correction-only patch relative to the imported source snapshot, and a manifest
with before/after hashes. The originating task verified the baseline hashes,
applied only the 34-file correction delta and verified its result hashes before
rebuilding the native application. The earlier native audit remains historical
baseline evidence only.

## Native acceptance closure (2026-09-21)

F1–F6 passed the scoped native retest in the authorized `mijn app` wallet. The
ignored evidence bundle is `output/profile-ux-retest-20260921/`: `report.html`,
`report.md`, `checkpoints.json`, `verification.json`, `send-evidence.jsonl`,
`native-editor-checks.json`, `native-chooser-verification.json` and
`cleanup.json`. The report contains 50 inspected new native captures and four
historical comparison captures; the original 53-capture audit is unchanged.

- `game564498.gamesession2@`: all fields confirmed in one transaction after an
  explicit smaller-header comparison and fresh review; fee 0.1923 VRSCTEST.
- `gamesession2.VRSCTEST@`: kept the current header and published avatar plus
  description first, retained the same plan/header through leave, lock and full
  app restart, then reviewed and approved the exact header fee separately. Fees
  were 0.13316 and 0.08647 VRSCTEST; nothing auto-sent.
- All three successful sends stayed pending without stale approval/discard
  actions. Both final profiles appeared automatically without a Back workaround.
  Canonical values/digests, one/two transaction grouping, confirmed source
  blocks, cleared continuation and unchanged identity controls passed
  assertions.
- Native editor/review/progress/final views were inspected in light and dark at
  920×620, with 1040×680 and 1440×900 reviews. Crop keyboard movement,
  cancellation/focus, description validation and unsubmitted removal/undo
  passed. Long Dutch text and deterministic accessibility/state cases retain
  browser or mounted-test proof; no full screen-reader or measured contrast
  audit is claimed.
- Actual macOS avatar chooser Open/Cancel and selected-file-to-populated-crop
  passed in the ordinary debug app. The WebDriver plugin's replacement macOS UI
  delegate lacks file-panel forwarding, so instrumented paid journeys attached a
  real File to the real input before native crop/encode/review/sign/broadcast.
- Check confirmation worked on the first pending split step. The final manual
  attempt raced automatic completion before it could click; positive manual
  final completion is covered by mounted tests, not claimed as live evidence.
- Expiry/conflict/reorg/ambiguous transport/discard recovery remain
  deterministic test coverage. No extra transaction manufactured a failure.
  Windows/Linux, release builds, mainnet and multisignature acceptance remain
  outside this scope.

Three live transactions cost 0.41193 VRSCTEST. Original player1/player6 profiles
were preserved. The normal development app was restored at 1040×680 in dark mode
on the completed profile, without an open dialog; the test-owned WebDriver
endpoint was verified closed. No commit, push, PR, merge or release was made.

## Delivery and completion record

Implement phases in order; each has a reviewable outcome and can be checked
before changing the next part. No commit, push, PR or release is implied by this
plan. Preserve user changes and follow the authorization in the implementation
task.

- [x] Native findings translated into a product spec and this execution plan.
- [x] F1–F6 implemented and accepted through A1–A13 at the proof levels above.
- [x] Screenshots and live assertions reviewed; remaining evidence gaps named.
- [x] Current product docs describe delivered local behavior and its evidence
      limits.
- [x] Normal app restored; temporary automation endpoints closed.

Automatic selection of extra lossy compression, mainnet/multisignature support,
connection publishing, a new durable unreviewed-draft store and broader profile
redesign are intentionally outside this completion definition.
