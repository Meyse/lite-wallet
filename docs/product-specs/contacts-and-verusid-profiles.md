---
owner: lite-wallet-team
last_reviewed: 2026-09-19
---

# Contacts and VerusID profiles

Status: first local implementation complete on 2026-09-19. Core scope and
preview actions were selected with the user. See the
[implementation evidence and call-site inventory](../references/contacts-implementation.md)
for delivered behavior, checks and native verification limits. Acceptance items
below remain a release checklist; they do not imply live-wallet certification.

## Outcome

People can save any supported receiving address in **Contacts**, optionally
associate a VerusID, and recognize that identity by the same published avatar
throughout the wallet. Hovering, focusing, or explicitly opening an identity
reveals its name, avatar, description, and a relevant contact action without
leaving the current task.

Saving should feel like recognition, not form filling. An unsaved ID starts as a
readable, quiet name with a neutral identity mark. Its preview reveals the full
public avatar and description. One click on **Add to contacts** saves it in
place; a brief green check confirms success, and the inline name gains the
wallet's blue accent and published avatar. Color means saved locally, not
verified ownership or a trusted payment destination.

The public profile has two editable fields: avatar and description. Its
displayed name comes from the resolved VerusID; this scope adds no third public
name field. An identity-backed contact uses that same name in the contact list,
profile heading, search, and recipient presentation. There is no separate
nickname: the example is `alex.example@` everywhere, never "Alex".

## Agreed scope

- Rename the user-facing Address book section and its references to
  **Contacts**.
- Retain individual Verus, Bitcoin, Ethereum, and private receiving addresses
  already supported by the wallet. A VerusID is optional.
- Use the resolved VerusID as the name of an identity-backed contact, without a
  separate editable alias. Preserve notes and manually saved addresses.
- Clicking a contact opens its full contact profile with saved addresses. The
  full profile header does not open another copy of the hover preview.
- Reuse the existing public avatar and description format and profile editor.
- Reuse the same public avatar wherever it appears. In wallet identity mentions,
  distinguish unsaved IDs from saved contacts using the presentation below; the
  public preview remains available before saving.
- **Add to contacts** saves the resolved ID directly from the preview, without a
  form, second confirmation, navigation, or on-chain action. Its in-place
  success feedback settles to **View in contacts**.
- Keep saving contacts private and local; it is not an on-chain publication.
- Defer reading/importing receiving addresses from public VerusID content until
  a follow-up establishes the existing format and address-update behavior.

No registration requirement, social feed, messaging, banners, social links,
contact photo uploads, profile themes, or new profile publishing format is added
by this core scope. The optional
[verified websites extension](./verusid-verified-websites.md) separately
specifies website discovery, proof checking, and the owner's guided
upload/publication flow. Its status applies only to the website connection and
never changes the meaning of a saved contact. The existing account profile in
Settings is a separate concept.

## Product model

See the [shared glossary](../../CONTEXT.md).

| Information                                                | Controlled by     | Behavior                                                       |
| ---------------------------------------------------------- | ----------------- | -------------------------------------------------------------- |
| Address-only contact name, notes, manually saved addresses | Wallet user       | Private contact data; profile refresh never overwrites it      |
| Identity-backed contact name                               | Identity record   | Use the selected resolved VerusID name; no local alias         |
| VerusID name and identity address                          | Identity record   | Resolve authoritatively and retain network/chain context       |
| Public avatar and description                              | VerusID publisher | Optional enrichment loaded through the existing profile reader |
| Which identity represents a contact                        | Wallet user       | Select one contact profile if several IDs are associated       |

The contact association records the canonical identity address and its
network/chain, not just a display name. A name or a regular receiving address
alone is insufficient evidence to attach an identity automatically. An
association is not a claim that the contact controls every saved address.

For one associated VerusID, use its name and profile automatically. Adding a
second ID keeps the existing selection. A small **Use as contact profile**
action is only needed when there are multiple associated IDs; the selected ID
supplies both name and profile. For legacy contacts containing several IDs and
no selection, preserve the existing record until one is chosen; never choose
according to endpoint ordering or present its old local label as a VerusID.
Removing the selected ID requires choosing a remaining ID when several remain.
With no IDs left, ask for an address-only contact name in the same edit flow.
Preserve legacy local labels in storage for migration/recovery; do not offer
them as editable aliases for an identity-backed contact.

