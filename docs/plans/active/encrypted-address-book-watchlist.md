---
owner: lite-wallet-team
last_reviewed: 2026-09-19
---

# Plan: encrypted Address Book and Watchlist storage

- Status: proposed; research and planning complete, implementation not started
- Owner: lite-wallet-team
- Last updated: 2026-09-19
- Research baseline: `main` at `88fb350`
- Authorization for this task: create the plan only; do not migrate wallet data
  or change runtime behavior yet

## Contacts implementation overlap (2026-09-19)

The Contacts/profile task implemented serialized, session-bound Contacts
read/change/commit operations and immediate preview save feedback using the
existing Stronghold snapshot. See
[its evidence record](../../references/contacts-implementation.md). This does
not complete the Watchlist migration, shared record-family abstraction,
production benchmarks, or native validation proposed below. The research
baseline in the following sections describes the earlier source.

## Goal

Keep Address Book and Watchlist metadata encrypted behind wallet unlock, make
every save/remove operation understandable, and reduce avoidable waiting without
weakening encryption or acknowledging a change before it is durable.

The intended user experience is simple: press Save or Remove, receive immediate
action-specific feedback, and see the change only after it is safely stored.
Balance lookups should not determine whether a Watchlist entry can be saved.

## Recommended decisions

1. Use the existing Stronghold protection for both features. Keep the existing
   Address Book file and add a separate encrypted Watchlist file, sharing the
   storage implementation and lifecycle rules. A single combined file is not
   required to give both features the same protection.
2. Treat the selection of watched addresses and identities as private metadata.
   An address can be public while its association with a particular wallet's
   Watchlist is private.
3. Implement one serialized read/change/commit operation per encrypted file,
   running its blocking work outside Tokio's async workers. Preserve the latest
   stored state, including the other network, on every mutation.
4. Migrate both plaintext Watchlist representations and the older encrypted
   watched-address representation. Never leave the duplicate plaintext address
   list as the source for discovery or funding checks.
5. Use concise busy labels and one delayed explanation. Do not simulate
   encryption stages with timers or call a network lookup encryption.
6. Measure before and after with production-equivalent encryption settings.
   Consider additional caching or a different encrypted metadata format only if
   the first implementation still misses the agreed latency target.

These are proposed implementation decisions, not claims about completed work.

## Scope and constraints

### Included in the later implementation

- Shared encrypted metadata storage behavior for Address Book and Watchlist.
- Existing Address Book list/save/delete/last-used operations and callers.
- Watchlist list/add/remove/refresh and the legacy watched-address interface.
- Safe, resumable migration of both mainnet and testnet Watchlists.
- Discovery, coin-scope and generic-request consumers of watched addresses.
- Accurate English/Dutch busy and error states, verified in both themes.
- Privacy-safe timing measurements and focused correctness/performance tests.
- Storage policy and wallet-loading documentation updates when behavior changes.

### Constraints

- Keep existing Argon2id parameters, Stronghold format and encryption work
  factors. Test-only reduced work factors are not performance evidence.
- Do not combine metadata with seed/spending-key snapshots or introduce another
  password prompt for normal saves.
- Do not introduce cloud sync, new remote storage, signing changes or a general
  wallet-storage rewrite.
- Preserve public command names and response shapes where practical. Make any
  necessary interface change explicit and update all callers together.
- Keep contacts, labels, notes and watched identities out of logs, telemetry,
  plaintext backups and new browser persistence.
- Preserve unrelated dirty work. At planning time the Settings plan and its
  index entry were already present and must remain intact.
- Scope any shared-helper change carefully: other Stronghold record families
  must not silently inherit untested persistence semantics.

## 1. Current behavior and evidence

The findings below come from the current source and installed versions of the
dependencies, not a timed native-wallet reproduction. No wallet contents were
read or modified to prepare this plan.

### Storage map

Paths below are relative to Tauri's app-data directory. The configured app
identifier is `com.maxtheyse.verus-express`; use Tauri's path resolver instead
of hardcoding platform paths.

| Data                          | Current location                                                              | Current behavior                                                                          |
| ----------------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Address Book                  | `stronghold/accounts/<account-id>/address_book.snapshot.stronghold`           | Encrypted record `address_book_v1`; account-level contacts snapshot                       |
| Watchlist entries             | `wallet_data/accounts/<account-id>/account_state.json`                        | Plaintext `mainnet.watchlistEntries` and `testnet.watchlistEntries`                       |
| Duplicate watched addresses   | Same `account_state.json`                                                     | Plaintext `watchedVrpcAddresses` in each network; updated with Watchlist entries          |
| Older watched-address storage | `stronghold/accounts/<account-id>/watched_vrpc_addresses.snapshot.stronghold` | Legacy migration input, not the current Watchlist's canonical storage                     |
| Linked identities             | `stronghold/accounts/<account-id>/linked_identities.snapshot.stronghold`      | Existing encrypted metadata implementation with a session cache and blocking-worker write |

Watchlist entries contain identifiers, target kind, display name, address,
optional system ID and creation/update timestamps. Balance snapshots returned to
the Watchlist UI are fetched data; they are not fields of the saved
`WatchlistEntry`.

The current policy deliberately puts watched addresses in account state and
Address Book/linked identities in Stronghold. This plan changes that policy for
Watchlist membership. It does not imply all public price data must be encrypted.

### Address Book operation path

1. The UI infers an address kind and asks the backend to validate candidates
   sequentially, potentially trying `zs`, `vrpc`, `btc` and `eth` per endpoint.
