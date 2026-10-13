---
owner: lite-wallet-team
last_reviewed: 2026-10-13
---

# Plan: Private testnet network

- Status: active. Steps 1 to 3, 4.3, 5.2, and 6 are done and merged to `main` on
  2026-10-13. A tester can create or import a private-testnet wallet. No funded
  transaction ran on the private chain.
- Owner: lite-wallet-team
- Last updated: 2026-10-13

## Goal

Add **private testnet** as a third wallet network next to mainnet and testnet. A
tester selects it when they create or import a wallet. The wallet then uses the
private VRSCTEST chain at `https://ptestapi.verustest.net` and its own Sepolia
bridge, so testers can run the DeFi and bridge flows. Mainnet and testnet
behavior and stored data do not change.

## Evidence

Read-only probes on 2026-10-13, between 11:15 and 12:10 UTC. Public testnet
height was about 1,273,560. Private testnet height was about 28,410 to 28,420.
Sepolia block was 11,905,758 (`ethereum-sepolia-rpc.publicnode.com`).

| Property                            | Public testnet                                                             | Private testnet                              |
| ----------------------------------- | -------------------------------------------------------------------------- | -------------------------------------------- |
| VRPC API                            | `https://api.verustest.net`                                                | `https://ptestapi.verustest.net` (no key)    |
| Chain name and ID                   | `VRSCTEST`, `iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq`                           | Same                                         |
| Genesis block (height 0)            | `027e3758…aae71`                                                           | Same; history differs from height 1          |
| Network magic, P2P port             | `-596868954`, `18842`                                                      | `-53398965`, `28331`                         |
| vETH, Bridge.vETH IDs               | `iCtawpxUiCc2sEupt7Z4u8SDAncGZpgSKm`, `iSojYsotVzXz4wh2eJriASGo6UidJDDhL2` | Same                                         |
| DAI.vETH, MKR.vETH IDs              | `iN9vbHXexEh6GTZ45fRoJGKTQThfbgUwMh`, `i3WBJ7xEjTna5345D7gPnK4nKfbEBujZqL` | Same                                         |
| Bridge delegator on Sepolia         | `0xcaa98a4ec79dac8a06cb3bfdcf5351b6576d939f`                               | `0x98f58684b05e7d40719e61955015af91ef7e77d3` |
| DAI.vETH ERC-20 on Sepolia          | `0x6ed1bac2b63bbd66eed738dd21cf970edc909813`                               | `0x1ffd3099e4e9fad339ef03fef4da0e64674f19a7` |
| MKR.vETH ERC-20 on Sepolia          | `0x287e234fb9d080271467fa2bc7374c1d3cd50c46`                               | `0x55fddfe763bcd2ff02b2f3070694117a9511f04d` |
| Delegator `bridgeConverterActive()` | `true`                                                                     | `true`                                       |
| Delegator `getTokenList(0,0)`       | 13 entries                                                                 | 6 entries                                    |

Other findings:

- The delegator is the `nativecurrencyid.address` of vETH on each chain.
- The private token list holds VerusNFT, VRSCTEST, Bridge.vETH, ETH (Testnet),
  DAI (Testnet), and Maker (Testnet).
- `getcurrencyconverters("VRSCTEST")` on the private chain returns 7 baskets:
  VRSC-USD, VRSC-BTC, VRSC-GOLD, VRSC-EUR, VRSC-KMD, VRSC-KMD-GOLD, and
  Bridge.vETH. Bridge.vETH started at height 2150.
- `estimateconversion` from VRSCTEST to vETH through Bridge.vETH returns a quote
  on the private chain.
- Every VRPC method the wallet calls reaches the private daemon when the call
  uses the wallet's real parameter shapes. The private proxy is strict: it
  answers an unexpected parameter shape with `-32601 Method not found`. For
  example, `fundrawtransaction` passes with UTXOs as `{txid, voutnum}` but fails
  with an empty list or with the long `getaddressutxos` objects. The public
  proxy passes all three shapes to the daemon. The wallet already sends
  `{txid, voutnum}` (`core/channels/vrpc/preflight.rs`, `vrpc/transfer.rs`,
  `vrpc/claim.rs`).
