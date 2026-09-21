---
owner: lite-wallet-team
last_reviewed: 2026-09-21
---

# Plan and specification: VerusID WebP and sequenced profile publishing

- Status: protocol publication and UX corrections accepted on native macOS
  VRSCTEST; remaining platform/failure evidence limits recorded below
- Owner: lite-wallet-team
- Last updated: 2026-09-21
- Scope: VRSCTEST profile avatar, header image and description publishing

## Local implementation and verification (2026-09-21)

Phases 1–4 are implemented in the local working tree. The optional image-only
cost disclosure is omitted; the actual per-update quote is the cost authority.
The smaller-image option is offered only when it removes an update. The encoder
targets 16 KiB and tries a smaller candidate near 5 KiB at qualities 86, 78, 70,
62, 55 and 45; actual template evidence determines partitioning.

macOS WKWebView returned PNG when asked to encode WebP. Production therefore
uses the native `webp` encoder on all desktop platforms, fed only bounded
cropped PNG pixels. Published candidates must pass the backend WebP decoder,
dimensions, MIME and 32 KiB checks. Browser fixtures retain a checked browser
encoder.

The separate `profile_publications.snapshot.stronghold` persists reviewed
intent, media digests, quotes and receipts. A generation-bound admission stores
the exact transaction ID before transport. An ambiguous broadcast response
remains waiting until canonical confirmation, a confirmed conflicting revision,
or expiry proves it safe to replan. Transactions expire 20 blocks after
preparation. This can delay retry after a failed network response; an RPC error
alone does not prove absence. A saved plan restores content, never permission to
send. Update 2 is prepared only when the user explicitly continues after
verified confirmation of update 1.

Verification includes focused Rust media/template/intent/recovery tests,
encrypted Stronghold reopen and failure tests, TypeScript
image/draft/confirmation tests and mounted editor/identity tests. Native macOS
fixture checks use the real Rust image encoder and production UI at a 920×620
window, with synthetic profile/fee responses. They establish rendering and
native encoding, not successful on-chain publication.

### Native VRSCTEST follow-up (2026-09-21)

The subsequent native test in `mijn app` confirmed avatar, header and
description for `player1.VRSCTEST@` in one transaction and `player6.VRSCTEST@`
in two. The second plan retained its header through lock, native app restart and
unlock, then required fresh review and explicit approval. All three transactions
confirmed and identity controls remained unchanged.

The baseline audit found that the editor regressed to ready after successful
sends and remained waiting after final confirmation. Corrections now address
both transitions and the visual flow, and the fresh native retest passed. The
[UX specification](../../product-specs/verusid-profile-publishing-ux.md) and
[correction plan](../done/verusid-profile-publishing-ux.md) track these defects
plus the visual/editor/review improvements. The local ignored evidence bundle is
`output/profile-ux-audit-20260921/`, with 53 inspected native screenshots and
backend assertions. The OS file chooser was not tested; a File was attached to
the real input before the native crop/encode/review/publication flow.

The correction retest used fresh empty `game564498.gamesession2@` (one update,
explicit smaller-header choice) and `gamesession2.VRSCTEST@` (two updates,
current header retained through lock/full restart). All sends stayed pending;
both final views completed automatically with matching canonical digests,
correct transaction grouping, unchanged controls and no unfinished continuation.
The second update required fresh exact review and explicit approval. Total paid
was 0.41193 VRSCTEST across three transactions. The separate ignored bundle
`output/profile-ux-retest-20260921/` contains 50 inspected native captures, four
historical comparisons and assertion records. Actual macOS avatar chooser
Open/Cancel and populated crop were verified in the ordinary debug app; the
instrumented app's WebDriver delegate does not forward native file-panel calls.
The normal app was restored and the temporary WebDriver endpoint closed.

### Manual VRSCTEST acceptance

1. Open an editable linked VRSCTEST identity and stage an avatar, header and
   description. Review the WebP previews and costs. A simple pair may use one
   update; a more detailed pair can require two. Bytes alone do not determine
   it.
2. For a two-update plan, approve avatar first. Keep the previous confirmed
   profile visible while pending. Lock/reopen, then return to Continue profile
   update and verify the unpublished header is retained.
