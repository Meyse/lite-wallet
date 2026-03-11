---
owner: lite-wallet-team
last_reviewed: 2026-02-26
---

# References index

External references and source repos used for parity or implementation checks.

## Repositories

- `valu-mobile` parity source: `/Users/maxtheyse/dev/valu-mobile` (branch:
  `newsend3`)
- Rust identity signing parity report:
  [`./verus-identity-rust-parity.md`](./verus-identity-rust-parity.md)
- Verus coin catalog parity and sync workflow:
  [`./verus-coin-catalog-parity.md`](./verus-coin-catalog-parity.md)
- Blockchain runtime config and ETH/ERC20 phase-1 parity boundaries:
  [`./eth-erc20-runtime-config.md`](./eth-erc20-runtime-config.md)
- GitHub Actions CI/release runbook:
  [`./release-pipeline.md`](./release-pipeline.md)
- Wallet storage policy: [`./storage-policy.md`](./storage-policy.md)

## Notes

- Prefer primary source code over stale copied notes.
- When parity behavior changes, update both references and local product-spec
  docs.
- Regenerate coin catalog artifacts with `yarn sync:verus-coins`.
- Verify catalog parity and drift with `yarn check:verus-coins`.