- `getcurrencyconversionpaths` is not a daemon method on either API. The wallet
  falls back to `listcurrencies` (`core/channels/eth/bridge/paths.rs`).
- No explorer, faucet, or dlight server was found for the private chain.
- Verus Mobile `dev-asf-parent` (`7d74cd6a`, 2026-10-07) has no private testnet
  constants. A tester reaches the private API through the VRPC overrides setting
  for VRSCTEST. Its Ethereum side still uses Goerli constants. That approach
  shares storage between both testnets, so this plan does not copy it.

## Current code facts

Paths are relative to `src-tauri/src` unless they start with `src/`.

- `WalletNetwork { Mainnet, Testnet }` uses `#[serde(rename_all = "lowercase")]`
  (`types/wallet.rs:9`). The frontend type is `'mainnet' | 'testnet'`
  (`src/lib/types/wallet.ts:37`), with inline copies in about 12 components.
- Each wallet has one fixed network (`AccountRecord.network`). Per-account
  storage holds one section for each network.
- About 43 exhaustive Rust `match` arms will fail to compile with a new variant.
  About 70 other Rust sites (`matches!(…, WalletNetwork::Testnet)`,
  `unwrap_or(WalletNetwork::Mainnet)`) and about 60 frontend
  `network === 'testnet'` checks will silently treat a new value as mainnet.
- `CoinDefinition.is_testnet` is a boolean. The registry writes VRPC and dlight
  endpoints into the shared coin definition from that boolean
  (`core/coins/registry.rs:145`). VRPC calls use the network-aware
  `VrpcProviderPool`. dlight reads the written endpoints
  (`core/channels/mod.rs:382`, `:418`; `commands/wallet.rs:1765`).
- Ethereum code selects network metadata and the delegator from the chain ID
  (`core/channels/eth/config.rs:46`, `eth/bridge/delegator.rs:98`,
  `eth/send.rs:469`). Both testnets use Sepolia `11155111`, so the chain ID
  cannot select the delegator.
- The bridge export fee probe sends the mainnet vETH ID
  `i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X` as `exportto` on every network
  (`commands/bridge_transfer.rs:25`, used in `estimate_bridge_export_fee`). This
  is also a bug on public testnet.
- Endpoints come from environment variables with `.invalid` defaults
  (`core/runtime_config.rs`). Debug builds load `.env`. Release builds read only
  the process environment.
- "Private" already means shielded in the interface: the Private Verus settings
  and `wallet.private.label.testnet` ("Verus PRIVATE Testnet",
  `src/lib/i18n/locales/en.ts:649`).

## Decisions

- **D1. Third network.** Add `WalletNetwork::PrivateTestnet`. The stored value
  is `privatetestnet`, from the existing lowercase rule. The owner chose this
  option on 2026-10-13. The rejected option was one switch that points testnet
  at the public or the private API. Both chains use the same IDs, so shared
  storage would mix their data.
- **D2. Test-chain family.** Private testnet follows VRSCTEST chain rules: the
  same address and key encoding, coin definitions, system and currency IDs, and
  Sepolia chain ID. Add `WalletNetwork::is_test_chain()` in Rust and
  `isTestChain()` in the frontend. Keep `CoinDefinition.is_testnet` with the
  meaning "test-chain coin". No coin catalog migration. One seed gives the same
  R-address on all three networks (P2PKH byte `0x3C`, WIF byte `0xBC`;
  `core/crypto/wif_encoding.rs`). Wallet names must be unique, but seeds need
  not be. So a tester can import a public-testnet seed as a private-testnet
  wallet under a new name and keep the same address.
- **D3. Network profile.** The wallet network selects the VRPC and dlight
  endpoints, the Ethereum delegator, the storage section, the default assets,
  the rates policy, and the labels. The coin, the system ID, and the Ethereum
  chain ID never select these. Keep one backend profile for each network.
