---
owner: lite-wallet-team
last_reviewed: 2026-09-09
---

# Wallet storage policy

This document defines where wallet data belongs after the Argon2id migration.

## Principles

- Stronghold is for secrets first.
- Data only belongs in Stronghold when it must stay hidden while the wallet is
  locked.
- High-churn state does not belong in Stronghold.
- Password UX stays unchanged in this pass, but new and migrated wallets use
  Argon2id-derived Stronghold keys.

## Storage classes

### Secret-critical

Spend-authorizing material and other secrets that can derive or expose those
secrets.

- Primary wallet seed or imported secret material.
- dlight seed or spending key material.

Policy:

- Store in Stronghold snapshots protected by the current Argon2id-derived key.
- Never move to `account_state.json`.
- Treat migration from legacy SHA-256 Stronghold records as automatic on
  successful unlock.

### Private-sensitive

Metadata that is not spend-authorizing but should stay hidden behind unlock.

- Address book.
- Linked identities.

Policy:

- Keep in Stronghold snapshots.
- Keep data volume small and low-churn.

Private Sapling notes, recipients, nullifiers, witnesses, transaction history,
and pending sends also belong to this class. Their frequent writes use an
authenticated encrypted cache with atomic generations and an exclusive writer,
as described in [Private Sapling wallet](../architecture/private-wallet.md). The
cache encryption key derives from the Stronghold-protected spending key;
plaintext private note databases are not permitted. Creation checkpoints remain
beside the secret in Stronghold. Pending-send state must survive lock and
restart.

### Account state

Account-scoped state that is useful after unlock but is not secret material.

- Watched VRPC addresses.
- Active assets.
- Identity provisioning jobs.

Policy:

- Store in `wallet_data/accounts/<account_id>/account_state.json`.
- Keep state network-scoped with `mainnet` and `testnet` sections.
- Legacy Stronghold copies are migration-only and should be deleted after
  successful migration.

### Public, derived, or cache data

Rebuildable or fetched state that does not need Stronghold protection.

- Transaction history and cursors.
- Balances and portfolio snapshots.
- Update-engine caches.
- Runtime-derived scope data.

Policy:

- Keep out of Stronghold.
- Use normal wallet data storage or runtime caches.

## Decision rule for new fields

A new field may go into Stronghold only if all of the following are true:

- Exposing it on disk while locked creates a meaningful security or privacy
  problem.
- It is small.
- It is low-churn.
- The app does not need it before unlock.
- Regular app storage is insufficient for the requirement.

If any of those checks fail, store it outside Stronghold.
