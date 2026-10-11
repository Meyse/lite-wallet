---
owner: lite-wallet-team
last_reviewed: 2026-10-11
---

# Watchlist portfolio boundary and startup latency

- Status: in progress (steps 1–2 done on testnet; steps 3–4 planned below;
  mainnet rate timing pending)

## Goal

Show watched holdings and their available fiat values in Watchlist without
adding them to the owned portfolio; restore prompt, truthful balance and fiat
loading at startup.

## Plan

1. **Establish a baseline.** Capture native cold-start timings for scope
   discovery, first owned balance, and first usable fiat rate. Reproduce with a
   small owned balance and a much larger watched VerusID balance; record
   provider timeouts separately from app delays.
2. **Enforce ownership at the source.** Supply portfolio balance loading and
   asset discovery with owned scopes only. Keep watched addresses on the public
   Watchlist path. Guard the Overview's scoped and pre-scope fallback
   aggregation so a watched balance cannot enter asset rows or the portfolio
   total, including during reload.
3. **Address measured startup bottlenecks.** Compare the same timings after
   removing watched-scope work from portfolio refresh. If rates still lag, let a
   failed direct VRSC quote start its Bridge.vETH fallback without waiting for
   unrelated quotes; preserve independent balance updates and explicit
   unavailable states when CoinPaprika or ECB fails. Do not display an
   unverified or expired fiat value as current.
4. **Verify the complete flow.** Cover add/remove watched ID, refresh,
   lock/unlock, and reload: owned balances and fiat totals stay unchanged while
   Watchlist retains token balances and per-coin fiat where a valid rate exists.
   Check timeout/recovery behavior, native startup timings, and affected UI in
   light and dark mode at 920×620.

## Done when

A high-value watched ID never changes owned asset discovery, balances, or
portfolio fiat total; Watchlist still displays it correctly; and measured
startup timings show whether the remaining delay is in app work or external
providers.

## Baseline (2026-09-25, native testnet)

Six cold starts of the disposable `mijn app` testnet wallet: three with its
existing Watchlist (one zero-balance ID) and three after adding `T.Betelgeuse@`
(100 VRSCTEST and 1000 Betelgeuse). The wallet owns 2.63337 VRSCTEST and has six
linked IDs. Each run relaunched `pnpm tauri dev` (debug, Stronghold work
factor 15) with the debug WebDriver feature, then used the guarded Keychain
helper to unlock. Times are seconds after the unlock action. They come from
`[WALLET_PERF]` lines and WebView IPC and render probes.

| Phase                                 |  Without watched ID |     With watched ID |
| ------------------------------------- | ------------------: | ------------------: |
| `unlock_wallet` (2.8 s seed snapshot) |    3.14 (3.13–3.19) |    3.14 (3.11–3.24) |
| First owned balance event             |    4.89 (3.64–5.28) |    4.87 (4.84–4.98) |
| Owned VRSCTEST row first shown        |    4.90 (3.72–6.21) |    4.93 (4.93–5.05) |
| `get_coin_scopes` VRSCTEST duration   |    8.62 (8.47–9.49) |    8.47 (8.46–8.57) |
| Scoped Overview balances complete     | 13.50 (12.06–14.83) | 13.39 (13.35–13.47) |
| Final owned VRSCTEST row              |              2.6334 |        **102.6334** |

Findings:

- **The boundary leak is reproduced.** After scope discovery, Overview sums the
  watched scope into the owned row. The row shows 2.6334 at about 4.9 s and then
  102.6334 at about 13.3 s. Adding an ID does not change Overview until the next
  restart, because display scopes stay cached. Manage assets' discovery also
  offers the watched-only 1000 Betelgeuse and shows VRSCTEST as 205.26674.
- **Discovery double-counts native balances.** `vrpc_balance_entries` adds
  `balance` and then `currencybalance[system]`; the daemon reports the native
  coin in both fields. This applies to owned scopes too.
- **The startup delay is local Stronghold work, not providers.** Scope discovery
  is 2.8–3.7 s waiting for the watchlist gate held by startup
  `get_watchlist_entries`, about 2.8 s reading the watchlist snapshot, and about
  2.8 s reading linked IDs on first use. Contacts, Watchlist entries and dlight
  status each take about 2.9 s in parallel. Provider balance reads took 40–60 ms
  each; the backend bootstrap finished in 0.45–0.61 s. No provider errors or
  timeouts occurred. The watched ID added one 42 ms read.
- **Fiat rates cannot be measured on testnet.** `fiat_rate_candidates` returns
  none there. The first usable rate, its fallback, and the hero total need a
  mainnet run.
- **The hero briefly shows untrue data.** In five of six runs, "€ 0.00 Partial"
  appeared 0.3–0.5 s before the first owned balance.
- Asset discovery took 20.4 s when first opened and 5.4 s when repeated.