- **D4. Endpoints.** Add `VRPC_PRIVATE_TESTNET_URL` with a `.invalid` default,
  like the existing variables. Add `DLIGHT_PRIVATE_TESTNET_ENDPOINTS` with no
  default. Local `.env` sets `https://ptestapi.verustest.net/`. When no dlight
  endpoint is set, shielded features are unavailable on private testnet.
- **D5. Bridge delegator.** Private testnet uses
  `0x98f58684b05e7d40719e61955015af91ef7e77d3`. Before bridge preflight and
  again before execution, compare the configured delegator with the vETH
  `nativecurrencyid.address` from the wallet network's VRPC, ignoring case. On a
  mismatch, block the bridge with a typed, translated error. Apply this check to
  both testnets.
- **D6. Default assets.** Private testnet starts with VRSCTEST and Sepolia ETH
  (the `GETH` ID). BTC testnet is not a default asset. Add asset follows
  `is_test_chain()`.
- **D7. Storage compatibility.** Each new `privatetestnet` section uses
  `#[serde(default)]` and is not written when it is empty. Mainnet and testnet
  wallets then keep a file format that older builds read. Older builds skip a
  private-testnet wallet (`core/wallet/manager.rs:105`); its data stays on disk.
  This is accepted for a test network.
- **D8. Requests and contacts.** A generic request names only the VRSCTEST
  system ID, so it cannot target one testnet. Private-testnet wallets accept
  testnet requests under testnet rules. Contacts keep the network in their
  identity key, so contacts stay separate.
- **D9. Rates and links.** Private testnet has no fiat rates and no Verus
  explorer links, the same as testnet. Ethereum links use Sepolia Etherscan.
- **D10. Parity.** This feature is desktop-led. Protocol behavior stays aligned
  with Verus Mobile: `sendcurrency` and `fundrawtransaction` shapes and the
  CReserveTransfer payload. See
  [ADR 0003](../../adr/0003-mobile-parity-scope.md).

## Open decisions

The defaults apply until the owner decides.

- **O1. Visibility.** Default: the network picker shows all three networks to
  every user. Alternative: show private testnet only when a setting or a build
  flag enables it.
- **O2. Interface name.** Default: "Private testnet". Risk: users can confuse it
  with Private Verus (shielded). Alternative: a name without "private". The
  stored value `privatetestnet` does not change.
- **O3. Funding.** Answered 2026-10-13: the owner bridges Sepolia ETH from
  MetaMask into the private chain and converts it to VRSCTEST. The bridge must
  call the private delegator `0x98f5…77d3`. A bridge interface that targets the
  public delegator `0xcaa9…939f` delivers to public testnet instead. Only
  Sepolia ETH and the private delegator's token list can cross this bridge.
- **O4. Chain services.** Answered 2026-10-13: no explorer and no dlight server
  exist for the private chain. Shielded features stay unavailable (D4). The
  public testnet dlight server cannot stand in: it serves public testnet's chain
  (height 1,273,638, block 1 `000004c5…3b33`), and shielded notes and the note
  commitment tree belong to one chain's history. Whether the chain can reset is
  still open; a reset makes local pending records and identities stale.

## Steps

### 1. Baseline and gates

1. Check the Git state. Preserve unrelated work. Record the test baselines.
2. Run the evidence probes again. Record the time and the heights. Stop if the
   chain ID, the vETH delegator, or `bridgeConverterActive()` differs from the
   evidence table.

Result, 2026-10-13: passed.

- Git: `main` at `0a8a9e3`. Unrelated uncommitted work stays in place:
  `docs/plans/active/tagged-invoices.md` and its line in `docs/plans/index.md`.
- Probes at 11:56 UTC. Private chain `VRSCTEST` / `iJhCez…` at height 28,448,
  tip 225 s old, vETH delegator `0x98f5…77d3`. Public chain at height 1,273,606,
  tip 366 s old, vETH delegator `0xcaa9…939f`. Sepolia chain `11155111` at block
  11,905,913. Both delegators have bytecode, and `bridgeConverterActive()` is
  `true` for both.