3. Wait for canonical confirmation. Review the header's fresh exact fee and
   compare it with the earlier estimate. Approve separately, then verify the
   confirmed header and disappearance of the continuation.
4. On a separate draft, exercise a beneficial smaller preview, Back, Keep for
   later, and confirmed discard. Confirm discard never changes the already
   published avatar/description.

Remaining evidence limits: failure/discard recovery is covered by deterministic
tests rather than manufactured live incidents; positive manual final completion
has mounted-test proof because automatic native completion won the attempted
manual-click race. Windows and Linux runtime checks and an upstream minimal
reproduction of the adjacent multipart-group behavior remain open. Successful
publication and restart recovery do not establish those cases. Retain the
partition fallback until corrected network behavior is deployed and positively
verified. The new UX specification supersedes the presentation prescriptions
below where explicitly identified; this plan remains the backend encoding,
planning, signing and persistence baseline.

## Decision summary

1. New avatar and header publications use lossy WebP (`image/webp`). Existing
   JPEG profiles remain readable.
2. Editing remains one guided profile flow. The user never has to delete a
   header, publish an avatar, and then find or select the header again.
3. The backend first tries to prepare one transaction containing every staged
   change. It creates two sequential updates only when the generated Verus
   evidence layout requires that split.
4. A two-update plan always requires two explicit approvals. The second update
   is never silently broadcast after the first confirms.
5. Before the first approval, Review shows the exact first fee, an estimate for
   the second fee, and the estimated total. The second fee becomes exact only
   after the first update confirms and a fresh transaction is prepared.
6. When a smaller WebP candidate turns two updates into one, Review offers a
   concrete comparison and preview. Fee savings alone do not trigger it. It does
   not describe a valid header as invalid or require the user to remove it.
7. If two updates are needed, publish avatar plus description first and retain
   the header for the second update. The remaining header is stored encrypted so
   the flow can resume after lock or restart.

## Why this change is needed

Before this change, the editor accepted JPEG, PNG and WebP source files, cropped
them to fixed dimensions, and published JPEGs of at most 32 KiB. Avatar and
header were passed to one `updateidentity` transaction.

Local VRSCTEST testing isolated the reported `bad-txns-failed-precheck` failure:

- one multipart profile image plus a description passed contextual output
  precheck;
- two small, single-part images plus a description passed;
- two adjacent multipart image evidence groups failed with
  `Invalid multipart evidence`, surfaced by RPC as `bad-txns-failed-precheck`;
- a 32 KiB header was valid by itself and with a description, so 32 KiB is not
  inherently too large for a profile image;
- the observed transition to multipart evidence was around 5,390 raw image bytes
  for the current wrapper. This is measured behavior, not a stable product
  limit. Serialization changes can move it.

The failure is therefore about putting two multipart evidence groups next to
each other, not simply about exceeding the wallet's 32 KiB image cap. The wallet
must classify the generated transaction structure. A raw byte threshold alone is
not authoritative.

## Product outcome

A person can select and crop an avatar and header, add a description, understand
the complete likely cost, and publish the profile without learning about Verus
multipart evidence.

Most profiles should still publish in one update. When two updates are required,
the experience remains one retained task:

```text
Edit profile
  -> Review 1 update
  -> Approve and publish
  -> Confirmed

or

Edit profile
  -> Review 2 updates and estimated total
  -> Approve update 1
  -> Wait for confirmation
  -> Review exact fee for update 2
  -> Approve update 2
  -> Confirmed
```

The word **update** is used in the interface. Protocol terms such as
`multipart`, `evidence output`, `descriptor` and `precheck` stay in diagnostics
and developer documentation.

## User experience specification

The
[publishing UX corrections specification](../../product-specs/verusid-profile-publishing-ux.md)
supersedes the original field list, disabled Connections, large fee card and
continuation prose. Its implementation passed local verification and the fresh
native macOS VRSCTEST retest in the originating task. Protocol, encoder, fee and
persistence requirements below are unchanged.

- Edit the effective combined profile, with independent image controls and an
  inline description. Choosing an image goes directly to the chooser and then a
  populated crop. Cancel or failure preserves the previous draft. Technical
  metadata belongs in Image requirements; Remove and Undo have distinct
  meanings.
