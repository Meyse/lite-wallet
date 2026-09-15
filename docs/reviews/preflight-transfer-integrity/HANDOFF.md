---
owner: lite-wallet-team
last_reviewed: 2026-09-15
---

# Preflight and transfer-integrity handoff

## Task identity

- Task: `01a0a2b2-c389-7430-acc8-5d07a88e2782`
- Parent reviewer task: `01a0a29a-32d0-7e40-8a17-4980bcea5288`
- Worktree: `/Users/maxtheyse/.codex/worktrees/c296/lite-wallet`
- Git state: detached `HEAD` at `aa1e541088575ba826a0b61ea713a53be8cca62e`
- Comparison base: the same investigated `HEAD`; all implementation is an
  uncommitted working-tree diff
- Investigation input: `/private/tmp/lite-preflight-4402/REPORT.md`
- No commit, push, PR, deployment, live transaction, signing, broadcast, or
  recovery-resume operation was performed.

## Outcome

The eight ranked findings and the two associated reliability gaps in the
supplied investigation are addressed in the working tree. Parent correction
rounds 1 and 2 are also closed: canonical VRPC source binding, invalid EVM gas
estimates, partial/initial scope states, canonical fee-asset lookup, the full
ETH Max recovery branch, a successful realistic ERC20 estimate, and production
transfer lifecycle coverage. The implementation keeps backend-owned preflight
records, source ownership checks, session binding, one-time consumption, and
submitted-versus-confirmed wording intact.

## Finding disposition

| Finding                                                      | Implementation                                                                                                                                                                                                                                                                                 | Verification                                                                                                                                                                                                                                                       | Status                                                 |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------ |
| F1 PBaaS token could become native payment                   | Direct non-native VRPC sends use the token-aware `sendcurrency` path. The registry-resolved source currency is passed through construction and intent validation; unresolved or conflicting `getcurrency` metadata fails with `CurrencyMetadataMismatch` before `sendcurrency`.                | Production-router JSON-RPC tests authenticate token input, recipient output, token change, native change, and fee; cover transparent and resolved identity destinations; reject native and other-token substitution.                                               | Implemented; no signing or live execution attempted.   |
| F2 recovery commands missing from ACL                        | Added only the six audited missing command grants, including the three recovery commands. Recovery review and terminal acknowledgement now derive trusted chain ID locally; resume still requires a provider. Native command strings and `EthNotConfigured` map to localized safe states.      | Generated ACL audit reports no registered handler missing a permission definition and all six grants present; historical recovery test uses a disabled provider for review/ack; browser fixture shows the safe error.                                              | Implemented; real Tauri-window IPC remains unverified. |
| F3 failed EVM gas estimation could succeed                   | Removed general ETH/ERC20 gas fallbacks. Reverts, malformed quantities, zero, and below-intrinsic estimates return `GasEstimationFailed`. ETH Max and fee-adjusted near-balance values are re-estimated at the exact final value; overbalance remains rejected.                                | Production-router JSON-RPC tests cover invalid estimates, full-balance failure followed by exact adjusted-value success, adjusted-value failure with no preflight, near-balance re-estimation, realistic ERC20 success, and overbalance-before-estimation.         | Implemented; synthetic provider only.                  |
| F4 VerusID handle failed in direct VRPC path                 | Trailing-`@` recipients take the server-side resolution/sendcurrency path. Both entered recipient and normalized destination are retained in the stored payload; review shows a localized resolution warning.                                                                                  | Production-router test resolves `alice@`, authenticates a type-4 identity token destination, and verifies entered/resolved payload fields and warning.                                                                                                             | Implemented; no live identity preflight or signing.    |
| F5 token name shown as destination network                   | Non-VRPC scopes now expose execution-network metadata. One channel/network-aware UI resolver names Ethereum, Sepolia, Bitcoin, Bitcoin Testnet, Verus, Verus Testnet, and PBaaS systems.                                                                                                       | Rust metadata unit, frontend label matrix, browser Ethereum and Sepolia evidence.                                                                                                                                                                                  | Implemented.                                           |
| F6 direct-send fiat fee and total hidden/wrong-rate fallback | Network fee uses its own fee-asset rate. Currency resolution matches only canonical asset ID/currency ID, never a token's `systemId` or presentation label. Mixed totals fail closed if any rate is absent. Small values do not render as zero.                                                | Token-before-native, native-missing, distinct-rate, and unknown-rate units plus light/dark USDC + ETH browser evidence.                                                                                                                                            | Implemented; synthetic rates only.                     |
| F7 Bitcoin recipient dust accepted                           | Final recipient output is checked against the destination script's dust threshold both before planning and after Max fee adjustment.                                                                                                                                                           | P2PKH, P2SH, P2WPKH, and Max-adjusted dust units.                                                                                                                                                                                                                  | Implemented; no relay or broadcast test.               |
| F8 malformed EVM monetary intent                             | Backend parsing now accepts only exact unsigned positive decimals, rejects excess token precision/overflow, and rejects submitted value above balance. The UI validates selected-asset precision, preserves invalid input for correction, and compares balances with exact decimal arithmetic. | Rust six-decimal/sign/exponent/overflow/over-balance units and frontend precision/large-number units.                                                                                                                                                              | Implemented.                                           |
| Scope-load reliability gap                                   | Loading, all-failed, partial-failure, and authoritative-empty states are distinct. Partial success remains usable and shows compact retry. Retry enters loading immediately, invalidates the scoped display cache, and reloads; stale generations cannot publish.                              | Mounted Svelte light/dark state and retry tests, scope-loader unit, and rendered loading/failed/partial/empty screenshots.                                                                                                                                         | Implemented.                                           |
| Wallet/session stale-preflight gap                           | The backend unlock session ID joins wallet name/network in the transfer lifecycle key. Send/Convert remount on even a same-wallet re-unlock, disposing the old request guard.                                                                                                                  | Mounted tests drive the production `WalletLayout` → `Send` → `TransferWizard` path. Wallet, network, and same-wallet session changes each remount and dispose the old guard; identical request fields plus late old success/failure cannot replace the new review. | Implemented; no native wallet-switch concurrency test. |

