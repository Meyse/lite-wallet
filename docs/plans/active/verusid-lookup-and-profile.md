---
owner: lite-wallet-team
last_reviewed: 2026-09-20
---

# Plan: integrate VerusID lookup and public profile

- Status: ready for implementation; not started by this documentation task.
- Owner: implementing agent / lite-wallet-team.
- Updated: 2026-09-20.
- Product contract:
  [VerusID lookup and public profile](../../product-specs/verusid-lookup-and-profile.md).
- Design:
  [Paper light](https://app.paper.design/file/01M2XWTH7058XSW3C332QYC5R6/p-1-0)
  and
  [Paper dark](https://app.paper.design/file/01M2XWTH7058XSW3C332QYC5R6/p-2-0).

## Brief for the implementing AI

Implement the linked Markdown specification against the latest working source.
Match the approved Paper layout, including the subsequent removal of the page
title, compact search/Link toolbar, missing-profile rows and divider rules. Keep
**Link VerusID** wired to its existing LinkIdentitySheet workflow. Build public
lookup and one shared full-profile destination with the sidebar visible. Wire
the profile's contact and Send actions to existing services and transfer
navigation. Preserve existing management, favorites, provisioning, draft and
session protections. Complete the scoped checks and rendered verification, and
report their evidence limits. Do not implement websites, covers, a Contacts
redesign, or a Send/Convert redesign. Do not commit, push or publish unless
asked.

## Read first and preserve active work

Read repository AGENTS.md, `src/AGENTS.md`, the design-taste contract and
`docs/ui-style-governance.md`; use the required frontend-design guidance for UI
implementation. Also read `docs/context-packs/ui-i18n.md` and the send-flow
context pack before touching transfer entry. Follow backend-local guidance only
if an evidenced gap actually requires Rust changes.

Inspect Git state before editing. This handoff was prepared from `3368053` with
unrelated modifications in WalletLayout.svelte and AssetDetails.svelte. They
belong to other work. Do not revert, overwrite or absorb them. Re-read rather
than assuming that these exact changes or the same base still exist.

Use Node 22.23.2 and the repository-pinned pnpm 11.24.0. Do not add a package or
change the lockfile for this UI feature. Read actual Paper JSX/computed styles
for measurements, then map them to existing wallet primitives and tokens.

## Source map and known gaps

Paths below are relative to the repository root and were checked during the
handoff. Proposed new component names are suggestions, not existing files.

| Area                | Source and integration responsibility                                                                                                                                     |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Shell/navigation    | `src/lib/components/wallet/WalletLayout.svelte`: active sections, account/session resets, transfer request/conflict dialog, retained TransferWizard and navigation lock   |
| VerusID entry       | `src/lib/components/wallet/sections/Identity.svelte`: current header, card/row threshold, linked search, management selection, favorites, provisioning, LinkIdentitySheet |
| Identity session    | `sections/identity/identitySectionSessionState.ts` beneath the wallet component directory: extend session-local tabs and lookup state                                     |
| Link workflow       | `sections/identity/LinkIdentitySheet.svelte` and `src/lib/services/identityLinkService.ts`: reuse unchanged behavior and testnet manual-entry gate                        |
| Linked rows         | `sections/identity/LinkedIdentityRow.svelte`, `LinkedIdentityCard.svelte`, `IdentityAvatar.svelte`: unify row presentation; preserve favorite and pending-update behavior |
| Existing management | `sections/identity/IdentityDetailView.svelte`, `IdentityProfileEditor.svelte`: retain as the Manage destination                                                           |
| Public rendering    | `src/lib/components/wallet/contacts/PublicProfile.svelte`: current avatar/name/description renderer; it is not a standalone page                                          |
| Public lookup       | `src/lib/contacts/service.ts`: resolveContactIdentity; `src-tauri/src/commands/address_book.rs`: existing typed, session-bound resolution                                 |
| Shared state        | `src/lib/contacts/identity.ts`, `profiles.ts`, `session.ts`: canonical keys, shared confirmed-profile cache and session cancellation                                      |
| Saving              | `src/lib/contacts/service.ts`, `src/lib/stores/addressBook.ts`: membership loading and durable, deduplicated contact creation                                             |
| Existing lookup UI  | `contacts/IdentityLookup.svelte`: reusable interaction examples, but its Contacts-loading dependency must not be inherited by public lookup                               |
| Existing navigation | `src/lib/contacts/navigation.ts`: currently opens private Contacts, not a public profile; keep the meanings separate                                                      |
| Send entry          | `sections/TransferWizard.svelte`, `sections/transfer-wizard/types.ts`, `entryContextGuard.ts`: source-prefill exists; canonical recipient intent does not yet exist       |
| UI and locales      | `src/app.css`, `src/lib/components/ui/`, `common/InlineTextActionButton.svelte`, `src/lib/i18n/locales/en.ts` and `nl.ts`                                                 |

The source already has a persistent sidebar and a mounted, hidden/inert transfer
host when the user navigates away. Reuse this current lifecycle. Do not restore
the old full-width focus shell from earlier design notes.

## Step 1 — Model the public destination and session state

- [ ] Define a small typed public-profile destination keyed by ContactIdentity
      (canonical address, chain, network) and a session-bound origin. Keep it
      separate from linked management selection and private contact selection.
- [ ] Add session-local selected tab, linked filter, lookup query, submitted
      query/result/status and scroll/focus restoration. Preserve existing
      IdentitySectionSessionState fields and confirmation reconciliation.
- [ ] Keep the public-profile host in the normal VerusID wallet shell. Opening
      from lookup highlights VerusID; returning restores the Find tab.
- [ ] Reset on lock, account/session/network replacement. Invalidate request
      generations on query edit, replacement and destruction. Do not store DOM
      references in serialized state or introduce persistent browsing history.

Suggested composition: Identity owns tab/list/lookup presentation;
VerusIdLookup.svelte owns lookup form/results; VerusIdProfilePage.svelte
composes shared profile data and page actions. WalletLayout owns any
cross-section navigation and calls its existing transfer request path. Avoid a
second router or a generalized navigation-history framework for this feature.

Checkpoint: state transitions are unambiguous and existing section/transfer
state survives unrelated tab or section navigation.

## Step 2 — Integrate the linked-list and tab layout

- [ ] Remove the redundant visible header and account for existing shell
      padding. Keep an accessible section name and semantic keyboard-operable
      tabs.
- [ ] Replace the count-dependent grid with consistent rows. Add between-row
      separators only; no trailing separator, including filtered single results.
- [ ] Use confirmed avatars or existing initials gradients. Omit missing
      descriptions and center the name; retain truthful loading/failure states.
- [ ] Make linked search flex to fill, Link content-sized, gap 12px, height
      40px. Allow translated button text to grow without overflow.
- [ ] Keep favorites available in their existing quiet star action slot and
      preserve grouping/limit/save behavior. Keep pending updates/provisioning
      visible when applicable; do not replace them with static Paper examples.
- [ ] Keep Manage and row activation routed to existing identity details.
- [ ] Preserve LinkIdentitySheet binding, discovery service, candidate
      filtering, callback, errors/retry and the testnet-only manual link gate.
- [ ] Keep Find available in linked-list empty, loading and failure states.

Checkpoint: compare mixed-profile and single-ID rows with Paper in both themes;
exercise Link and Manage before proceeding.

## Step 3 — Implement explicit lookup and the exact result

- [ ] Reuse resolveContactIdentity; classify typed IdentityNotFound separately
      from transient/unavailable errors using existing error helpers.
- [ ] Match the field/network/button/result layout. Enter and button submission
      use one handler with duplicate prevention; typing performs no lookup.
- [ ] Resolve independently of contact membership and optional profile loading.
      Reuse the canonical profile cache for result enrichment.
- [ ] Render empty, busy, exact result, not-found and unavailable states. Query
      edits immediately invalidate an old result and pending response.
- [ ] Open the profile with a confirmed identity result; Back restores the
      lookup and originating result focus without repeating the request.

Checkpoint: an unsaved, unlinked identity opens while Contacts loading fails; a
slower previous lookup cannot replace the latest query/result.

## Step 4 — Build the canonical full profile and contact action

- [ ] Match Paper's sidebar, 620px content rule, Back, 72px avatar, exact name,
      network, action group, plain-text description and Identity details.
- [ ] Reuse/extract shared profile rendering without changing compact previews
      or private ContactDetail presentation. Do not make a second full-page
      size.
- [ ] Add explicit missing/loading/error/cached states and image fallback. Show
      only confirmed public content; preserve existing pending-edit semantics.
- [ ] Implement identity details from public resolved data; do not require the
      identity to be linked or call a management endpoint merely to view it.
- [ ] Reuse membership and addIdentityContact for Add → Saving → Saved → In
      contacts. Reserve state-label width, handle retry independently and
      preserve private data. Multiple matches are membership, not a prompt to
      merge/edit.
- [ ] Omit the optional website row entirely, and do not add covers or public
      editing controls. A complete implemented profile has no fake extension
      data.

Checkpoint: unlinked, empty-profile, cached-refresh-failed and save-failed
states remain usable; successful contact persistence is the only source of Saved
state.

## Step 5 — Wire Send without changing transfer policy

- [ ] Add a typed, optional recipient intent separate from the asset-only
      TransferEntryContext. Carry canonical identity/network/chain; retain all
      existing source-entry call sites and behavior.
- [ ] Route through WalletLayout's requestTransfer/pendingTransfer decision.
      Include the pending recipient intent in new/replace state so it cannot be
      dropped by the existing conflict dialog. Resume/cancel must not apply it.
- [ ] Apply intent once for the new draft, scoped to its ID/session and a
      compatible Verus destination. Wait for necessary source readiness; do not
      repeatedly assign destinationAddress from a reactive identity prop.
- [ ] Respect destination changes, no eligible source, unsupported address
      kinds, revoked/inactive identities, account/network changes and navigation
      locks. Keep the exact intended identity visible when user correction is
      required.
- [ ] Preserve recipient validation, acknowledgment, review and preflight
      freshness. Never infer an amount, fee, spend authority or token source
      from a profile.

Checkpoint: a new Send carries the identity, an existing draft can be resumed
unchanged, and a later user recipient edit is never overwritten by enrichment.

## Step 6 — Verification and handoff

Use focused behavioral tests for new lookup/session/navigation behavior. Avoid
snapshot-only tests of spacing or tests that merely restate CSS classes.

| Area               | Minimum meaningful coverage                                                                                                                                |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Linking regression | Same sheet opens; discovery/filter/link/callback work; linked list updates; manual linking remains testnet-only                                            |
| Linked list        | Empty, one, filtered one, mixed profiles, seven-plus rows, favorites and long names; correct between-row separators                                        |
| Existing state     | Favorite persistence failure; profile removal/confirmation reconciliation; pending provisioning remains available                                          |
| Lookup             | Empty/Enter/double submit; exact/not-found/unavailable; no contact-load dependency; query race; lock/network/account race                                  |
| Profile            | Empty/partial/failed/broken image/cached content; canonical name/copy; disclosure and Back focus restoration                                               |
| Contacts           | Save pending/success/failure, repeated activation, uncertain commit, existing duplicate membership, session switch                                         |
| Transfer entry     | New compatible Send; no eligible source; incompatible destination; dirty-draft resume/replace/cancel; one-shot prefill; navigation lock and session change |
| Presentation       | Light/dark at 920×620 and 1200×800; English/Dutch; keyboard tabs/results/Back; scrolling and long identifiers                                              |

Existing test starting points:

- `src/lib/components/wallet/sections/Identity.mounted.ts`
- `src/lib/contacts/contacts.test.ts`
- `src/lib/components/wallet/contacts/IdentityPreview.mounted.ts`
- `src/lib/components/wallet/sections/identity/IdentityDetailView.mounted.ts`
- `src/lib/components/wallet/sections/identity/IdentityProfileEditor.mounted.ts`
- `src/lib/components/wallet/sections/transfer-wizard/preflightLifecycle.mounted.ts`

Add focused lookup/profile/recipient-intent tests alongside the owning modules.
The transfer lifecycle suite already covers retained drafts and sidebar
navigation; extend its real production-component path instead of making a second
mock navigation implementation.

Run from the implementation checkout with the pinned runtime:

```sh
pnpm exec vitest run src/lib/contacts/contacts.test.ts
pnpm exec vitest run --config vitest.mounted.config.ts src/lib/components/wallet/sections/Identity.mounted.ts src/lib/components/wallet/contacts/IdentityPreview.mounted.ts src/lib/components/wallet/sections/transfer-wizard/preflightLifecycle.mounted.ts --maxWorkers=1 --no-file-parallelism
pnpm check
pnpm lint:ui
pnpm build
pnpm docs:check
git diff --check
```

Also run newly added tests and the existing management/profile tests affected by
shared rendering changes. Run scoped ESLint/Prettier on changed code/docs rather
than formatting unrelated files. Broaden tests only for actual affected paths or
unresolved failures; document unrelated baseline failures separately.

Capture real rendered screens for the five Paper states in each theme, plus
relevant errors/long content and Dutch controls. Compare at matching viewport
sizes, fix gaps, then retain evidence paths. Browser fixtures prove composition
and mounted behavior, not native wallet calls. Where a configured native test
wallet is available, verify read-only lookup, linking regression, durable
contact save and transfer prefill up to review under its runbook. Do not
broadcast a transaction solely for this UI task. Report unavailable native proof
explicitly.

## Completion criteria

- [ ] Every specification acceptance criterion is addressed with evidence.
- [ ] Link VerusID and all preserved management capabilities still function.
- [ ] New profile/lookup code uses shared services and one canonical page
      layout.
- [ ] No website/cover/Contacts/transfer redesign has slipped into the change.
- [ ] Source changes, tests, rendered screenshots and proof limits are reported.
- [ ] Update relevant implementation docs, record decisions/deviations, and move
      this plan to done only after implementation and validation are complete.