- One and two updates share the same compact review composition. Always show the
  proposed result and explicit removals. The split review keeps the first exact
  fee, second estimate, total and “Two updates. You’ll approve each fee.”
  visible together. Fee details holds balance, quote metadata and unchanged
  estimates. Missing fiat is omitted; integer fee formatting preserves satoshi
  precision.
- A beneficial, backend-proven Publish in one update option appears before split
  costs, with estimated saving and Compare images. Current image and Smaller
  image use the same crop and display scale. Use smaller image and Keep current
  image both obtain fresh review, never publish. Do not silently change quality.
- Publish labels follow the actual first-step fields. The second update uses
  Publish header and always requires a fresh exact fee and explicit approval.
  Show the public-history consequence once before each paid approval.
- One progress view shows the identity and actual field statuses. First-step
  receipts identify fields already published; header intent is marked saved,
  ready or waiting. One-update and final-step waiting never promise another
  step. Transaction details holds distinguishable previous and pending receipts.
- Back to profile preserves intent and pending transactions. Discard is
  contextual, explicitly confirmed and unavailable while waiting, including
  ambiguous transport. It never reverses already confirmed fields.
- A receipt-bound frontend owner rejects older observations and queues a
  follow-up refresh. Completion uses backend transaction/block proof and
  canonical value/digest matching. The read-only confirmation command works
  after another observer has consumed Complete; null and settled IDs are never
  success evidence. Existing expiry/conflict and reorg recovery remain valid.

## Image format specification

### New publications

| Property         | Avatar                                                | Header                               |
| ---------------- | ----------------------------------------------------- | ------------------------------------ |
| Wire MIME        | `image/webp`                                          | `image/webp`                         |
| Dimensions       | 256×256                                               | 960×160                              |
| Hard encoded cap | 32 KiB                                                | 32 KiB                               |
| Normal target    | smallest acceptable candidate, preferably single-part | 16 KiB or less                       |
| Matte            | deterministic white, matching current JPEG behavior   | deterministic white                  |
| Metadata         | stripped by crop and native encoding                  | stripped by crop and native encoding |

Keep the 32 KiB application cap for the first WebP release. WebP is intended to
improve quality at a given byte size, not to expand the on-chain storage budget.

### Encoding policy

Generate bounded candidates from the cropped pixels rather than repeatedly
re-encoding a previous compressed image:

1. a balanced candidate using a descending lossy-quality ladder;
2. a smaller candidate that attempts to fit a single evidence part without
   crossing a visual-quality floor;
3. reject the source only if no valid WebP candidate is at or below 32 KiB.

The publication planner, not the encoder, decides whether a candidate is
single-part. The encoder may target roughly 5 KiB based on current observations,
but must not treat that number as protocol law.

The current Tauri WebView path may use
`canvas.toBlob(..., "image/webp", quality)` only when all of these checks pass:

- the returned blob is non-null;
- the returned MIME is exactly `image/webp`;
- bytes have a valid RIFF/WEBP container signature;
- the backend successfully decodes the image and verifies dimensions;
- supported macOS, Windows and Linux runtimes produce acceptable candidates.

Browsers can silently return a different format when an encoder is unsupported.
There must be no silent PNG or JPEG fallback under an `image/webp` descriptor.
If WebView encoding is not reliable across the supported runtime matrix, add a
dedicated lossy WebP encoder behind the Rust boundary before release. The
currently used Rust `image` crate is sufficient for WebP decoding but its
built-in WebP encoding path is lossless, so it is not automatically a
replacement for a quality-controlled lossy encoder.

### Reading and validation

The reader must continue to accept existing `image/jpeg` descriptors and add
`image/webp`. New writes use WebP only.

Backend validation remains authoritative and must bind together:

- descriptor MIME;
- container signature;
- successful bounded decode;
- exact dimensions for the field;
- encoded length at or below 32 KiB;
- content digest and authenticated Verus evidence.

Snapshots and frontend media values need to carry MIME explicitly. Remove
hard-coded `data:image/jpeg` construction from profile display, draft, review,
pending and confirmation paths. Preserve the existing avatar, header and about
VDXF keys; only the media encoding and MIME change.

