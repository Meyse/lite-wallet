---
owner: lite-wallet-team
last_reviewed: 2026-02-12
---

# Docs index

This directory is the repository knowledge system of record.

## How to use this map

1. Start with the smallest context pack for the task.
2. Read domain docs only if the context pack is insufficient.
3. Update docs in the same pull request when behavior changes.

## Domain map

- Architecture: [`./architecture/index.md`](./architecture/index.md)
- Product specs: [`./product-specs/index.md`](./product-specs/index.md)
- Plans: [`./plans/index.md`](./plans/index.md)
- Context packs: [`./context-packs/index.md`](./context-packs/index.md)
- References: [`./references/index.md`](./references/index.md)

## Existing parity and UI trackers

- Verus/PBaaS parity matrix:
  [`./product-specs/verus-pbaas-core-parity-matrix.md`](./product-specs/verus-pbaas-core-parity-matrix.md)
- Verus/PBaaS parity fixtures:
  [`./product-specs/verus-pbaas-core-parity-fixtures.json`](./product-specs/verus-pbaas-core-parity-fixtures.json)
- Identity guard signed-out UX spec:
  [`./product-specs/identity-guard-signed-out-flow.md`](./product-specs/identity-guard-signed-out-flow.md)
- UI component matrix: [`./ui-component-matrix.md`](./ui-component-matrix.md)
- UI style governance: [`./ui-style-governance.md`](./ui-style-governance.md)
- Wallet activation source-of-truth: wallet-scoped active assets are persisted
  per account + network and drive both Add Asset state and chain scope
  availability.
- Wallet storage policy:
  [`./references/storage-policy.md`](./references/storage-policy.md)

## Ownership and freshness

- Keep each doc small and task-oriented.
- Prefer links over duplicated content.
- Add "last updated" notes when docs describe behavior that changes frequently.