Multiple contacts may refer to the same identity. Do not merge or delete them
automatically. Individual VerusID mentions always display that ID's own profile,
even when a different ID represents the overall contact.

## Contact journeys

### Add an address without a VerusID

Save a contact name and at least one valid supported address using the existing
flow. Keep network validation, private notes, copying, editing, deletion, and
recipient selection. Show the existing neutral contact initial. Do not show an
empty profile panel or prompt the user to register an ID.

### Add a VerusID as a contact

Resolve the entered ID on the active supported network, then show its name and
available public profile as a preview. Use the resolved ID as the contact name,
without a second name field, and include it as a Verus receiving destination.
Activating **Add to contacts** is the save action. It immediately starts a local
save of the canonical identity association, resolved name, and ID receiving
destination. It requires no additional fields or confirmation. Additional
addresses and private notes can be added later in Contacts. Do not import
addresses from public content or change the current Send destination.

Profile loading is not a prerequisite for saving an otherwise valid resolved
identity. Do not save partially typed or unresolved names as associations. The
same direct-save behavior applies after searching for an ID through Contacts;
ordinary address-only contacts retain their existing name/address editor and
Save action.

Keep the preview open under the existing pointer/focus rules and preserve the
originating draft. The action follows the save-feedback states below. Hover
alone never saves. No success dialog or duplicate toast is needed while this
in-place feedback is visible.

Match and deduplicate by canonical identity address plus account, network, and
chain. Coalesce repeated clicks and re-check at the persistence boundary. A
concurrent save of the same identity resolves to the existing contact, without
creating another or overwriting its notes/addresses. Existing duplicate records
remain intact and use the **View in contacts** action. Do not expose Add until
the local contact match is known.

### Associate an ID with an existing contact

In contact editing, adding a VerusID receiving destination also offers its
resolved profile for the contact. Review the association before saving. Keep the
existing addresses and notes; show that the resolved VerusID becomes the
contact's name. Removing an association or deleting a contact never edits,
unlinks from the VerusID tab, or removes the public profile on-chain.

### Open an existing contact from a profile

With one matching contact, **View in contacts** opens the Contacts section with
that contact selected. A visit from Send or Convert preserves the current form
and provides a Back action to return to it. Hidden forms are inactive; changing
the wallet or session discards them. Do not open a second profile detail popup.

With several matches, **View in contacts** shows the matching local contacts in
Contacts for the user to choose. Do not arbitrarily select one or display
another contact's private notes inside the public profile preview.

Inside **Contacts**, selecting a list row opens the full contact profile
directly: avatar, VerusID name, description, and manually saved addresses. Show
the name once in the profile heading, without a separate alias or duplicate ID
line. Search identity-backed contacts by their VerusID; address-only contacts
remain searchable by their local names and saved addresses. The profile heading
copies the human-readable VerusID. Omit its canonical i-address from contact
details and editing; retain it unchanged in storage. Additional saved addresses
remain accessible. Add contact is the primary action.

## Profile preview

### Content and visual hierarchy

Use the selected B layout: a circular avatar above the resolved VerusID name,
the optional short description below, and one quiet contact action at the
bottom. The name preserves parent/namespace information needed to distinguish
the ID; where current display formatting omits a system suffix, retain the
canonical identity and chain context and show a concise disambiguator when
necessary. Do not match contacts by a shortened display name.

Keep the preview about 300–340 CSS pixels wide, constrained to the viewport,
with a roughly 56-pixel avatar and existing wallet typography, radii, colors,
surface, and shadow tokens. These are starting dimensions to validate, not new
global tokens. Use the wallet's light and dark themes, normal arrow cursor, and
Lucide icons where needed. Avoid a separate "Profile" heading, decorative cover,
repeated name, balances, badges implying trust, raw protocol fields, or an Edit
profile action here. Profile editing remains in the VerusID section.

Use plain text for the description with natural wrapping; show the full
supported short description. No HTML, Markdown, auto-linked URLs, or remote
image fetches from arbitrary profile content. Avatar failure uses the shared
identity initial and gradient. Contacts without an ID retain their neutral
contact initial.

### Recognition in the wallet