## Publication planning specification

### Authoritative classification

Create one backend-owned publication planner that builds and inspects the real
daemon transaction templates. It returns `single` only when the complete staged
change produces a valid supported evidence layout.

For current Verus behavior:

- zero or one multipart image group: keep all staged changes in one update;
- two multipart image groups: create two ordered updates;
- unknown or malformed evidence shape: fail preparation without broadcasting.

Do not classify solely from Base64 length, encoded byte length or a hard-coded
5,390-byte threshold.

### Partition rule

When two updates are necessary:

1. update 1 contains the avatar change and description change;
2. update 2 contains the header change;
3. unrelated identity controls and content remain unchanged in both;
4. update 2 is created from the confirmed state produced by update 1.

This order gives the identity its most recognizable information first while the
wider, usually larger decorative image waits. Avatar or header removal remains a
small record and should not cause a split by itself.

### Fee quote rules

At initial Review the backend performs three bounded operations:

1. build and validate the complete single-update candidate;
2. if a split is required, create a normal sendable preflight only for update 1;
3. create a non-sendable quote template for update 2 against the same confirmed
   chain snapshot and current live fee parameters.

The result contains:

- exact update 1 fee and expiring preflight ID;
- estimated update 2 fee with quote height and timestamp;
- estimated total;
- encoded bytes, evidence bytes and evidence layout for diagnostics;
- optional image-only estimates for `Cost details`;
- the selected image candidate and any proven smaller alternative.

An update 2 quote never includes a preflight ID and cannot be passed to the send
command. After update 1 confirms, create a fresh preflight and mark its fee
`Exact`.

### Suggested boundary types

Names are illustrative; keep the public command surface as small as practical.

```ts
type ProfilePublicationPlan = {
  planId: string;
  identityAddress: string;
  mode: 'single' | 'sequential';
  steps: ProfilePublicationStep[];
  estimatedTotalFeeSats: string;
  feeAsset: 'VRSCTEST';
  optimization?: ProfileImageOptimization;
};

type ProfilePublicationStep = {
  number: 1 | 2;
  fields: Array<'avatar' | 'header' | 'description'>;
  feeSats: string;
  feeAccuracy: 'exact' | 'estimated';
  preflightId?: string;
};
```

The backend coordinator should own planning, persistence, confirmation
reconciliation and rebuilding the next preflight. Svelte should render returned
states and invoke explicit transitions rather than recreate partition rules.

### State machine

```text
Draft
  -> Reviewing single -> Submitted -> Confirmed -> Complete
  -> Reviewing 1 of 2 -> Submitted 1 -> Waiting for confirmation
      -> Reviewing 2 of 2 -> Submitted 2 -> Confirmed -> Complete
      -> Paused
      -> Remaining update discarded
  -> Stale; fresh review required
  -> Failed; draft or remaining update retained
```

Only one active profile publication plan is allowed per account, network and
identity. A confirmed unrelated identity update makes a waiting plan stale. Keep
its unpublished bytes, reload canonical profile state, and require a fresh
review rather than guessing how to merge.

## Persistence and privacy

The current frontend draft is deliberately memory-only, and the current
`account_state.json` store is plaintext JSON. Neither is sufficient for a
reliable two-update continuation:

- memory-only state loses the selected header on application exit;
- plaintext account state would expose an unpublished image before the user has
  placed it on-chain.

Before broadcasting update 1, persist the sequential plan in a dedicated,
account-scoped Stronghold snapshot. If encrypted persistence fails, do not
broadcast update 1.

Persist only what is needed to resume:

- schema version, plan ID, network and identity address;
- selected encoded WebP bytes, MIME and digest for unpublished fields;
- intended description/removals;
- step state, submitted transaction IDs and confirmation anchors;
- quote height, timestamp and displayed estimates.

Do not persist a private key, seed, Stronghold password material, signed
transaction, reusable preflight ID or session authorization. Preflights remain
short-lived and session-bound. Unlocking the wallet restores access to the plan,
not authority to broadcast it.

Delete unpublished media from the encrypted plan after update 2 confirms or the
user explicitly discards it. Keep ordinary pending/confirmed metadata only as
long as required by the existing reconciliation policy.

