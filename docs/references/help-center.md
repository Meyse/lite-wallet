---
owner: lite-wallet-team
last_reviewed: 2026-09-22
---

# Help center

Help above Settings and Get help on Welcome/Unlock open the same full-window
reader. The underlying screen remains mounted. The shared dialog traps focus,
Escape closes it, and closing restores the entry control. During Help, the
transfer wizard is inactive: a retained review requires refresh on return. Help
cannot open during a locked transfer operation. Replacing the wallet session
dismisses Help; incoming requests wait until it closes.

## Content and search

Help uses a centered layout without a category sidebar. Home presents a
prominent search field and six icon tiles; articles appear after choosing a
category or searching. Search stays visible for results and is hidden in
categories and articles. Category Back returns home; article Back returns to its
category, search results or previous related article. An article opened directly
returns to its category. The host Back control and community link remain in the
header.

The catalog in `src/lib/help/catalog.ts` defines six categories and 29 stable
article IDs, related links and official external references. Copy lives in
`src/lib/i18n/locales/help/en.ts` and `nl.ts` and is read through `i18n.t(...)`.
Dutch is checked against the English key set. German and Spanish use the
existing English fallback for Help. No content or search query is fetched or
sent to a service. Core Help therefore works offline and before unlocking.

Search matches all entered terms across titles, synonyms, summaries and body
text, ranking title matches first. Opening related articles keeps a navigation
stack; Back restores the earlier search/category and scroll position. Article
text uses a small allowlisted token, `[[destination-id|localized words]]`,
parsed into escaped text and buttons. It never renders HTML, Markdown URLs or
remote embeds. Only the screen reference is linked, usually once per article.

## Screen navigation

`src/lib/help/destinations.json` is the shared destination authority. It maps
Manage assets, Send, Receive, Convert, VerusID, Contacts, Watchlist and four
Settings pages to intentional navigation targets. WalletLayout owns dispatch:
Manage assets opens the actual asset manager, Settings selects the requested
page, and Send/Convert reuse the existing resume/replacement guard. Links cannot
submit, sign, reveal recovery material, or bypass unlocking. Before unlock,
clicking a screen link explains the requirement and leaves Help open.

The dialog host retains its reader state across closing: article, query,
category, history, scroll position and the focused screen link. The state is
memory-only and discarded when the wallet session changes. Existing transfer and
identity sessions remain owned by WalletLayout; Help does not replace drafts.
English and Dutch contain localized links; German and Spanish retain the
existing English article fallback and have localized unlock notices.

## Editing the copy

The [English browser editor](./help-copy/editor.html) is a standalone HTML file.
It contains all articles and labels, preserves a browser-local draft, and
exports `wallet-help-edits.json` with stable translation keys and before/after
values. Select text in an article, choose **Link to screen**, and use **Add
link**. The links below the text show each label and destination; select one to
change its destination or remove it. Editing the label retains the destination
ID. Edits crossing a link boundary remove that link rather than guessing its
target. Open it in a browser, edit the English copy, then use Download edits.
The user has requested that other supported languages be updated when those
edits are applied. Import/export is local; the editor has no network
dependencies.

The balances-and-prices article has been removed from app navigation, search,
related links and the editing documents. Its original baseline keys remain in
the editor only for draft/import compatibility. Existing edits to a removed
article are retained in backups and exports, with `removedArticleIds`
identifying content that must not be reapplied. A saved selection of that
article returns to the first available article. Other pending editor changes
remain unapplied.

The original text `baseline`, `sourceId`, `initialChanges`, and `copyRevision`
remain intact. The Secret Recovery Phrase correction is still a pending editor
copy revision; it is not assumed to have been applied to app text. Existing
browser edits remain local and are never loaded as applied application copy.

Version 2 drafts/exports add `linkChanges`. Each exported entry has a
translation `key` and `before`/`after` arrays of `{ start, end, destination }`,
using UTF-16 positions in that translation's plain text. The embedded
`baselineLinks` and `destinations` are checked against app source by
`editor.test.ts`. Exports contain only changed text fields and changed link
arrays, including link-only edits. Version 1 text-only drafts and files still
load. Default links survive in unchanged fields; edited legacy fields stay
unlinked rather than matching words. Invalid/corrupt drafts are preserved;
conflicting source IDs, versions, link baselines, ranges, overlaps and unknown
destinations are rejected before writes.

For an exported edits file, compare each `before` value with the **plain text**
of current source (strip only parsed link tokens), and compare link `before`
arrays with current source links. Apply ranges to the corresponding `after`
text, encoding links as `[[destination|label]]`; update localized links in the
other languages. Resolve conflicts instead of guessing range positions or
matching visible labels. For text-only legacy files, retain or re-author links
explicitly.

Align Dutch, German and Spanish copy for the affected content. Browser autosave
is local to that browser and origin; the downloaded file is the portable handoff
and backup. Preserve that file and browser drafts when refreshing the editor
snapshot.

Use the editable [English copy](./help-copy/en.md) or
[Dutch copy](./help-copy/nl.md). Each contains all 29 articles, category
headings and Help labels. Edits are applied to the app only when requested.