Keep the same inline geometry in both membership states: a 22-pixel identity
slot and the dotted VerusID name. For an unsaved ID, use a neutral @ mark in
that slot and readable secondary text. Do not make it look disabled. Its hover
card already shows the full-color public avatar and description. Give the
neutral mark a visible gray circle with no hover color change. On hover, change
the name's text color while keeping its muted gray dotted underline. Apply this
to both unsaved gray and saved blue names. Keep the layout fixed.

After durable saving, replace the neutral mark with the published avatar and use
the wallet's accessible blue action color for the inline name. If the avatar is
absent or unavailable, use the shared identity fallback; saving does not depend
on having a photo. Keep the dotted affordance and explicit focus indicator in
both states. The trigger's accessible name includes whether the ID is in
Contacts, so color and imagery are not the only status signals.

Use this distinction for resolved recipient presentations and eligible compact
identity mentions in the wallet. Contacts and VerusID management retain their
full profile presentation; an unsaved ID's public profile is never hidden or
desaturated inside its preview. Do not dim recipient validation or hide the
actual destination. Removing the local contact restores the unsaved inline
presentation, without changing the identity or public profile.

### Save feedback and motion

| State          | Action area                                                         | Inline identity                            |
| -------------- | ------------------------------------------------------------------- | ------------------------------------------ |
| Not saved      | Add to contacts                                                     | Neutral mark and secondary name            |
| Saving         | Spinner and Saving… in the same button bounds; no repeat activation | Remains neutral until persistence succeeds |
| Save succeeded | Green check and Saved                                               | Published avatar/fallback and blue name    |
| Saved, settled | View in contacts                                                    | Avatar/fallback and blue name              |
| Save failed    | Couldn't save contact, with Try again in the same preview           | Remains unsaved; no green check            |

Start saving on activation and acknowledge input immediately. Show success only
after the encrypted contact store reports a durable save; an optimistic check
must not claim success before that point. Do not add a minimum Saving delay. For
a quick save the user can go directly to the check; for a slower one, the
existing spinner and action-specific label explain the wait.

Proposed motion: draw the check over roughly 160 ms and crossfade the inline
identity in roughly 180 ms, without resizing or moving the name. Keep **Saved**
visible for about 1.2 seconds, then settle to **View in contacts** in the same
bounds. The transient Saved state cannot trigger a second save or open the
contact. Keep the same focus target through label changes; respect normal
dismissal, and do not force the card to remain open just to finish an animation.
On the next opening, show View in contacts immediately without replaying
success.

Reduced motion skips the drawing/crossfade. Announce Saving and Saved once
through a polite live region; announce a failure and leave keyboard-accessible
retry in place. Reserve translated button width across Add, Saving, Saved, and
View. Success uses text plus a check, not color alone, and never adds a
permanent check badge beside the identity name.

A save is bound to the identity and wallet context captured at activation.
Changing the recipient must not retarget the write or show its result beside the
new recipient. Dismissed previews do not reopen on completion. Account switch or
lock discards obsolete presentation updates and must not replay a write in the
new context. If a failure leaves the outcome uncertain, reconcile the durable
store before retrying rather than creating a duplicate.

### Interaction

- Use the selected A trigger treatment in the wallet: the membership-aware
  identity mark/avatar and a lightly dotted underline outside the editable
  input. Keep a visible keyboard focus indicator. The full Contacts profile
  needs no dotted trigger on its own heading.
- Open immediately on hover, with no added delay. Keep the 200 ms close delay so
  the pointer can cross into the card. Only one preview can be open at a time.
- Keep the card open while the pointer or keyboard focus is in either its
  trigger or content. The pointer must be able to move into the card and select
  its action.
- Keyboard focus makes the same preview available; opening does not steal focus.
  Tab can reach the contact action and continue through a predictable focus
  order.
- Where the name/avatar has no existing action, click, Enter, or Space
  explicitly opens the preview, also providing a touch path. Where a row/name
  already opens identity details or selects a recipient, preserve that action
  and provide a separate accessible preview trigger without nesting interactive
  elements.
- Escape dismisses the preview; if focus was inside, return it to the trigger.
  Suppress immediate reopening until the pointer/focus leaves or the user
  explicitly opens it again. Outside click dismisses an explicitly opened
  preview.
