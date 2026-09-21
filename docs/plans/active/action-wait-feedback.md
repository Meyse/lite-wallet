---
owner: lite-wallet-team
last_reviewed: 2026-09-21
---

# Action wait feedback: product spec and implementation plan

- Status: proposed
- Scope: foreground work started by a user action in the desktop wallet
- Outcome: after an action, people can tell what is happening, whether they need
  to wait, and what actually completed, without a second modal for every delay.

## Interaction contract

1. **Acknowledge immediately.** Keep the action in place, prevent duplicate
   activation, and use a specific verb such as **Preparing review**, **Saving
   contact**, or **Looking up address**. Do not hold a completed action on
   screen just to make its spinner visible.
2. **Add detail when the wait matters.** After about one second, reveal one
   quiet explanation near the action if it is still running. Around 10 seconds,
   make the current real phase and useful next step more prominent. These are
   initial visibility thresholds to check in the native app, not claims about
   duration. Do not invent percentages, countdowns, or phases from elapsed time.
3. **Use the existing task surface.** An inline row/status suits local saves and
   lookups; a focused progress area within an existing wizard suits transaction,
   recovery, and setup work. A separate modal is for a choice, required secret,
   or destructive confirmation. Do not stack a waiting modal over an existing
   sheet or wizard merely to repeat its busy state.
4. **Distinguish outcomes.** `Working`, `submitted`, `pending confirmation`,
   `confirmed`, `failed`, and `submission uncertain` have different meanings. A
   locally known txid alone does not prove network acceptance; use backend
   broadcast evidence for Submitted, and chain evidence for Confirmed. A local
   item becomes saved only after durable persistence. On uncertain broadcast,
   check authoritative status before offering another send.
5. **Make interruption honest.** Offer Cancel only when the underlying operation
   can safely stop. If closing only hides the view, label it Close and preserve
   the operation/status. Lock and wallet/session replacement must invalidate
   late UI results; they must not imply that a broadcast was reversed.
6. **Keep it accessible and calm.** One polite status announcement per
   operation, `aria-busy` on the affected control/region, visible focus, reduced
   motion, stable button width, translated sentence-case copy, normal arrow
   cursor, and light/dark layout at 920×620. Keep errors in context with a safe
   next action.

