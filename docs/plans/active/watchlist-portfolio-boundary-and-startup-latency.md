---
owner: lite-wallet-team
last_reviewed: 2026-09-25
---

# Watchlist portfolio boundary and startup latency

- Status: in progress (steps 1–2 done on testnet; mainnet rate timing pending)

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