- Baselines on the main tree: `cargo test --manifest-path src-tauri/Cargo.toml`
  579 passed and 3 ignored. Unit Vitest 57 files and 352 tests passed. Mounted
  Vitest 28 files and 312 tests passed. Package-policy tests 7 passed.
  `pnpm check` 0 errors and 0 warnings.
- Harness fault, not a product fault: `pnpm test` also collects test files from
  Claude worktrees under `.claude/worktrees/`. It then reports 108 failed files,
  all inside those copies, and skips the mounted suite. Until the unit config
  excludes that folder, run
  `pnpm exec vitest run --exclude '.claude/**' --exclude '**/node_modules/**'`
  and `pnpm test:mounted`.

### 2. Network model

1. Add `PrivateTestnet` to the Rust enum. Add `is_test_chain()` and one mapping
   to the crypto `Network::Testnet`. Replace the three mapping helpers with it:
   `wallet_network_to_crypto_network` (`commands/generic_request.rs:106`),
   `to_derivation_network` (`commands/guard.rs:91`), and
   `crypto_network_for_wallet` (`commands/wallet.rs:903`).
2. Fix the arms that the compiler reports.
3. Audit each `matches!(…, WalletNetwork::Testnet)`, `== Testnet`, `!= Testnet`,
   and `unwrap_or(WalletNetwork::Mainnet)`. Use `is_test_chain()` for chain
   rules. Use the network profile for endpoints, bridge, storage, and labels.
   Start with these sites:
   - `core/channels/mod.rs:246` (`network_root_system_id`)
   - `commands/bridge_transfer.rs` (six network checks)
   - `commands/identity.rs:1100` (hard-coded VRSCTEST system ID)
   - `commands/generic_request.rs:1980` (request network check)
   - `core/updates/engine.rs:237` (no fiat rates on testnet)
   - `core/coins/registry.rs:145` and `:201` (endpoint defaults and asset keys)
4. Frontend: export one `WalletNetwork` type and its helpers from one module.
   Replace the inline unions and the `=== 'testnet'` checks. Prefer
   `Record<WalletNetwork, T>` maps, so the type checker reports a missing
   network.
5. Search again for `'testnet'` and `WalletNetwork::Testnet`. List each
   remaining site and the reason it stays in the change description.

Result, 2026-10-13: done on branch `claude/private-testnet`, not committed.

- The compiler reported 43 exhaustive matches. Most of them select storage,
  endpoints, or Ethereum settings, so this step also did the parts of steps 3,
  4, and 5 that the compiler forced. No arm falls back to testnet data. Those
  steps list what remains.
- `From<WalletNetwork> for Network` replaces the three mapping helpers.
  `WalletNetwork::storage_key()` names per-network files, folders, and cache
  keys.
- 41 chain-rule checks now use `is_test_chain()`. The dlight runtime cache key
  now uses the network itself.
- Remaining sites and why they stay:
  - About 30 `session.active_network().unwrap_or(WalletNetwork::Mainnet)`: they
    apply only when no wallet is unlocked. An unlocked private wallet reports
    its own network.
  - `commands/identity.rs:946` (`matches!(…, Mainnet)`): equal to "not a test
    chain".
  - Legacy migration in `core/auth/stronghold_store.rs` writes old testnet data
    to the testnet section. Old files predate private testnet.
  - `core/rates/coinpaprika.rs` `catalog_lookup_key` and
    `core/coins/registry.rs` `canonical_asset_key`: they name the chain family,
    and each network keeps its own storage section.
  - `core/coins/registry.rs:145` writes testnet endpoints into the shared
    VRSCTEST coin. This is step 4.3.
  - `core/channels/eth/config.rs` `metadata_for_chain_id(11155111)` still
    returns testnet metadata. This is step 5.2.
  - Frontend: `StepperLayout.svelte` (step 6.1) and the request `testnet` flag
    in `GenericRequestFlowHost.svelte` and `genericRequest/session.ts` (D8).