2. The save command captures account/network and a copy of the unlocked storage
   key, loads the encrypted snapshot, and validates/updates the contact.
3. `store_address_book` calls `commit_record_to_path`, which creates a new
   Stronghold instance and loads the existing file again before writing.
4. Delete follows the same load/change/store pattern. Last-used updates do too.
5. Reads use `spawn_blocking`; this Address Book write helper currently does
   not.

Inspection of pinned `iota_stronghold 2.1.0` confirms that
`load_client_from_snapshot` reads/decrypts the snapshot. Thus an existing-file
save/delete has two explicit snapshot load/decrypt paths and one encrypted
commit in the ordinary, non-migration case. This is a source-level operation
count, not a measured attribution of the reported 5–10 second delay. A first
save to a missing file and a legacy contact migration have different paths.

The Argon2id password derivation happens at unlock; normal contact operations
copy the already-derived session key. Stronghold snapshot protection has its own
cryptographic cost. Do not describe every save as re-running the wallet's
password derivation.

### Existing feedback and lifecycle

- Address Book Save already uses `Saving`, a spinner, reserved label width,
  `aria-busy` and a polite announcement. Delete still uses `common.loading` and
  lacks the equivalent visible progress treatment.
- Watchlist Add uses `Adding…`, lookup uses `Looking up…`, and removal uses
  `Removing…`. Adding currently re-resolves the target and fetches balances
  before persisting the entry, even after the preview has already done a lookup.
- Watchlist hydration/refresh check component lifetime; the mutation handlers
  need equivalent stale-result protection when the storage path changes.
- Address Book is a shared frontend store. The wallet route clears it on
  teardown and guards its background hydration; preserve and extend those
  protections to mutations and any new cache.
- Address Book captures a storage context without a session ID. Watchlist
  commands perform current-session checks, but loading entries before network
  work and storing them later still permits an outdated full-list write unless
  the entire mutation is serialized against the latest data.
- Sending can issue last-used updates without awaiting them. They must share the
  Address Book mutation queue with edits and deletion.

### Relevant implementation references

| Source                                                                                    | Symbols or responsibility                                                               |
| ----------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| [Address Book commands](../../../src-tauri/src/commands/address_book.rs)                  | `address_book_context`, `load_snapshot`, save/delete/mark-used/validate commands        |
| [Address Book rules](../../../src-tauri/src/core/address_book/manager.rs)                 | `upsert_contact`, normalization, duplicate checks and legacy shielded-address migration |
| [Stronghold storage](../../../src-tauri/src/core/auth/stronghold_store.rs)                | load/commit helpers, linked-identity optimization and legacy KDF migration              |
| [Password derivation](../../../src-tauri/src/core/auth/kdf.rs)                            | Current Argon2id configuration and salt handling                                        |
| [Session context](../../../src-tauri/src/core/auth/session.rs)                            | Captured context, storage key, cache and session invalidation                           |
| [Session cleanup](../../../src-tauri/src/core/auth/lifecycle.rs)                          | Lock, expiry and replacement-session cleanup                                            |
| [Account state](../../../src-tauri/src/core/wallet/account_state_store.rs)                | Both plaintext Watchlist fields and full-file read/modify/write                         |
| [Watchlist commands](../../../src-tauri/src/commands/watchlist.rs)                        | Normalization, preview, add/remove and bounded refresh                                  |
| [Wallet commands](../../../src-tauri/src/commands/wallet.rs)                              | Watched-address compatibility commands, discovery, coin scopes and unlock               |
| [Generic requests](../../../src-tauri/src/commands/generic_request.rs)                    | Watched-address funding-source restrictions                                             |
| [Private-cache persistence](../../../src-tauri/src/core/channels/dlight_private/cache.rs) | Existing exclusive-writer, encrypted migration and atomic-publication examples          |
| [Address Book UI](../../../src/lib/components/wallet/sections/AddressBook.svelte)         | Form validation, saving feedback, deletion and shared-store updates                     |
| [Watchlist UI](../../../src/lib/components/wallet/sections/Watchlist.svelte)              | Lookup/add/remove, hydration and balance refresh                                        |
| [Command invocation](../../../src/lib/services/invokeWalletCommand.ts)                    | Forced-lock behavior versus session-bound invocation                                    |
| [Wallet route](../../../src/routes/wallet/+page.svelte)                                   | Background contact loading and store cleanup                                            |
| [Transfer wizard](../../../src/lib/components/wallet/sections/TransferWizard.svelte)      | Recipient persistence and last-used mutations                                           |

## 2. Shared encrypted storage design

### Keep separate record families

Keep the current Address Book path and payload compatible. Introduce
`watchlist.snapshot.stronghold` with a versioned `watchlist_v1` record under the
same per-account Stronghold directory. These names are proposed; verify naming
against the implementation at kickoff.

The encrypted Watchlist payload should contain a schema version, both network
lists, a revision and migration state. Preserve entry IDs, names, target kind,
system IDs, order and timestamps. Derive watched addresses from the encrypted
entries in memory rather than maintaining another durable address list.

Do not restructure the Address Book into artificial mainnet/testnet lists as
part of this work. Preserve its existing account-level format and endpoint
network validation/migration semantics.

A common storage module should own the repeated behavior behind a small
interface. Feature modules keep their validation rules; callers should not
coordinate encryption, locks, cache publication and file replacement themselves.

Conceptual operations, not a required public API:

```text
read_address_book(session_context)
mutate_address_book(session_context, contact_operation)
read_watchlist(session_context, network)
mutate_watchlist(session_context, network, watch_operation)
ensure_watchlist_migrated(account_unlock_context)
```

The implementation can use a private typed snapshot mutation helper. Do not
expose arbitrary filesystem paths, record keys, storage keys or unchecked
serialized payloads to the frontend. Do not create an abstraction framework or
pluggable encryption adapters for hypothetical future backends.

### Mutation contract

For each operation:

1. Capture the account, network where relevant, unlock-session identity and
   storage access in the backend. Reject a locked or expired session.
2. Acquire the shared mutation gate for the canonical account/file. Cloned
   storage handles must share this gate; mainnet and testnet share a gate when
   they share a file. Include migration and last-used writes.
3. Read the latest complete snapshot once inside the gate. Missing data is
   different from unreadable, corrupt or unsupported-version data. Fail closed
   on the latter; never substitute an empty list and overwrite it.
4. Validate and mutate the latest snapshot. Preserve unrelated fields/records
   and the other network. Return a no-change result without an encrypted commit
   where that is semantically correct.
5. Commit through the same loaded Stronghold instance. Put the bounded
   load/change/encrypt/file work in `spawn_blocking`, with bounded admission to
   CPU-heavy work. Acquire async queue capacity before occupying a blocking
   worker unnecessarily.
6. Publish cache changes and return success only after durable publication.
   Failures before publication leave the previous committed cache/UI view
   authoritative. Reconcile uncertain outcomes as described below.
7. Discard delivery to a replaced session. Do not allow old errors to lock a new
   wallet through the generic command wrapper.

Serializing only the final write is insufficient: two commands could have read
the same old list. The read and domain mutation must occur under the same gate.
Network RPCs must happen outside this gate; after they finish, revalidate the
session and duplicate/limit checks against current encrypted data.

### File publication and durability

Pinned `stronghold_engine 2.0.1`, `snapshot/logic.rs::encrypt_file`, already
writes an encrypted sibling temporary file, calls `sync_all`, then renames it.
Do not replace that with plaintext serialization or assume it currently writes
directly over the canonical snapshot. Its inspected implementation does not
explicitly synchronize the containing directory after rename.

Use the existing mechanism and add only the publication guarantees needed by the
new lifecycle/migration contract: private file/directory permissions,
same-directory staging, supported-platform replacement, directory sync where
appropriate, and cleanup of task-owned encrypted staging files. The existing
private-cache module is a useful local example, not a reason to share its key or
file format with these features.

An error after replacement but before durability confirmation is different from
an error before replacement. Re-read/reconcile the canonical state and its
stored revision where available, invalidate uncertain caches and avoid blind
retry of a create operation. Prefer a stable operation/entry ID or equivalent
deduplication where a lost reply can otherwise produce duplicates. Do not report
“nothing was saved” when the commit outcome is uncertain.

### Lock, expiry and account switching

Define and test one publication point shared with session invalidation:

- If invalidation wins before publication is admitted, do not replace the
  canonical file; discard staged work and ignore its frontend result.
- If publication wins first, allow that already-admitted commit to finish for
  its captured account. Do not publish its result into a new session.
- Do not hold the global async session mutex during the expensive encryption
  step or any network request. A small publication gate or equivalent guarded
  transaction should coordinate the final admission with invalidation.
- If staging is needed to enforce this, encrypt to a private sibling staging
  path before admission and publish under the gate. Document the lock order and
  distinguish staged ciphertext from committed state.
- Clear session caches and frontend metadata on lock/replacement. Outstanding
  workers may retain captured buffers until they exit; a started blocking task
  cannot simply be aborted. Use scoped ownership and zeroizing transient
  serialized/key buffers, and do not promise instantaneous physical erasure of
  every in-flight copy.

The app already uses a single-instance plugin. Verify whether every supported
write path is covered before relying on it for disk exclusion. If a second
process can write the same account, use the existing `fs2` dependency for
exclusive ownership; never silently continue after failing to acquire it.

### Session cache: optional, measured follow-up

First ship the single-load mutation path. A cache of contact/Watchlist data can
coalesce list reads, but by itself it does not remove the encrypted load inside
each writer or the cost of committing a snapshot.

If measurements justify caching, key it by account, unlock generation and record
family, publish only committed revisions, and invalidate it on lock, expiry,
account replacement, migration and storage errors. A long-lived loaded
Stronghold instance could remove further decryptions, but extends memory
lifetime and needs explicit commit-failure rollback and disposal behavior.
Evaluate that separately; do not mutate a cached live snapshot and expose it
after a failed commit.

## 3. Watchlist migration and compatibility

### Sources and affected readers

Migration must account for all of these, including mixed installations:

- Structured `watchlistEntries` for both networks in `account_state.json`.
- `watchedVrpcAddresses` for both networks in the same file.
- The legacy encrypted watched-address snapshot and any interrupted migration
  artifacts associated with it.
- An already-created encrypted Watchlist, including a deliberately empty one.

Route these consumers to the new encrypted source:

| Consumer                           | Required behavior                                                                                                                                  |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Watchlist list/add/remove/refresh  | Read or mutate encrypted canonical entries                                                                                                         |
| `get_watched_vrpc_addresses`       | Return addresses derived from encrypted entries                                                                                                    |
| `set_watched_vrpc_addresses`       | Preserve the command contract but reconcile through encrypted entries; preserve metadata for retained addresses and document replacement semantics |
| `discover_vrpc_assets`             | Use the unlocked encrypted selection without writing it to a plaintext discovery index                                                             |
| `get_coin_scopes`                  | Preserve watched/read-only classification and account/network isolation                                                                            |
| Generic-request funding validation | Continue excluding watched-only sources; storage failure must not silently erase this restriction                                                  |
| Legacy KDF migration               | Stop exporting watched addresses into plaintext account state                                                                                      |

Search again at implementation time for every watched-address getter/setter and
serialized field. Include any derived cache that persists Watchlist membership
or account-to-address associations. Public network values do not automatically
make the wallet-specific association public.

### Migration protocol

Perform migration after correct key access is established and before any
consumer can read or mutate a legacy Watchlist. Use a per-account migration gate
shared by both networks and by all affected callers. Do not migrate again on
every balance refresh.

1. Read and validate all relevant source versions. Do not use
   `AccountStateStore::load_snapshot`'s current unknown-version-to-default
   behavior for destructive migration. Unknown/corrupt sources are errors.
2. Merge structured entries with legacy-only addresses deterministically. Prefer
   existing structured metadata for exact canonical duplicates. Preserve stable
   IDs/order/timestamps; assign missing IDs once in the committed result. Use
   chain-appropriate normalization, not newly introduced lowercasing of
   case-sensitive Base58 addresses.
3. Do not blindly reuse the current normalizer as a lossless migration: it can
   skip invalid rows and cap the result at 100 entries. If source data cannot be
   represented without loss, stop with a recoverable error and retain the
   source. Do not truncate or silently drop conflicting metadata. A separately
   specified encrypted recovery archive is an option if real fixtures require
   one, not permission to discard data.
4. Write the full encrypted target, including both networks, a schema/revision
   and durable migration state. Preserve sufficient migration provenance to
   distinguish an interrupted import from an already-authoritative list.
5. Reopen with the expected key and verify the complete logical payload before
   retiring any source. Verification is required for migration; an extra
   decryption after every normal save would reintroduce avoidable latency.
6. Under a shared account-state mutation gate, freshly read the JSON and remove
   both legacy fields for both networks. Preserve assets, hidden assets,
   provisioning jobs, pending profiles and unrelated/unknown supported fields.
   Write the remainder atomically. Do not remove the entire account-state file.
7. Remove the obsolete encrypted watched-address source only after its contents
   are verified in the new target and its KDF migration is safely finalized.
   Record a non-sensitive storage-version marker so a missing target later is
   not mistaken for a fresh empty Watchlist.
8. Finish the migration state and enable consumers. On restart, encrypted state
   is authoritative once verified; retry unfinished cleanup without merging
   deleted entries back from old plaintext.

All account-state writers that can overlap cleanup must participate in the same
read/modify/write gate or an equivalent exclusive initialization phase.
Otherwise an unrelated asset preference write can restore the old plaintext
fields from a stale copy. Narrowly improve the existing account-state mutation
primitive as needed; do not refactor its unrelated domain rules.

### Restart and failure rules

| Interruption or condition                            | Required outcome                                                                             |
| ---------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Before encrypted target is committed                 | Legacy source remains intact; retry migration                                                |
| Target written but verification fails                | Keep sources; return recoverable error; no empty fallback                                    |
| Verified target exists; cleanup incomplete           | Target remains authoritative; complete cleanup before allowing dependent Watchlist mutations |
| Crash during JSON cleanup                            | Either old or cleaned JSON is readable; encrypted target remains complete                    |
| Cleanup finished but final marker update interrupted | Recognize target migration state and finalize idempotently                                   |
| Final marker exists but target is missing/corrupt    | Fail closed and preserve artifacts; never reinitialize silently                              |
| Encrypted Watchlist is intentionally empty           | Keep it authoritative; never resurrect old entries                                           |
| Other network has entries                            | Preserve and migrate them even if that network is not currently active                       |
| Legacy source contains unsupported/conflicting data  | Keep source, report failure, and avoid claims that migration/privacy cleanup completed       |

While cleanup is incomplete, do not describe the Watchlist as fully migrated or
encrypted-only. Allow unrelated wallet capabilities to remain usable where their
invariants permit it; block readers whose read-only/security decisions depend on
successful watched-address loading.

### Legacy password migration integration

Today `ensure_account_password_hash` reads old snapshots, writes current-key
secret snapshots, exports watched addresses through
`migrate_legacy_account_state`, promotes files and removes old snapshots.

Change the watched-address part of that sequence so it feeds the encrypted
Watchlist directly, under the correct current key, without an intermediate new
plaintext copy. Preserve the other KDF migration behavior. Test an old-key
wallet, an already-upgraded wallet with JSON entries, and an interrupted mixed
state. Do not infer a working general password-change/rotation system from older
plans; verify the current checkout. Any existing backup, export, removal or
key-change path must include the new file; inventing a new password-change
feature is outside this plan.

### Rollback and privacy limits

The migration is one-way for normal use. Rolling back the executable is not a
data rollback: an older build may ignore the new file or recreate plaintext
entries. Document downgrade incompatibility and prefer a forward repair. Do not
implement rollback by exporting the Watchlist to plaintext.

Retiring the live plaintext fields does not erase historical OS backups, APFS
snapshots, copied files or SSD history. Do not promise secure erasure. At-rest
encryption also does not conceal balance requests from the configured RPC
provider. These limits belong in developer documentation or relevant help, not a
repeated warning in every save flow.