The requested Ethereum fee guidance is also applied: Economy is described as
lower priority and potentially slower, Standard as recommended for timely
inclusion, and one shared note says timing varies with network activity. No time
guarantee is invented.

## Changed inventory

### Backend and native command contract

- `src-tauri/build.rs`
- `src-tauri/permissions/default.toml`
- Six generated permission definitions in `src-tauri/permissions/autogenerated/`
  for `touch_session_activity`, the three pending-ETH commands, and the two
  provisioning-signature commands
- `src-tauri/src/commands/wallet.rs`
- `src-tauri/src/commands/{vrpc_transfer,bridge_transfer}.rs`
- `src-tauri/src/core/channels/mod.rs`
- `src-tauri/src/core/channels/preflight_integration_tests.rs`
- `src-tauri/src/core/channels/btc/preflight.rs`
- `src-tauri/src/core/channels/eth/{preflight,provider,send}.rs`
- `src-tauri/src/core/channels/vrpc/{common,preflight,transfer}.rs`
- `src-tauri/src/types/errors.rs`
- `src-tauri/src/types/wallet.rs`

### Frontend, localization, and tests

- `src/lib/components/wallet/WalletLayout.svelte`
- `src/lib/components/wallet/sections/{Send,Conversions,TransferWizard}.svelte`
- `src/lib/components/wallet/sections/transfer-wizard/{directSendFee,preflightRequest,scopeLoader,transferDisplay}.*`
- `src/lib/components/wallet/sections/transfer-wizard/TransferSourceStatus.svelte`
- Mounted source-state tests and production transfer-lifecycle tests. The latter
  keep `WalletLayout`, `Send`, and `TransferWizard` unmocked while using
  test-only upstream navigation/context controls and unrelated-child stubs
- `src/lib/transfer/transferCurrency.ts` and its tests
- `src/lib/i18n/locales/{en,nl}.ts`
- `src/lib/services/walletService.ts` and `src/routes/wallet/+page.svelte`
- `src/lib/utils/walletErrors.ts` and `walletErrors.test.ts`
- `package.json`, `pnpm-lock.yaml`, and `vitest.mounted.config.ts` for the
  pinned jsdom mounted-test environment

### Review evidence

- This handoff and eleven PNG screenshots under
  `docs/reviews/preflight-transfer-integrity/`
