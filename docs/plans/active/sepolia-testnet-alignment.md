---
owner: lite-wallet-team
last_reviewed: 2026-09-17
---

# Plan: Sepolia testnet alignment

- Status: local implementation complete; native render and live settlement
  deferred
- Owner: lite-wallet-team
- Last updated: 2026-09-17

## Goal

Make Sepolia the only supported Ethereum testnet, consistently paired with
VRSCTEST, with correct bridge routing and clear labels for new and existing
wallets. Preserve mainnet behavior and existing wallet data.

## Evidence and remaining uncertainty

The wallet already defaults to Sepolia RPC, signing chain ID `11155111`,
Etherscan history queries, and transaction links. This is an alignment and
bridge-configuration correction, not a migration of funds between chains.

Confirmed in current wallet source:

| Area                                                         | Current behavior                                                                                                                               | Implementation target                                                                 |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `src-tauri/src/core/channels/eth/config.rs`, `provider.rs`   | Sepolia defaults; endpoint override allowed; expected chain ID assigned locally                                                                | Verify the endpoint's actual chain ID before trusting reads or executing transactions |
| `src-tauri/src/core/channels/eth/bridge/delegator.rs`        | Testnet uses `VERUS_BRIDGE_DELEGATOR_GOERLI_CONTRACT`, address `0x85a7de2278e52327471e174aeeb280cdfdc6a68a`; every non-1 chain maps to testnet | Verified Sepolia deployment; exact supported-chain matching                           |
| `src-tauri/src/core/coins/registry.rs`, `commands/wallet.rs` | Testnet ETH uses `GETH` identifiers and display metadata                                                                                       | Preserve identifier compatibility, separate user-facing names                         |
| `src/lib/coins/verusCoinCatalog.generated.json`              | Imported `gETH` ticker                                                                                                                         | Durable Sepolia presentation policy that survives catalog regeneration                |
| `src/lib/i18n/locales/en.ts`, `nl.ts`, `Receive.svelte`      | GETH receive label                                                                                                                             | Localized Sepolia receive label                                                       |
| ETH preflight/send and persisted recovery records            | Fee currency and asset references can use `GETH`; bridge payloads retain transaction details                                                   | Correct presentation without rewriting transaction intent or signed data              |

Upstream `VerusCoin/VerusBridgeWebsite`, inspected at
`2006c3ffe178fc3d41acb980650e3532c6deb848`, provides independent network
evidence:

- [Network connector](https://github.com/VerusCoin/VerusBridgeWebsite/blob/2006c3ffe178fc3d41acb980650e3532c6deb848/src/connectors/networkconnector.js)
  selects Sepolia by default.
- [Bridge configuration](https://github.com/VerusCoin/VerusBridgeWebsite/blob/2006c3ffe178fc3d41acb980650e3532c6deb848/src/constants/contractAddress.js)
  pairs Sepolia with VRSCTEST, but reads its delegator from deployment
  environment.
- [Chain definitions](https://github.com/VerusCoin/VerusBridgeWebsite/blob/2006c3ffe178fc3d41acb980650e3532c6deb848/src/constants/chain.js)
  retain Goerli entries. Do not copy that legacy supported-chain list.

The separate local bridge website checkout uses
`0xCaA98A4eC79dAC8A06Cb3BfDcF5351b6576d939f` for testnet. Treat this as a
candidate, not a verified deployment. Neither source inspection nor a contract
name proves current deployed code, bridge availability, or successful
settlement. Earlier public RPC probes failed at network resolution; these checks
remain open.

### Implementation evidence recorded 2026-09-17

Read-only checks at `2026-09-17T11:21:16Z` resolved the deployment gate:

- `https://ethereum-sepolia-rpc.publicnode.com` returned chain ID `11155111` and
  block `11,723,385`.
- The proposed delegator `0xCaA98A4eC79dAC8A06Cb3BfDcF5351b6576d939f` had
  deployed bytecode; the old wallet address
  `0x85a7de2278e52327471e174aeeb280cdfdc6a68a` had none on Sepolia.
- The expected `bridgeConverterActive()` ABI call returned `true`, and
  `getTokenList(0,0)` returned 13 entries.
- `https://api.verustest.net` reported VRSCTEST block and header height
  `1,237,268`. `getcurrency("vETH")` returned native type `9` mapped to
  `0xcaa98a4ec79dac8a06cb3bfdcf5351b6576d939f`; `Bridge.vETH` resolved to
  `iSojYsotVzXz4wh2eJriASGo6UidJDDhL2` on system
  `iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq`.
- The separate bridge website checkout was clean at
  `04ea8816f62eb01e4cb2789262ea7fb54043a021`; its deployment definition was
  introduced by `9ddaa6c` and retained by `a081d44`.

These checks establish deployment identity and read-only bridge availability.
They do not prove signed submission, notarization, settlement, or the reverse
direction.

## Decisions and scope

- Supported Ethereum chains: mainnet `1` and Sepolia `11155111` only. Reject
  Goerli `5` and all other chain IDs. No network picker or Goerli fallback.
- Keep `GETH` / `eth.GETH` as compatibility identifiers in storage and internal
  contracts for this change. Document their meaning as Sepolia native ETH. This
  does not retain Goerli network support. A storage-key rename adds risk without
  improving user-visible behavior and is outside this plan.
- Asset name: **Sepolia ETH**; network name: **Sepolia**. Use **ETH** as the
  amount/fee ticker when the network is already clear, and **Sepolia ETH** in
  otherwise ambiguous contexts. Never show `GETH`, `gETH`, or Goerli as the
  current supported asset/network label.
- Retain existing addresses, key derivation, active assets, account/network
  isolation, and encrypted storage. Do not import or reinterpret Goerli
  balances, token contracts, transaction history, or pending operations as
  Sepolia data.
- New interface copy uses `i18n.t(...)`, English and Dutch translations, and
  existing compact desktop patterns. No explanatory banner or new onboarding.
- Preserve backend-owned preflight, signing, session invalidation, single-use
  preflight IDs, explicit recovery, and exact-byte uncertain-broadcast handling.
- No dependency upgrade, mainnet deployment change, unrelated refactor, commit,
  push, or transaction broadcast is included in this planning request.

## Implementation sequence

### 1. Establish the Sepolia deployment evidence

1. Recheck Git state and applicable instructions. Preserve unrelated dependency,
   native-test, and other active work. Record a baseline before implementation.
2. Read the current testnet bridge deployment configuration or published bundle
   and cross-check its delegator with an authoritative Verus deployment source
   or the VRSCTEST gateway definition where available. Record source revision,
   chain ID, full address, observation time, and observed block heights.
3. Use read-only RPC to verify `eth_chainId == 11155111`, deployed bytecode at
   the proposed delegator, expected ABI calls, and the native ETH/VRSCTEST token
   mappings. Verify the VRPC endpoint identifies VRSCTEST and resolve `vETH` and
   `Bridge.vETH` from that network. Bytecode presence alone is insufficient.
4. Check bridge availability separately from deployment identity. A paused
   converter must not be confused with a missing or incorrect deployment;
   conversion and plain cross-chain transfer may have different availability.

**Gate:** do not enable a replacement bridge target based only on the local
website constant. If evidence is unavailable or conflicting, report the blocker;
labels and direct Sepolia validation can proceed, but bridge readiness stays
open.

### 2. Make network and bridge routing explicit

1. Centralize backend Ethereum network metadata within the existing ETH module:
   wallet network, expected chain ID, and verified delegator. Keep frontend
   execution-network metadata backend-derived where the current API supports it.
2. Replace the Goerli delegator constant with a Sepolia-named constant holding
   the verified address. Match chain IDs explicitly; unknown IDs return a typed
   error instead of falling through to testnet.
3. Add a shared asynchronous RPC identity check. Cover balance/history loading,
   token mapping, route discovery, and preflight so data from a wrong endpoint
   is not shown as Sepolia. Keep Etherscan requests scoped to the expected ID.
4. Freshly validate RPC identity before signing or broadcasting, including each
   approval, final bridge transfer, and recovery/rebroadcast path. Do not rely
   solely on startup validation. Honor existing session checks and send locks.
5. Before bridge preflight and execution, verify deployed code and the approved
   contract binding. Approval spender and transfer target must come from the
   same verified network configuration. Revalidate after any approval wait.
6. On wrong chain, missing code, or failed validation, prevent execution and
   surface a localized actionable error. Do not silently try another network.

Primary files: ETH `config.rs`, `provider.rs`, `preflight.rs`, `send.rs`, bridge
`delegator.rs`, `preflight.rs`, `token_mapping.rs`, route consumers, and
existing error-to-UI mappings. Keep public command signatures stable where
possible.

### 3. Preserve existing wallets and recovery intent

1. Inventory active-asset storage, dynamic token definitions, cached balances
   and history, preflight records, and durable ETH recovery records. Determine
   their network provenance before deciding whether any targeted cache
   invalidation is necessary. Do not clear wallet data as a migration shortcut.
2. Keep existing `GETH` activations resolving to exactly one Sepolia asset;
   reopening a wallet must not hide it or create a duplicate asset.
3. Scope token definitions and mappings to the verified network. Do not reuse
   Goerli ERC20 addresses or infer chain provenance solely from a legacy ticker.
4. Preserve recorded chain, contract, amounts, fees, transaction hash, and
   signed bytes in pending records. Keep review/reconciliation available for
   older records; do not silently relabel an explicitly recorded Goerli
   transaction.
5. Block any old-contract continuation or rebroadcast under the new deployment
   policy. Reconcile already-submitted transactions without creating a
   replacement transaction. An unsubmitted operation needs a fresh preflight and
   review; never change the target inside an old payload. Preserve valid current
   Sepolia recovery and mainnet behavior.

### 4. Align all visible asset names

1. Introduce one shared frontend presentation rule for the compatibility asset
   in `src/lib/coins/presentation.ts` (or its existing shared metadata layer).
   Apply it to both imported-catalog and backend-fallback paths. Do not rely on
   hand-editing only the generated JSON or modify the mobile repository.
2. Align backend display metadata and wallet scope metadata with that policy.
   Keep fee/asset identifiers distinct from user-facing display strings.
3. Update receive, asset lists/Add Asset, asset details, send/convert selectors,
   amounts and fees, review, recovery review, receipts, and history surfaces
   wherever they expose the old ticker. Reuse translated copy rather than
   scattering substitutions across screens.
4. Preserve Sepolia explorer links and network labels already implemented in
   `transferDisplay.ts` and `transferReceipt.ts`. Keep native ETH distinct from
   its VRSCTEST-side `vETH` representation.
5. Ensure a catalog refresh cannot reintroduce `gETH` in rendered output; retain
   the generated catalog's upstream provenance if using a presentation override.

### 5. Verify and document the final behavior

Add focused tests for the changed behavioral boundaries, then run the repository
checks below. Update the ETH core/bridge plans with a dated pointer to the new
behavior where their descriptions are stale; avoid rewriting historical
evidence.

## Acceptance matrix

| Check                  | Required result                                                                                                                           |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Network mapping        | `1` and `11155111` resolve correctly; `5` and unknown IDs fail                                                                            |
| Endpoint override      | Wrong-chain RPC, RPC failure, or a chain change between preflight and send prevents signing/broadcast; reads do not masquerade as Sepolia |
| Bridge binding         | Missing bytecode or mismatched target/spender blocks execution, including approval continuation and recovery                              |
| Existing wallet        | Saved `GETH` activation remains one Sepolia asset after reopen; address and mainnet data unchanged                                        |
| Catalog regression     | Imported `gETH` metadata and backend fallback both render the approved labels                                                             |
| Recovery regression    | Old contract/chain is never retargeted; known submissions remain reviewable; valid current-network retry retains exact signed bytes       |
| Direct ETH and ERC20   | Sepolia balances, history, fees, preflight, and explorer links agree on the network; mainnet regression checks pass                       |
| Native interface       | Changed surfaces checked in light/dark mode and English/Dutch, including old-wallet and recovery fixtures; no legacy labels leak          |
| Bridge read-only smoke | Verified deployment, ABI reads, token mappings, route discovery, and ETH/ERC20 bridge preflight agree with VRSCTEST                       |
| Live settlement        | Separately authorized small test transfers complete in both directions; ERC20 approval path is exercised if supported and funded          |

Use RPC fixtures to assert **zero signing/broadcast side effects** on mismatches
and to simulate an endpoint changing chains. Keep offline recovery review usable
when providers are unavailable. Do not equate mocked success with live
readiness.

Commands, using the repository-pinned pnpm version at implementation time:

- Focused Rust ETH/provider/bridge/recovery tests, then the existing Rust suite
  (`cargo test --manifest-path src-tauri/Cargo.toml`).
- Focused frontend presentation/receipt/recovery tests, then `pnpm test`.
- `pnpm check`, `pnpm lint`, `pnpm check:verus-coins`, and `pnpm build`.
- `pnpm docs:check`, scoped Prettier checks on changed files, Cargo formatting
  checks, and `git diff --check`. Record unrelated baseline failures separately.

For native testing with the designated disposable `mijn app` wallet, follow
`docs/references/test-wallet-keychain.md`. Do not expose wallet credentials or
secret material. This plan does not authorize test sends; get bounded transfer
authorization before live spending/broadcast and record source and destination
transaction evidence without claiming completion from submission alone.

## Completion and rollback

- Mark local implementation complete only after code, compatibility, regression,
  and rendered checks pass, with remaining live verification listed explicitly.
- Mark bridge integration verified only after deployment evidence, read-only
  checks, and authorized settlement checks pass. Record pauses/unavailability as
  blockers rather than changing expected results.
- No wallet-storage migration is intended. If implementation uncovers a required
  schema change, revise this plan with explicit migration and rollback tests
  first.
- If the new bridge configuration cannot be validated, keep that route
  unavailable while retaining direct Sepolia functionality. Do not roll back to
  the old Goerli address or auto-resume saved operations. Preserve recovery
  records for review.
- Report exact files changed, tests run, rendered evidence, live limits, and
  unrelated work retained. Move this plan to `done` when its acceptance scope is
  met, or explicitly record which live gates remain deferred.

## Local implementation result (2026-09-17)

Implemented on branch `codex/sepolia-testnet-alignment` in the isolated worktree
`/Users/maxtheyse/.codex/worktrees/6808/lite-wallet`:

- Centralized Ethereum mainnet/Sepolia chain metadata and bridge targets.
- Added live chain-identity validation to balances, history, ERC20 discovery,
  token mapping, route discovery, preflight, signing, approval continuation,
  recovery, and broadcast boundaries.
- Verified bridge bytecode and `bridgeConverterActive()` before bridge reads or
  execution; chain IDs other than `1` and `11155111` fail closed.
- Preserved `GETH` as a compatibility ID while presenting `ETH` / `Sepolia ETH`
  throughout imported-catalog and backend-fallback paths.
- Kept existing testnet activations deduplicated, enabled network-scoped Sepolia
  ERC20 discovery, and prevented old Goerli-target recovery records from being
  retargeted or rebroadcast.

Verification completed:

- `cargo test --manifest-path src-tauri/Cargo.toml`: 470 passed, 3 ignored.
- `pnpm test`: 181 unit tests and 42 mounted tests passed, plus 7 package-policy
  tests.
- `pnpm check`, `pnpm build`, `pnpm lint:eslint`, `pnpm lint:ui`,
  `pnpm docs:check`, scoped Prettier, Cargo formatting, and `git diff --check`
  passed. ESLint retained one pre-existing warning in
  `WalletHeroBackground.svelte`.
- `pnpm check:verus-coins` is blocked independently because the existing sync
  script imports undeclared package `acorn`; this change does not modify package
  dependencies or generated catalog provenance.

Deferred evidence:

- Native light/dark and English/Dutch wallet-state screenshots were not
  captured. Port `1420` was already owned by the original checkout, and the
  guarded `mijn app` credential reported an account/checkout binding mismatch
  for this worktree. The existing app/session was not interrupted or rebound.
- No signing, broadcast, approval, notarization, or settlement was attempted.
  Authorized small transfers in both directions remain the final live gate.