## 4. Address Book integration

- Route list/save/delete/mark-used through the shared storage module.
- Keep existing contact IDs, endpoint IDs, notes, ordering, timestamps and
  normalization behavior. Preserve legacy shielded-address migration, but
  perform its read/repair/write under the same serialized operation and avoid
  rewriting an unchanged snapshot on every read.
- Validate user input authoritatively in Rust. Replace sequential candidate IPCs
  with one backend classification/validation request or a compatible backend
  classification path in Save. Keep existing validation commands for other
  callers until their usage is accounted for; do not require the UI to assert an
  unvalidated address kind.
- Make errors identify the relevant form field without leaking storage details.
- Include Save recipient and last-used updates from Send. Failure to store a
  last-used timestamp must not change the status of an already-submitted send;
  handle rejected background promises rather than leaving them unobserved.
- Use immutable captured contact IDs for pending mutations, as deletion already
  does. Apply results only to their originating account/unlock instance.
- Skip truly unchanged updates where possible. Compare domain values before
  changing timestamps; do not make every no-op appear changed by updating
  `updated_at` first.

## 5. Watchlist lookup and save separation

Encryption must not copy the current network delay into a generic saving state.
The first storage integration can preserve existing return types, but the first
usable release should separate persistence from balance enrichment.

Recommended flow:

1. Resolve and validate the target in the backend. Direct valid R-addresses do
   not need a successful balance lookup to be eligible for saving. VerusID names
   still require authoritative identity resolution.
2. Show the resolved address/identity. Balance count is optional enrichment and
   may be unavailable; it must not mean the target cannot be saved.
3. On Add, save the validated canonical entry through the encrypted mutation
   operation, rechecking duplicates and limits under the gate.
4. Return the persisted entry promptly, using the existing snapshot shape with
   an explicit not-yet-loaded state if compatible. Never represent an unfetched
   balance as zero or mark it available without data.
5. Refresh balances independently and preserve the saved entry if refresh fails.
   Removing an entry is entirely local and must work offline.

For the first pass, keeping authoritative resolution on Add is acceptable; show
lookup feedback honestly. Removing the repeated preview/Add resolution requires
a backend-owned, short-lived resolved-target handle tied to the exact query,
account, network and session, or equivalent revalidation. Do not accept the
frontend's display name/system/address as trusted resolved metadata merely to
save an RPC call. Treat the handle optimization as a separate measured step.

The existing shared lookup semaphore limits entry/system work to eight. Keep
that bound and the partial/unavailable behavior unless measurements justify a
separate change. A late refresh result must not resurrect a removed entry; merge
against current membership/revision and reject superseded responses.

## 6. UI feedback and copy

Keep the established desktop layout and button treatment. This is a focused
state/copy change, not an Address Book or Watchlist redesign.

| Action                                  | Proposed English                      | Proposed Dutch                              |
| --------------------------------------- | ------------------------------------- | ------------------------------------------- |
| Save contact, including recipient saves | Saving securely…                      | Veilig opslaan…                             |
| Delete contact                          | Deleting…                             | Verwijderen…                                |
| Resolve Watchlist target                | Looking up…                           | Opzoeken…                                   |
| Persist Watchlist entry                 | Saving securely…                      | Veilig opslaan…                             |
| Remove Watchlist entry                  | Removing…                             | Verwijderen…                                |
| Refresh public balances                 | Refreshing balances…                  | Saldi vernieuwen…                           |
| Slow contact operation explanation      | Updating your encrypted address book. | Je versleutelde adresboek wordt bijgewerkt. |
| Slow Watchlist persistence explanation  | Updating your encrypted watchlist.    | Je versleutelde volglijst wordt bijgewerkt. |

Proposed strings must become translation keys through `i18n.t(...)`. Confirm
terminology against the existing locales during implementation. Centralize the
shared saving label and busy treatment without creating a large form framework.

Interaction requirements:

- Show the busy label and a small Lucide spinner immediately after an accepted
  action. Retain the normal arrow cursor and reduced-motion behavior.
- Reserve width for every translated state and balanced spinner space; do not
  let the button or footer jump when its label changes.
- Keep one explanation near the action, only if it remains pending for about one
  second. This is a visibility delay, not a fake backend phase. Cancel its timer
  on completion, failure, unmount or session replacement.
- Use `aria-busy` and a single polite status announcement. Do not announce the
  same message from both a button and duplicate live regions.
- Disable duplicate submissions and conflicting controls. Keep a draft or
  existing row visible until persistence succeeds. Preserve recoverable input on
  error and give a clear retry action.
- Do not offer a cancellation control that only hides the UI while a write
  continues. Navigation/lock behavior follows the publication contract above.
- Do not show “Decrypting…” as the whole delete operation. It reads/decrypts,
  removes a record, and encrypts/writes the remainder.
- If exact phases are later exposed, use operation/session-scoped backend
  events, not elapsed-time guesses. No percentage bar without measurable work.

Verify at 920 × 620 and a normal larger desktop size, in English/Dutch and
light/dark mode. Check focus, error placement, long names, long translated
labels and the delayed explanation. Browser fixtures prove presentation; native
tests must prove storage and actual waiting behavior separately.

## 7. Performance research and later opportunities

### What is known and what remains unmeasured

The reported 5–10 seconds is user-observed. Source inspection identifies
redundant decryptions, sequential validation IPC, synchronous crypto on an async
worker and Watchlist network work before persistence. It does not establish
which dominates on the user's running build or how fast the result will be.

