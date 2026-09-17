---
owner: lite-wallet-team
last_reviewed: 2026-09-17
---

# Plan: Verus dependency upgrade

- Status: dependency maintenance approved for local main integration; Noble
  verification passed; generic-request product acceptance deferred
- Owner: lite-wallet-team
- Last updated: 2026-09-17

## Scope decision on 2026-09-17

The user prioritizes updating the libraries and explicitly authorized the Noble
migration, commits, and merge into local `main`. Full generic-request feature
validation and repairs are deferred to later work; they no longer block this
maintenance integration. No push or transaction/callback authorization is
implied. Keep the demonstrated compatibility corrections and automated coverage.
Require clean installation, existing contract/hash vectors, application checks,
production browser hashing/parsing, and native build verification before
integration.

Generic requests already exist in `main`: commits `611368e` and `eb2ae93`
entered through wallet-hardening PR #2 (`b485253`). The local generic-request
branch has no unique commits relative to `main`; it is 46 commits behind the
pre-upgrade baseline. Live remote main was verified at `aa74048`. This upgrade
adds no new request UI. Shared Rust signature verification also serves VerusID
profiles.

## Goal

Upgrade the Verus JavaScript dependency family with reproducible installation,
documented behavior changes, JavaScript/Rust protocol agreement, and verified
browser/native builds. Track live generic-request acceptance separately. Treat
Noble hashes as an independently verified part of this maintenance change.

## Scope and fixed candidates

Use the commits inspected on 2026-09-16, not moving branch heads. If a candidate
changes, inspect its additional diff before adopting it.

| Package                        | Current commit                             | Candidate commit                           |
| ------------------------------ | ------------------------------------------ | ------------------------------------------ |
| `verus-typescript-primitives`  | `30c951804228a285546a45457be57ead39d90888` | `7a7b01db697222cd68507a9dbf15f289615ea890` |
| `verusd-rpc-ts-client`         | `fd2c6b0d35d468ee6174bcdd569f5aa990b05282` | `58689ea52500a6a6e4aa741e5d2ed41d7bc6ddd7` |
| `verusid-ts-client`            | `48a1d48a5d076f313022a5891986339851ad7c0c` | `0c39c0a4ba81378ac898cbe75ae1da7511617ccd` |
| `@bitgo/utxo-lib` (Verus fork) | `f39bb2206bac9c3fbf199f7496e495e3a427c771` | `9582a20f7211a7a6aed7bfae3c651e6b76c1f9bb` |

- Upgrade these four as one compatible set. Retain the Verus BitGo fork and its
  `utxo-lib-verus` lineage; the registry BitGo package is not a substitute.
- Keep the existing `bitcoin-ops` and `blake2b` fork pins. They already matched
  their default-branch heads at inspection.
- Upgrade the direct `@noble/hashes` dependency to exact `2.4.0` in the final
  phase. Preserve transitive v1 requirements instead of overriding their major
  version.
- No unrelated frontend, Tauri, Rust dependency, or package-manager upgrades.
- Do not introduce a wallet-storage migration unless evidence requires one; a
  discovered persistence incompatibility is a separate design decision.

## Evidence and limits

The initial review compared upstream source and distributed JavaScript, mapped
production imports, and tested candidates in an isolated Node harness using the
wallet's existing transitive dependencies. Both dependency sets passed the four
existing dependency-vector cases plus the Rust generic-request byte/hash
fixture.

The review also reproduced rejection of the new RPC and ID manifests by
`.pnpmfile.cjs`, single-use parser enforcement, rejection of extra ordinal
bytes, disabled legacy signed-session construction, and corrected
uncompressed-key recovery. This was not a clean candidate installation,
production browser build, native test, or live signing/broadcast certification.

Production frontend imports primarily use primitives for generic-request
parsing, provisioning, and identity-update review. No production direct imports
of the JS RPC or ID interfaces were found. Rust implements the wallet's
RPC/signing paths, so updating JavaScript alone does not update backend protocol
behavior.

## Steps and acceptance gates

### 1. Establish a reproducible baseline

- Recheck Git state and active work. At planning time, the checkout was clean on
  `main` at `aa740488a25c17ccd3a6a6baa6be7227b9b88156`. Use an isolated `codex/`
  branch/worktree for implementation and preserve unrelated work.
- Use the repository-pinned Node `22.23.2` and pnpm `11.24.0`. The investigation
  shell used Node `22.18.0`; do not mistake that probe environment for the
  baseline.