- Frontend: `WalletNetwork` gained `privatetestnet`. `utils/walletNetwork.ts`
  holds `isTestChain()`. `networkLocaleKey()` uses a `Record` map. 11 inline
  unions now use the shared type. 30 chain-rule checks use `isTestChain()`. New
  keys: `common.network.privatetestnet` (en, nl, de, es) and
  `wallet.manageAssets.environment.privatetestnet` (en, nl).
- `src/lib/stores/assetVisibility.ts` is imported nowhere. It got the minimal
  type fix; its removal is a separate task.
- Tests added: the stored value and key encoding (`types/wallet.rs`), the VRPC
  pool isolation (`vrpc/provider.rs`), account-state isolation and file format
  (`account_state_store.rs`), the Watchlist file format for older builds
  (`stronghold_store.rs`), default assets (`commands/wallet.rs`), and test-chain
  assets and contact chain (`coins/supportedAssets.test.ts`). The Watchlist test
  fails when its skip rule is removed.
- Checks: `cargo test` 584 passed and 3 ignored. `cargo fmt --check` clean. Unit
  Vitest 57 files and 353 tests passed. Mounted Vitest 28 files and 312 tests
  passed. `pnpm check` 0 errors and 0 warnings. ESLint and `pnpm lint:ui`
  passed. Prettier is clean on every changed file that was clean at `0a8a9e3`;
  seven changed files were already unformatted there and stay as they were.
- No user can select private testnet yet. The picker is step 6. Do not expose it
  before steps 4.3 and 5.2 are done.

### 3. Storage isolation

Status: done in step 2, including the tests.

1. Add the `privatetestnet` section to:
   - the Stronghold snapshots in `core/auth/stronghold_store.rs`: watched
     addresses, Watchlist, active assets, linked identities, provisioning jobs,
     dlight seed metadata, and the `network_mut` and `network_value` helpers
   - the Ethereum pending-submission file name in the same file
   - the `account_state.json` sections (`core/wallet/account_state_store.rs`)
   - the dlight cache directory (`core/channels/dlight_private/store.rs`)
   - the rate cache key (`core/rates/cache.rs`)
2. `WatchedVrpcAddressesSnapshot` and `WatchlistSnapshot` use
   `deny_unknown_fields`. Add the new field with `#[serde(default)]` and skip it
   when it is empty (D7).
3. Map `contact_chain_id(PrivateTestnet)` to the VRSCTEST ID
   (`commands/address_book.rs:21`).

### 4. Endpoints and network profile

Status: items 1 to 4 are done. Item 5 remains.

Result of item 3, 2026-10-13: `dlight_endpoints_for_network` in
`core/channels/mod.rs` resolves shielded endpoints from the wallet network.
Private testnet reads only `DLIGHT_PRIVATE_TESTNET_ENDPOINTS`, so it has none
today and the backend refuses its shielded channel. A server must also prove its
chain: for both testnets, `refresh_tip_info` compares the server's block 1 hash
with the network's known value (`expected_height_one_hash` in
`dlight_private/runtime.rs`). Chain name, Sapling height, and branch alone
cannot tell the two testnets apart. A test with the real block 1 bytes from
`lightwalletd.verustest.net:8125` checks that private testnet rejects that
server. The ignored live test reaches the public server and passes the new
check, then fails in `creation_birthday` with `NetworkError`. Unchanged `main`
fails at the same point, so a separate task covers it.

1. In `core/runtime_config.rs`, add the two variables (D4) and the private
   testnet arms of the `*_for_network` functions. Update `.env.example`.
2. Add a root provider and configured systems for private testnet to
   `VrpcProviderPool` (VRSCTEST only).
3. Resolve dlight endpoints from the wallet network when they are used, not from
   the shared coin definition. With no endpoint, do not start the dlight
   runtime, and show shielded features as unavailable.
4. Add the default active assets (D6) next to `TESTNET_DEFAULT_ACTIVE_COIN_IDS`
   (`commands/wallet.rs:137`).
5. Never call `fundrawtransaction` with an empty UTXO list. Map `-32601` on a
   method the wallet knows to a clear "endpoint rejected the request" error.