Do not reuse old debug timings as a current baseline. Record build profile,
dependency versions, machine, crypto settings, contact count, endpoint count,
file size and whether the run is cold or warm. Debug and release settings differ
in this repository. Do not lower crypto settings to meet a speed target.

### Measurement design

Use disposable synthetic accounts in task-owned directories. For native tests
with the designated disposable wallet, follow the existing Keychain runbook;
never expose its password in arguments, logs or screenshots.

Measure separate intervals for:

```text
UI action -> first rendered busy feedback
input classification/validation
mutation queue wait
blocking-worker queue wait
snapshot load/decrypt
domain mutation/serialization
encrypted commit and durability
IPC completion -> rendered success
Watchlist resolution and balance refresh (separate from persistence)
```

If the dependency combines encryption and file writing, report that interval as
combined; do not invent a breakdown. Log operation category, anonymous per-run
operation token, phase, duration and safe error category only. Never log
addresses, names, notes, stable account identifiers, raw payloads or keys. Keep
diagnostics opt-in/development-only unless an existing approved telemetry policy
explicitly permits more.

Use enough repeated trials to report a median and range; collect enough samples
before claiming p95. Compare the same datasets/build/settings. Include empty,
typical and bounded larger fixtures; single/multiple endpoints;
save/edit/delete; last-used racing an edit; cold/warm reads; offline and slow
RPC conditions.

### Ranked opportunities

| Priority                       | Opportunity                                                                     | Expected effect and proof needed                                                                                             |
| ------------------------------ | ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| First implementation           | One loaded Stronghold instance per mutation                                     | Removes the second existing-file load/decrypt; prove operation count and measure latency                                     |
| First implementation           | Bounded blocking-worker execution                                               | Improves scheduler responsiveness; may not reduce crypto wall time                                                           |
| First implementation           | Serialize complete mutations                                                    | Prevents lost updates and duplicate/conflicting writes; measure queue time separately                                        |
| First implementation           | Single backend endpoint classification/validation                               | Removes repeated IPC; do not claim it explains several seconds without timings                                               |
| First implementation           | Skip unchanged mutations                                                        | Avoids unnecessary encrypted commits; preserve timestamp and duplicate semantics                                             |
| First implementation           | Persist Watchlist before balance enrichment                                     | Keeps network slowness out of durable-save latency; verify saved entries survive RPC failure                                 |
| After baseline                 | Coalesce/cache reads per unlock                                                 | Avoids repeated decryptions for lists/discovery; adds invalidation and memory-lifetime obligations                           |
| After baseline                 | Reuse a backend-resolved Watchlist target                                       | Avoids duplicate lookup; requires a session/query-bound trusted handle and expiry                                            |
| After baseline                 | Retain a loaded Stronghold instance per unlock                                  | Could remove further reads; requires failure rollback, revision discipline and clear-on-lock ownership                       |
| Later, only if needed          | Coalesce last-used updates                                                      | Could reduce metadata writes; explicitly decide acceptable timestamp loss on crash; never defer explicit Save/Delete success |
| Separate architecture decision | Authenticated encrypted metadata file with a random key protected by Stronghold | Could avoid per-commit snapshot KDF costs; larger format/key/migration/recovery scope, not part of this first integration    |

The last option is technically plausible because the repository already uses
authenticated encryption for a private cache. It is not a drop-in reuse: do not
derive this metadata key from a Sapling spending key, since these features must
work for accounts without private-wallet setup. A future design would need an
independent random metadata key, authenticated account/record/version binding,
safe nonce generation, durability, backup and key-rotation/recovery analysis.

### Proposed performance acceptance

- Busy feedback should render on the next practical frame; use 100 ms as an
  initial measurement budget on the reference machine, not a claimed result.
- No duplicate snapshot decryption in one ordinary mutation.
- No crypto/disk operation running directly on an async worker in these paths.
- No balance RPC in the interval between a validated Add and persistence
  success.
- No regression in cold unlock or unrelated wallet responsiveness.
- Record before/after median and tail observations with unchanged production
  crypto settings. Establish the durable-save time budget after the baseline; a
  sub-second target is an aspiration, not a condition to weaken protection.
- If crypto still dominates, document the measured floor and select a later
  optimization explicitly rather than disguising the wait with longer copy.

## 8. Implementation sequence and checkpoints

### Phase 0 — confirm baseline and record measurements

- [ ] Recheck Git state, current sources, storage policy and pinned
      dependencies.
- [ ] Re-run the consumer inventory and inspect any new backup/key-change paths.
- [ ] Add scoped timing instrumentation and build synthetic migration fixtures.
- [ ] Measure current save/delete and split Watchlist network/storage timing.
- [ ] Record agreed performance budgets and unresolved platform constraints
      here.

Exit: reproducible baseline with no real contact data in artifacts.

### Phase 1 — shared storage contract and Address Book

- [ ] Implement shared per-file mutation ownership and session publication
      rules.
- [ ] Implement single-load blocking-worker mutations using current encryption.
- [ ] Preserve Stronghold's existing atomic staging and define durability
      errors.
- [ ] Route Address Book commands, legacy contact repair and last-used writes.
- [ ] Consolidate endpoint classification without weakening backend validation.
- [ ] Verify no-op behavior, rollback/cache behavior and concurrent mutations.

