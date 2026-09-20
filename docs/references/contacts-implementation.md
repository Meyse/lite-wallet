---
owner: lite-wallet-team
last_reviewed: 2026-09-19
---

# Contacts and public profiles: first implementation

This implements the
[Contacts specification](../product-specs/contacts-and-verusid-profiles.md) in
the `codex/contacts-verusid-profiles` worktree. Evidence below separates
synthetic UI and Rust storage tests from native wallet execution.

## Data and persistence

The existing account-level `address_book.snapshot.stronghold` file and
`address_book_v1` record remain the canonical encrypted store. Schema 2 adds
optional canonical identity associations, a selected identity, and a retained
legacy label. Schema 1 records deserialize with empty associations; ordinary
addresses are never reverse-resolved into identities. Existing IDs, notes,
endpoint labels, creation times and last-used times survive a profile edit.
Unchanged addresses remain valid to retain even when another network is active.
Editing a destination still invokes the existing network validation.

New associations are authoritatively resolved on the active Verus root chain
before persistence. Already stored associations can be retained while offline;
the writer rechecks them against the latest record. Canonical membership uses
network, chain and case-sensitive identity address, scoped to the active wallet
session. Multiple contacts for one identity are retained. One-click Add
coalesces in the frontend and deduplicates again against the encrypted snapshot.

Contacts list/save/delete/last-used operations share one process-wide writer
gate and execute the complete read/change/encrypted-commit in a blocking worker.
The owned gate stays with a worker if its caller disappears. The application
already installs `tauri-plugin-single-instance`; the gate assumes that normal
application ownership, and does not coordinate external tools editing snapshots.
The temporary encrypted snapshot is synced and replaced under the session
admission guard; Unix builds also sync the containing directory. An uncertain
post-replacement failure is reconciled by reading before retry. Corrupt,
missing-record and unsupported-schema existing files fail closed.

Encryption format, Argon2id parameters and production Stronghold work factors
are unchanged. No public profile content or contact associations are written to
browser storage. Public profile RPCs contain the required public identity only,
never private notes or the contact list. This change does not implement the
separate Watchlist encryption plan. Older application versions can discard the
new optional fields when rewriting a snapshot; downgrading is not lossless.

## Shared profile behavior

The existing backend profile reader remains the authority for supported content.
The frontend cache uses exact network/chain/address keys and clears at lock or
session replacement. It retains at most 100 displayed entries, permits three
concurrent RPCs, and bounds its waiting queue to 100. Deliberate open previews
and selected VerusIDs take priority over queued list avatars. Fresh results live
for 60 seconds; failures retry after 15 seconds. Explicit VerusID refresh and
pending-publication reconciliation bypass freshness. Failed refreshes retain the
last valid session result; confirmed empty content removes old fields.

The VerusID editor keeps its existing pending-update rules. Draft avatar and
text changes are never inserted into the shared confirmed cache. Images use only
validated inline PNG/JPEG/WebP payloads, with a shared fallback after decode
failure; descriptions render as plain text.

The preview uses the installed Bits UI non-modal Popover through a local UI
wrapper: immediate hover open, 200 ms hover close, portal collision handling,
one open preview, keyboard access and Escape dismissal. Each new pointer entry
clears reopening suppression, including when the delayed close ran after pointer
leave. Escape is consumed before the underlying Send cancel shortcut. Tab
returns to the document order at the anchor. Only the name changes color on
hover; its gray dotted underline and the adjacent icon stay unchanged. Initial
profile loading shows a circular avatar skeleton and two description lines,
keeping the known name visible. Cached content stays visible during refreshes;
skeleton motion respects reduced motion. Long content scrolls independently
using the available popover height, with a keyboard-focusable viewport and a
fixed contact action. The action keeps the same button node and width across
Add, Saving, Saved, retry and View. Saved appears only after the encrypted
command succeeds, lasts 1.2 seconds, then becomes View. Motion is 160 ms for the
check and 180 ms for recognition, disabled by the reduced-motion preference.
**View in contacts** navigates to the Contacts section, selecting a single match
or showing matching contacts for duplicate selection. Send and Convert remain
mounted but hidden and inactive during this visit, with a Back action that
restores the form and focus. Wallet or session replacement discards the
suspended form. Contacts shows and copies the VerusID name without repeating its
canonical i-address; storage and payment destinations retain their original
values.

## Call-site inventory

| Surface                               | Implemented behavior                                                                                                                                                                            |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Contacts list and full detail         | Selected identity name/avatar, public description, manual addresses and private note; ordinary contacts keep local names. No recursive header preview.                                          |
| Contacts create/edit                  | Direct ID lookup and Add; ordinary address form; explicit optional association and profile selection; removing all associations requests a local name.                                          |
| VerusID rows/cards/detail             | Shared confirmed source and lazy visible avatars; existing favorite, selection, editor and pending-publication behavior retained.                                                               |
| Send contact suggestions              | Contact avatar and blue identity name; separate accessible preview control preserves recipient-selection action.                                                                                |
| Send entered recipient                | Debounced typed resolution beside the input on supported root chains; changing input immediately hides the old result.                                                                          |
| Send review                           | Preview resolves the canonical `activePreflight.toAddress`; existing reviewed destination and warning remain visible. Profile data does not modify preflight inputs, fees, approval or signing. |
| Send completion                       | Preview uses the immutable submitted receipt destination and chain; current profile presentation is not historical identity proof.                                                              |
| Identity authority details            | Already named canonical revocation/recovery references use the shared mention. Existing authority warnings remain.                                                                              |
| Request review                        | Resolved requester and update target on the active root chain use the shared mention. Raw IDs and unsupported-chain requests keep their existing representation.                                |
| Receive                               | Raw wallet receiving addresses; no canonical resolved identity projection to enrich. No reverse lookup added.                                                                                   |
| Activity / asset transaction details  | Activity remains its existing placeholder; asset details expose raw transaction destinations and currency names, not a canonical identity projection. No historical reverse lookup added.       |
| Identity linking/management selectors | Existing selection/link actions remain. Full identity management uses its existing avatar/profile presentation, rather than a redundant preview over its full header.                           |