### 5. Ethereum and bridge

Status: items 1 to 3 are done. Items 4, 5, and 6 remain.

Result of item 2, 2026-10-13: each `EthNetworkProvider` records its
`wallet_network`, and the pool keeps a separate Sepolia provider for private
testnet. `delegator_contract_for(network, provider_chain_id)` selects the
delegator from the wallet network and rejects a provider on another chain.
`metadata_for_chain_id` and `delegator_contract_for_chain_id` are removed,
because Sepolia no longer names one network. Their two tests became
`wallet_network_selects_the_delegator_on_its_own_chain` and
`a_provider_on_another_chain_is_rejected`. Pending Ethereum records keep using
their recorded network through the provider of the active wallet.

1. Add private-testnet `EthNetworkMetadata` in `core/channels/eth/config.rs`:
   chain `11155111`, network name Sepolia, and the delegator from D5.
2. Select the delegator from the wallet network. Keep `metadata_for_chain_id`
   only to check the RPC identity. Update `eth/bridge/delegator.rs:98` and
   `eth/send.rs:469`. Recovery uses the `wallet_network` recorded in each
   pending Ethereum record.
3. Private testnet uses the Sepolia RPC and Etherscan settings.
4. Add the delegator check (D5).
5. Read token mappings from the private delegator `getTokenList`. Do not reuse
   public-testnet ERC-20 definitions. The private DAI and MKR contracts differ.
6. Fix the export fee bug. First write a test that fails because testnet sends
   the mainnet vETH ID. Then use the network's vETH ID
   (`iCtawpxUiCc2sEupt7Z4u8SDAncGZpgSKm` on both testnets). Check the other
   mainnet vETH constants for the same fault: `core/coins/registry.rs:17`,
   `core/updates/engine.rs:57`, and
   `src/lib/components/wallet/sections/TransferWizard.svelte:229`.

### 6. Interface and copy

1. Give the network picker in `src/lib/components/shared/StepperLayout.svelte`
   three options. It serves the create, import, and guard flows. Keep the
   compact desktop control and `cursor: default`.
2. Add translation keys for the new network to `en`, `nl`, `de`, and `es`. Cover
   the unlock list, the receive address labels, the Manage assets environment
   label, the identity lookup label, and the profile read-only note. Use
   sentence case.
3. Show an unavailable state for shielded features when no dlight endpoint
   exists.
4. Apply `max-product-design-taste` and `frontend-design`. Check light and dark
   mode.

Result, 2026-10-13: items 1 to 4 are done.

- The picker renders `WALLET_NETWORKS` from `utils/walletNetwork.ts` with
  `networkLocaleKey()`. The buttons keep the quiet compact style, size to their
  label (minimum 70px), and expose `aria-pressed`. The create, import, and guard
  flows pass the selected value through unchanged.
- New label: `wallet.identity.lookup.network.privatetestnet` (en, nl), selected
  through a `Record<WalletNetwork, …>` map. The receive labels name the coins
  (VRSCTEST, Sepolia ETH) and stay shared with testnet. The profile read-only
  note appears only on mainnet.
- `max-product-design-taste` was applied. The `frontend-design` skill was not
  available in this session.
- Mounted test: `StepperLayout.mounted.ts` checks the three labels, the pressed
  state, and that a click sends `privatetestnet`.
- Native check, debug build with the WebDriver endpoint, from this worktree: the
  create flow's step 1 shows Mainnet, Testnet, and Private testnet. A click
  selects Private testnet. Light and dark captures were inspected. The script
  typed nothing, created no wallet, toggled the `dark` class only for the
  capture, restored the selection, and closed the flow. The app was stopped and
  ports 4445 and 1420 were closed afterwards.