Exit: Address Book format preserved; focused storage/lifecycle tests pass and
the redundant load is removed. Do not migrate Watchlists through an unverified
new helper.

### Phase 2 — encrypted Watchlist and migration

- [ ] Add the versioned encrypted Watchlist record, including both networks.
- [ ] Implement lossless import, readback verification and restart state
      machine.
- [ ] Add safe account-state cleanup with shared mutation ownership.
- [ ] Integrate legacy KDF migration without creating fresh plaintext copies.
- [ ] Route Watchlist, legacy commands, discovery/scopes and funding checks.
- [ ] Verify empty-list authority, downgrade documentation and failure handling.

Exit: canonical data is encrypted, legacy live fields are removed after verified
import, and watched-only classification remains correct for every consumer.

### Phase 3 — feedback and lookup separation

- [ ] Separate durable Watchlist saving from public balance refresh.
- [ ] Add concise busy labels, delayed explanations and shared translations.
- [ ] Guard all pending UI results by operation and account/unlock lifetime.
- [ ] Prevent late hydration/refresh from undoing a successful mutation.
- [ ] Verify keyboard, accessible announcements, width stability and both
      themes.

Exit: users can distinguish lookup, persistence and refresh; success always
corresponds to a durable change.

### Phase 4 — acceptance, documentation and optional next decision

- [ ] Run the focused tests and native disposable-account scenarios below.
- [ ] Record before/after performance with evidence locations and limits.
- [ ] Update storage policy and wallet-loading documentation to match code.
- [ ] Review the final diff for migration/lifecycle/read-only regressions.
- [ ] Decide whether any measured follow-up from section 7 is warranted.
- [ ] Move this plan to `done` only after acceptance, recording any deferrals.

Storage changes can be developed in small reviewable commits, but migration,
consumer routing and plaintext-writer retirement must ship coherently. Do not
release a partial cutover that reads encrypted data while another path still
writes the old JSON fields.

## 9. Verification matrix

### Rust storage and migration behavior

| Scenario                                                                 | Required evidence                                                                                                          |
| ------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| Address Book create/edit/delete/reopen                                   | Full round trip including notes, IDs, endpoints and unchanged format                                                       |
| Mainnet/testnet Watchlist writes                                         | Updating one network preserves the other                                                                                   |
| Edit vs edit; edit vs last-used; add vs remove                           | No lost updates or stale full-list replacement                                                                             |
| Lock/expiry/switch during read, queued write, encryption and publication | Publication rules hold; no old-session result/key/cache reuse                                                              |
| Invalid key, corrupt bytes, unsupported schema                           | Error with original file retained; no empty fallback                                                                       |
| Write failure before publication                                         | Previous file/cache remains authoritative                                                                                  |
| Lost reply or failure after replacement                                  | Reconciliation avoids duplicate creation and false rollback claims                                                         |
| Plaintext-only, legacy-encrypted-only and mixed Watchlists               | Verified lossless migration of both networks                                                                               |
| Failure/crash at every migration transition                              | Restart resumes without loss or resurrection                                                                               |
| Legacy data over limits, invalid rows or metadata conflicts              | No silent truncation or normalization loss                                                                                 |
| Account-state cleanup racing asset/preferences writes                    | Unrelated fields survive; plaintext Watchlist fields do not reappear                                                       |
| Missing encrypted target with migration marker                           | Recoverable error instead of empty reinitialization                                                                        |
| Duplicate instances or cloned stores                                     | Ownership enforced; no simultaneous conflicting writer                                                                     |
| Synthetic marker scan                                                    | Contact/Watchlist marker strings absent from active plaintext files and task-created staging/log artifacts after migration |
| Wrong account/network and watched-only funding source                    | Isolation/read-only restrictions maintained, including storage failure                                                     |

Use behavior tests across the real module interface. Fault-injection seams can
be private to the module; do not test only that a mocked save function was
called. Reduced work factors are acceptable for correctness fixtures only;
serialize tests that mutate the process-global Stronghold work factor and keep
them separate from benchmark processes.

### Frontend mounted and rendered behavior

- Pending actions show the correct spinner/label immediately; no double submit.
- Save/Delete/Remove preserve visible data until successful persistence.
- Errors preserve drafts/entries and clear busy state without hiding the error.
- Old session/unmounted component results cannot mutate shared state or force a
  replacement wallet to lock. Test hydration, mutation and refresh
  independently.
- A refresh started before deletion cannot re-add the deleted watch.
- Delayed explanations appear only while relevant and do not repeat live text.
- English/Dutch geometry stays stable at 920 × 620 in light/dark mode.
- Reduced motion and keyboard focus work throughout pending/error states.
- Saved Watchlist entries remain present when balances are unavailable offline.

Extend the existing
[Address Book mounted tests](../../../src/lib/components/wallet/sections/AddressBook.mounted.ts),
[Watchlist mounted tests](../../../src/lib/components/wallet/sections/Watchlist.mounted.ts)
and [Watchlist merge tests](../../../src/lib/utils/watchlist.test.ts).

### Native and performance acceptance

Use the repository's Tauri test harness and
[disposable-wallet Keychain runbook](../../references/test-wallet-keychain.md).
Capture busy/success/error states with synthetic entries; verify persistence
after lock/unlock and process restart, plus offline removal and unavailable
balance enrichment. Do not modify a real wallet to benchmark migration.

For synthetic accounts, inspect only task-owned files to verify removal of
plaintext copies, wrong-key failure and ciphertext readback. Screenshots and
mounted mocks do not prove encryption, crash recovery or native latency.