- Record the baseline revision, toolchain, lockfile, and results of the project
  checks listed below. Classify existing failures before changing dependencies.
- Confirm the candidate manifests and distributed entry points are available;
  inspect required transitive changes and scripts before installation.

Gate: reproducible baseline and a known rollback revision. A failing baseline
must be explained before candidate failures can be attributed to this upgrade.

### 2. Establish compatibility fixtures before the version change

Extend the existing tests only where they do not cover changed contracts. Use
synthetic fixtures, daemon-derived vectors, or existing public vectors, never
real wallet secrets. Keep expected bytes independently justified rather than
regenerating snapshots from the candidate and accepting the result.

| Area               | Required coverage                                                                                                                                                               |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Existing contracts | Provisioning bytes/hash, Sapling transaction bytes/ID, WIF validation, Axios adapter, generic-request bytes/hash                                                                |
| Generic requests   | Hex and deep-link import; authentication, identity update, provisioning, and supported combinations; one fresh parser per attempt; repeat imports and failed-then-valid imports |
| Parsing changes    | Truncated, noncanonical, oversized, and extra payload bytes fail safely without signing or sending a callback; inspect actual application error handling                        |
| Identity content   | Omitted versus empty fields, append/remove operations, unknown content preservation, nested VDXF/DataDescriptor values, and user-visible review JSON                            |
| Signature parity   | Mainnet/testnet context, compact names versus addresses, optional system IDs, Unicode metadata ordering, and compressed/uncompressed public keys                                |
| Provisioning       | Request/response JSON, decision hash, signer/system validation, pending/complete/failure states, restart and resume                                                             |
| Session boundaries | Cancel, lock, and wallet-switch invalidate pending actions and prevent stale signing or callbacks                                                                               |

Record an explicit classification for each changed fixture: preserved contract,
intended protocol correction, deliberately rejected malformed input, or
unresolved regression. Existing valid serialized requests and persisted
provisioning records need an explicit compatibility decision; do not silently
discard or rewrite them.

Gate: relevant baseline fixtures pass; new expectations that represent fixes are
documented as candidate requirements. No unexplained byte/hash differences.

### 3. Update the four packages and installation policy together

Expected edit surface:

- `package.json`: replace only the four approved commit pins.
- `.pnpmfile.cjs`: recognize the exact reviewed dependency URLs in the candidate
  manifests; retain rejection of unexpected versions/URLs and root-supplied
  peers.
- `scripts/pnpmfile.node-test.cjs`: verify accepted candidate manifests and
  rejection of changed origins, URLs, or unexpected versions.
- `pnpm-workspace.yaml`: update commit-specific build-policy keys, retaining
  denied build scripts where distributed files suffice, strict release-age
  policy, transitive Git restrictions, Axios overrides, and the `bitcoin-ops`
  patch.
- `pnpm-lock.yaml`: regenerate using the pinned pnpm version and review the
  diff.
- Application code/tests: only compatibility corrections demonstrated by
  evidence.

Verify the resolved graph uses the intended primitives copy across peers and
clients. The frontend uses `instanceof` checks, so duplicate incompatible class
copies are a runtime concern even when compilation succeeds. Verify patched
Axios and base-x resolutions and the presence of `bitcoin-ops/evals.json`.

Perform a clean, frozen-lockfile install in a disposable checkout without
borrowing the original `node_modules`. Diagnose missing dependencies or
packaging failures; do not hide them with unrelated root dependencies, broad
hoisting, disabled policies, or indiscriminate lifecycle-script permission.

Gate: clean installation works, lockfile is stable, dependency-policy tests
pass, and all transitive/package-script changes are accounted for.

### 4. Verify application and backend behavior

Run focused compatibility tests first. Correct demonstrated failures and then
run the full project checks once the candidate is stable:

