---
owner: lite-wallet-team
last_reviewed: 2026-10-10
---

# Approval before any action on a request

Verus Mobile and the earlier desktop flow broadcast an identity update, and
mobile signs data packets, before the user reaches the final screen of a
request. The desktop wallet signs, broadcasts, and sends nothing for a request
until the user's single Approval on the final Confirm step. After Approval it
runs three stages in a fixed order: the on-chain transaction (identity update or
invoice payment), then signing the response, then delivery. A failed stage is
retried on its own, so within one open request the wallet never broadcasts the
same transaction twice. The user consents once with every consequence in view,
and the transaction ID is available to the response. See the request terms in
[`CONTEXT.md`](../../CONTEXT.md).

## Consequences

Requests are held in memory only, one at a time, as in Verus Mobile. Before
Approval, a lock keeps the request in memory and it reopens after unlock. After
Approval, the auto-lock timer pauses while the stages run. A manual lock or a
quit after a broadcast still loses the transaction ID and the unsent response,
because nothing is stored; reopening the same link can offer that broadcast
again. This matches Verus Mobile and was chosen over storing an encrypted resume
record.
