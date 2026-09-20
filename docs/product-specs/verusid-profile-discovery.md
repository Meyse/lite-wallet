---
owner: lite-wallet-team
last_reviewed: 2026-09-20
---

# VerusID profile discovery

Status: proposed product specification and Paper design, 2026-09-19. This task
authorizes documentation and design only. No wallet behavior is implemented by
this document or by its Paper artboards.

> **Superseded for the implemented increment:**
> [`VerusID lookup and public profile`](./verusid-lookup-and-profile.md) and its
> focused Paper file are authoritative for the VerusID tabs, linked-list rows,
> public lookup, canonical profile layout, contact action, and Send entry. This
> broader document remains background for later Contacts, preview, website, and
> cross-flow work.

Paper:
[Lite Wallet · VerusID profiles & discovery](https://app.paper.design/file/01M2X49JSSK4JP0DX2XAA51VX2/p-1-0).
The numbered artboards are a state storyboard, not a running wallet or a wired
interactive prototype. Identity names and website states are illustrative;
wallet addresses and authority identifiers use nonfunctional placeholders.

## Outcome

People can look up a VerusID, read its complete public profile, optionally save
it to Contacts, and return to their original task. The same public profile is
available before and after saving, and whether or not the wallet controls the
identity. Contacts remains a compact private address book.

The three surfaces have distinct jobs:

| Surface             | Purpose                          | Content                                                                                     |
| ------------------- | -------------------------------- | ------------------------------------------------------------------------------------------- |
| Identity preview    | Recognize an identity in context | Avatar, exact resolved name, short description, View profile, contact action/state          |
| Contacts            | Find and manage saved contacts   | Compact rows, saved destinations, private notes, View profile                               |
| Full public profile | Learn about one VerusID          | Avatar, exact name, full description, published website, identity details, relevant actions |

Looking up an identity does not link it to the wallet or save a contact. Saving
a contact does not establish ownership, publish anything, or modify a Send
draft. Deleting a contact leaves the VerusID and its public profile unchanged.

## Relationship to existing specifications

The [Contacts specification](./contacts-and-verusid-profiles.md) remains the
authority for encrypted persistence, address-only contacts, canonical identity
associations, duplicate contacts, private notes, and saved recognition styling.
This proposed follow-up supersedes these presentation decisions when
implemented:

- Full public descriptions move from selected-contact detail to the independent
  profile. Contact detail retains saved destinations and private notes.
- Preview **View profile** remains available in every contact state. After
  saving, the secondary action settles to **In contacts**, replacing the current
  preview's **View in contacts** navigation.
- A public profile opens independently of a contact match; opening it never
  depends on successfully loading or saving Contacts.
- Lookup becomes accessible under VerusID, outside the Add contact flow.

The [verified website specification](./verusid-verified-websites.md) remains the
authority for discovery, proof policy, owner setup, checks and publication. For
this follow-up, the independent full profile becomes its primary public
presentation surface. Its previous full-contact-profile placement moves here.
Website setup stays in the existing owner's Edit profile flow. The default
compact preview contains no website row or website-check action; this
deliberately keeps website inspection on the full profile. No new website-fetch
behavior is introduced by passive contact browsing or hover.

## Scope and navigation

Keep the existing **VerusID** sidebar item. Add two local tabs:

- **Linked IDs**: existing linked-identity management. The label avoids implying
  that every linked identity is controlled by this wallet.
- **Find a VerusID**: deliberate public lookup on the active supported network.

The initial VerusID entry opens Linked IDs, preserving its existing purpose.
Remember the selected tab during the current unlocked session. Opening a public
profile from elsewhere does not overwrite the tab's saved query or selection.
The Find tab remains available when there are no linked IDs.

Create one internal public-profile destination, keyed by canonical identity
address plus network and chain. It is reusable from search, Contacts, inline
previews and linked identity management. A profile entered through Contacts or
search uses the normal wallet shell with VerusID highlighted. The subsequent
approved direction requires every full profile to retain this sidebar and
content-width rule, including any future entry from Send; only the contextual
Back destination and applicable actions change.

External share links, an unauthenticated wallet mode, a web profile site, a
public directory index, suggestions of popular people, social feeds, messaging,
and new profile publishing formats are outside this increment. Public data does
not imply that the wallet's existing session requirement is removed.

## Lookup journey

1. Open VerusID → Find a VerusID.
2. Enter a full name such as `alex.example@` and activate **Find profile** or
   press Enter. Display the active network beside the lookup context.
3. Resolve through the existing authoritative identity resolver. Show one exact
   match with its canonical name, available avatar and a short description.
4. Activate the result or its **View profile** action to open the full profile.
5. **Back to search** restores the query, result, scroll position and focus.

Use existing resolver normalization; do not invent suffixes, silently choose a
similarly named ID, or search a different network. Partial-name autocomplete
requires a separately specified index and is not implied by this search field.
An identity-address lookup may reuse existing supported resolver behavior, but
the first-version UI teaches full-name entry only.

| State              | Presentation and behavior                                                                                   |
| ------------------ | ----------------------------------------------------------------------------------------------------------- |
| Empty              | Full-name field, short example, network context; Find profile disabled until input is nonempty              |
| Looking up         | Keep input and submitted query visible; stable button bounds with Looking up…; coalesce repeated submission |
| Exact match        | One result; canonical name visible independently of profile-image availability                              |
| Not found          | No VerusID found; retain query; prompt to check the full name and displayed network                         |
| Lookup unavailable | Couldn't look up this VerusID; retain query and provide Try again; do not call it not found                 |
| New query/session  | Discard obsolete results; only the most recent request for the active session may update the view           |

Viewing and searching create no persistent public browsing history. Keep lookup
state in the current session and clear it on lock, account replacement or
network change. Any future history feature requires an explicit privacy
decision.

## Full profile

The profile is a normal detail view with one clear subject. At 920×620 use the
existing 244px sidebar and approximately 620px content lane. The profile body
scrolls when content is longer than the available height; Back remains
reachable.

Display, in order:

1. Contextual Back action.
2. Published avatar or existing deterministic fallback.
3. Full resolved VerusID with copy action, then network context.
4. Context-relevant actions.
5. Full published description, rendered as plain text.
6. Website row, when a published record exists and the website extension is
   implemented.
7. Collapsed **Identity details** containing canonical identity address,
   chain/network and known identity status. Copyable values remain selectable.
   Do not reproduce the full authority-management interface here.

| Entry/context              | Actions                                                                                                       |
| -------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Search, unsaved identity   | Send; Add to contacts                                                                                         |
| Search, saved identity     | Send; In contacts; quiet Edit contact action                                                                  |
| Contacts                   | Back to contacts; Send; In contacts; Edit contact                                                             |
| Existing Send draft        | Back to send; Add to contacts or In contacts; no second Send action                                           |
| Existing Convert draft     | Back to convert; same preservation rule as Send                                                               |
| Linked identity management | Back to VerusID; Manage VerusID leads to existing capability-aware details; contact state remains independent |

**Send** starts the existing Send flow with the canonical identity selected,
using supported current-network destination rules. It does not send funds or
bypass recipient validation or transaction review. If another draft is already
retained, entering a different recipient must use an explicit draft-resolution
choice; never silently replace that draft. Returning from an inspection of the
current recipient always resumes it unchanged.

**Manage VerusID** is shown only when a known linked identity has an applicable
management/detail destination. **Edit profile** belongs to that existing flow
and follows current editability checks. Linked status alone never grants signing
or editing authority. No owner controls appear because a contact is saved.

One matching contact opens its editor. Multiple existing contacts for the same
identity open a compact chooser before editing; never merge them or arbitrarily
pick one. The public profile stays a single identity view.

### Profile and contact states

- No avatar or description: show the resolved identity and fallback. When all
  supported public fields are confirmed absent, show **No public profile details
  yet**. The ID remains valid and can still be saved or selected for Send.
- Profile loading: render the known identity immediately; reserve avatar/content
  space. Optional enrichment must not block lookup or contact saving.
- Profile unavailable: **Profile details couldn't be loaded** with Try again.
  Distinguish this from confirmed absence. Keep known identity information.
- Cached data: retain confirmed content while refreshing. If refresh fails,
  disclose **Couldn't refresh profile details**; do not imply current
  verification.
- Contact membership loading/error: View profile and public content remain
  usable. Show a separate checking/retry state for contact membership; do not
  expose Add until absence is known.
- Saving: reserve the final label width; disable repeated activation. Show Saved
  only after persistence succeeds, then settle to In contacts. Saving does not
  close the view or change navigation.
- Save failure: keep the profile open, show **Couldn't save contact** and a
  retry action for the save. Retrying must not discard notes or duplicate the
  record.
- Revoked/inactive/unusable identity: disclose its actual known status near the
  identity and apply existing destination-validity rules to Send. Do not treat
  saved-contact status or an attractive profile as payment eligibility.

## Preview journey

The same preview is reachable with hover, keyboard focus and explicit
activation. Show the exact name, avatar and at most three lines of description;
**View profile** exposes the rest. Avoid nested scrolling for long biographies.

Keep **View profile** in a stable action position. The other position contains
Add to contacts → Saving… → Saved → In contacts, or a contact-specific retry
state. In contacts is a quiet status, not a third navigation destination. Keep
existing saved-name/avatar recognition styling; it means saved locally.

Use an interactive non-modal popover rather than a tooltip role. Pointer
movement from trigger to card keeps it open. Both actions are reachable by
keyboard. Escape dismisses only the topmost preview and restores focus without
cancelling Send. Explicit opening must work without precise pointer hovering.

Opening View profile dismisses the preview and records its origin. It never
requires adding the identity first. Returning restores the originating control
without automatically reopening the preview.

## Contacts journey

Keep compact list rows: avatar/fallback, resolved VerusID and useful destination
context. Omit public biographies, website checks and cover imagery from rows.
The selected-contact detail uses a small avatar/name header, **View profile**,
saved destinations, private notes and **Edit contact**. Retain copy actions.
Name and public avatar use the existing confirmed profile source.

Clicking a row selects its private contact detail. View profile opens the public
destination, and Back to contacts restores the selected contact, query, scroll
position and focus. A public profile contains no private notes, manually saved
addresses, or assertion that the identity controls those addresses.

Inside a recipient picker, row activation continues to select the recipient;
profile inspection remains a distinct action. Address-only contacts keep their
existing functionality and do not receive empty public-profile affordances.
Existing multi-identity contact rules and private data remain unchanged.

## Return to a payment

Capture a session-bound origin when entering a profile from Send or Convert.
Preserve the exact draft: source, amount, asset, destination network, recipient,
memo/options, current step, and any visible validation state. Preserve the
existing mounted-flow approach where practical. A return to the form must not
imply that an old preflight is still valid; existing freshness checks still
apply.

Use a single **Back to send** or **Back to convert** action. Returning restores
the recipient control's focus (or the next valid form control if it disappeared)
and leaves the preview closed. Saving a contact may update recognition styling
but cannot alter the recipient or other draft values.

Scope origin state to the wallet session and network. Lock/account/network
changes clear the navigation snapshot. Never resurrect a previous wallet's draft
or private contact details. For nested website/identity disclosures, Back/Escape
first closes that disclosure, then returns from the profile on a subsequent
action.

## Websites and cover imagery

The website row is an optional extension surface, illustrated in Paper using
`alex.example`. Its domain and all verification states are fixtures, not live
proof. The main profile is complete when the row is absent.

- A status describes only the identity-to-website connection. It never decorates
  the identity name with a general trust check or changes contact recognition.
- A previously checked connection may show Website verified with details for the
  exact origin, network and last-check time, subject to the website spec's
  freshness policy. Unknown/unreachable is **Couldn't check website**, not
  invalid proof.
- Unchecked, checking, verified, invalid and unreachable states remain distinct.
  Checking/retry uses deliberate full-profile opening/refresh under the website
  spec; hovering over an identity causes no website request.
- The external-link action opens the system browser. Details opens evidence
  inside the wallet. Keep those two actions distinct.
- Owner setup, signing, upload help and publication remain in the separate
  website spec and existing Paper website-setup project.

Cover images are a future design-only extension. The dedicated Paper page
illustrates optional placement; it is not an approved on-chain field, storage
format, upload flow or implementation requirement. Before implementation,
specify data provenance, fetching/privacy rules, size limits, crop behavior and
fallback. Never reserve a blank banner when no cover is present. Keep the exact
name and actions readable and visible without depending on the image.

## Implementation boundaries for a future authorized task

Reuse the existing contact resolver, canonical identity keys, session-bound
profile cache, profile reader and contact persistence. Do not fork profile data
per entry point. A semantic navigation contract should carry the identity key
and origin, independently of contact membership. Public profiles and private
contact detail must remain separate presentation responsibilities.

Current integration points (inspect again before implementation):

- `src/lib/components/wallet/WalletLayout.svelte`: section lifecycle, retained
  transfer origin and navigation context.
- `src/lib/components/wallet/sections/Identity.svelte`: linked-ID section and
  tab placement.
- `src/lib/components/wallet/contacts/PublicProfile.svelte`: shared confirmed
  avatar/name/description rendering; not yet an independent routed profile.
- `src/lib/components/wallet/contacts/IdentityPreview.svelte`: preview and
  saving actions.
- `src/lib/components/wallet/contacts/ContactDetail.svelte` and
  `sections/AddressBook.svelte`: compact private contact detail and profile
  entry.
- `src/lib/contacts/navigation.ts`, `identity.ts`, `profiles.ts`, `service.ts`
  and `session.ts`: navigation, canonical association, bounded shared loading
  and session isolation.

Keep all new UI copy in English/Dutch translation keys; reuse existing common
labels. Candidate namespace: `wallet.profiles.*`. Use sentence case, Lucide
icons, existing selection tokens, the normal arrow cursor and shadcn ScrollArea.
Keyboard focus must remain visible in both themes. Long fully qualified names
wrap on full profiles; a compact truncation always has an accessible full value.

## Acceptance criteria

- [ ] An unsaved, unlinked VerusID can be found and viewed without
      saving/linking.
- [ ] Linked IDs retains existing behavior; Find remains available for an empty
      list.
- [ ] All entry points resolve to the same canonical identity and confirmed
      content.
- [ ] Exact lookup is network-scoped; not-found and unavailable are distinct.
- [ ] Obsolete queries and previous-session requests cannot replace current
      results.
- [ ] View profile works while Contacts is loading or unavailable.
- [ ] Confirmed empty profile content differs from loading and failed
      enrichment.
- [ ] Preview supports hover, click and keyboard; three-line summary and actions
      fit.
- [ ] Saving stays in place, shows success only after persistence, and
      deduplicates.
- [ ] Save failure preserves the profile and supports a scoped retry.
- [ ] Saved state is distinct from website proof and identity management rights.
- [ ] Compact Contacts retains private notes/destinations and their copy/edit
      flows.
- [ ] Private contact content never appears as public profile content.
- [ ] Duplicate matching contacts require a choice only for private contact
      editing.
- [ ] Recipient-picker row activation still chooses a destination.
- [ ] Profile Send enters existing validation/review and cannot silently replace
      a draft.
- [ ] Send/Convert inspection and return preserve every draft field and the
      current step.
- [ ] Escape closes the topmost surface; focus returns without reopening the
      preview.
- [ ] Lock/account/network changes prevent stale origins, data and drafts
      reappearing.
- [ ] Website states/freshness follow their separate spec; no hover-triggered
      fetches.
- [ ] Unknown and invalid website evidence remain distinguishable and
      explainable.
- [ ] Owner actions follow existing capabilities and confirmed-publication
      semantics.
- [ ] Light/dark and long-content states work at 920×620 and a larger desktop
      viewport.
- [ ] All new labels are translated; focus, selection, copy and normal cursors
      work.

## Paper inventory and static review

The new file contains 27 editable artboards:

| Page                                                                                     | Coverage                                                                                                                                                                                                          |
| ---------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Main journeys](https://app.paper.design/file/01M2X49JSSK4JP0DX2XAA51VX2/p-1-0)          | Start-here map and 11 light-mode screens: lookup, exact match, unsaved profile, compact Contacts, saved profile, unsaved/saved Send previews, profile inspection, restored draft, linked IDs and management entry |
| [Dark mode](https://app.paper.design/file/01M2X49JSSK4JP0DX2XAA51VX2/p-2-0)              | The same 11 screens in dark mode                                                                                                                                                                                  |
| [States and behavior](https://app.paper.design/file/01M2X49JSSK4JP0DX2XAA51VX2/p-3-0)    | Paired light/dark state sheets: not found, lookup unavailable, no published details, refresh failure, contact-save failure and website evidence                                                                   |
| [Future cover extension](https://app.paper.design/file/01M2X49JSSK4JP0DX2XAA51VX2/p-4-0) | Paired light/dark optional-cover concepts, outside first-version implementation scope                                                                                                                             |

The main screens were visually inspected at 920×620 for spacing, typography,
contrast, aligned repeated controls, hierarchy and fit. The state sheets and
cover concepts were also inspected in both themes. Google Sans, existing wallet
color roles, Lucide icons and the existing example profile image are reused.
Dark icon strokes, text contrast and the initial empty-search button state were
corrected during review. Larger desktop geometry and interactive behavior remain
future implementation acceptance checks, not claims of this static delivery.

## Evidence and next validation

The design draws on Microsoft 365's compact/expanded profile cards and
independent contact-saving action, Slack's searchable people directory, and
ENS's public name lookup alongside a separate My Names view. These are
precedents, not comparative usability testing of this wallet.

- [Microsoft profile cards](https://support.microsoft.com/en-us/outlook/profile-cards-in-microsoft-365)
- [Slack directory](https://slack.com/help/articles/360003534892-Browse-people-and-user-groups-in-Slack)
- [ENS name lookup](https://support.ens.domains/en/articles/8874842-where-do-i-find-my-ens-names)
- [Progressive disclosure](https://www.nngroup.com/articles/progressive-disclosure/)
- [W3C hover and focus](https://www.w3.org/WAI/WCAG22/Understanding/content-on-hover-or-focus.html)

Validate three tasks with users before implementation: find an unsaved identity;
open the full profile from Contacts; inspect a recipient and resume Send.
Observe whether Find under VerusID is discoverable, whether public versus
private content is understood, and whether people return to payment without
losing their place. Paper screenshots establish static composition only. They do
not establish keyboard operation, live resolution, persistence, website
verification, signing, transaction safety or native draft restoration.