Caveats: this is a debug build using the Vite dev server. Its module transforms
add about 1.5 s between unlock and the first wallet command after a fresh
relaunch; `control-1` reused a warm Vite server, which explains its lower
minimums. Release timings, mainnet providers, and p95 were not measured. Local
traces are in the ignored `output/watchlist-startup-baseline-20260925/`;
addresses are replaced with ownership labels.

## Step 2 result (2026-09-25, native testnet)

Watched addresses are no longer wallet scopes. `get_coin_scopes` and
`discover_vrpc_assets` use only the primary address and linked IDs and no longer
read the Watchlist. `CoinScope.isReadOnly` and discovery `includesReadOnly` stay
in responses for compatibility and are always false. Discovery counts a native
balance once. As a backstop, Overview skips read-only scopes. Before scopes
load, it sums only the primary address and known owned scope addresses.

Three cold starts with `T.Betelgeuse@` still watched, same method as the
baseline:

| Phase                               | Baseline, watched ID | Owned-only scopes |
| ----------------------------------- | -------------------: | ----------------: |
| `get_coin_scopes` VRSCTEST duration |     8.47 (8.46–8.57) |  3.08 (3.07–3.15) |
| Scoped Overview balances complete   |  13.39 (13.35–13.47) |  8.29 (8.28–8.32) |
| First owned balance event           |     4.87 (4.84–4.98) |  5.31 (5.24–5.70) |
| Final owned VRSCTEST row            |             102.6334 |            2.6334 |

- Watchlist still shows `T.Betelgeuse@` with 1,000 Betelgeuse and 100 VRSCTEST.
- Manage assets now lists owned holdings only: VRSCTEST 2.63337, gamecurrency
  200 and gamesession5 0.00009996. It checks seven owned scopes.
- The remaining 3.1 s is the first uncached linked-ID Stronghold read.
- The first owned balance arrived about 0.4 s later. The whole shift happens
  before the first wallet command after unlock, which this change does not
  touch. Recheck it in step 4.

## Steps 3–4 plan (2026-10-11)

### Source findings

A source review on 2026-10-11 found these causes. They explain the step 1–2
timings, but no native run has confirmed them yet.

- **Every encrypted file read runs scrypt.** Each Stronghold file has its own
  salt, so each read derives its key again. Debug builds use work factor 15
  (`src-tauri/src/lib.rs`) and build the crypto crates without optimization;
  `src-tauri/Cargo.toml` has no profile overrides. This explains the 2.8 s per
  read. Release builds use work factor 19 (512 MiB for each decrypt) with
  optimized code. Release read time and peak memory are not measured.
- **Four reads overlap after unlock.** Linked IDs (scopes) and dlight metadata
  serve the Overview. The Contacts list and Watchlist entries start from
  `src/routes/wallet/+page.svelte`, but the Overview does not use them. The
  Watchlist read holds the global Watchlist lock.
- **The first owned balance does not wait for storage.** The update engine
  fetches it directly. The complete scoped Overview total waits for the
  linked-ID read.
- **Cause of the "€ 0.00 Partial" flash.** The hero shows a number as soon as
  one row has a snapshot. A zero-balance row that loads first therefore gives
  "0.00" and the Partial label. The hero skeleton shows only while the value is
  "—" (`Overview.svelte`, `heroSummary`).
- **The rate fallbacks wait for unrelated quotes.** The Bridge.vETH VRSC
  fallback starts only after every direct CoinPaprika quote finishes. PBaaS
  rates start after that (`src-tauri/src/core/updates/engine.rs`,
  `run_rate_refresh_cycle`). The ECB snapshot applies only after the ECB fetch
  and the full rate cycle both finish. Before that, a cold start has USD rates
  only, so a EUR user sees "—". Rates are not kept across restarts.
- **A fallback rate can be older than its timestamp.** Bridge.vETH and PBaaS
  rates can use a cached `getcurrency` response up to about 180 s old, but the
  rate gets `fetched_at = now`.
- **No test covers rate ordering.** No test calls `run_rate_refresh_cycle`, the
  Bridge.vETH fallback, or `apply_ecb_refresh`. CoinPaprika and ECB URLs are
  constants, so a native run cannot inject their failures.

### Constraints

- Do not lower the Stronghold work factor or skip durable sync. Do not change
  the storage format. The deferred
  [fast Contacts storage plan](./fast-encrypted-contacts-storage.md) owns that
  work.
- Do not run a release build against the disposable wallet's data. A release
  write stores files at work factor 19, and later debug reads of those files
  become much slower. Debug unlock lowers only the seed file again.
- Do not show a fiat value as current when its source data is older than the
  rate cache limits. Missing, failed, and expired rates stay "—".
- Keep each change only if the step 1–2 timing method shows a measured gain.

### Step 3: address the measured bottlenecks