English and Dutch visible Address book references are renamed Contacts. Internal
service names, command names and storage paths remain compatible.

## Verification and limits

Focused and complete frontend unit/mounted checks cover canonical keys,
idempotent save activation, uncertain-outcome reconciliation, failed loads,
multiple associations, removal, duplicate choice, late responses, session
replacement, profile cache/failure behavior and keyboard focus restoration.
Existing VerusID favorite and pending-publication tests and Send preflight
lifecycle tests are included. Rust tests exercise legacy snapshots, unchanged
cross-network endpoints, duplicate prevention, concurrent encrypted updates and
corrupt-file protection. The storage test uses the repository's reduced test
work factor and establishes correctness, not production latency.

The synthetic browser fixture lives in `dev/browser-fixtures/`. Run it with the
repository Node runtime and
`node_modules/.bin/vite --config dev/browser-fixtures/vite.config.js`, then open
`http://127.0.0.1:1428/dev/browser-fixtures/contacts.html`. It mounts real
Contacts, Send, sidebar and profile components with a synthetic invoke boundary.
Parameters include `screen=send`, `theme=dark`, `locale=nl`, `delay=2500`,
`failOnce=1`, `profileDelay=2500`, `long=1`, `broken=1`, `lines=1`, `empty=1`,
`unavailable=1` and `saved=1`. `simulateReceipt=1` returns a synthetic typed
receipt without cryptography or network submission; otherwise fixture signing is
disabled. Screenshots are local artifacts in the ignored
`output/contacts-evidence/` directory.

For the initial verification below, no live wallet was unlocked or modified, no
real public profile RPC was executed, and no live transaction was signed or
broadcast. Native Tauri rendering, real screen reader speech, physical touch,
pointer travel between the hover trigger and card, OS-level reduced-motion
behavior, cross-platform crash/power-loss durability and production encryption
latency remain unverified. Automated DOM semantics and synthetic browser
rendering are bounded evidence for this UI, not substitutes for those native
checks.

## Recorded results

- Frontend unit suite: 208 passed across 41 files.
- Mounted suite: 128 passed across 17 files; final preview/Send corrections also
  have a focused regression rerun.
- Rust library suite: 486 passed, 3 intentionally ignored. The first sandboxed
  run could not bind loopback mock servers; the rerun with local networking
  passed.
- Svelte check: no errors or warnings. Production build, UI style lint, docs
  validation and diff whitespace checks passed. ESLint has no errors and retains
  one pre-existing unused-disable warning in `WalletHeroBackground.svelte`.
- Browser fixture: Contacts at 920×620 and 1280×800 in light and Dutch dark
  themes; direct lookup/save, slow Saving/failure/retry, saved recognition,
  read-only detail, focus restoration, Escape isolation and native Tab
  continuation. Send review and simulated completion use the prepared/returned
  canonical destination and keep the existing acknowledgment. The picker preview
  does not select the row; selecting the row still fills its exact saved
  address.
- Long-name and broken-image fallbacks render without horizontal overflow. The
  75-line profile stress case produced a 316×392 card at y=12, ending at y=404,
  above its trigger at y=414; its standard action remained visible while
  keyboard End scrolled the profile viewport. Empty and unavailable profiles
  preserve saved address access. No nested buttons were found in the picker.

The original captures cover Contacts in both themes, Send previews, saving and
retry, direct Add, long-picker content, review, and a simulated receipt. They
remain local artifacts in `output/contacts-evidence/`; a fresh checkout can
reproduce the screens with the browser fixture above.

## Contacts navigation follow-up (2026-09-19)

View in contacts now opens the main Contacts section. The name copies the
human-readable VerusID, its canonical i-address stays out of the profile and
edit form, and Add contact uses the primary button style. Other saved endpoints
remain available and keep their stored payment values. Recipient lookup now
shows progress and a retryable failure. Escape closes a hover preview even when
focus remains in the recipient input, without invoking Send's discard shortcut.

- Focused mounted checks: 26 passed across profile preview, Contacts and the
  production WalletLayout/Send lifecycle. Coverage includes retry, late results,
  duplicate selection, copying the name while retaining the stored canonical
  address, draft preservation and disposal on session replacement.
- Svelte check, scoped ESLint, UI style lint, docs validation, diff whitespace
  check and production build passed.
- Native macOS Tauri WebDriver used the designated disposable `mijn app` testnet
  wallet and real profile RPCs at 920×620 in light and dark mode. Alice's saved
  profile and Bob's gray unsaved trigger appeared; a nonexistent ID showed the
  retry message. No payment was reviewed, signed or submitted in these checks.
- View in contacts selected Alice without a second detail dialog. Back restored
  the exact recipient input node, `1.25` amount and `TestAlice.antafri@` value.
  Escape while Contacts was open did not invoke the hidden Send form.
- The initial missing trigger did not reproduce after restarting the native app.
  Its original cause remains unconfirmed. The WebDriver's `moveTo` emits
  mousemove rather than pointerenter, so hover opening was verified with an
  injected pointer-enter event in the native webview; physical pointer travel
  remains unverified. The Escape regression failed before the fix and passed
  afterward in both the mounted test and native webview.

Follow-up screenshots were captured in `output/contacts-followup-evidence/` and
remain local verification artifacts, outside version control.
