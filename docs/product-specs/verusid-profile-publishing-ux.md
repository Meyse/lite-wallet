---
owner: lite-wallet-team
last_reviewed: 2026-09-21
---

# VerusID profile publishing: UX corrections

- Status: implemented; local checks and native macOS VRSCTEST retest passed
- Scope: editing and publishing avatar, header image and description on
  supported VRSCTEST identities
- Implementation:
  [execution plan](../plans/done/verusid-profile-publishing-ux.md)
- Baseline: [profile screen](./verusid-profile-screen.md) and
  [WebP/sequenced publishing](../plans/active/verusid-profile-webp-sequenced-publishing.md)

## Outcome and authority

A person can see the profile they are changing, approve its cost and understand
whether it is editing, publishing or finished without interpreting transaction
mechanics. One-update and two-update publication feel like the same retained
task. Each fee-bearing update still requires its own explicit approval.

This is the target specification requested after the 2026-09-21 native test. It
supersedes the earlier editor's disabled Connections rows, separate description
page, repeated draft/file metadata, oversized single-update fee,
unavailable-fiat dash and generic continuation copy. The linked product and
implementation documents now describe the delivered corrections. The public
profile layout, signing rules, image validation, encrypted continuation and
on-chain confirmation requirements remain authoritative.

Apply Max's design principle: minimum reading required for confident use. Reuse
the wallet's existing components and tokens. This specification does not
authorize unrelated profile features, a visual rebrand or transaction-protocol
changes.

## Evidence and issue coverage

The earlier run used the actual macOS Tauri webview, real Rust IPC and live
VRSCTEST services in `mijn app`. Both initially empty profiles acquired all
three fields: `player1.VRSCTEST@` in one transaction and `player6.VRSCTEST@` in
two. The latter retained its remaining header through lock, app restart and
unlock. Total paid was 0.30469 VRSCTEST. Publication succeeded; status
presentation failed.

The local, ignored evidence bundle is `output/profile-ux-audit-20260921/`:
`report.html`, `report.md`, `verification.json`, `checkpoints.json` and 53
inspected screenshot checkpoints. Capture numbers below refer to that bundle.
The findings are recorded here so the specification does not depend on
committing generated screenshots.

| Finding | Observed problem                                                                                                        | Required correction                                                                        | Evidence                       |
| ------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | ------------------------------ |
| F1 · P1 | Each successful send briefly returned to ready/review/discard actions                                                   | Sending establishes pending immediately; older responses cannot regress that step          | 17, 41, 51                     |
| F2 · P1 | Final confirmation left the editor waiting; Check confirmation did not finish                                           | Settle against confirmed evidence and return to the complete profile                       | 54–56 and backend verification |
| F3 · P2 | One-update and final-step copy promised a nonexistent next update; identity context disappeared                         | Status and field rows follow the actual plan; identity remains visible                     | 18, 38, 46, 52                 |
| F4 · P2 | Tiny disconnected previews, repeated metadata/actions, disabled Connections and separate description/crop setup screens | Edit and review the combined profile with independent field controls                       | 05–13, 23–33                   |
| F5 · P2 | A useful one-update alternative appeared after a dense two-update explanation                                           | Surface the backend-proven alternative before that explanation and let the user compare it | 35–36                          |
| F6 · P2 | Fee hierarchy, repetitive explanations, duplicate Back actions and unavailable fiat increased reading                   | One recognizable result, compact cost summary and action appropriate to the current step   | 14, 35, 39, 47, 49             |

This was an agent-executed functional/visual test, not a measured user study.
The OS file chooser was not exercised: a File was attached to the actual input,
then real crop/encode/review/sign/broadcast actions were used. No Windows/Linux,
release-build or complete accessibility coverage is implied.

### Correction acceptance

The separate Astra extra-high implementation task delivered F1–F6; the
originating task integrated its verified correction delta and rebuilt the native
app. The fresh 2026-09-21 retest passed on `game564498.gamesession2@` in one
update and `gamesession2.VRSCTEST@` in two, including explicit image comparison,
lock/full restart, retained header and fresh second approval. All three sends
stayed pending and both final profiles appeared automatically. Canonical field
digests, transaction grouping, unchanged controls and completed continuation
were asserted. Total paid was 0.41193 VRSCTEST.

