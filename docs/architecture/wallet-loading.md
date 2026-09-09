---
owner: lite-wallet-team
last_reviewed: 2026-09-09
---

# Wallet loading lifecycle

After the unlock check, the wallet route starts event-listener setup and loads
the active wallet, addresses, coin registry, and active asset selection in
parallel. It renders the overview as soon as that essential display metadata is
ready. Address-book contacts continue in the background. The route still waits
for listener setup before it starts the update engine, so an early balance or
rate event cannot be missed. Adding or removing an asset restarts the engine
with the updated active set.

## Readiness and background updates

- The update engine polls only active assets and limits concurrent balance RPCs
  to four. It starts transparent balance work without waiting for dlight public
  metadata. When that metadata arrives, shielded channels join the same
  schedule.
- Initial balance results are emitted as each request finishes. The prioritized
  balance bootstrap ends after the active overview channels complete; fiat rates
  continue as optional enrichment.
- The overview renders known crypto balances while other balances or fiat rates
  are still missing. Unknown amounts remain unavailable rather than becoming
  zero, and a partial total is labelled as incomplete.
- Regular balance refreshes use the same bounded worker path as bootstrap.
  Balance and rate schedules are polled independently under one cancellation
  token, so a slow rate provider cannot hold the balance cadence. Every event is
  checked against the current unlock session before publication.

## Public fiat-rate scheduling and cache

The rate schedule publishes results progressively. Up to four independent
CoinPaprika requests run first. A failed direct VRSC lookup can use the guarded
Bridge.vETH DAI path to establish a VRSC/USD anchor. PBaaS derivations then run
with the available anchor rates, followed by the strict ETH/Bridge.vETH alias
fallback. Provider timeouts and failures retry after 30, 60, 120, 240, and then
at most 300 seconds. A rate failure does not delay balance work.

`PublicRatesCache` retains only public market and ECB reference data for the
life of the app process. A market entry is keyed by the exact wallet network,
coin ID, currency ID, system ID, protocol, and configured CoinPaprika ID, and
records its fetch time and actual source. A registry change to price-source
configuration therefore cannot reuse the old entry. Entries refresh after five
minutes and can still be displayed while a refresh runs until they are 15
minutes old. If an active asset has no displayable entry when the engine starts,
or a previously published entry expires after failed refreshes, the engine emits
an empty rate snapshot so the frontend removes that fiat value. A later
successful refresh replaces the empty snapshot normally. Rebuilt snapshots are
also emitted when an expired ECB reference removes a display currency.

ECB reference rates refresh after six hours and retain the publication date from
the feed. A response with a missing, invalid, future, or more than five-day-old
publication date, or without usable reference currencies, is rejected and
retried with the normal bounded backoff. The five-calendar-day publication
window allows for weekends and common multi-day TARGET closures without claiming
a complete holiday calendar. An accepted cache entry also has a seven-day
maximum age since it was fetched. Successful ECB refreshes rebuild cached fiat
conversions from their retained USD price. Cache replay also rebuilds those
conversions against the currently usable ECB snapshot, and falls back to
USD-only rates when that snapshot is no longer usable.

The cache survives wallet locks and unlocks in the same app process, but it is
not written to disk and is empty after an app restart. It contains no account,
balance, address, seed, or signing data.

## Session metadata

The backend caches linked identity records and derived dlight public metadata
for one unlock session. Concurrent callers share the same storage read. The
cache is replaced on every unlock and discarded on lock, so account and network
changes cannot reuse an earlier session's values. Identity mutations update the
cached records only after Stronghold persistence succeeds.

Only non-secret records and derived public addresses are cached. Seeds and other
signing material still load from Stronghold on demand and are not retained in
the public metadata cache. Unlock keeps Argon2id password derivation, Stronghold
seed snapshot work, and public-profile key derivation off Tokio's async worker
threads. The Argon2id parameters and Stronghold encryption format are unchanged.

## Timing evidence

`[WALLET_PERF]` logs identify the unlock account lookup, password KDF, seed
snapshot, public-profile derivation, total unlock, and dashboard
essential-metadata phases. Update-engine logs mark the first successfully
emitted balance and non-empty rate events. Dashboard logs mark receipt of those
events. Event receipt does not prove that Svelte painted a value, and the first
non-empty rate event may not yet contain the user's selected fiat currency.
Render and native latency need separate runtime measurement.

## Display request reuse

`src/lib/services/walletDisplayService.ts` owns frontend-only request sharing:

- coin scopes and dlight configuration are reused for the wallet route session;
- display balance reads have a short five-second cache;
- identical in-flight history, scope, status, and balance requests are joined;
- generation checks reject results that finish after reset or invalidation;
- event updates prime the balance cache and invalidate affected history pages.

Route teardown, lock navigation, account changes, and network changes reset the
display session. Scope-changing mutations invalidate scope data. These policies
apply only to display reads. Send and identity preflight commands continue to
use fresh backend-owned state and do not consume the frontend display cache.

## Transaction history

VRPC history reads scan one 2,000-block window per page. An empty recent window
can still return a continuation cursor, and the UI retains page state while the
user moves between wallet sections. Older windows load only after the user uses
the **Load older transactions** action; an empty page does not trigger an
automatic full-chain scan.

Browser fixtures can prove the visible staged-arrival, partial-value, and
explicit-pagination states, including light and dark rendering. They do not
prove Tauri transport, daemon responses, or live network timing; Rust tests and
runtime integration evidence cover those boundaries separately.

The provider coalesces identical cached VRPC reads through a shared result
channel. The leader writes a successful value to the provider cache before
publishing it to waiters. Leader cancellation or failure publishes an error and
removes the in-flight key, allowing a later request to retry.