```sh
pnpm lint
pnpm format:check
pnpm check
pnpm test
pnpm docs:check
pnpm build
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

Use focused Rust test filters for the generic-request/signature work while
iterating. Run the full Rust suite for final acceptance; classify environment or
baseline failures rather than hiding them. Broaden or repeat checks after
relevant changes, not merely because time remains.

Load the production frontend bundle and exercise real request parsing in a
browser. Check for unresolved Node builtins, Buffer/CommonJS errors, broken hash
aliases, duplicate primitives, and import-time failures. Unit tests alone do not
prove Vite or the Tauri webview works.

Build and launch the native candidate. Verify request review and rejection
states in light and dark mode. Preserve backend-owned preflight/signing
boundaries.

Gate: no unexplained new failures, unchanged independent vectors agree, intended
corrections have supporting evidence, and the production bundle/native app runs.

### 5. Validate end-to-end testnet flows

Use only the designated disposable testnet wallet and the
[Keychain helper runbook](../../references/test-wallet-keychain.md). A worktree
changes executable paths; the existing helper binding must not be bypassed.
Prepare fixtures and exact intended actions before any external-action approval.
Implementation approval does not itself authorize broadcast or external
callbacks.

- Authentication: import, inspect signer/network, approve, verify the response
  with the counterpart, and test cancellation/lock/wallet-switch behavior.
- Provisioning: verify submitted JSON/signature, pending state, app restart,
  completion detection, and identity linking with a controlled test counterpart.
- Identity updates: inspect authority/content changes and fees; exercise append,
  remove, and unrelated-field preservation. For separately authorized
  broadcasts, verify both submitted transaction details and subsequent on-chain
  state.
- Smoke-test ordinary testnet send preparation and identity display. Mainnet
  signature context is covered offline without mainnet transactions.

Gate (deferred by the user on 2026-09-17): record exact scenarios and gaps for
future generic-request acceptance. These live feature checks do not block the
authorized dependency maintenance merge. Do not claim full end-to-end
validation.

### 6. Separate Noble hashes migration

As explicitly authorized on 2026-09-17, migrate `@noble/hashes` without waiting
for generic-request product acceptance. The upstream latest release and security
support policy were checked: `2.4.0` is the target.

- Migrate `src/lib/shims/create-hash.cjs` and any necessary bundler integration
  for the v2 exports: `sha2.js` and `legacy.js`, with ESM-compatible loading.
- Preserve the shim's hash results, Buffer return values, supported encodings,
  and chunked-update behavior. Do not broaden its supported algorithms casually.
- Validate known SHA-256/RIPEMD-160 vectors, browser production bundling, the
  existing dependency vectors, and affected native request flows.
- Review the lockfile for accidentally overridden transitive Noble versions; do
  not force a major version into dependencies that require v1.

Gate: accept or revert this migration independently of the Verus upgrade.

## Rollback and delivery

- Keep baseline and candidate manifests, hook, workspace policy, patches, and
  lockfile as complete coherent sets. If reverting, restore the upgrade's owned
  changes together and reinstall from that set's frozen lockfile.
- Never reset, stash, or overwrite unrelated work to roll back. Commits are made
  only when requested; otherwise retain a scoped patch and baseline revision.
- Rolling back application code cannot undo broadcast transactions or callback
  effects. Check persisted request/job compatibility before reverting after live
  testing; do not delete wallet state or imply that chain history can be rolled
  back.
- Report installed commits, intended behavior changes, test evidence, unresolved
  limits, and rollback instructions. Push, PR, merge, publication, and release
  remain separately authorized actions.

## Exit criteria

- [x] Exact reviewed Verus candidates and necessary transitive changes are
      pinned.
- [x] Verus clean frozen installation and dependency-policy tests pass.
- [x] Existing protocol vectors, application tests and Rust tests pass.
- [x] Previous production browser parser and native dark-mode smoke checks pass.
- [x] Existing serialized/persisted data compatibility and rollback are
      documented.
- [x] Final Noble hash vectors, complete JS checks, clean install, production
      browser bundle and native build pass.
- [x] User authorized commits and local main integration; required maintenance
      checks are complete.

Full native request review, light-mode request review, live authentication,
provisioning/resume, callbacks, broadcasts and transaction fee preflight remain
explicitly deferred. The historical fixture's live signature rejection is
unresolved; it is not evidence of an upgrade-specific regression.

## Sources

- [Primitives comparison](https://github.com/VerusCoin/verus-typescript-primitives/compare/30c951804228a285546a45457be57ead39d90888...7a7b01db697222cd68507a9dbf15f289615ea890)
- [RPC client comparison](https://github.com/VerusCoin/verusd-rpc-ts-client/compare/fd2c6b0d35d468ee6174bcdd569f5aa990b05282...58689ea52500a6a6e4aa741e5d2ed41d7bc6ddd7)
- [ID client comparison](https://github.com/VerusCoin/verusid-ts-client/compare/48a1d48a5d076f313022a5891986339851ad7c0c...0c39c0a4ba81378ac898cbe75ae1da7511617ccd)
- [Verus BitGo comparison](https://github.com/VerusCoin/BitGoJS/compare/f39bb2206bac9c3fbf199f7496e495e3a427c771...9582a20f7211a7a6aed7bfae3c651e6b76c1f9bb)
- [Noble v2 breaking changes](https://github.com/paulmillr/noble-hashes/releases/tag/2.0.0)

## Implementation results — 2026-09-16

### Scope and installation

- Worktree: `/Users/maxtheyse/.codex/worktrees/b486/lite-wallet`, branch
  `codex/verus-dependency-upgrade`, baseline
  `aa740488a25c17ccd3a6a6baa6be7227b9b88156`. The original checkout and its
  uncommitted plan/index were left untouched.
- All four fixed candidates above are installed. Hook expectations match the
  exact candidate manifests and still reject changed versions, origins, missing
  edges, and changed URLs. Root peers retain one primitives class identity.
- Node `22.23.2`, pnpm `11.24.0`. Candidate distributed entry points were
  present. No new runtime registry dependencies or registry package versions
  were introduced. pnpm reselected existing compatible transitive versions:
  base-x uses safe-buffer 5.2.1; bs58check 1.3.4/2.0.0 and create-hmac 1.1.3 use
  create-hash 1.2.0; create-hmac 1.1.7 uses safe-buffer 5.2.1. Both versions
  already existed in the graph.
- Upstream script changes were inspected: BitGo's expected test count changed;
  primitives added an MMR fixture-generation command; ID's maintenance command
  became a Node script. None is an install lifecycle requirement. All four
  commit-specific build permissions remain denied; esbuild remains allowed.
- Axios 1.20.0, base-x 3.0.11, bitcoin-ops and blake2b pins, the evals.json
  patch, strict release-age/transitive-Git policy, and Noble 1.8.0 are retained.
- Clean source copy `/private/tmp/verus-upgrade-clean-b486` installed with
  `pnpm install --frozen-lockfile --store-dir /private/tmp/verus-upgrade-clean-store-b486`.
  Both node_modules and store were initially absent. It borrowed neither the
  original checkout's dependencies nor the research harness. Installation
  passed; provisioning/Sapling/WIF/Axios vectors and session fixtures passed
  there (13 tests). Frozen install did not change the lockfile. SHA-256:
  `7e618496f3a9d1f6cd78d1c4ed5ea122f6e60b364ee79af36e314b0467fb2338`.

### Application correction and compatibility decisions

Rust protocol fixes accompany the dependency set: respect the optional system-ID
flag and infer the root system from the envelope network; sort hash metadata by
bytes (including UTF-8 names), hash bound hashes as fixed-width values, exclude
statements from extra signature metadata, and honor compressed/uncompressed
recovery flags. The response builder now sets the system-ID flag when emitting
an explicit system. Old internal test requests with that flag missing were
malformed and were corrected, not grandfathered into the parser.

The new session fixture first exposed the existing hex-import Buffer cast: a
Uint8Array was only asserted to be a Buffer. The importer now constructs an
actual Buffer. This was verified with the old libraries before installing the
candidate. The importer also rejects incomplete consumption and noncanonical
round trips; upstream deep-link parsing otherwise silently drops trailing bytes
before backend verification. Parsing errors are caught by WalletLayout before
signature verification, opening review, signing, or callback delivery.

| Evidence                                                                                 | Classification and result                                                                                                                                       |
| ---------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Existing provisioning challenge, Sapling transaction, WIF, Axios vectors                 | Preserved; unchanged expectations pass                                                                                                                          |
| Rust generic-request bytes, raw SHA-256 and identity signature hash                      | Preserved; independently fixed Rust vector agrees in Node and production browser                                                                                |
| Five supported authentication/update/provisioning groupings                              | Preserved; captured old serialized bytes import repeatedly through hex and deep links, with a fresh parser per attempt                                          |
| Truncated, extra bytes, noncanonical CompactSize and oversized declared value            | Deliberately rejected before review; failed-then-valid import succeeds                                                                                          |
| Single-use parser, failed parser reuse, extra ordinal bytes, SignedSessionObject         | Intended upstream rejection; explicit candidate tests pass; no production legacy SignedSessionObject imports                                                    |
| Empty versus omitted primary-address/content fields                                      | Intended correction preserves explicit empty updates; omitted fields remain omitted                                                                             |
| Omitted/empty content, opaque unknown values, remove operation and nested DataDescriptor | Preserved; old bytes and expected review JSON round-trip unchanged                                                                                              |
| Append/remove review                                                                     | Preserved; focused review-model tests check change kind, inspection payload and unrelated unknown content                                                       |
| Compact names/addresses, implicit systems and mainnet/testnet context                    | Shared JS/Rust vectors cover explicit/implicit systems; compact signer FQNs remain explicitly unsupported in Rust, preserving the existing fail-closed behavior |
| UTF-8 metadata ordering                                                                  | Intended protocol correction; independent SHA-256 preimages and shared JS/Rust vectors agree, including keys, bound hashes and excluded statements              |
| Compressed/uncompressed recovery                                                         | Intended correction; synthetic scalar-1 public-key recovery agrees in both JS and Rust for both forms                                                           |
| Pending/complete/failed provisioning response JSON and decision hashes                   | Preserved; baseline-captured request/response JSON and hashes pass                                                                                              |
| Cancel/disposal during signing or callback                                               | Existing response-submission lifetime tests pass; native lock/wallet-switch and provisioning lifecycle remain unverified                                        |

The `legacy-*` fixtures under `src/lib/genericRequest/fixtures/` were captured
using baseline primitives `30c9518`; candidate output was not substituted as the
expectation. `signature-parity.json` uses independently encoded SHA-256
preimages and public scalar-1 recovery vectors; both Rust and JavaScript consume
it. `unsigned-response.json` was serialized by the candidate and compared
byte-for-byte against the independent Rust response builder, catching its
missing system flag. The group fixtures reuse a public signature marker and are
**not validly signed counterpart requests**. They prove parser compatibility
only. The Unicode hash fixture uses Node crypto independently of the Verus hash
implementation.

No wallet-storage migration was added. ProvisioningJobRecord and account-state
persistence are unchanged: jobs store request hex, identifiers, status and
metadata, not live JavaScript class instances. Tested valid old request bytes
and provisioning JSON remain supported. This does not establish compatibility
for every historical record. Malformed/noncanonical encodings and upstream
unsupported nonempty audience, alt-auth-factor, or attestation fields are
deliberately rejected, never silently rewritten or deleted. Existing queued
requests/jobs must remain available for inspection if rejected; real
restart/resume is still an acceptance gap.

### Verification and remaining gates

Baseline: frozen install, lint, typecheck, 130 unit tests, 37 mounted tests,
docs check and production build passed. Formatting failed on 218 pre-existing
files. The initial sandboxed install had DNS failures; the authorized network
install succeeded.

The selected Xcode installation initially failed Rust linking because its
license is unaccepted. The separately installed Command Line Tools work with
`DEVELOPER_DIR=/Library/Developer/CommandLineTools`; no global
developer-directory setting or license was changed. Rust tests need unrestricted
loopback sockets for local mock servers. With those conditions, the untouched
baseline backend in the disposable source copy passes **458 tests, 3 ignored**.
The final candidate passes **462 tests, 3 ignored**, plus binary/doc-test
targets. The ignored tests already require optional Sapling proving resources;
none was newly ignored.

Candidate: lint (one existing WalletHeroBackground unused-disable warning),
**161 unit tests, 37 mounted tests, seven dependency-policy tests**, typecheck,
docs check and production browser build pass. Formatting still reports the same
218 unrelated files; upgrade-owned frontend/policy files are formatted. Rust
signature fixtures, response serialization, mock callback cancellation and
wallet session-change tests pass. Native packaging/build is recorded below;
passing these checks is not live signing or broadcast certification.

A task-owned Playwright browser loaded the real production bundle on loopback
port 4179. Importing its actual session chunk exercised all five groupings four
times each, matching hex and deep-link results; malformed inputs were rejected.
The raw hash was
`eebf8a0602c116bd35d14319883dcb547a493df882b734b509fe5f86b851edd7` and signature
hash was `a07b699e8c927fbc1dc680e9d1ed61ed139c70efa48b2839f0522e7b21fb46ef`.
There were no Buffer/CommonJS/hash-alias import failures. The welcome page's
expected missing-Tauri-invoke deep-link initialization error was observed in the
ordinary browser. This is production browser parser evidence, not Tauri IPC or
native review evidence. The task browser was closed and session cleanup
verified.

Native evidence and deferred generic-request acceptance work:

1. Native build **passed** using
   `DEVELOPER_DIR=/Library/Developer/CommandLineTools pnpm tauri build --debug --no-bundle`.
   After explicit user approval, the original wallet process was closed and the
   exact candidate was launched. Executable SHA-256:
   `90025ad8fb608ec8dd415fe8517daeb88a91e2ea2b8d8971996658b8af60b96e`. The
   candidate runs from the original checkout's `src-tauri` working directory to
   load its existing runtime environment files without copying their contents.
2. Native dark-mode malformed-request rejection **passed**: the public Rust
   fixture plus a trailing byte produces “This request could not be parsed.” The
   error and surrounding identity profile were visually inspected in
   `/private/tmp/verus-native-request-dark.png`. The unmodified public fixture
   parses but produces “The request signature could not be verified.” against
   the live provider. That result does not establish whether the historical
   fixture remains valid against current identity authority; valid native
   request review remains unverified. A current controlled counterpart request
   is required. Light mode also remains unverified: the native app uses
   `ModeWatcher` system appearance and exposes no theme selector in Display and
   language. The system appearance was not changed.

   The computer-use connector could not address this unbundled app. After
   explicit user approval, a temporary native Accessibility helper targeted only
   the exact candidate executable and one window. It inspected labels, opened
   safe navigation controls, and typed public fixtures into the request textarea
   using process-targeted keyboard events. Input-value reads and consequential
   signing/submission controls were excluded. No macOS permission settings were
   changed. Direct Accessibility value setting did not update the Svelte input
   binding; real keyboard input did.

3. The dedicated Keychain helper was removed and set up for this checkout with
   explicit approval; the user entered the existing password in the native
   masked dialog. Attribute-only status confirmed the binding. Initial preflight
   saw no web area, then passed after startup settled. Guarded unlock confirmed
   the testnet wallet's updated success timestamp. No password was exposed or
   read by the agent, and no guard was loosened. After the UI checks the wallet
   was locked through its normal control; helper preflight confirmed the guarded
   unlock screen without Keychain access. The candidate remains running, locked,
   with the dedicated helper bound to this checkout.

4. Use a controlled test counterpart with valid testnet requests to verify
   authentication signatures, signer/system validation, provisioning submission,
   restart/resume/completion/linking, and cancellation/lock/wallet-switch
   behavior. The synthetic fixtures are ready for offline review only, not
   submission.
5. Native identity overview and public profile display **passed**, including the
   existing profile avatar and description. The ordinary Send form opens,
   selects VRSCTEST, shows the public source, testnet destination network and
   available balance, and cancels back to wallet home. The rendered dark-mode
   form was inspected in `/private/tmp/verus-native-send.png`. No recipient or
   amount was entered, so transaction preflight/fee preparation remains
   unverified. A read-only `getinfo` probe using the configured endpoint and
   system TLS trust returned VRSCTEST, chain ID
   `iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq`, testnet=true, height 1,236,378. The
   Python probe lacked a trusted issuer in its local CA store; system curl
   succeeded with certificate verification enabled. Wallet signing, external
   callbacks, broadcasts and on-chain changes remain untested.

6. Identity append/remove broadcasts and callback delivery still need separately
   authorized concrete actions and a counterpart. No funds were spent.

### Noble migration on 2026-09-17

Direct Noble hashes is pinned to **2.4.0**. The adapter keeps its existing
CommonJS interface and uses v2 `sha2.js` / `legacy.js` exports. The pinned Node
22.23.2 supports synchronous require of ESM; Vite handles the browser bundle.
The transitive Noble 1.8.0 required by `@noble/curves@1.9.7` remains untouched.
Seven adapter tests compare SHA-256/RIPEMD-160 against independent Node crypto,
covering Unicode, empty input, chunking, encodings, Buffer results, typed-array
view boundaries, ArrayBuffer/DataView inputs, and unsupported algorithms. Final
verification results will be recorded before commit.

Upstream references:

- [2.4.0 release](https://github.com/paulmillr/noble-hashes/releases/tag/2.4.0)
- [Security support policy](https://github.com/paulmillr/noble-hashes/blob/main/SECURITY.md)

### Local rollback

The user authorized committing and integrating this change into local main. To
roll back, restore the upgrade-owned manifest, hook, workspace policy, lockfile,
Noble adapter, and both Rust protocol files together from
`aa740488a25c17ccd3a6a6baa6be7227b9b88156`, then install that set with
`pnpm install --frozen-lockfile` using the pinned toolchain. Review the importer
fix and tests separately: the real Buffer fix passed against baseline libraries
and may be retained independently. Remove candidate-specific correction tests if
reverting the libraries; preserve the report and useful legacy fixtures. Do not
reset the tree or delete persisted wallet state. No external effects occurred
here; any later callbacks/broadcasts cannot be undone by restoring source files.
