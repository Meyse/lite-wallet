# VerusID profile design QA

## Scope

- Identity detail profile hero, identity fields, and Authorities and safety
  disclosure
- Compact profile editor and publication review
- Borderless surfaces in the VerusID detail, editor, and provisioning states
- English and Dutch copy

## Rendered evidence

The current visual correction was compared against
`L2 — Detail editable with profile (2).png` at the wallet's 920 x 620 target
size. The real Svelte components were rendered through a temporary local QA
route in the Codex in-app browser; that route was removed after the check.

- Light and dark: left-aligned profile hero, 96 px avatar, identity detail grid,
  and safe Authorities and safety summary
- Light: warning summary and expanded authority warnings
- Light and dark: compact editor with a 96 px avatar, no duplicate profile
  preview, no outline cards, and the review action inside the target-height
  content bounds
- A full-width visually hidden file input initially caused horizontal overflow.
  The input was constrained to one pixel and the scrollbar was no longer
  present.

The follow-up correction was also rendered at 920 x 620 in light and dark modes:

- Identity details and Authorities and safety both start collapsed.
- The safe Authorities and safety summary has no leading green dot.
- Identity details show a blockchain name instead of a raw system i-address.
- Primary addresses are grouped under Authorities and safety and an external or
  additional primary address produces a warning.
- Primary addresses in the warning remain compact but use the shared monospaced
  identifier renderer instead of inheriting the surrounding prose font.
- Revocation and recovery authority cards show resolved VerusID names only; raw
  authority i-addresses are retained for comparison logic but are not presented
  in those cards.

The authority-card consistency correction was compared directly against the
user-supplied 1530 x 926 @2x capture, `CleanShot 2026-09-16 at 13.13.47@2x.png`,
with the reference and the real implementation rendered in one comparison view.
The implementation used unmistakable QA placeholders instead of wallet data and
was checked at the 920 x 620 target viewport.

- Authorities and safety now uses the same small, muted, regular-weight labels
  and foreground values as Identity details.
- The primary-address value remains compact and monospaced, but no longer uses
  the muted label color.
- Both disclosures use the same header padding and the same card padding, corner
  radius, and borderless background treatment.
- The safe summary is neutral supporting text instead of green status text.
- The duplicate green safe-state panel was removed; actual authority warnings
  retain the amber warning treatment.
- The expanded safe state was visually checked in both dark and light mode with
  no clipping or horizontal overflow.

Earlier functional QA covered the native macOS wallet at 1040 x 680 in both
system appearances using the disposable VRSCTEST wallet.

- Light: overview, identity detail, editor, and successful publication review
- Dark: overview, identity detail, editor, and successful publication review
- Keyboard navigation reached the identity, editor, description field, and
  review action
- Empty public profiles remain a valid state and do not disable otherwise
  eligible testnet editing

The successful live review displayed the proposed description, the current
funding address, a fresh 0.01472000 VRSCTEST network fee, and the explicit
not-confirmed disclosure. No transaction was published.

## Verification limits

- The current 920 x 620 correction was rendered in the browser harness rather
  than recaptured in the native Tauri window because the running unbundled
  development app was not exposed to the native app selector.
- The browser harness logged the expected deep-link initialization error because
  it has no Tauri bridge; no component runtime errors were observed.
- The live run covered public profile reads and transaction preparation only.
- Publishing, mempool handling, confirmation, rejection, and reorg presentation
  were not exercised against the live chain because broadcasting requires
  explicit confirmation.
- Pending and confirmed states are covered by implementation and automated
  checks, not by a live broadcast in this QA pass.

final result: passed