## Failure and recovery behavior

| Event                                         | Required behavior                                                        |
| --------------------------------------------- | ------------------------------------------------------------------------ |
| Preparation fails                             | Broadcast nothing; keep the full editable draft.                         |
| Update 1 broadcast fails                      | Keep the full plan; require a fresh review/preflight.                    |
| Update 1 is pending                           | Do not prepare or send update 2 yet.                                     |
| Update 1 confirms                             | Load the new canonical identity revision and prepare update 2.           |
| Update 1 is rejected or disappears in a reorg | Return to fresh review with all unpublished changes retained.            |
| Update 2 fee differs from estimate            | Show estimate and new exact fee; require explicit approval.              |
| Update 2 broadcast fails                      | Keep the confirmed avatar/description and encrypted header; allow retry. |
| Wallet locks or app exits                     | Resume after unlock from encrypted state; never auto-send.               |
| Another identity update lands                 | Mark plan stale and rebuild from canonical state.                        |
| User discards remaining header                | Delete encrypted remainder; do not undo update 1.                        |

The legacy `bad-txns-failed-precheck` string should map to a useful diagnostic,
but the planned classifier should prevent the known two-multipart construction
from reaching broadcast.

## Implementation plan

### Phase 1: WebP media contract

- Replace JPEG-only image constants with a typed MIME-aware profile media model.
- Add bounded WebP validation and retain legacy JPEG reading.
- Produce balanced and smaller WebP candidates from the crop canvas.
- Verify WebP output and previews in supported Tauri WebViews, light and dark
  mode, and at the minimum 920×620 window.
- Update snapshots, drafts, pending records and display URLs to carry MIME.

### Phase 2: Publication planner and quotes

- Extract template building, validation and fee calculation behind one profile
  publication module.
- Inspect actual evidence layout and decide `single` versus `sequential`.
- Build only the current step as a sendable preflight.
- Add quote-only future-step and image-only calculations.
- Return exact/estimated markers, quote height and image byte sizes.

### Phase 3: Durable sequential coordinator

- Add an isolated Stronghold snapshot for profile publication plans.
- Persist the plan before update 1 crosses the broadcast boundary.
- Extend pending confirmation reconciliation with step and plan identity.
- Rebuild update 2 only after update 1 is canonically confirmed.
- Add resume, keep-for-later and discard-remainder commands.

### Phase 4: Review and progress UI

- Keep the current one-update review compact.
- Add the two-update explanation, cost table and estimated total.
- Add conditional smaller-image comparison and preview.
- Add `Update 1 of 2`, waiting, paused and `Update 2 of 2` states.
- Add all copy through translation keys and verify English and Dutch layouts.

### Phase 5: Compatibility and upstream follow-up

- Freeze JPEG and WebP read fixtures with descriptor/evidence provenance.
- Re-run the bounded VRSCTEST matrix against the supported daemon version.
- Prepare a minimal Verus Core reproduction for the adjacent multipart-group
  behavior. Keep the wallet partition fallback until fixed network behavior is
  deployed and positively detected.

## Verification matrix

### Encoding

- JPEG, PNG and WebP sources publish as `image/webp`.
- Exact avatar/header dimensions survive decode.
- Metadata is not copied from source files.
- Backend rejects wrong MIME, spoofed container, corrupt WebP, wrong dimensions,
  decode bombs and output over 32 KiB.
- Transparent source behavior matches the specified white matte.
- Balanced and smaller candidates are generated from the same cropped pixels.

### Planning and fees

- two single-part images plus description -> one exact-fee update;
- one multipart image plus one single-part image -> one exact-fee update;
- two multipart images -> two-update plan, never one broadcast attempt;
- 32 KiB header plus description -> one valid header update within the current
  cap;
- update 2 initial fee is estimated and non-sendable;
- update 2 post-confirmation fee is exact and has a fresh preflight;
- image-only detail estimates are labeled non-additive;
- smaller-candidate callout appears only when it reduces two updates to one;
- fee-only savings, existing single-update plans and the remaining second update
  never show the callout.

### Sequence and recovery

