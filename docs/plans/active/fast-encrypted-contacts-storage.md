---
owner: lite-wallet-team
last_reviewed: 2026-09-24
---

# Fast encrypted Contacts storage — deferred handoff

- Status: deferred at the user's request on 2026-09-24. Start implementation
  only after a new decision to proceed.
- Scope: Contacts persistence. Keep Contacts encrypted. Watchlist membership
  remains in its current Stronghold store until it is measured separately.
- Related work:
  [Encrypted Address Book and Watchlist](./encrypted-address-book-watchlist.md),
  especially its native measurement and proposed storage sections.

## Why this exists

The current scoped speed changes improve cache use, avoid one unnecessary
Stronghold read for an explicit empty identity list, and show busy feedback
quickly. Three post-change native trials still took **5.612–5.719 seconds to
create** and **5.911–5.912 seconds to delete** a synthetic Contact. The durable
write itself is slow, so the current changes cannot meet a sub-second Contacts
goal.

| Native phase                    | Median of three trials |         Range |
| ------------------------------- | ---------------------: | ------------: |
| UI create to completed view     |                5.713 s | 5.612–5.719 s |
| UI delete to closed dialog      |                5.912 s | 5.911–5.912 s |
| Save Stronghold snapshot load   |                2.802 s | 2.798–2.831 s |
| Save durable commit             |                2.811 s | 2.796–2.825 s |
| Delete Stronghold snapshot load |                2.816 s | 2.808–2.829 s |
| Delete durable commit           |                2.808 s | 2.797–2.813 s |

The first visible busy feedback appeared in 16–32 ms across both actions. Gate
and worker waits, address classification, and serialization were negligible in
these trials. One initial Contacts read took 2.905 seconds. The trials used the
designated disposable testnet wallet, a fresh macOS Tauri debug build,
Stronghold work factor 15, and synthetic receiving-address contacts. They are
**post-change measurements**: there is no native pre-change baseline or
release-equivalent timing. Contact count, snapshot size, and the stored
snapshot's work factor were not recorded. Do not generalize these numbers to
other accounts, platforms, or p95 latency.

## Candidate design

Use a separate authenticated encrypted Contacts file with an independent random
256-bit key protected by Stronghold. Load that key once per unlock into
zeroizing, account- and session-scoped backend memory; clear it on lock. Keep
the public Contacts commands and data model stable. Do not derive the key from a
spending key or store it in the browser.

1. Encrypt each complete Contacts generation with a fresh nonce and associated
   data binding its account, record purpose, and format version. Bound file
   size; fail closed on corruption, unknown versions, or a wrong key. Review the
   existing private-cache AEAD and atomic-file patterns for reuse, without
   sharing its key.
2. Serialize writes per account and take an exclusive cross-process lock. Keep
   the existing session publication guard. Return Save/Delete success only after
   encrypted temporary-file write, file sync, atomic rename, and directory sync.
   Reconcile an uncertain write before retrying.
3. Migrate under the same gate: decrypt the legacy Stronghold Contacts snapshot,
   write and read back the new encrypted file, then durably mark the new format
   canonical in the protected key record before accepting writes. Before that
   marker, migration may restart from the legacy snapshot. After it, a missing
   or corrupt new file must fail closed; never silently restore stale legacy
   Contacts.
4. Define backup, restore, recovery, and key rotation for the Stronghold key
   record **and** encrypted file together before rollout. Preserve legacy data
   until a verified cleanup policy exists.

Retaining a loaded Stronghold snapshot is a smaller possible experiment, but the
measured commit alone was about 2.8 seconds in this build. Do not reduce the
Stronghold work factor or omit durable sync to make the operation appear faster.

## Work to authorize and verify later

1. Confirm the current checkout, dirty state, storage policy, and platform
   backup paths. Measure a pre-change or current baseline under comparable
   production crypto settings, with typical and larger synthetic contact sets.
2. Produce a storage and migration design with explicit crash states,
   cross-process coordination, account separation, file permissions, and
   key/file recovery rules. Obtain independent security review before replacing
   the current store.
3. Build a prototype behind the existing Contacts interface. Test wrong key,
   tampering, nonce uniqueness, interrupted migration and replacement, two
   processes, lock/reopen, and backup/restore. Test native behavior on macOS,
   Windows, and Linux.
4. Compare durable save/delete on the same reference machine and settings. A
   **median under 500 ms and p95 under 1 second** is a provisional target, not
   an observed result or a reason to weaken encryption or durability. Record
   enough samples before making a p95 claim.

The scoped speed changes and measurement instrumentation received an independent
code review with no blocking finding and passed focused Contacts and mounted
tests, `pnpm check`, and Rust checks. The native trials above show the remaining
latency; they do not validate the candidate store. Do not include real contact
values or wallet secrets in performance artifacts. Use the designated
disposable-wallet Keychain helper runbook for future native trials.