The new ignored bundle `output/profile-ux-retest-20260921/` includes the report,
machine-readable assertions, 50 inspected native captures and four historical
comparison captures. Native light/dark minimum-window checks and the actual
macOS avatar chooser passed. Long Dutch text and failure injection retain
browser/mounted/Rust test proof. Manual checking worked while pending; final
automatic completion won the attempted manual-click race, so manual final
completion is supported by mounted tests. Full proof levels and remaining
platform limits are recorded in the execution plan. This remains agent-executed
testing, not a measured human usability study.

## 1. Visual profile editor

### Composition

- Keep one destination-labelled **Back to profile** control and a visible
  identity name. Use the existing profile header, avatar overlap and description
  hierarchy. Omit public-profile tabs and linked-wallet badges from the editing
  task; the public profile itself retains them.
- Use the established 20px page gutters and at most 800px content width. Header
  and avatar proportions follow the current profile screen. At 920×620, keep the
  primary footer action visible; allow the editor body to scroll inside the
  shared `ScrollArea` instead of shrinking text or controls to fit.
- Show the effective draft as a combined profile, including unchanged public
  fields. Header and avatar each have a clear Add/Change control in their own
  image area. Text labels may collapse to a familiar Lucide image control on an
  existing image only when its accessible name remains explicit.
- Place the optional description directly below the name, using the established
  input treatment. No separate description page or Add to draft button.
- Each field remains optional and independently editable. Changing only a
  description must not require selecting either image. Unchanged fields are
  excluded from the publication intent.
- Show **Unpublished changes** once when the effective profile differs from the
  confirmed profile. Do not repeat Draft, WebP or byte counts on every field.
  Disable **Review changes** when there are no effective changes or validation
  errors. Keep it the only emphasized primary action.
- Hide Websites, Social accounts, Addresses and VerusIDs from this editor until
  those actions are implemented. Do not replace them with promotional or “coming
  soon” content.

### Local edits and removal

Description edits update the existing session-scoped draft. Normalize and
validate with the existing 160-grapheme/1,024-byte rules. Preserve typing and
focus while editing; show the count near the limit and an actionable error when
exceeded. Do not erase text merely because it is invalid for review.

Place image replacement/removal in that image's contextual controls, not in
three permanent red rows. Use separate meanings: **Remove image** changes the
effective profile to no image; **Undo change** restores that field's confirmed
value. Removing an unpublished image costs nothing. Removing published content
is an unpublished removal until reviewed and confirmed. The review must
explicitly label a removal, because an empty preview alone is ambiguous. Allow
undo without a transaction. Do not silently retain the old double-Remove
semantics behind a newly labelled action.

Unreviewed drafts retain the current storage boundary: memory only, isolated by
wallet session and identity, surviving navigation but clearing on lock,
wallet/network change or restart. Do not say “Saved for later” before the
backend has persisted a reviewed plan. A draft-status disclosure may explain the
session lifetime; do not add generic warning paragraphs to the editor.
Persistence of unreviewed drafts is outside this change.

### Image selection and cropping

The Add/Change control invokes the native file chooser directly. Cancel returns
to the unchanged editor. Only show the crop view after a valid image is loaded;
do not make users pass through an empty crop canvas with disabled zoom.

The crop view shows the real avatar/header shape, image and working zoom. Keep
drag and arrow-key support. Use **Use image** to apply the crop and **Cancel**
to leave the prior effective field untouched. Replacement/decode/encoding
failure also preserves the prior field. Keep a single Back/Cancel path rather
than duplicate navigation controls.

Supported sources, decoding bounds, white matte, metadata removal, native WebP
encoding and legacy JPEG reading stay unchanged. File types belong in the
chooser filter; actual input failures explain the limit that failed. Output
dimensions, 32 KiB image caps and codec details belong under **Image
requirements**, not above every empty crop. Technical limits remain enforced
even when their explanatory text is hidden.

## 2. Review and image alternatives

### Shared review

Keep the identity and a combined proposed-profile preview above a compact cost
summary. Use the same composition for one and two updates. Mark proposed
removals and make the scope of the current approval explicit. Review preserves
all staged values when going back. Any edit or candidate change invalidates the
previously prepared approval and requires fresh review.

Use one **Back to edit profile** control. Keep the reviewed primary action at
the bottom, in the same position across steps. For a resumed header-only step,
Back returns to the retained publication progress view without discarding it.

