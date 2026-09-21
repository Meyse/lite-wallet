---
owner: lite-wallet-team
last_reviewed: 2026-09-21
---

# VerusID profile screen · 05

Design authority:
[05 · Profile screens](https://app.paper.design/file/01M2Z8Q0HWDARVV19SKWF0HH16/p-3-0)
and
[states and small windows](https://app.paper.design/file/01M2Z8Q0HWDARVV19SKWF0HH16/p-2-0).
This replaces the profile layout in
[the lookup specification](./verusid-lookup-and-profile.md).

## Screen

- 20px page gutters; shared contextual Back at the main screen’s left inset.
  Center profile content at a maximum width of 800px; shrink with the available
  width in smaller windows. Verify 1040×680, 920×620 and a wider window.
- Neutral 84px cover when no header is available. An uploaded header uses a 6:1
  crop. The avatar displays at 80px with a 4px surface-colored ring.
- Canonical name, copy action and one description; no added title or network
  subtitle.
- Secondary Send with the wallet's ArrowUp icon. Add → Adding… → green **In
  contacts**; clicking the saved state opens Contacts. A linked identity shows
  **Linked to this wallet**, without Send or Add. An unknown link/contact state
  must not offer Add. Both green status labels are non-selectable.
- Text-only tabs: Websites, Addresses, Identity details. Website rows show the
  actual hostname, without invented labels. The profile header and tab bar stay
  fixed; only the selected tab body scrolls. Its flex layout fills the remaining
  height down to the 20px bottom inset, with a fade only while more content
  remains. Omit Chain ID; raw addresses use the shared `IdentifierText`
  component, including primary and unresolved authority addresses.
- Official LinkedIn/X artwork at 18px in 32px targets. Hover, focus or click
  opens an interactive popover with verification state, profile link, and proof
  link when available. Escape dismisses the popover first. A saved contact is
  not a verification claim. Only a reader-confirmed claim may display Verified.

## Image recommendations

| Image  | Recommended upload                 | Budget                           | Display             |
| ------ | ---------------------------------- | -------------------------------- | ------------------- |
| Avatar | 256×256 WebP                       | Existing 32 KiB app limit        | 80×80 circle        |
| Header | 960×160 WebP, crop before encoding | Target ≤16 KiB; 32 KiB app limit | Full-width 6:1 crop |

The header budget is an application recommendation, not a protocol maximum. Keep
important content away from the lower-left avatar overlap. Review the final crop
and storage-aware fee before publishing; do not assume a fixed fee. The existing
header VDXF key is `vrsc::system.identity.profile.header`
(`iP9hXXzqYXQhnY9EXikYjqraGBCrvaysAe`); the website's public profile format uses
`public.header.image`. The wallet reader and publisher use the header VDXF key
and the same authenticated on-chain descriptor/evidence path as avatars.

## Profile editing

The [publishing UX specification](./verusid-profile-publishing-ux.md) governs
the implemented editor and publication presentation. Local checks, browser
fixtures and fresh native macOS VRSCTEST acceptance cover the corrections,
including one/two updates, restart, actual avatar chooser and automatic final
completion. The [execution plan](../plans/done/verusid-profile-publishing-ux.md)
records exact proof levels and remaining platform/accessibility limits.

- Edit profile shows the effective header, overlapping avatar, identity name and
  inline optional description. Image controls open the chooser directly and show
  a populated crop only after valid selection. Connections are hidden. Review
  changes is the only primary action; one Unpublished changes disclosure
  explains the memory-only draft lifetime.
- Each field remains independent. Remove image stages an empty field; Undo
  change restores its confirmed value. Removing unpublished content is local.
  Invalid description text remains editable and blocks review. Review explicitly
  names removals and shows the combined proposed profile.
- Unreviewed drafts live only in memory, isolated by wallet session and
  identity. They survive navigation, and clear on lock, wallet/network change,
  restart or submission. Reviewed publication intent remains encrypted in
  Stronghold.
- JPEG, PNG and WebP sources retain the 10 MiB/20 million decoded pixel limits.
  Drag, arrow keys and zoom crop to the existing dimensions. Native Rust encodes
  metadata-free WebP, with white matte and the existing bounded quality ladder;
  legacy authenticated JPEG remains readable. Image requirements holds technical
  details. Description is trimmed and limited to 160 graphemes/1,024 UTF-8
  bytes.
- Review shows a compact exact fee. Split review shows the exact first fee,
  estimated header fee and estimated total together, followed by separate
  explicit approval for the second update. Fee details holds balance and quote
  metadata. Coin amounts use integer formatting; unavailable fiat is omitted and
  a mainnet VRSC rate never values VRSCTEST.
- A backend-proven smaller candidate appears before split costs only when it
  reduces two updates to one and lowers the estimated total. Comparison uses the
  same crop and scale. Both choices obtain a fresh review; neither publishes or
  silently selects additional lossy compression. Failure retains the original.
- A single selected-identity owner retains submitted receipts, queues concurrent
  refresh requests and rejects results from earlier generations or sessions.
  Send establishes waiting synchronously. Ambiguous transport blocks review and
  discard until backend reconciliation resolves it. Fresh conflict/expiry and
  first-step reorg evidence can return the retained intent to recovery.
- Final completion requires the exact submitted revision, backend transaction
  and block confirmation, and matching canonical values/digests including
  removals. The read-only `confirm_identity_profile_update` command reuses the
  publication confirmation path when another observer consumed Complete. Null or
  settled transaction IDs alone never imply success. After proof, the editor
  closes once and shows the canonical profile with Profile updated.
- Progress keeps the identity, actual fields and update number visible. Prior
  and pending receipts are distinguished in Transaction details. Back to profile
  preserves the plan. Review header fee always prepares a fresh approval after
  canonical first-step confirmation; a contextual, confirmed discard removes
  only the permitted unpublished remainder. See the
  [sequenced publishing plan](../plans/active/verusid-profile-webp-sequenced-publishing.md).
- Publication remains restricted to supported VRSCTEST identities with a single
  matching primary signing address and threshold one. It preserves unrelated
  identity controls/content and rejects unavailable or inconsistent current
  profile data. Mainnet and multisignature publishing remain unsupported.

## Current implementation boundary

The implementation reads authenticated avatar/header/description, contact
membership, linked identities and public identity details. Addresses includes
the canonical VerusID destination; signing addresses remain in Identity details.
Private Contacts notes/endpoints are never public data. Website, social and
additional-address rendering accepts a separate typed presentation contract,
exercised by browser fixtures. No new connection claim readers, verification or
remote image fetching are included. Without an authenticated header the cover is
neutral; socials are omitted and Websites says “No websites to show.” This is
not evidence that an identity has published no websites. Native local editing
and backend preflight evidence do not by themselves establish successful
on-chain publication and confirmation.

Navigation from recipient hover previews remains a separate increment; the
existing lookup entry and Back-to-search behavior are preserved.