These rules follow
[Apple's progress guidance](https://developer.apple.com/design/human-interface-guidelines/progress-indicators),
[Apple's alert guidance](https://developer.apple.com/design/human-interface-guidelines/alerts),
[NN/g's long-wait guidance](https://www.nngroup.com/articles/designing-for-waits-and-interruptions/),
and
[W3C status-message guidance](https://www.w3.org/WAI/WCAG21/Understanding/status-messages).
The exact thresholds and wording remain product decisions to verify in runtime.

## User-action inventory

Source scan on 2026-09-21. Rows group actions that can share one presentation;
they do not claim measured latency. `Inline` means an action-specific button or
row status, `focused` means a more visible status within the current task, and
`micro` means brief local feedback without a waiting panel.

| Area                                                                                                                                                                                                                                                                                                                              | Actions that can wait after user input                                                                                                                                        | Target                                                                                                                                                                                                                                                            |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Unlock and lock](../../../src/lib/components/wallet/UnlockScreen.svelte), [sidebar](../../../src/lib/components/wallet/AppSidebar.svelte)                                                                                                                                                                                        | Unlock wallet; lock wallet                                                                                                                                                    | Keep the existing Unlocking button; add a nearby slow-wait explanation only if needed. Lock normally goes straight to the Unlock screen; show only a brief busy state if that transition is perceptible, with no extra modal or success toast.                    |
| [Create](../../../src/lib/components/flows/WalletCreation/WalletCreation.svelte), [backup](../../../src/lib/components/flows/WalletCreation/BackupStep.svelte), [import](../../../src/lib/components/flows/WalletImport/WalletImport.svelte), [seed entry](../../../src/lib/components/flows/WalletImport/SeedPhraseStep.svelte)  | Generate seed after entering backup step; create/import wallet; open new wallet; validate entered seed                                                                        | Focused current-step status for creation/import/open; inline validation. Never expose a seed in status or logs.                                                                                                                                                   |
| [Send/Convert](../../../src/lib/components/wallet/sections/TransferWizard.svelte)                                                                                                                                                                                                                                                 | Estimate route/fee after selection; prepare or refresh review; sign/build/broadcast; reconcile uncertain ETH submission; refresh history after submission                     | Focused progress in the existing wizard. Use actual private-send events where available; otherwise use truthful broad phases. Move to the submitted receipt as soon as the send result is authoritative. Do not hold the receipt behind optional history refresh. |
| [Generic Request import](../../../src/lib/components/wallet/WalletLayout.svelte), [import sheet](../../../src/lib/components/flows/GenericRequest/GenericRequestImportSheet.svelte)                                                                                                                                               | Paste/import, parse and verify a request before opening its flow                                                                                                              | Inline verification, with an actionable error in the import sheet.                                                                                                                                                                                                |
| [Generic Request flow](../../../src/lib/components/flows/GenericRequest/GenericRequestFlowHost.svelte)                                                                                                                                                                                                                            | Load linked identities, analyze update, load funding, preflight, authenticate/sign response, deliver callback, provision identity, broadcast identity update, finish response | Focused status for signing/provisioning/broadcast/callback; inline status for analysis and funding reads. Separate a signed response from successful external delivery.                                                                                           |
| [Guard](../../../src/lib/components/flows/VerusIdGuard/GuardFlowHost.svelte)                                                                                                                                                                                                                                                      | Begin guard session, look up target, preflight revoke/recover, sign and submit                                                                                                | Focused status in the existing flow. Retain target/review context; keep uncertain submission distinct from failure.                                                                                                                                               |
| [VerusID management](../../../src/lib/components/wallet/sections/Identity.svelte), [link sheet](../../../src/lib/components/wallet/sections/identity/LinkIdentitySheet.svelte)                                                                                                                                                    | Discover/link/unlink identity; toggle favorite; open/retry detail or profile; refresh/link provisioning jobs                                                                  | Inline row/sheet status for mutations and refresh; contextual detail loading. Keep successful rows visible until the mutation resolves.                                                                                                                           |
| [Profile publication](../../../src/lib/components/wallet/sections/identity/IdentityProfileEditor.svelte), [image editor](../../../src/lib/components/wallet/sections/identity/ProfileImageEditor.svelte)                                                                                                                          | Decode/crop/encode image; prepare/review or resume an update; publish; check confirmation; discard retained publication                                                       | Reuse the existing editor progress and pending states. Refine wording/slow feedback locally; preserve one explicit approval per fee-bearing update and canonical confirmation rules.                                                                              |
| [Contacts](../../../src/lib/components/wallet/sections/AddressBook.svelte), [lookup](../../../src/lib/components/wallet/contacts/IdentityLookup.svelte), [preview](../../../src/lib/components/wallet/contacts/IdentityPreview.svelte), [profile](../../../src/lib/components/wallet/sections/identity/VerusIdProfilePage.svelte) | Resolve an address/VerusID, reveal note, save/edit/delete contact, add from lookup/preview/profile                                                                            | Inline lookup/save/delete status. Do not show Saved before the encrypted store succeeds. Keep the existing delete confirmation and form input on error.                                                                                                           |
| [Watchlist](../../../src/lib/components/wallet/sections/Watchlist.svelte)                                                                                                                                                                                                                                                         | Resolve target, add/remove entry, refresh public balances                                                                                                                     | Inline status in the current sheet, row, or list. Show persistence and optional balance refresh as separate outcomes.                                                                                                                                             |
| [Manage assets](../../../src/lib/components/wallet/AddAssetSheet.svelte)                                                                                                                                                                                                                                                          | Discover/refresh assets, resolve manual currency/contract, register/add asset, show/hide/dismiss an asset                                                                     | Inline sheet/row status; preserve partial discovery results and row-specific errors.                                                                                                                                                                              |
| [Asset details](../../../src/lib/components/wallet/sections/AssetDetails.svelte), [Receive](../../../src/lib/components/wallet/sections/Receive.svelte)                                                                                                                                                                           | Select scope, load/retry balance or transactions, load older history, fetch receive addresses after opening Receive                                                           | Contextual content status and retry; do not cover already usable balances/history with a modal.                                                                                                                                                                   |
| [Private Verus](../../../src/lib/components/wallet/settings/PrivateVerusSettings.svelte)                                                                                                                                                                                                                                          | Set up/import a private seed, resume setup, manually retry status                                                                                                             | Focused setup status with a clear completion/activation result. Never display sensitive material in waiting copy.                                                                                                                                                 |
| [Recovery keys](../../../src/lib/components/wallet/settings/RecoveryKeysSettings.svelte), [password dialog](../../../src/lib/components/common/PasswordConfirmOverlay.svelte)                                                                                                                                                     | Verify password and reveal secrets; generate QR                                                                                                                               | Busy state in the existing password dialog; contextual QR status. Clear secrets on dismissal as today.                                                                                                                                                            |
| [Other settings](../../../src/lib/components/wallet/sections/Settings.svelte)                                                                                                                                                                                                                                                     | Persist auto-lock choice; refresh private status/version information                                                                                                          | Micro or inline save/error feedback. An optimistic choice must not silently mask a failed persistence call.                                                                                                                                                       |

The current
[lock coordinator](../../../src/lib/services/walletLockCoordinator.ts) redirects
to Unlock even if the backend lock command fails. This feedback work must not
display a "Locked" success claim based only on that redirect. Handling backend
or navigation failure is a separate security-behavior decision, not a copy-only
change.

Clipboard copy/paste, external-link opening, tab changes, and local navigation
get micro success/error feedback where needed. They should not acquire a delayed
progress panel solely because their handler returns a Promise. Passive startup,
polling, rates, and automatic balance refresh remain governed by
[wallet loading lifecycle](../../architecture/wallet-loading.md).

## Implementation plan

1. **Baseline and state audit.** Time the user action to first painted feedback
   and completion for representative slow paths in a native development build.
   Record only operation kind, phase, and duration; never names, addresses,
   secrets, payloads, or stable wallet IDs. Confirm the action inventory against
   runtime and prioritize waits actually observed. Preserve existing transaction
   and profile state machines.
2. **Small presentation primitive.** Add one reusable localized status treatment
   for an inline action and a focused wizard state, using existing tokens and
   Lucide spinner. It renders supplied state; it does not own signing,
   persistence, navigation, timers for fake phases, or global operation state.
   Keep one announcement source and stable dimensions.
3. **High-stakes flows first.** Apply to Send/Convert, Generic Request, Guard,
   and profile publication. Feed phases from backend evidence where present;
   otherwise use a broad truthful label. Audit cancellation, txid/pending,
   uncertain broadcast, lock/session replacement, and retry copy before visual
   polish.
4. **Local mutations next.** Apply to Contacts, Watchlist, VerusID linking,
   favorites, Manage assets, and settings saves. Follow the existing
   [encrypted Address Book and Watchlist plan](./encrypted-address-book-watchlist.md)
   for their storage/latency work; do not solve slow persistence by extending a
   spinner or trusting renderer-supplied resolution data.
5. **Read and entry flows.** Apply contextual status to lookups, asset history,
   Receive, unlock/create/import, private setup, and secret reveal. Keep partial
   usable content visible. Recheck any remaining `await` reached from a user
   handler; classify fast clipboard/navigation work as micro feedback.

## Acceptance

- Every inventory action acknowledges activation, settles to a truthful result,
  and exposes an actionable failure. Mutations cannot submit twice while busy;
  stale lookups cannot replace a newer result.
- Slow and failure-injected cases show a useful status without fake progress or
  misleading Cancel/Retry. Late results cannot update another wallet/session.
- Transaction submission, chain confirmation, local persistence, and external
  callback delivery remain distinct in UI and tests.
- Verify English/Dutch, light/dark, reduced motion, keyboard/screen-reader
  status, and 920×620 plus a larger desktop size. Use mounted tests for state
  transitions; use native Tauri checks for real wait behavior, storage, signing,
  and recovery. Browser fixtures alone do not prove native timing or settlement.