When applying edits, compare the document with `help-copy/baseline.json` and
apply only the changed fields to the corresponding locale. Reference comments
identify categories and articles even when their titles change. The article
heading maps to `.title`, its first paragraph to `.summary`, and its remaining
paragraphs to `.body`; the summary is displayed as ordinary article text. Label
table references are translation keys. Preserve search keywords, related links
and untouched app copy. Check current wallet behavior before applying changed
factual claims, and flag conflicts with newer app copy instead of overwriting
it.

Treat these documents as user drafts. Do not regenerate them over manual edits.
After applying an agreed revision, update its baseline and keep the editing
documents aligned with the applied text. English is the user's editing language;
apply the requested translations to the other supported languages.

## Source and claim review

Public sources were checked on 2026-09-22. Wallet behavior is grounded in
current source, not in capabilities of other Verus wallets or unimplemented
plans.

| Articles                                                              | Local evidence and public references                                                                                                                                                                  |
| --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| wallet, assets, networks                                              | `WalletCreation/`, `WalletImport/`, `Overview.svelte`, `AddAssetSheet.svelte`, `transferSources.ts`, `walletDisplayService.ts`                                                                        |
| receive, send, pending, fees, uncertain                               | `Receive.svelte`, `AssetDetails.svelte`, `TransferWizard.svelte`, `transferReceipt.ts`, `transferFeeReview.ts`, `src-tauri/src/commands/transaction.rs` and `src-tauri/src/core/channels/eth/send.rs` |
| convert, conversion-estimate                                          | Transfer route/estimate/review UI and [Verus DeFi and payments](https://verus.io/build/defi-payments)                                                                                                 |
| cross-chain, ethereum                                                 | `src-tauri/src/core/channels/eth/bridge/`, saved Ethereum submission recovery, and [Verus-Ethereum Bridge](https://verus.io/ethereum-bridge)                                                          |
| verusid, link-identity                                                | `Identity.svelte`, `VerusIdProfilePage.svelte`, identity commands and [VerusID](https://verus.io/verusid)                                                                                             |
| public-profile, publish-profile, profile-pending, profile-unavailable | `IdentityProfileEditor.svelte`, `VerusIdProfilePage.svelte`, `identitySectionSessionState.ts`, `src-tauri/src/commands/identity.rs`, profile publication services and encrypted continuation storage  |
| recovery, forgot-password, private-verus, private-send                | `WalletImport/`, `RecoveryKeysSettings.svelte`, `PrivateVerusSettings.svelte`, `TransferWizard.svelte`, private preflight/send backend                                                                |
| guard, requests                                                       | `VerusIdGuardDock.svelte`, `VerusIdGuard/`, `GenericRequest/`, `WalletLayout.svelte`, [VerusID control model](https://verus.io/verusid)                                                               |
| contacts, watchlist, local-data                                       | `address_book.rs`, `watchlist.rs`, [contact storage reference](./contacts-implementation.md), account state storage                                                                                   |
| settings, support                                                     | `Settings.svelte`, settings detail screens, `externalLinks.ts`                                                                                                                                        |

Maintain these distinctions when editing:

- Submitted, confirmed on the source network, and received on the destination
  are separate states. An uncertain submission is not a failed transaction.
- Quotes and timing estimates are not guarantees. Help does not report live
  bridge availability. Route capability checks remain authoritative.
- The current profile publisher is restricted to supported VRSCTEST identities.
  Profile publication is separate from identity registration.
- Profile removal does not erase public blockchain history. Display by other
  apps depends on support for the profile format.
- Importing keys does not restore local contacts, notes or every setting. A
  separate privacy secret needs a separate backup.
- Guard acts through configured identity authorities. It is not a password reset
  or a general recovery mechanism for ordinary address funds.
- Activity and Apps are currently placeholder sections. Asset history is the
  supported place to inspect transactions.

## Verification

`catalog.test.ts` covers complete localized articles, connected references and
ordinary search terms. `HelpCenter.mounted.ts` covers search/history,
categories, empty results, translated content, pre-unlock entry, Escape and
focus return. `links.test.ts` checks allowlisting, safe parsing and all
supported locales. `editor.test.ts` uses jsdom for link authoring, range edits,
version compatibility, changed-only exports, corrupt-draft protection and inert
HTML-like copy. The transfer lifecycle suite covers direct Help routing,
preserved drafts, conflicting Send/Convert guards, review invalidation and
blocked Help entry during submission. Overview and Settings mounted tests
exercise the actual destination views and confirm that opening Settings does not
retrieve recovery secrets.

`dev/browser-fixtures/help.html` renders the real Help components and sidebar
with a harmless editable field. `?frame` uses the browser viewport directly;
without it, an iframe supplies the 920×620 desktop minimum. Add `theme=dark` or
`locale=nl` to inspect those states. This is browser layout/navigation evidence,
not a native wallet or network transaction test.

The originating task's browser tool denied the standalone editor file URL. This
implementation does not reopen that file through another browser surface or
serve it over localhost to bypass the denial. Editor evidence is limited to DOM
behavior and static layout review; localhost visual evidence covers the app Help
fixture. No native Tauri flow or network operation is proved by these checks.
