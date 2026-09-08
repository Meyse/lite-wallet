---
owner: lite-wallet-team
last_reviewed: 2026-09-09
---

# Wallet loading lifecycle

The wallet route starts display loading only after the unlock check, active asset
selection, channel construction, and event listeners are ready. It passes every
active coin and channel to the update engine. Adding or removing an asset
restarts the engine with the updated active set.

## Readiness and background updates

- The update engine polls only active assets and limits concurrent balance RPCs.
- Initial balance results are emitted as each request finishes. Balance bootstrap
  completion makes the wallet usable; fiat rates continue as optional enrichment.
- The overview renders known crypto balances while other balances or fiat rates
  are still missing. Unknown amounts remain unavailable rather than becoming
  zero, and a partial total is labelled as incomplete.
- Regular balance refreshes use the same bounded worker path as bootstrap. Chain
  info, transaction events, and rates retain their independent refresh cadence.

## Session metadata

The backend caches linked identity records and derived dlight public metadata for
one unlock session. Concurrent callers share the same storage read. The cache is
replaced on every unlock and discarded on lock, so account and network changes
cannot reuse an earlier session's values. Identity mutations update the cached
records only after Stronghold persistence succeeds.

Only non-secret records and derived public addresses are cached. Seeds and other
signing material still load from Stronghold on demand and are not retained in the
public metadata cache. Blocking Stronghold snapshot reads run outside Tokio's
async worker threads.

## Display request reuse

`src/lib/services/walletDisplayService.ts` owns frontend-only request sharing:

- coin scopes and dlight configuration are reused for the wallet route session;
- display balance reads have a short five-second cache;
- identical in-flight history, scope, status, and balance requests are joined;
- generation checks reject results that finish after reset or invalidation;
- event updates prime the balance cache and invalidate affected history pages.

Route teardown, lock navigation, account changes, and network changes reset the
display session. Scope-changing mutations invalidate scope data. These policies
apply only to display reads. Send and identity preflight commands continue to use
fresh backend-owned state and do not consume the frontend display cache.

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
