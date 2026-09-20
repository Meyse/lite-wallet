---
owner: lite-wallet-team
last_reviewed: 2026-09-20
---

# VerusID lookup and public profile

Status: implementation handoff. The user approved the focused Paper direction
and its follow-up refinements. This document specifies future implementation;
the documentation task does not implement wallet behavior.

Implementation plan:
[`../plans/active/verusid-lookup-and-profile.md`](../plans/active/verusid-lookup-and-profile.md).

## Outcome and scope

Users can open **VerusID**, switch between **Linked IDs** and **Find a
VerusID**, look up an identity they have not linked or saved, and open its
public profile. The full profile has one consistent layout inside the normal
wallet sidebar.

Implement the tab structure, linked-list presentation, explicit lookup, exact
result, standalone profile, profile contact action, and profile Send entry.
Preserve existing identity management and linking behavior.

Do not implement website publishing or verification, cover images, a directory
index, autocomplete, social features, a new Contacts layout, or a redesign of
Send/Convert. Adding new profile entry actions to Contacts or recipient previews
is a separate increment. This increment opens the new full page from lookup; its
shared navigation contract must not require another layout for later entry
points.

## Design authority

Use this file and the following Paper project together:

- [Approved light screens](https://app.paper.design/file/01M2XWTH7058XSW3C332QYC5R6/p-1-0)
- [Approved dark screens](https://app.paper.design/file/01M2XWTH7058XSW3C332QYC5R6/p-2-0)

The file is **Lite Wallet · VerusID lookup & profile**. Each theme has five
920×620 artboards. The light page also has a short layout-rule note.

| Artboard title, before the dark-mode suffix     | What it establishes                                                                           |
| ----------------------------------------------- | --------------------------------------------------------------------------------------------- |
| 01 · Linked IDs · With and without profile data | Three aligned rows; two have initials and no description; adjacent search and Link controls   |
| 01b · Linked IDs · Single ID, no divider        | A lone row has no separator                                                                   |
| 02 · Find a VerusID · Start                     | Empty, disabled lookup; network beside the field label                                        |
| 03 · Find a VerusID · Exact result              | One exact result and View profile                                                             |
| 04 · VerusID profile · Canonical layout         | Full profile with the sidebar, contextual Back, identity, actions, description and disclosure |

The latest approved refinements are mandatory: **no visible VerusID page title
above the tabs**, content moved up, missing profile data omitted from linked
rows, dividers only between rows, and a 12px search/button gap.

This focused specification supersedes the tab/list/profile-layout portions of
[`verusid-profile-discovery.md`](verusid-profile-discovery.md) and its older,
broader Paper project. In particular, do not copy the older full-width profile
from Send. The broader document is background, not a checklist for this task.

Paper is static design evidence. Its identities and avatar are sample content;
do not ship them as default wallet data. The website row is explicitly a future
field: **omit it in this implementation**, including its separators and empty
space. The profile must be complete without it.

Paper does not depict every existing favorite, provisioning, loading, or pending
update state. Omission from an artboard does not authorize removing that
behavior. Use the existing semantic tokens and primitives rather than literal
Paper CSS.

## Layout and navigation

### VerusID section

- Keep the existing VerusID sidebar entry and icon. Remove only the repeated
  visible page heading, not the navigation label or accessible page name.
- Show **Linked IDs** and **Find a VerusID** as local tabs. Linked IDs is the
  default for a new unlocked session; remember tab selection within that
  session.
- Both tabs remain reachable when linked IDs are empty, loading, or unavailable.
  A linked-list failure must not prevent public lookup.
- Keep each tab's query, result/selection and scroll state separate. Moving
  between tabs must not link or save an identity or clear a valid lookup result.
- At 920×620, preserve the 244px sidebar and a 620px content lane starting at
  window x=272. Tabs begin at approximately window y=40, with no heading gap.
  Account for the shell's existing drag-region spacing; do not add it twice.
- The tabs are 36px high, with a quiet baseline and an active underline. Main
  groups have 24px spacing. Use Google Sans and existing light/dark tokens.
- At larger desktop sizes keep this same content-width rule for all full
  profiles: 620px maximum, left-aligned within the available main area with 28px
  minimum gutters. Wider windows add space, not a second profile layout.
- Use the shared ScrollArea for long content. Keep Back and the active tab
  controls reachable without horizontal scrolling.

### Linked IDs

Use one row presentation at every list count instead of switching between cards
and rows at seven IDs. Preserve current ordering, favorite grouping/limit,
loading, refresh, provisioning and pending-profile reconciliation semantics.

The toolbar contains **Search linked IDs** and **Link VerusID** in one flex row:
search grows into the available width; the button remains content-sized and does
not shrink; gap is 12px; both controls are 40px high. The English Paper button
is 122px wide, but translations must be allowed more width. Do not use
space-between to create a second, larger gap. Linked search filters the current
list; it does not query public identities.

Rows have a 48px avatar slot, a flexible name/description area, and aligned
trailing management controls. Paper uses 14px gaps and 18px vertical padding.
Preserve a fixed trailing-action lane across rows.

| Available profile data           | Row presentation                                                                                              |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Confirmed avatar and description | Photo, exact display name, one concise description line                                                       |
| No avatar                        | Existing IdentityAvatar initials and deterministic gradient                                                   |
| No description                   | Name only, vertically centered; no empty second line or No profile placeholder                                |
| Neither avatar nor description   | Initials and name; identical action alignment                                                                 |
| Loading or failed enrichment     | Identity remains usable; do not report failure as confirmed absence; preserve existing loading/retry feedback |

Render subtle separators **between adjacent visible rows only**. No top line, no
line below the last row, and no divider for a single visible result, including
when search reduces a larger list to one. The tab underline is unaffected. Do
not add separators around favorite-group headings.

The row's primary activation and **Manage** action continue to open the existing
identity management detail. They do not become public-profile lookup or Link.
Preserve favorite toggling through its existing accessible star control in a
fixed trailing slot; it is an existing capability omitted from the happy-path
Paper row. Retain its busy state, persistence-before-success, rollback and
limit. Keep the new Manage label and existing favorite control from becoming
nested buttons or overlapping click targets.

### Link VerusID is unchanged

This button remains the existing **LinkIdentitySheet** entry point:

1. Set the existing link-sheet open state.
2. Discover linkable identities using the current service and eligibility rules.
3. Link through the current link action and update the linked list through
   `onLinkedChange={applyLinkedIdentities}`.
4. Preserve candidate filtering, busy/linked states, errors, retry,
   cancellation, and current behavior after a successful link.
5. Preserve `allowManualLinkEntry={walletNetwork === 'testnet'}`. Do not
   introduce manual linking on mainnet.

The visible position and size change; the workflow and backend contract do not.
Finding a public VerusID must never implicitly invoke linking or open this
sheet. Empty-list Link actions must use the same existing flow.

## Find a VerusID

The form contains **Full VerusID**, active network context, a text field, and
**Find profile**. Keep the short example under the field. Use a deliberate click
or Enter submission; no requests on hover or every keystroke.

Reuse `resolveContactIdentity` and its session-bound backend resolver. Trim
surrounding whitespace and use its existing name normalization. Do not invent a
name suffix, choose a similar name, search another network, or build a
directory. The UI teaches full-name entry; existing supported identity-address
resolution may remain available without advertising a new lookup mode.

| State                   | Required behavior                                                                                   |
| ----------------------- | --------------------------------------------------------------------------------------------------- |
| Empty                   | Find profile disabled; no empty result card                                                         |
| Looking up              | Keep input/query visible; stable button width; Looking up…; prevent duplicate submission            |
| Exact result            | One canonical identity, optional avatar/description, and View profile; row activation also opens it |
| Not found               | No VerusID found; keep query and network visible; allow correction                                  |
| Unavailable             | Couldn't look up this VerusID; retain query and offer Try again                                     |
| Query changed           | Invalidate the previous result and request generation immediately                                   |
| Session/network changed | Clear query/result/navigation state; ignore every late response from the old context                |

Classify only the resolver's typed not-found result as absence. Provider errors,
timeouts and malformed/unavailable responses are failures, not not-found.
Suppress stale results after query changes, even before a new submission.

Public resolution and profile enrichment must work if Contacts is empty, still
loading, or failed. Do not copy the existing IdentityLookup component's combined
`Promise.all([resolveContactIdentity(...), loadContacts()])` as a dependency for
this new surface. Contact membership loads independently for the profile action.

Result activation opens the canonical full profile. Back to search restores the
query, result, scroll position and keyboard focus without another lookup.
Persist no public search history, query or profile destination to disk.

## Full public profile

The profile is a normal wallet detail page with VerusID active in the sidebar.
It is independent of saving the identity as a contact, linking it, or
controlling it. All future entry points must reuse this page and its width; only
the origin and applicable actions change. Compact hover previews remain separate
components.

Display the following, in order:

1. **Back to search** for this increment. Use a contextual return contract
   rather than hard-coding a second full profile for another origin.
2. 72px published avatar or the existing identity fallback.
3. Full resolved VerusID and icon-only copy action; active network below. Paper
   uses 28px/32px, weight 600 for the name. Long names wrap rather than hiding
   the distinguishing parent/suffix.
4. **Send** and **Add to contacts**, or its membership/save state.
5. Full confirmed description as plain text, retaining line breaks.
6. Collapsed **Identity details**: canonical identity address, network/chain
   context, and actual known identity status. Use IdentifierText and CopyButton
   for raw identifiers; always copy the complete value.

At 920×620, the Back row starts at approximately x=272/y=36; the avatar/content
group starts at x=272/y=108, width 620px, with 20px group spacing. Do not put
the profile in a dialog, a transfer aside, or a full-window layout without the
sidebar. The top-level VerusID heading removed from tab screens must not
reappear here.

Optional profile enrichment cannot block the known identity, Back, or contact
saving. Keep confirmed content during refresh. Confirmed absence uses the
fallback and omits the description; failed loading shows a concise retry message
instead of claiming that no profile exists. A broken image falls back cleanly.
Pending edits must not be presented as confirmed public content.

An identity's photo, linked status, or contact membership is not a trust badge.
Display revoked/inactive status when known and respect existing transfer
eligibility checks. Do not infer management authority from profile availability.
Profile editing, unlinking, authorities and provisioning stay in existing
Manage.

### Contact action

Reuse `matchingContacts`, `loadContacts`, and `addIdentityContact`:

- Known absent: Add to contacts.
- Membership loading/failed: a bounded checking/retry state; do not guess
  absence.
- Saving: stable action width, duplicate activation blocked.
- Persisted: brief Saved feedback, then quiet In contacts status.
- Failed: keep the page open, report Couldn't save contact and allow retry.

Saving is optional and stays on the profile. Reconcile uncertain saves and
retain canonical deduplication; never overwrite private notes or existing
endpoints. Multiple matching contacts mean In contacts, not an arbitrary
merge/edit choice. This increment does not add a private contact editor to the
public profile.

### Send action

Send starts the current transfer flow with this canonical identity carried as a
recipient intent. It does not submit funds, pick an amount, skip recipient
validation, or bypass review. Let the user select an eligible source normally.
Only apply the identity to a compatible Verus destination; do not manufacture an
ETH/BTC address or silently change networks to make it fit.

Use WalletLayout's existing transfer request/draft-conflict flow. A retained
dirty draft requires its existing explicit resume/replace choice. Resume retains
that draft exactly; cancel keeps the profile; replace/new carries the new
recipient. When navigation is locked during submission, profile actions must
respect it. Do not create another transfer host or change Send/Convert sidebar
policy.

The current source's TransferEntryContext contains asset/source fields only. Add
a separate typed recipient intent, or a backward-compatible entry contract; do
not fabricate a coin/channel context from an identity. Apply it once for the new
draft and current session when the destination is compatible. Later user edits
or reactive data refreshes must not reapply/overwrite the recipient. All
existing validation, unsaved-recipient acknowledgement and preflight freshness
rules continue to apply.

## Shared state and accessibility

Identity keys are the exact canonical identity address plus network and chain.
Use `identityKey`; Base58 addresses are case-sensitive. Scope tab/profile/origin
state to wallet account/session/network, and clear it on lock or replacement.
Reuse the shared bounded profile cache instead of fetching separately per view.

Use semantic tabs, labelled inputs and keyboard-operable result/Manage actions.
Restore focus after Back; Escape closes a disclosure or sheet before any parent
navigation. Announce lookup and save outcomes without moving focus on
completion. Provide visible focus, normal arrow cursors for actions, text
cursors in inputs, and selectable/copyable public data. Use the shared UI
primitives, ScrollArea, Lucide icons, InlineTextActionButton, and English/Dutch
translation keys.

## Acceptance criteria

- [ ] Approved light/dark frames match at 920×620; no redundant heading or old
      heading-sized gap remains. Larger desktop windows use the same profile
      rule.
- [ ] Search and Link controls have equal height and a 12px gap; Dutch fits.
- [ ] Linked lists use rows at every count; mixed/missing profile data, one
      item, a filtered single result, favorites and long names work without
      misalignment.
- [ ] No divider appears below a last/only row.
- [ ] Existing Link sheet, testnet-only manual entry, linking updates, Manage,
      favorites, provisioning and profile confirmation behavior still work.
- [ ] Unlinked/unsaved identities resolve and open without a contact/link write.
- [ ] Empty, busy, not-found, unavailable and stale-query states are distinct;
      Contacts failure does not block lookup or public viewing.
- [ ] Back restores search state and focus; every full profile retains the
      sidebar.
- [ ] Missing, failed, cached and pending profile data are handled truthfully.
- [ ] Contact success follows durable persistence; repeated/retried saves do not
      duplicate records or alter private notes/endpoints.
- [ ] Send prefills only an eligible new draft, applies once, and preserves all
      existing conflict, session, navigation-lock and validation rules.
- [ ] Late requests cannot repopulate a locked or replacement wallet session.
- [ ] Keyboard, copy, long content, empty states and both locales work in both
      themes.
- [ ] Website/cover placeholders and the broader Contacts/preview redesign are
      absent.

## Evidence boundary

This handoff was checked against the Paper inventory/styles and the working
source at `3368053` on 2026-09-20. WalletLayout and AssetDetails had unrelated
uncommitted changes; re-inspect the actual implementation checkout. Current
WalletLayout already shows the sidebar during transfers and retains drafts.
Older descriptions of a sidebar-free transfer shell are stale.

Paper screens establish composition only. They do not prove runtime navigation,
keyboard behavior, native wallet resolution, encrypted persistence or transfer
prefill. The implementation plan defines those checks separately.