Show the public-history consequence once, before every paid approval: **These
changes are public. Earlier versions remain on-chain.** It must not be hidden in
a tooltip. Do not add a second paragraph explaining the same fact.

### Cost presentation

| Review           | Always visible                                                                                             | Primary action                                                                  |
| ---------------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| One update       | Exact transaction fee for the selected changes                                                             | Publish profile                                                                 |
| First of two     | First update's exact fee, header's estimated fee, estimated total; “Two updates. You’ll approve each fee.” | Publish avatar and description, or Publish avatar when description is unchanged |
| Remaining header | Fresh exact header fee; “Update 2 of 2”; a changed-estimate explanation only if it changed                 | Publish header                                                                  |

Use the actual changed-field set for labels; do not promise to publish a field
that is unchanged. Exact versus estimated must be distinguishable through text,
not color. On the first split review, the three cost rows and the separate-fee
approval sentence must be visible together at 920×620 without opening a
disclosure. These costs must not compete with an oversized single-update amount.

Use the existing integer-based fee formatter everywhere, preserving all
significant precision and removing trailing zeros. For example, show
`0.08522 VRSCTEST` rather than `0.08522000 VRSCTEST`. Do not round to five
places or convert the amount through floating-point arithmetic. Fiat is
secondary when a rate for VRSCTEST and the selected display currency exists;
omit that line when it does not. Never substitute a mainnet VRSC rate.

Move spendable balance, quote metadata and an unchanged earlier estimate into
**Fee details**. Show insufficient funds or a changed second fee next to the
decision where it matters. This reduces routine reading without concealing the
required exact first fee, estimated second fee or total.

### Offer a one-update alternative

Preserve the current balanced candidate as the initial selection. A separate
smaller candidate may be offered only when the backend proves that it reduces
two updates to one and lowers the estimated total. Bytes or a guessed multipart
threshold must never determine the user-visible update count.

When available, put one quiet **Publish in one update** option before the cost
summary, with **Compare images** and an estimated saving. Replace “Two updates
needed” with the factual “Two updates” for the currently selected version. The
available alternative must be visible without reading the explanation of the
two-update protocol.

Comparison shows the same crop and dimensions at the same display scale, with
**Current image** and **Smaller image** labels. Include the one-update estimate
and saving. Byte counts are optional details. **Use smaller image** selects the
candidate and obtains a fresh exact review; it never publishes. **Keep current
image** leaves the draft intact and also requires a valid current review before
publishing. Replanning failure cannot discard the original image.

This increment does not automatically select additional lossy compression. The
one example in the audit is insufficient to establish a general visual quality
floor. Retain the existing bounded encoder ladder and explicit visual choice; a
future adaptive default requires a separate representative image quality
evaluation. This decision does not block correcting F5 now.

## 3. One publication progress view

The identity stays visible throughout. Use the same compact field/status rows as
the state changes; the person should not need to interpret a new page after
every update. For two updates, show the current update number and total, plus
the actual field names. For one update, omit the unnecessary “1 of 1” label.

| State                             | Visible result/status                                                                                 | Available action                                                 |
| --------------------------------- | ----------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Preparing/signing                 | Publishing profile, or the actual fields being published                                              | No second Publish or discard action                              |
| One update submitted              | Waiting for confirmation                                                                              | Back to profile; Check confirmation; Transaction details         |
| First of two submitted            | Update 1 of 2; avatar/description waiting; header saved for later                                     | Back to profile; Check confirmation; Transaction details         |
| First confirmed                   | Avatar/description published; header ready; Update 2 of 2                                             | Review header fee; Back to profile; contextual discard remainder |
| Final update submitted            | Update 2 of 2; avatar/description published; header waiting                                           | Back to profile; Check confirmation; Transaction details         |
| Final confirmed                   | Canonical complete profile; one quiet Profile updated status                                          | Normal Edit profile                                              |
| Refresh unavailable while pending | Retain last known pending state; Couldn’t check confirmation                                          | Check confirmation; Back to profile                              |
| Review expired before sending     | Changes retained; Review expired. Review the fee again.                                               | Review fee                                                       |
| Backend confirms conflict/expiry  | Retained changes require review; explain changed public profile or expired transaction as appropriate | Fresh review; contextual discard when backend permits            |