- Item 3: `get_dlight_seed_status` returns `available`, which is true when the
  wallet network has a dlight server (`root_dlight_endpoint`). The owner
  accepted on 2026-10-13 that shielded features are unavailable on private
  testnet. Settings shows "Not available on this network" in the Private Verus
  row, and the Private Verus page shows that title with "This network has no
  Private Verus server yet." instead of setup. `setup_dlight_seed` refuses setup
  without a server, so request flows cannot create a shielded seed there either.
  Setting `DLIGHT_PRIVATE_TESTNET_ENDPOINTS` makes the feature available again.
  A mounted test covers the page; the rendered page was not opened, because
  automation must not open Private Verus. The card reuses the existing
  status-card classes.

### 7. Verification and documentation

1. Run the checks in the next section.
2. Update [storage policy](../../references/storage-policy.md),
   [private wallet](../../architecture/private-wallet.md),
   [wallet loading](../../architecture/wallet-loading.md), and
   [runtime configuration](../../references/eth-erc20-runtime-config.md).
3. Record D1 to D3 in a new ADR.
4. Add a dated pointer to this plan in the
   [Sepolia testnet alignment plan](./sepolia-testnet-alignment.md).

## Verification

Automated tests. Keep them few. Take expected values from the evidence table.

- Rust: the profile for each of the three networks gives the expected endpoint
  variable, delegator, chain ID, default assets, and rates policy.
- Rust: for chain `11155111`, testnet selects `0xCaA9…939f` and private testnet
  selects `0x98f5…77d3`.
- Rust: a delegator mismatch blocks bridge preflight and execution.
- Rust: the export fee probe uses `iCtawp…` on both testnets. This test fails
  before the fix.
- Rust: storage files in the older format still load. A mainnet or testnet
  wallet writes no `privatetestnet` field. State written for one testnet is not
  visible to the other.
- Mounted: the picker offers three networks and sends `privatetestnet` to
  `create_wallet` and `import_wallet_text`.
- Commands: `cargo test --manifest-path src-tauri/Cargo.toml`, `pnpm test`,
  `pnpm check`, `pnpm lint`, `pnpm build`, `pnpm docs:check`, scoped Prettier,
  `cargo fmt --check`, and `git diff --check`.

Native and live tests. Use disposable wallets only. Record each txid, height,
and time.

1. Create a new disposable private-testnet wallet. Do not reuse the `mijn app`
   testnet wallet from the
   [Keychain helper runbook](../../references/test-wallet-keychain.md).
2. Confirm the reads: balance, history, conversion paths, and one estimate.
3. Send VRSCTEST to a second address.
4. Convert through one basket, for example VRSCTEST to VRSC-USD.
5. Bridge out: send VRSCTEST to a Sepolia address through Bridge.vETH. Confirm
   arrival on Sepolia.
6. Bridge in: send Sepolia ETH and test DAI through `0x98f5…77d3`. Confirm
   arrival on the private chain.
7. Lock and unlock during a flow. Reload the app. Confirm that no state crosses
   between networks.
8. Regression: a public-testnet wallet still uses `api.verustest.net` and
   `0xCaA9…939f`. A mainnet wallet does not change.
9. Wrong endpoint: set `VRPC_PRIVATE_TESTNET_URL` to the public testnet API. The
   delegator check blocks the bridge.

## Risks

- A missed silent site makes private testnet act like mainnet. Mitigation: the
  helpers, `Record` maps, the search in step 2.5, and mounted checks of the main
  screens.
- Both testnets use the same IDs. A cache keyed only by an ID can mix data.
  Mitigation: storage for each network, separate provider instances, and a
  review of in-memory caches.
- A chain reset (O4) makes pending records and identities stale.
- The strict private proxy can reject a new call shape with `-32601`. Probe each
  new call shape against the private API before you rely on it.
- Older builds hide private-testnet wallets (D7).

## Exit criteria

- A tester can create and import a private-testnet wallet. Every screen shows
  the correct network name.
- Every VRPC call of a private-testnet wallet goes to the private API. Every
  bridge call uses `0x98f5…77d3`.
- Live evidence exists for a send, one basket conversion, a bridge out, and a
  bridge in, each with a txid and observed settlement.
- Mainnet and testnet behavior and stored files do not change. The automated
  checks pass.
- The documents in step 7 are updated. O1 to O4 are answered or recorded as
  accepted defaults.
