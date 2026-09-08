---
owner: lite-wallet-team
last_reviewed: 2026-09-08
---

# Test wallet Keychain helper

macOS-only developer tooling for the explicitly disposable **mijn app** testnet
wallet. It uses the app's normal password unlock flow; it does not alter wallet
security or expose recovery material.

## Setup

From this checkout, run:

```sh
./scripts/test-wallet-keychain.sh setup
```

Enter the **existing wallet password in the native masked dialog**, then save.
Never put a password in chat, command arguments, environment variables, files,
or the clipboard. Setup stores the password; it does not prove it is correct.
The password must be unique to this disposable wallet.

The helper uses the local login Keychain, service
`com.maxtheyse.verus-express.test-automation`, account `mijn app`. The item is
bound to the wallet's actual testnet account ID and this checkout's development
executable path. Recreating the wallet or moving to another checkout requires
explicit removal and setup again. Setup can replace the password for an unchanged
binding.

## Repeat testing

Start the wallet normally with the repository-pinned `pnpm tauri dev`. Select
**mijn app**, close drawers/dialogs, and leave the password hidden.

```sh
./scripts/test-wallet-keychain.sh status
./scripts/test-wallet-keychain.sh preflight
./scripts/test-wallet-keychain.sh unlock
```

- `status` reads only the dedicated item's attributes, never its password.
- `preflight` checks testnet metadata, the exact running development executable,
  one window/web area, the visible wallet name, and the sole writable secure
  `unlock-password` field. It does not access Keychain.
- `unlock` rechecks the binding and screen after Keychain retrieval, brings the
  exact wallet app and secure field into focus, and types using native keyboard
  events sent only to that wallet process. Before every key it rechecks account,
  executable, screen, and focus. It then presses the normal English/Dutch unlock button. It
  waits up to 20 seconds for `last_unlocked_at` to advance in the named wallet's
  metadata. This is backend unlock evidence; UI navigation and subsequent test
  actions still need their own verification.
- `fill` performs the same guarded entry and verifies Unlock becomes enabled,
  without pressing it. It deliberately
  leaves the password masked in the field for a subsequent user/automation action.

Agents may use this helper for user-authorized testing of this fixture. Never
retrieve the item with `security ... -w`, print the password, reveal the input,
or inspect secret/recovery screens. Credential access does not authorize other
wallets, mainnet transactions, or unrelated testing actions. No screenshot or
input-value dump is needed to use the helper.

Avoid interacting with the wallet during the brief automated entry. Focus or
screen changes abort entry. Cleanup clears the field only if the same guarded
screen can still be safely focused; otherwise a partial masked value may remain.
Clear that field manually before continuing.

Run native commands outside the restricted command sandbox, using the normal
Codex permission mechanism when needed. A sandbox may report Accessibility as
unavailable even when the host has permission. Honor macOS prompts; do not disable
protections or grant all applications access. If a native permission prompt needs
the owner, let the owner handle it.

## Remove access

```sh
./scripts/test-wallet-keychain.sh remove
```

This removes only this dedicated login Keychain item. It does not delete or lock
the wallet, erase an already-filled field, or end an already-unlocked session.
Lock the wallet separately to end that session. Removal also works if the wallet
metadata was deleted or changed.

## Implementation and limits

The wrapper builds a dependency-free Swift helper under the ignored
`src-tauri/target/test-wallet-keychain/` directory. Xcode command-line tools are
required. Keep that executable at its stable path. Rebuilding it may cause
Keychain to ask for authorization again. The item trusts the helper's own code
identity; it is not configured with an all-applications ACL.

The password passes from a native secure text field to Keychain, then from
Keychain through process-targeted keyboard events into the guarded secure field.
Direct Accessibility value setters did not enable the app's Unlock button during
runtime testing; real input events are required. No global keyboard posting is used. It is never
intentionally written to a plaintext file, shell argument, environment variable,
log, or clipboard. Swift/AppKit/Accessibility may retain in-memory string copies;
this is not a claim of guaranteed memory erasure or protection from a compromised
local account or local input-monitoring software. The helper uses macOS login-Keychain APIs (some carry Apple
legacy API deprecation warnings) to support a local unsigned development tool.

The guard is intentionally narrow: only this checkout's
`src-tauri/target/debug/verus_express` is supported, not release apps, browsers,
other wallets, revealed fields, or ambiguous windows. English and Dutch unlock
button labels are supported. UI structure changes fail closed and require a
helper update. Computer-use tooling may not list an unbundled development
executable; the native helper can still perform this guarded unlock.

Run the focused checks without accessing real credentials:

```sh
./scripts/test-wallet-keychain.sh self-test
sh -n scripts/test-wallet-keychain.sh
```

Self-tests cover explicit testnet metadata, malformed account IDs, missing or
ambiguous wallet names, revealed/multiple/non-writable inputs, modal or ambiguous
windows, traversal bounds, deterministic binding encoding, dummy Unicode
round-trip, locale labels, and rapid re-unlock timing. They do not substitute for a real setup and unlock run.