No “remaining changes” or “next update” copy appears for a one-update plan or
after sending step two. No full transaction hash occupies the main progress
view. **Transaction details** contains the ID and copy action for the correct
step; previous receipts remain distinguishable from the pending transaction.

Back never cancels a submitted transaction or deletes a reviewed plan. Label it
**Back to profile**, not Keep for later or an ambiguous Done. Returning to the
public profile shows confirmed content plus one concise pending/continue notice;
do not duplicate the same explanatory paragraph in multiple places. A projected
header in the progress view is labelled as pending/ready and must never
masquerade as confirmed public content.

Discard is unavailable while the backend considers a transaction pending,
including an ambiguous broadcast response. Once allowed, hide it in a secondary
plan action and require a concrete confirmation explaining which unpublished
fields will be removed. It never undoes the confirmed first update. Failure to
discard leaves all remaining content available.

## 4. State and evidence invariants

These requirements take precedence over visual simplification:

1. A successful send synchronously establishes a pending presentation for the
   submitted plan/step/transaction. An older ready response must not reopen
   review, enable discard or clear that receipt.
2. A manual refresh made during an existing refresh schedules a follow-up; it is
   not silently dropped. Out-of-order responses from an old session, identity,
   plan or request generation cannot overwrite newer state.
3. Retain an explicit completed result long enough for the UI to consume it. The
   current backend returns Complete once and removes the stored plan; later
   reads can return null. Null alone, disappearance of a pending cache entry, or
   `settledTxids` alone is not proof of successful publication. The latter also
   includes transactions resolved through conflict/expiry.
4. Final success requires authoritative confirmation for this submitted
   transaction and matching canonical profile values/digests, including
   removals. If another observer consumed Complete, reconcile the known receipt
   against the existing authenticated confirmation path. When proof is
   unavailable, retain the pending state and allow another check.
5. After proof and canonical refresh, close the editor once, settle the matching
   pending receipt and show the complete profile. Automatic refresh and Check
   confirmation use the same transition. A late response cannot reopen it.
6. A fresh canonical conflict or reorg is not an old response. Do not implement
   “never regress” as a blanket rank ordering that rejects legitimate stale or
   recovery states. Backend evidence continues to govern recovery.
7. Step two is reviewable only after canonical confirmation of step one. A saved
   plan restores bytes and intent, never a preflight, signature or permission to
   send. Refresh, relaunch and unlock cannot publish automatically.
8. Keep current support limits: VRSCTEST, supported editable identity, matching
   single primary signer and threshold one. Preserve unrelated content, primary
   addresses, authorities and identity controls.

## 5. Accessibility and visual acceptance

- All labels and errors use existing i18n patterns, with both English and Dutch
  keys. The English text above is the target copy, not permission to hard-code
  it in components. Centralize repeated field/status/action definitions.
- Keep default arrow cursors for actions, native text cursors for input,
  recognizable Lucide icons, visible keyboard focus and existing disabled
  treatment. Do not replace necessary labels with unexplained symbols.
- Keyboard users can select/cancel an image, crop with arrows, zoom, edit the
  description, review, compare, publish and return. Image menus have explicit
  accessible names. Focus returns to the initiating image control after cancel
  and moves to the new stage heading after a real stage change.
- Announce meaningful stage changes with a polite status region; do not repeat
  the announcement on every poll or move focus while someone is reading. Errors
  remain associated with the field/action that caused them.
- Verify light and dark at 920×620, normal 1040×680 and a wider window. At the
  minimum size, no action/footer is clipped; scrolling is local and visible.
  Check real contrast and long Dutch labels, not only DOM existence.
- Compare with the captured baseline: one identity context, one draft indicator,
  one primary action, no disabled future sections, no repeated byte/codec
  metadata, no permanent red removal rows, and visible combined preview before
  payment. Do not claim a reduction in task time without measurement.

## Completion criteria

To mark this work complete, F1–F6 must each have passing acceptance evidence in
the [execution plan](../plans/done/verusid-profile-publishing-ux.md). Native
testing must again complete both transaction paths and a restart between steps,
with screenshots of every meaningful state in both appearances. A successful
chain write with a stuck completion screen fails acceptance. Record any OS
chooser or platform evidence gaps explicitly; fixtures alone do not close native
coverage.
