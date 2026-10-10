---
owner: lite-wallet-team
last_reviewed: 2026-10-10
---

# Mobile parity covers protocol and behaviour, not the flow

Request handling follows Verus Mobile on VerusCoin/Verus-Mobile
`dev-asf-parent`: which requests are accepted, what is checked, and what is
signed, encrypted, broadcast, and sent. The desktop flow is its own design and
deliberately differs where websites cannot tell the difference. Mobile acts at
each step, while desktop waits for one Approval
([ADR 0001](./0001-approval-before-any-action.md)). Mobile re-signs on each
delivery attempt, while desktop retries only the failed stage. When a new link
arrives during a review, mobile keeps the old review and loses the new link,
while desktop replaces the review before Approval.

## Consequences

Two deviations are deliberate. Desktop has no experimental setting: every
supported detail type is available, while mobile hides four behind one. Desktop
rejects wallet backup requests, because they write to an NFC card. Behaviour
that exists only in the mobile `redesign` branch is not a parity target.
