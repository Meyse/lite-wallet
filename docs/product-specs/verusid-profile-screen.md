---
owner: lite-wallet-team
last_reviewed: 2026-09-20
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

| Image  | Recommended upload                 | Budget                                      | Display             |
| ------ | ---------------------------------- | ------------------------------------------- | ------------------- |
| Avatar | 256×256 JPEG                       | Existing 32 KiB app limit                   | 80×80 circle        |
| Header | 960×160 JPEG, crop before encoding | Target 16–32 KiB; proposed 32 KiB app limit | Full-width 6:1 crop |

The header budget is an application recommendation, not a protocol maximum. Keep
important content away from the lower-left avatar overlap. Review the final crop
and storage-aware fee before publishing; do not assume a fixed fee. The existing
header VDXF key is `vrsc::system.identity.profile.header`
(`iP9hXXzqYXQhnY9EXikYjqraGBCrvaysAe`); the website's public profile format uses
`public.header.image`. Reader and publisher compatibility still needs an
explicit implementation; this screen does not add header publishing.

## Current implementation boundary

The approved increment is the screen and this spec. Production reads the
existing avatar/description, contact membership, linked identities and public
identity details. Addresses includes the canonical VerusID destination; signing
addresses remain in Identity details. Private Contacts notes/endpoints are never
public data. Header, website, social and additional-address rendering accepts a
separate typed presentation contract, exercised by browser fixtures. No new
claim readers, verification, remote image fetching, or upload editor are
included. With the current reader, the cover is neutral, socials are omitted,
and Websites says “No websites to show.” This is not evidence that an identity
has published no websites.

Navigation from recipient hover previews remains a separate increment; the
existing lookup entry and Back-to-search behavior are preserved.
