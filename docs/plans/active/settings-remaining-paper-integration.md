---
owner: lite-wallet-team
last_reviewed: 2026-09-17
---

# Plan: Integrate the remaining settings designs from Paper

- Status: ready for implementation; this document does not implement the
  changes.
- Source review: 2026-09-17. Recheck the checkout and live Paper document before
  starting.
- Repository: `/Users/maxtheyse/dev/lite-wallet`

## Assignment

Implement the approved About and support, Private Verus, and Recovery and keys
designs in the existing Svelte/Tauri app, including private-key QR views. Match
the current compact settings design in light and dark mode at the minimum 920 ×
620 app window. Complete local implementation and verification.

The settings hub, Display and language, appearance controls, currency picker,
and shared dropdowns have already been integrated. Preserve those as the working
design baseline; change their routing only where required by this plan. Do not
repeat the earlier dropdown migration or redesign Profile and security.

Do not modify Paper, redesign the wallet shell, change cryptography or key
derivation, change backend storage semantics, or add unrelated settings. Do not
commit, push, create a PR, or deploy unless separately requested.

## Design authority and required reading

Read the repository and frontend `AGENTS.md`, `docs/context-packs/ui-i18n.md`,
and `docs/ui-style-governance.md`. Apply these local skills before UI
implementation:

- `/Users/maxtheyse/.agents/skills/max-product-design-taste/SKILL.md`
- Its `references/taste-contract.md`
- `/Users/maxtheyse/.codex/skills/frontend-design/SKILL.md`

The approved Paper design and the explicit requirements below determine visual
direction. General design skills must not introduce a different aesthetic.
Current code determines supported data, backend behavior, and security
boundaries. If a Paper example assumes unavailable data, adapt the state
truthfully rather than fabricate it or broaden backend scope.