- The Playwright/Vite fixture was copied from the supplied immutable fixture
  into ignored local test support. The original
  `/private/tmp/lite-preflight-4402` archive and fixture were not edited or
  deleted.

The original report and parent review evidence remain unchanged at
`/private/tmp/lite-preflight-4402/`,
`/private/tmp/lite-preflight-review-4402-round1/`, and
`/private/tmp/lite-preflight-review-4402-round2/`. Their reviewed Markdown
artifacts currently hash to
`826a6f01e6136b1943b28ea813fa0424954244dc50de96d05186c41f6ac3a7dd`,
`2b2bb64833cd8f485900a331db4f54389180d392c7761053b9e4a4074e7c35b7`, and
`ef0cfcef35e2a9193862abe772b9af7a3583f4e1ec477b310e907abdf2330d9d`,
respectively. Use `git status --short`, `git diff --stat`, and
`git diff --no-index /dev/null <new-file>` when reviewing because no commit
exists.

## Verification commands and results

| Command                                                                   | Result                                                                                                                                                                                                                         |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `cargo fmt --all -- --check`                                              | Passed.                                                                                                                                                                                                                        |
| `cargo test --lib`                                                        | 436 passed, 0 failed, 3 ignored. Includes nine production-router/provider integration tests added after parent review.                                                                                                         |
| `node --test scripts/pnpmfile.node-test.cjs`                              | 3 passed.                                                                                                                                                                                                                      |
| `pnpm test`                                                               | 3 Node tests, 21 standard Vitest files with 109 tests, and 2 mounted Svelte files with 8 tests passed. The mounted suite covers source states plus the production transfer lifecycle's six wallet/network/session race cases.  |
| `pnpm test:mounted`                                                       | The independently runnable mounted gate passed: 2 files, 8 tests.                                                                                                                                                              |
| `./node_modules/.bin/svelte-kit sync && ./node_modules/.bin/svelte-check` | 0 errors, 0 warnings.                                                                                                                                                                                                          |
| ESLint on every changed frontend file                                     | Passed with no output.                                                                                                                                                                                                         |
| Full repository ESLint                                                    | 0 errors; one pre-existing warning in unchanged `WalletHeroBackground.svelte` for an unused disable directive.                                                                                                                 |
| `node ./scripts/lint-ui-style.mjs`                                        | Passed.                                                                                                                                                                                                                        |
| `node ./scripts/docs-check.mjs`                                           | Passed after this handoff (38 Markdown files).                                                                                                                                                                                 |
| `./node_modules/.bin/vite build`                                          | Production build passed; existing chunk-size advisory only.                                                                                                                                                                    |
| Prettier check on all changed frontend files                              | Passed.                                                                                                                                                                                                                        |
| `git diff --check`                                                        | Passed.                                                                                                                                                                                                                        |
| Generated ACL audit                                                       | 77 registered handlers, 78 app commands, no registered handler missing a permission definition; all six audited grants present in generated ACL. The one pre-existing manifest-only command is `sign_identity_signature_hash`. |

An earlier pre-correction sandboxed run reported fixture failures after
localhost bind was denied and poisoned shared test state. The current
authoritative rerun above passed all 436 executable tests.

Environment note: repository-pinned pnpm is `11.24.0`; installed Node is
`22.18.0`, while `package.json` requests `22.23.2`. The lockfile-pinned
`jsdom@27.0.0` test dependency was installed with pnpm's existing v11 store;
Vitest resolved to `4.1.11`. This was not a clean dependency install.

## Investigation regression: before and after