- update 2 cannot be approved or sent before update 1 confirms;
- first approval cannot authorize update 2;
- lock, restart and navigation retain the remaining header through Stronghold;
- stale identity revision forces replan without losing unpublished media;
- failure or expiry never reuses a preflight;
- discarding update 2 never changes the confirmed update 1 result;
- the previous confirmed profile remains visible until each submitted step
  passes canonical confirmation checks.

### UI and accessibility

- exact and estimated costs remain distinguishable without color;
- two-step explanation, costs and primary action fit at 920×620 without hiding
  critical information;
- long fees, fiat values and localized copy do not overlap;
- keyboard and screen-reader order follows plan, costs, optimization and action;
- light and dark mode show the same hierarchy;
- no pointer cursor is introduced for clickable actions.

### Required checks

- focused TypeScript image/planner tests;
- focused Rust codec, template, fee, intent and persistence tests;
- mounted editor tests for single, split, optimize, pause, resume and failure;
- `pnpm check`, scoped lint/format checks, `pnpm docs:check` and relevant Rust
  tests;
- native Tauri screenshot verification in light and dark mode;
- bounded non-mainnet VRSCTEST publication and confirmation test using a
  disposable identity.

## Acceptance criteria

- New publications use authenticated `image/webp`; legacy JPEG profiles still
  display.
- A person can stage avatar, header and description once and never has to delete
  or reselect a field because of transaction partitioning.
- Review shows the full expected cost before the first send: exact update 1,
  estimated update 2 and estimated total.
- A smaller-image option is offered only when it removes the second update, with
  a preview before replacement.
- Every broadcast has a fresh exact fee and explicit approval.
- The known two-multipart layout is split before broadcast and does not surface
  `bad-txns-failed-precheck` to the user.
- An interrupted two-update flow resumes without exposing unpublished media in
  plaintext storage or silently broadcasting it.

## Non-goals

- Increasing the 32 KiB per-image cap.
- Publishing AVIF, PNG or JPEG for new profile media.
- Automatically accepting the estimated second fee.
- Reserving funds for update 2 or promising its fee cannot change.
- Mainnet publishing, multisignature identity publishing or remote image URLs.
- Treating removal from the current profile as erasure from blockchain history.

## Source map

- Native lossy encoder:
  [`images.rs`](../../../src-tauri/src/core/channels/vrpc/identity/profile/images.rs)
- Durable planner and coordinator:
  [`publication.rs`](../../../src-tauri/src/core/channels/vrpc/identity/profile/publication.rs)
- Current frontend encoder:
  [`src/lib/identity/profileImages.ts`](../../../src/lib/identity/profileImages.ts)
- Current profile editor:
  [`IdentityProfileEditor.svelte`](../../../src/lib/components/wallet/sections/identity/IdentityProfileEditor.svelte)
- Profile codec and validation:
  [`codec.rs`](../../../src-tauri/src/core/channels/vrpc/identity/profile/codec.rs)
- Profile preflight and fee calculation:
  [`preflight.rs`](../../../src-tauri/src/core/channels/vrpc/identity/profile/preflight.rs)
- Profile reader:
  [`read.rs`](../../../src-tauri/src/core/channels/vrpc/identity/profile/read.rs)
- Current plaintext account state:
  [`account_state_store.rs`](../../../src-tauri/src/core/wallet/account_state_store.rs)
- Stronghold-backed storage:
  [`stronghold_store.rs`](../../../src-tauri/src/core/auth/stronghold_store.rs)
- Verus Core update storage construction:
  [pbaasrpc.cpp](https://github.com/VerusCoin/VerusCoin/blob/5ec1d83b16253aa56abda5588be76ce2d84dee8f/src/rpc/pbaasrpc.cpp#L16215-L16295)
- Verus multipart evidence parser:
  [notarization.cpp](https://github.com/VerusCoin/VerusCoin/blob/5ec1d83b16253aa56abda5588be76ce2d84dee8f/src/pbaas/notarization.cpp#L996-L1029)
- Verus contextual output precheck:
  [notarization.cpp](https://github.com/VerusCoin/VerusCoin/blob/5ec1d83b16253aa56abda5588be76ce2d84dee8f/src/pbaas/notarization.cpp#L11637-L11720)