- Show at most one preview at a time. Close it when its anchor disappears, the
  underlying identity changes, a new dialog takes over, or wallet context
  changes.
- Position through a portal with viewport collision handling. Never clip inside
  list scroll areas or obscure the triggering name or focused control. Keep
  keyboard focus visible and support reduced motion.
- Use accessible non-modal popover semantics for interactive content. A standard
  text tooltip is insufficient. Screen readers must identify the preview
  trigger, expanded state, profile name, and contact action; the decorative
  avatar needs no duplicated name announcement.

### States

| State                                            | Avatar and description                                                                                     | Contact action                         |
| ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| Resolved ID, profile loading                     | Circular avatar skeleton and two description lines; keep the known name and any cached profile visible     | Available without waiting for profile  |
| Avatar and description                           | Published image and plain-text description                                                                 | Add or View according to local matches |
| Avatar only                                      | Published image; omit empty description area                                                               | Available                              |
| Description only                                 | Identity initial and description                                                                           | Available                              |
| Confirmed empty profile                          | Identity initial and name; no "No profile" clutter                                                         | Available                              |
| Profile unavailable or unsupported               | Last valid session snapshot when allowed, otherwise initial; concise "Profile unavailable" in open preview | Available if the ID itself is resolved |
| Image fails to render                            | Identity initial, no broken-image icon                                                                     | Available                              |
| Own profile update pending                       | Previous confirmed profile; pending draft stays in the editor                                              | Available                              |
| Name unresolved, wrong network, or lookup failed | Preserve entered text and existing validation feedback; no resolved profile                                | No identity-based Add/View action      |

Profile failure is separate from identity-resolution failure. Supported fields
may be shown when another field is unavailable, consistent with the backend's
validated result. A successfully confirmed removal clears the old field; failed
refreshes must not be mistaken for removal or successful publication. Existing
identity status warnings, including revoked IDs, remain authoritative and are
never masked by an attractive avatar.

## Where it appears

| Surface                                                                 | First-version behavior                                                                                                                                                                                                  |
| ----------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Contacts list and detail                                                | Selected VerusID name and avatar; no editable alias or duplicate ID line. Selecting a contact opens its full profile with description and manually saved addresses. Address-only contacts retain local names            |
| VerusID list and detail                                                 | Same profile source and avatar; preserve existing selection, favorites, edit, and pending-publication behavior                                                                                                          |
| Send recipient suggestions                                              | Saved identity contacts use their avatar and blue name; unsaved resolved IDs use the neutral mark. The actual selected destination and network remain explicit                                                          |
| Send after entering an ID                                               | After authoritative resolution and contact matching, show the membership-aware dotted name beside the input. Its preview permits direct saving; do not put hover behavior inside the editable input or replace its text |
| Send review and completion                                              | Reuse the exact resolved identity associated with the prepared transaction. Keep the actual reviewed destination visible and copyable; profile data never controls it                                                   |
| Existing resolved ID mentions elsewhere                                 | Adopt the shared presentation in receive, activity details, request reviews, or authority references where canonical identity context is already available                                                              |
| Raw addresses, unresolved strings, editable fields, logs, exported data | Keep their existing representation; do not guess identities or insert avatars into serialized data                                                                                                                      |

Audit actual call sites during implementation and record coverage. This scope
does not introduce transaction-history reverse lookup or retroactively infer
identities from ordinary addresses. No recursive preview inside an already open
profile preview, and no redundant preview over a detail header that already
shows the same complete information. Existing transaction details remain the
historical record; a freshly loaded profile is current presentation, not
historical proof.

In Send, changing input invalidates the previous resolved identity and avatar
immediately. Slow responses for the old recipient must never appear beside the
new one. The final preflight/signing result remains the source of truth; a
cached profile or contact association must not change destination resolution,
fees, acknowledgments, or approval behavior. Copied images and descriptions are
not evidence that an identity belongs to the intended person.

## Loading, privacy, and persistence

- Share profile retrieval and presentation across surfaces instead of
  maintaining separate Address book, Send, and VerusID caches that disagree.
- Identify requests by canonical identity address plus network and chain. Scope
  contact matching and retained profile state to the current account/unlock
  session; cancel or discard late results after lock or account/network switch.
  Use validated canonical address forms; do not lowercase arbitrary Base58
  strings or use display labels as cache keys.