| Boundary                      | Supplied investigation before                                                                       | Working-tree after                                                                                                                                                            |
| ----------------------------- | --------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Same-chain PBaaS token        | Controlled router produced a native VRSC payment; parent review reproduced provider substitution.   | Actual-router tests now authenticate correct token transactions and reject native/other-token metadata before construction.                                                   |
| EVM revert/estimation failure | ETH/ERC20 returned usable preflights from fixed fallbacks; parent review found zero-fee preflights. | Invalid estimates stop preflight; Max recovers only after estimating its exact adjusted value, and adjusted failure yields no preflight. A realistic ERC20 estimate succeeds. |
| EVM amounts                   | Negative/over-balance could become Max; excess token precision truncated.                           | Signed/exponent/excess-precision/overflow/over-balance inputs are rejected; exact F8 units pass.                                                                              |
| Bitcoin dust                  | One-satoshi P2PKH recipient output was accepted.                                                    | Script-aware thresholds reject below-dust P2PKH/P2SH/P2WPKH and Max-adjusted outputs.                                                                                         |
| Send opening recovery         | Missing ACL became `Unknown error`; provider was required before reading local pending state.       | Six explicit grants generated; local review/ack need no provider; fixture shows safe actionable copy.                                                                         |
| VerusID handle                | Frontend accepted `name@`, native direct path returned `InvalidAddress`.                            | Handle takes server resolver/sendcurrency path and entered/resolved recipients are both retained.                                                                             |
| USDC network label            | Details/review showed `USD Coin`.                                                                   | Mainnet shows Ethereum and testnet shows Sepolia.                                                                                                                             |
| Direct fiat                   | Fee and total fiat were hidden; fallback could use the source asset's rate.                         | Review/drawer show ETH fee fiat and mixed USDC + ETH total using separate rates; missing rate suppresses the estimate.                                                        |
| Scope failure                 | Transient failure was marked as a loaded empty wallet; partial failure was hidden.                  | Loading, partial, all-failed, and authoritative-empty states are explicit; partial success remains usable with retry.                                                         |
| Stale preflight               | Direct signature lacked wallet/network context and same-wallet re-unlock identity.                  | The mounted production owner remounts/disposes the real transfer flow for wallet, network, and session changes; all six late-outcome races stay stale.                        |

This after-evidence uses the real current `TransferWizard` in a browser with
synthetic service responses copied from the supplied fixture. It is
component/runtime regression evidence, not native-wallet, provider, signing,
broadcast, settlement, or confirmation evidence.

## Screenshots

- [Light review](./light-review.png): 10 USDC, 0.00042 ETH maximum fee,
  approximately EUR 1.34 fee and EUR 10.34 mixed total.
- [Dark review](./dark-review.png): the same authoritative values in dark mode.
- [Dark fee picker](./dark-fee-picker.png): Economy/Standard inclusion guidance,
  timing caveat, ETH fee, and fee fiat.
- [Sepolia label](./testnet-sepolia.png): ERC20 testnet source displays Sepolia.
- [Scope failure](./scope-load-failure.png) and
  [recovered retry](./scope-retry-recovered.png): failure is not an empty wallet
  and retry restores USDC.
- [Initial loading](./source-loading-light.png),
  [partial failure](./source-partial-light.png),
  [all failed](./source-failed-dark.png), and
  [authoritative empty](./source-empty-dark.png): the four source states remain
  concise and distinguishable across light and dark mode.
- [Native command unavailable](./native-command-unavailable.png):
  ACL/command-contract failure is actionable rather than `Unknown error` or
  clean no-pending state.

Browser console output contained only the fixture's missing `favicon.ico` 404.
Task-owned Playwright sessions `pw-857f0662-preflight` and `pw-01a0a2b2-r1` and
their Vite servers were closed; `playwright-cli list` reported no browsers after
the correction-round capture.

## Proof limits and next review focus

- No native Tauri window was unlocked or invoked, so generated ACL presence is
  not a packaged-app IPC smoke test.
- No user wallet secret was accessed. No transaction was signed, submitted,
  broadcast, resumed, acknowledged against user state, confirmed, or settled.
- No funded Ethereum, ERC20, Bitcoin, Verus, PBaaS, private dLight, conversion,
  or bridge execution was attempted.
- The parent review's copied 44-scenario harness and evidence were preserved
  unchanged. Its relevant adversarial cases were promoted into maintainable
  product integration tests rather than continuing to depend on a disposable
  archive overlay.
- Provider reverts and malformed estimates are covered by local JSON-RPC
  fixtures, but no new live-provider revert was induced. Bitcoin thresholds are
  library-policy units, not relay observation.
- Wallet, network, and same-wallet-session replacement are covered through the
  actual mounted production keyed owner and transfer component, including guard
  disposal, DOM remount, identical request parameters, and late success/failure.
  A native Tauri unlock/switch race was not performed.
- Review the direct VRPC token route, EVM intrinsic-gas validation, and session
  key propagation first because they are the highest-consequence corrections.