Paper project:
[Verus Express — Settings · Light & dark](https://app.paper.design/file/01M2NWWRC7BRZZGJ7BPX8GEFDN/2-0).
Use page **Next · Settings** (`2-0`), containing 22 artboards: 11 states in both
themes. Page 1 is context for already integrated screens, not this task's
backlog.

Use the Paper MCP directly:

1. Read its guide and open file `01M2NWWRC7BRZZGJ7BPX8GEFDN`, page `2-0`.
2. Read the live artboard inventory and inspect both themes.
3. Use `get_tree_summary`, `get_jsx` with inline styles, and
   `get_computed_styles` to obtain exact structure, spacing, colors, and type.
4. Use screenshots for visual comparison, not as the sole source of CSS values.
5. Translate Paper output into the project's Svelte components and semantic
   tokens; do not paste exported JSX or absolute-position an entire screen.

If Paper is unavailable, complete source analysis and report that exact visual
implementation is blocked. Do not invent a replacement design or claim a match.

### Approved artboard map

IDs are lookup aids; verify live names because the document may change.

| State                                     | Light   | Dark    |
| ----------------------------------------- | ------- | ------- |
| 01 · About and support                    | `209-0` | `22R-0` |
| 02 · Private Verus · Set up               | `256-0` | `283-0` |
| 03 · Private Verus · Configured           | `2B0-0` | `2DW-0` |
| 04 · Private Verus · Import               | `2GS-0` | `2K9-0` |
| 05 · Private Verus · Advanced replacement | `3BE-0` | `3ED-0` |
| 06 · Private Verus · New phrase backup    | `3QG-0` | `3TZ-0` |
| 07 · Recovery · Password confirmation     | `3XI-0` | `3Y0-0` |
| 08 · Recovery and keys · Unlocked         | `2XI-0` | `30R-0` |
| 09 · Recovery · Primary secret revealed   | `340-0` | `37P-0` |
| 10 · Recovery · Private key actions       | `3HC-0` | `3LW-0` |
| 11 · Recovery · Private key QR            | `403-0` | `43W-0` |

The old locked recovery landing page and old redesigned password dialog were
removed deliberately. Do not reconstruct them from older history.

### Visual contract

- Clean, restrained, compact desktop UI using the wallet's existing language.
  Google Sans, Verus blue, neutral surfaces, and Lucide icons.
- No persistent card borders, row dividers, or outlined dropdown triggers. Use
  subtle surface changes and spacing. Keep visible keyboard focus rings.
- Reuse `bg-app-canvas`, `bg-settings-surface`, settings control, muted text,
  and focus tokens in `src/app.css`; avoid a parallel palette.
- Paper geometry at 920 × 620: existing 244px sidebar; content starts at x276
  and spans 612px. Right sheets are 378px wide with 24px horizontal padding.
  Preserve the real app shell and native window controls; do not draw duplicate
  traffic lights or an extra sidebar in settings components.
- Page headings are approximately 20px/28px semibold; body and controls mostly
  13–14px, secondary copy 12–13px. Read exact values from the relevant nodes. Do
  not enlarge controls or add explanatory paragraphs.
- Keep icon/action columns aligned, with adequate button hit areas around small
  icons. Use the normal arrow cursor, accessible labels, and focus treatment.
- Use existing Sheet and ScrollArea primitives. Override default sheet borders
  within these settings views; avoid global changes that affect other flows.
- All new copy, tooltip labels, errors, network names, and close labels use
  `i18n.t(...)`, with English and Dutch translations and sentence case.
- Reuse shared dropdowns if a selector is needed. No new visual variant.

## Source map

Paths below are relative to the repository root.

| File or directory                                                   | Role                                                                             |
| ------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `src/lib/components/wallet/sections/Settings.svelte`                | Category routing and recovery return destination                                 |
| `src/lib/components/wallet/settings/AboutSupportSettings.svelte`    | About implementation                                                             |
| `src/lib/components/wallet/settings/PrivateVerusSettings.svelte`    | Setup, import, configured state, replacement, backup                             |
| `src/lib/components/wallet/settings/RecoveryKeysSettings.svelte`    | Authentication orchestration, overview, detail sheets, QR state                  |
| `src/lib/components/wallet/settings/ProfileSecuritySettings.svelte` | Existing recovery entry point; preserve other controls                           |
| `src/lib/components/wallet/settings/DisplayLanguageSettings.svelte` | Existing approved page and sheet styling reference                               |
| `src/lib/components/common/PasswordConfirmOverlay.svelte`           | Reuse unchanged; visual exception to new settings styling                        |
| `src/lib/components/common/IdentifierText.svelte`                   | Existing raw value display                                                       |
| `src/lib/components/common/CommunityHangoutButton.svelte`           | Existing community destination/open behavior                                     |
| `src/lib/components/common/DropdownSelectTrigger.svelte`            | Shared dropdown treatment                                                        |
| `src/lib/components/ui/sheet`, `scroll-area`, `copy-button`         | Reusable interaction primitives                                                  |
| `src/lib/utils/appInfo.ts`, `clipboard-feedback.svelte.ts`          | Runtime metadata and truthful clipboard feedback                                 |
| `src/lib/services/walletService.ts`, `src/lib/types/wallet.ts`      | Existing command contracts and result types                                      |
| `src-tauri/src/commands/wallet.rs`                                  | Read to verify setup and recovery semantics; backend changes are not the default |
| `src/lib/i18n/locales/en.ts`, `nl.ts`                               | Localized text                                                                   |

## Implementation sequence

### 1. Establish the baseline

- Check Git status, branch, applicable instructions, and active work before
  editing. Preserve unrelated changes. Do not reset, stash, or blanket-format.
- Inspect current screens and semantic tokens against live Paper.
- Inventory existing sheet, copy, dialog, and settings primitives. Extract only
  small reusable pieces genuinely shared by these screens.
- Use the pinned Node and pnpm versions from `package.json` (currently Node
  22.23.2 and pnpm 11.24.0). Read the package-manager policy before dependency
  work.
- Keep a short state checklist using the artboards above plus loading, error,
  unavailable-data, and cancellation cases below.

### 2. About and support

- Match artboard 01: compact brand/app identity and version, followed by the
  community support row with its external-link affordance.
- Use the existing Verus logo asset and `loadRuntimeAppInfo()`. Never hardcode
  Paper's example version or duplicate app metadata.
- Preserve the existing community URL and native external-opening mechanism.
  Reuse or minimally adapt the shared component without changing other callers.
- Handle metadata loading/failure without an endless loading state; retain the
  usable support action. Do not add unrelated links or diagnostics.

### 3. Private Verus

- Artboard 02: show the unconfigured status and three compact choices: reuse the
  wallet's recovery phrase, create a separate phrase, and import a secret. Keep
  import fields out of the initial page.
- Preserve `getDlightSeedStatus()` and `setupDlightSeed()` with modes
  `reuse_primary`, `create_new`, and `import_text`.
- Reuse-primary accepts a valid 24-word mnemonic in the current backend. Do not
  imply that custom seed text, WIF, or a raw private key can be reused this way.
  Use existing nonsecret capability metadata if available; otherwise surface the
  backend's actionable validation error without fetching recovery secrets early.
- Artboard 03: show configured status, actual network and shielded address with
  existing copy feedback, a Recovery and keys row, and collapsed Advanced
  options. Never present a status-fetch failure as permission to replace an
  existing seed; keep setup mutations disabled until status is known.
- Artboard 04: Import opens a right sheet with a labeled surface textarea,
  Cancel and Import. Accept the existing phrase/spending-key formats, preserve
  backend validation, prevent duplicate submission, and show local errors. Clear
  submitted secret text after success and on dismissal/unmount.
- Artboard 05: reveal replacement choices only after Advanced is selected, with
  the approved warning about backing up the current private secret and losing
  access to existing funds. Merely opening Advanced must never mutate storage.
  Cancel returns to configured status without calling setup.
- Artboard 06: after `create_new` succeeds, show the generated phrase in the
  backup sheet, with Copy phrase, the saved-phrase acknowledgement, and Done.
  Done is disabled until acknowledged. Avoid accidental backdrop/Escape
  dismissal before acknowledgement; retain the explicit close path shown in
  Paper. Closing must not claim to undo setup. Locking or leaving the wallet
  must still clear the displayed phrase; it remains recoverable through
  password-gated recovery.
- **Existing persistence semantics:** setup stores the secret before returning
  `generatedSeedPhrase`; Done acknowledges backup, not storage. Do not delay or
  repeat setup on Done. Display activation/relogin copy based on
  `requiresRelogin`; do not claim private transactions are immediately active.
- Use real mainnet/testnet labels, not Paper's hardcoded Mainnet example.
- Prevent stale status/setup results from populating a different wallet/network
  or an unmounted view. Respect the existing wallet session lifecycle.

### 4. Recovery entry and unchanged password confirmation

- Selecting Recovery and keys opens `PasswordConfirmOverlay` immediately. Remove
  the intermediate page/button labeled View recovery and keys.
- Reuse `PasswordConfirmOverlay.svelte` **without changing its appearance**. Do
  not restyle its Input or Button globally, add a dialog card, title, scrim, or
  substitute the settings sheet. Its existing opaque full-screen layout, focus
  treatment, password visibility control, Cancel, and Reveal remain.
- Separate successful confirmation from cancellation. Success stores the result
  only for the active recovery view and reveals the overview. Cancel clears
  transient state and returns to the originating screen.
- Track origin explicitly in Settings: entry from Profile and security returns
  there; entry from Private Verus returns there. Preserve Settings' reset
  behavior. Add the necessary Private Verus callback. Do not add another
  top-level category.
- Continue calling `getWalletRecoverySecrets()` only after explicit
  confirmation. Preserve invalid-password and secure-storage-unavailable errors
  and loading behavior. Opening the prompt is not authorization to retrieve
  secrets.
- Invalidate pending requests on cancellation, navigation, lock, wallet/network
  changes, and destruction. A late success must not restore secrets or reopen
  UI.

### 5. Recovery overview and detail sheets

- Artboard 08: one concise warning, then compact rows for primary recovery
  secret, Private Verus recovery secret when present, derived private keys, and
  addresses. Show real secret-kind descriptions.
- Use right sheets for detail. Show one detail at a time, rather than the
  current long page containing every secret and address.
- Artboard 09 is a revealed-state reference. Start secrets masked and reveal
  only through the relevant explicit action. Keep full values readable/wrappable
  using IdentifierText; never irreversibly truncate the only available value.
- Artboard 10: aligned per-key Reveal/Hide, Copy, and Show QR actions. Copy uses
  the existing clipboard utility and reports success only after the write
  succeeds.
- Keep every currently supported recovery value accessible. Map primary kinds
  `seed_text`, `wif`, and `private_key_hex` correctly. Map optional Private
  Verus mnemonic/spending-key data according to `WalletRecoverySecretsResult`.
- For the Private Verus spending-key row use `dlightDerivedSpendingKey` when
  returned, or `dlightSecret` when its kind is `spending_key`. Never put a
  mnemonic in a field or QR labeled spending key. Do not derive new keys in the
  frontend.
- Addresses and Private Verus recovery detail are not separately drawn for every
  variant in Paper. Compose them from the same approved sheet/row primitives;
  preserve current fields and handle missing values honestly.
- Back within a detail returns to the overview; closing a sheet remasks its
  contents. Leaving recovery clears secrets, passwords, visibility flags,
  copy-feedback timers, and QR data. Re-entering requires password confirmation.

### 6. Private-key QR detail

- Artboard 11: clicking a key's QR action replaces the key-list content inside
  the same sheet. Do not stack another modal or reveal every key.
- Display the selected key's name, actual wallet/network, and representation
  (WIF, private key, or spending key). Back returns to the key list; X dismisses
  the sheet. Clear the QR on either action.
- Render black modules on an opaque white background in both themes, preserving
  the quiet zone and crisp edges. Match the approximately 260px Paper QR region.
- Show the localized warning: “This QR code contains your private key.”
- Generate QR locally only after explicit selection. Encode exactly the selected
  export value used by Copy, not its masked display, address, explanatory text,
  JSON wrapper, or an invented URI scheme. Do not promise compatibility with an
  external wallet without testing its supported import format.
- Reuse a suitable local QR encoder if one exists. If a dependency is needed,
  inspect compatibility and add a narrowly scoped browser-compatible package
  with pinned pnpm. Do not hand-roll QR encoding or use a remote QR service.
- Keep payload and rendered QR ephemeral. No persistence, logging, analytics,
  network requests, download/share actions, URL parameters, or hidden prefetched
  QR images. Do not put secret payloads in alt text or accessible labels.
- Invalidate asynchronous encoding on selection change, back, dismissal, lock,
  navigation, and wallet/network changes. Stale QR output must never reappear.
- Provide local loading/error feedback and an explicit retry if encoding fails.
  Missing keys have no enabled Copy/Reveal/QR action.
- Paper's QR encodes a nonfunctional design example. Never ship that image as
  the actual QR implementation. QR for recovery phrases is outside this scope.

## Verification and acceptance

Use deterministic synthetic fixtures for all secret-bearing screenshots, DOM
snapshots, QR decoding tests, and clipboard tests. Never capture real wallet
secrets in artifacts, logs, Paper, prompts, or clipboard automation. Stub
external opening and clipboard operations in automated tests.

Add focused behavioral tests where this work changes behavior:

| Area           | Required coverage                                                                                                                                                   |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Recovery entry | Immediate existing password prompt from both origins; correct Cancel/Back destination; no preliminary secret fetch                                                  |
| Authentication | Empty/invalid password, storage error, loading, success; cancel/unmount/lock during pending request; stale results discarded                                        |
| Details        | Masked defaults; independent reveal/copy states; missing Private Verus data; all original fields remain accessible; cleanup on exit                                 |
| QR             | Each supported key mapping; exact synthetic payload round-trip through a decoder; black/white output and quiet zone; back/close cleanup; stale encoding/error cases |
| Private Verus  | Known/unknown/error status; each setup mode and failed validation; duplicate submission prevention; import cleanup; wallet/network change during requests           |
| Backup         | Generated phrase after persistence; Done requires acknowledgement; closing does not rerun or undo setup; activation wording follows result                          |
| About          | Runtime version, loading/failure, unchanged external destination                                                                                                    |

Follow the existing `*.mounted.ts` and Vitest conventions. Add tests for
behavior, not assertions that simply mirror CSS or component internals.

Run the narrow tests while iterating, then the appropriate repository checks:

```sh
pnpm check
pnpm lint
pnpm test
pnpm build
pnpm docs:check
```

Run Prettier checks on changed files. Do not rewrite unrelated formatting to
make a global check pass. Separate pre-existing failures from failures
introduced here. If backend changes unexpectedly become necessary, justify the
scope, read its local guidance, and add the relevant Rust checks.

Inspect rendered app states, not only Paper or jsdom:

- All 11 approved states in light and dark at 920 × 620, plus a larger desktop
  window. Use actual-length synthetic values, English and Dutch, and long
  errors.
- Check hierarchy, compactness, matching tokens, no accidental borders, aligned
  actions, wrapping, scrolling, and absence of clipped controls.
- Check keyboard operation, visible focus, sheet focus containment and return,
  accessible icon names, Enter/Cancel behavior, and reduced-motion handling.
- Confirm the shared password screen remains visually identical to the baseline.
- Smoke-test existing settings home, Display and language, theme persistence,
  currency picker, dropdowns, and auto-lock for regressions.
- Start native verification with `pnpm tauri dev` using the pinned toolchain and
  the repository's Tauri testing guidance. Browser fixtures prove rendering and
  mocked interactions only; distinguish them from native command integration. Do
  not mutate an existing wallet's private seed just to obtain screenshots. Use a
  disposable test wallet only within its runbook and authorized scope.

## Completion report

Deliver the implementation with a concise file/change summary, Paper state
coverage, tests and rendered evidence, any intentional deviations with reasons,
and remaining verification gaps. Specifically report whether native
authentication, setup/storage, clipboard, and external-opening paths were
exercised or only mocked.

The task is complete when the in-scope screens match the approved Paper designs
in both themes, supported data and existing backend protections are preserved,
the redundant recovery step is gone, the original password UI is unchanged, QR
output is correct and transient, and the relevant checks pass. Do not label
unperformed native or real-wallet checks as passed.