- Load only visible identity avatars and deliberate preview/recipient requests.
  Deduplicate concurrent requests, bound concurrency and cache size, and
  prioritize the selected recipient or open preview. Do not crawl the entire
  contact list or look up every keystroke. Contacts and sending remain usable
  while images load.
- Reuse valid session data, revalidate on explicit section refresh and after a
  confirmed profile change, and use bounded retry/backoff for failures. Before
  implementation, choose and document finite cache freshness and negative-cache
  intervals consistent with the existing wallet lifecycle.
- Clear session caches and open previews on wallet lock or account change. Do
  not persist profile/contact associations in plaintext browser storage.
  Preserve the existing encrypted contact storage and follow the
  [storage work plan](../plans/active/encrypted-address-book-watchlist.md) when
  implementation overlaps that work.
- The RPC provider can observe identities being requested. Send only required
  public identity queries, never local contact labels, private notes, the full
  contact list, or analytics identifying contact relationships. Saving a contact
  does not publish that relationship.
- Only the existing profile reader decides whether content is supported and
  valid. This feature does not expand supported storage formats, cross-chain
  retrieval, publication permissions, or external media transports.

## Current source and implementation seams

Source inspected on 2026-09-19; this is static evidence, not live-wallet
validation.

| Existing area             | Starting point and required extension                                                                                                                                                                                                                                            |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Contact records           | [Address book types](../../src/lib/types/addressBook.ts) already hold local name, notes, and several typed endpoints; add optional canonical identity associations and a selected contact profile with backward-compatible defaults                                              |
| Contact UI                | [AddressBook.svelte](../../src/lib/components/wallet/sections/AddressBook.svelte) currently uses a neutral initial in selected-contact detail; reuse its list/detail and editor structure                                                                                        |
| Contact persistence       | [Service](../../src/lib/services/addressBookService.ts), [store](../../src/lib/stores/addressBook.ts), and [backend manager](../../src-tauri/src/core/address_book/manager.rs); preserve existing records and durable-save semantics                                             |
| Avatar and public profile | [IdentityAvatar.svelte](../../src/lib/components/wallet/sections/identity/IdentityAvatar.svelte), [service](../../src/lib/services/identityLinkService.ts), and [profile reader](../../src-tauri/src/core/channels/vrpc/identity/profile/read.rs); share them with new consumers |
| Current profile state     | [Identity.svelte](../../src/lib/components/wallet/sections/Identity.svelte) and [section session state](../../src/lib/components/wallet/sections/identity/identitySectionSessionState.ts); extract shared retrieval without losing pending-update reconciliation                 |
| Identity naming           | [identityDisplay.ts](../../src/lib/utils/identityDisplay.ts); preserve readable names while retaining canonical identity and chain context                                                                                                                                       |
| Send                      | [TransferWizard.svelte](../../src/lib/components/wallet/sections/TransferWizard.svelte) and transfer services; expose a typed canonical identity-resolution result if the present validation/preflight projection is insufficient, without interpreting warning prose            |
| Preview primitive         | No local hover-card wrapper exists; evaluate the installed Bits UI primitives and existing dialog/tooltip conventions. Add a shared interactive preview rather than screen-specific hover timers                                                                                 |

Keep internal command names and existing storage paths stable unless a
functional change requires migration. The visible Contacts rename alone does not
require renaming all `addressBook` code. Translate new and renamed copy through
the existing English and Dutch locale keys and update all visible references.

The older
[avatar and description plan](../plans/active/verusid-profile-avatar-description.md)
still describes implementation as future work, but the current source contains
the reader and editor. Treat that plan as historical design/research context;
reuse the implementation and verify its current limits rather than rebuilding
it. This spec extends profile consumption and does not lift publishing
restrictions.

## Delivery slices

1. **Shared identity presentation:** session-scoped profile retrieval, stable
   avatar/fallback, interactive preview, and preserved editor/pending behavior.
2. **Contacts:** visible rename, optional identity associations,
   backward-compatible records, shared editor/detail views, and preview Add/View
   actions.
3. **Wallet integration:** Send resolution/review/completion and eligible
   resolved ID surfaces, followed by a recorded coverage inventory.