1. **Measure the release storage cost.** Add an ignored Rust timing test that
   writes a synthetic snapshot at work factor 19. Run it with `--release`, and
   record one read, four parallel reads, and peak memory. Use no wallet data. If
   one release read is short compared with provider time, do not do item 3.
2. **Optimize the crypto crates in debug builds.** Add `[profile.dev.package.*]`
   with `opt-level = 3` for scrypt, salsa20, sha2, pbkdf2, hmac, and argon2.
   This changes only the speed of debug builds, not their settings. Repeat the
   step 2 native run to get a new debug baseline. Use that baseline for all
   later comparisons.
3. **Shorten the path from unlock to a complete Overview.**
   - After `unlock_with_profile` succeeds, start the linked-ID and dlight
     metadata loads in the background. The session cache already makes later
     callers share one read. Do not start them before the password check
     succeeds.
   - Start `list_address_book_contacts` and `get_watchlist_entries` after the
     Overview's scoped balances finish, not at mount. Keep the on-demand loads
     in Contacts, Watchlist, and the transfer and identity flows.
   - Record the second dlight decrypt before the shielded balance as a
     follow-up. Removing it needs a security review.
4. **Remove the "€ 0.00 Partial" flash.** While a balance is pending and no
   loaded row has a fiat value above zero, show the existing hero skeleton. Keep
   the Partial label for a subtotal above zero. Add a "zero row first" state to
   the `wallet-loading` browser fixture. Check whether the mounted test near
   `Overview.mounted.ts:374` describes this case, and update it with a reason if
   it does. Apply `max-product-design-taste`.
5. **Get a mainnet rate baseline** (needs decision 1). Add to the first-rate
   `[WALLET_PERF]` lines whether the display currency is present. Run three cold
   starts. Record the first USD rate, the first display-currency rate, the first
   hero fiat value, and any provider timeouts.
6. **Change the rate order only if item 5 shows a delay.**
   - Start the Bridge.vETH fallback as soon as the direct VRSC quote fails.
     Start PBaaS derivation as soon as a VRSC anchor exists.
   - Apply the ECB snapshot as soon as it arrives.
   - Give the fallback provider loop an overall deadline, and remove duplicate
     providers by URL.
   - Give a fallback rate the age of its source data, or reject stale source
     data.
   - Add Rust tests with mock providers. A failed direct VRSC quote must start
     the fallback before a slow, unrelated quote finishes. ECB must apply before
     the cycle ends. Stale source data must not give a current rate.
   - Optional: add debug-only base URL overrides for CoinPaprika and ECB, like
     the `VRPC_*` variables, so a native run can inject failures.

### Step 4: verify the complete flow

1. **Automated checks.** Run `cargo test`, `pnpm test`, `pnpm check`, and
   `pnpm lint`. Report the known `AppSidebar.svelte` color lint failure
   separately.
2. **Native testnet, with `T.Betelgeuse@` watched.** Use the same method as the
   baseline: three cold starts with the Keychain helper and WebDriver. Fill in
   the same table, and add release-read and memory figures from step 3.1. Check
   these flows:
   - Add and remove the watched ID. The owned VRSCTEST row and total do not
     change. The Watchlist shows 100 VRSCTEST and 1,000 Betelgeuse.
   - Refresh, lock and unlock (including an automatic lock), and reload. The
     result is the same each time.
   - Manage assets lists owned holdings only.
   - The hero shows a skeleton, then a true value. It never shows "0.00 Partial"
     before the owned balance.
   - Contacts and Watchlist open with their data, also when opened right after
     unlock.
3. **Failure and recovery.** Point `VRPC_TESTNET_URL` at a local server that
   does not answer. Balances show as unavailable, no watched balance enters the
   total, and recovery works after the server answers. The rate failure tests
   from step 3.6 cover CoinPaprika and ECB failures.
4. **Mainnet** (needs decision 1). The owned total does not include the watched
   ID. The Watchlist shows its holdings with fiat values where a rate exists.
   `Verus Coin Foundation@` can be added. Record the rate timings from step 3.5.
5. **UI states.** Check the hero loading state, the Watchlist partial value, and
   Manage assets in light and dark mode at 920×620.

### Decisions

1. **Mainnet wallet (open; owner decides).** Rates exist only on mainnet. The
   Keychain helper unlocks only the testnet wallet.
   - Recommended: a new mainnet wallet with no funds that the owner creates and
     unlocks by hand, and that watches `Verus Coin Foundation@`. Its owned total
     must stay 0.00. An agent can then drive the timing runs.
   - Alternative: the owner checks their own wallet with a short checklist and
     the `[WALLET_PERF]` lines.
2. **Contacts and Watchlist load after the Overview balances, not on section
   open.** This keeps the sections ready without a visible wait.
3. **Out of scope:** keeping rates across restarts, a faster storage format, and
   removing the second dlight decrypt. Each needs its own decision.