### Relevant commands for the later implementation

Use Node `22.23.2` and repository-pinned pnpm `11.24.0`; recheck pins at
kickoff. These commands are a starting set, not tests already run for this plan:

```sh
pnpm exec vitest run --config vitest.mounted.config.ts src/lib/components/wallet/sections/AddressBook.mounted.ts src/lib/components/wallet/sections/Watchlist.mounted.ts --maxWorkers=1 --no-file-parallelism
pnpm exec vitest run src/lib/utils/watchlist.test.ts
pnpm check
pnpm lint:ui
pnpm docs:check
cargo test --manifest-path src-tauri/Cargo.toml --lib address_book
cargo test --manifest-path src-tauri/Cargo.toml --lib watchlist
cargo test --manifest-path src-tauri/Cargo.toml --lib account_state_store
git diff --check
```

Add filters for new storage/migration/lifecycle tests and affected
generic-request funding tests. Format/check only scoped files. Run meaningful
tests for changed behavior rather than the entire suite repeatedly without a new
reason.

## 10. Documentation changes at implementation time

- [Storage policy](../../references/storage-policy.md): classify Watchlist
  membership as private-sensitive; document encrypted authority and legacy
  cleanup. Remove the instruction to keep current watched addresses in JSON.
- [Wallet loading](../../architecture/wallet-loading.md): explain migration
  admission, optional caching, invalidation and independent balance refresh.
- [Core wallet hardening plan](./security-hardening-core-wallet.md): add a
  concise follow-up reference so its historical migration description is not
  mistaken for the new Watchlist policy.
- Update developer recovery/backup instructions if the current implementation
  enumerates snapshot files rather than copying the account directory.
- Record the resulting schema, file names and downgrade limitation here.

Do not rewrite the current policy as though this proposal were already live.

## 11. Decisions to resolve from evidence during implementation

These are bounded engineering questions, not prerequisites for creating this
plan or reasons to request repeated user permission for routine implementation.

| Question                                           | Default recommendation                                                                         | Trigger for reconsideration                                                    |
| -------------------------------------------------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| Separate or combined encrypted files?              | Separate files, common module                                                                  | Measured cross-file transaction requirement, not aesthetic symmetry            |
| Cache in the first pass?                           | Single-load mutations first                                                                    | Repeated read/decrypt timings remain material                                  |
| General new encryption format?                     | Keep Stronghold                                                                                | Measured production latency remains unacceptable after simpler fixes           |
| Migration during unlock or first access?           | Account-gated initialization before any affected reader; old-key import participates in unlock | Cold-unlock measurements justify lazy initialization with identical guarantees |
| Cross-process lock?                                | Reuse existing dependency if ownership can overlap                                             | Proved single-instance coverage may permit a simpler process-local gate        |
| Exact backend progress events?                     | Busy action plus delayed explanation                                                           | Persistently long operations make actual phases useful                         |
| Preserve preview through a resolved-target handle? | Optional later optimization                                                                    | Repeated identity lookup contributes material delay                            |
| Oversized or malformed legacy source?              | Stop and retain originals                                                                      | Real fixtures justify a specified encrypted recovery archive                   |

## 12. Completion checklist

- [ ] Address Book and Watchlist use the same encryption standard and shared
      persistence/lifecycle rules, with no plaintext canonical Watchlist copies.
- [ ] Current and legacy Watchlist sources migrate without loss across both
      networks, including interrupted migration recovery.
- [ ] No normal writer recreates plaintext watched-address fields.
- [ ] Contacts and watches remain local/read-only metadata; signing authority
      and watched-source restrictions are unchanged.
- [ ] Complete mutations are serialized; no stale-session result reaches a new
      wallet; uncertain commit outcomes are handled explicitly.
- [ ] Save/delete feedback is immediate, localized, accessible and geometrically
      stable; public balance refresh does not block a validated durable
      Watchlist save.
- [ ] Benchmarks use unchanged production-equivalent crypto settings and report
      evidence, not an assumed speedup.
- [ ] Native persistence/migration tests and light/dark UI verification are
      recorded, with any unverified platforms or scenarios identified.
- [ ] Documentation matches the final behavior and the remaining optimization
      backlog contains only evidence-supported follow-ups.

## Research references and proof limits

- Current repository references in section 1 are the primary authority for
  application behavior. Recheck them if the baseline changes.
- Pinned dependencies were inspected locally from Cargo's registry source:
  `iota_stronghold-2.1.0/src/types/stronghold.rs`,
  `iota_stronghold-2.1.0/src/types/snapshot.rs`, and
  `stronghold_engine-2.0.1/src/snapshot/logic.rs`. These establish snapshot
  loading/decryption and encrypted temporary-file publication behavior.
- [Tokio blocking tasks](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn_blocking.html)
  support bounded blocking work outside async workers, require care with CPU
  concurrency, and cannot be aborted once started. This informs the worker and
  session-invalidation design; it does not prove a latency improvement.
- [Rust file synchronization](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all)
  and [file replacement](https://doc.rust-lang.org/std/fs/fn.rename.html) inform
  durability and same-filesystem staging. Verify platform-specific guarantees
  through the implementation and tests rather than assuming a rename alone
  proves recovery from every power failure.

Research completed on 2026-09-19. This planning task performs no runtime storage
migration, source implementation, dependency change, native benchmark or live
wallet validation. The reported delay remains a user observation until phase 0.