These slices form the first version. Public receiving-address discovery is a
separate follow-up: inspect the existing website/Mike@ representation, confirm
network-qualified schemas and provenance, define refresh/removal/change review,
and prove that importing published addresses cannot silently overwrite manually
saved destinations. No compatibility claim for that format is made here.

## Acceptance criteria

- [ ] Existing contacts load with legacy data, notes, endpoints, order, and
      saved-recipient behavior preserved; Contacts requires no VerusID or paid
      publication.
- [ ] A Bitcoin-only contact can gain a VerusID without losing its address or
      notes. Its displayed name becomes the VerusID; legacy labels are retained
      safely in storage, without an editable alias in the identity contact UI.
- [ ] Identity contacts show and search by their VerusID name. Opening one in
      Contacts shows the full profile and saved addresses directly.
- [ ] Single-ID, multiple-ID, removed-profile, duplicate-contact, and
      mixed-network cases follow the explicit association rules.
- [ ] The same resolved ID shows the same confirmed avatar/description in
      Contacts, VerusID, and Send. A draft or pending publication never replaces
      it early.
- [ ] All state-table cases work, including partial content and image
      decode/render failure, without blocking saving or sending.
- [ ] Hover, click, keyboard, touch, Escape, pointer travel into the card,
      repeated dismissal, and focus restoration work without stealing row
      actions.
- [ ] One Add activation durably saves the resolved ID without opening a form,
      requiring another confirmation, or changing the Send draft/destination.
      Hover alone never saves; optional addresses and notes can be added later.
- [ ] Saving feedback appears immediately; the check, Saved label, and colorful
      inline identity appear only after durable success. Failure keeps the ID
      unsaved with inline retry. Repeated/concurrent activation creates no
      duplicate, and existing duplicate contacts are never merged implicitly.
- [ ] The Saved state settles to View in contacts without a second save, layout
      shift, or focus loss. Reopening does not replay the success animation.
      Keyboard and reduced-motion paths expose equivalent status information.
- [ ] Unsaved IDs remain readable and show their full public profile on preview;
      saved contacts use the avatar/fallback and blue name across eligible
      mentions. Removing a contact restores neutral presentation.
- [ ] View and duplicate-contact selection preserve the Send draft.
- [ ] Recipient changes, lock/unlock, account/network switch, and out-of-order
      responses cannot show another recipient's profile or reuse another
      context.
- [ ] Contacts never mutate published data; profile refresh cannot change a
      saved address or a prepared transaction's recipient.
- [ ] English and Dutch, light and dark, long names, full descriptions, list
      scroll boundaries, and viewport edges render correctly at 920×620 and a
      larger desktop size. No pointer cursors or nested interactive elements are
      added.
- [ ] Screen-reader names and focus order are verified with the chosen popover;
      hover is never the only access path.
- [ ] Focused automated tests cover resolution races, cache isolation, contact
      migration/persistence, and interactive preview behavior. Rendered
      screenshots cover Contacts and Send in both themes. Distinguish synthetic
      browser proof from native-wallet evidence; live signing/broadcast is not
      needed to prove a profile preview.

## Design authority and implementation evidence

This specification uses the current source and the user's selected scope. The
approved desktop light/dark visual references were inspected for hierarchy and
restraint; their colors, typography, and page layout are not imported into the
wallet.

The
[Paper exploration](https://app.paper.design/file/01M2VS4T2ES6S4293RWAMZ76Q9/p-1-0)
contains a four-moment discovery/save/recognition sequence, saving and retry
feedback, the selected dotted trigger with the larger B preview, full Contacts
profiles, and before/after light/dark Send context at 920×620. Earlier
alternatives are preserved on a separate page. The preferred frames use
`alex.example@` as the contact name throughout. They currently keep receiving
addresses in the full contact profile; showing them in the hover remains an open
optional decision. These are static design frames, not evidence of working
hover, animation, persistence, or focus behavior.

The initial specification was read-only. The later implementation uses the
selected Paper geometry and shared wallet tokens; its
[verification record](../references/contacts-implementation.md) distinguishes
synthetic rendering, automated persistence checks and unverified native
behavior. Public receiving-address schema research remains explicitly deferred.
