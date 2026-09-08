---
owner: lite-wallet-team
last_reviewed: 2026-03-10
---

# Plan: core and wallet security hardening

- Status: active
- Owner: lite-wallet-team
- Last updated: 2026-03-10

## Scope

Harden desktop wallet security posture in four areas:

1. Session auto-lock with strict allowlisted durations (`5/15/30/60` minutes,
   default `15`).
2. Runtime `.env*` file loading only in debug builds.
3. Tauri command/capability tightening (app ACL + opener URL scope).
4. Explicit production CSP instead of disabled CSP.
5. Stronghold password derivation migrated from legacy SHA-256 to explicit
   Argon2id with compatibility migration for existing wallets.

## Explicit non-goal

Password scoring/minimum length behavior remains unchanged for valu-mobile
parity.

## Risk note for unchanged password policy

The current minimum password length and scoring UX are intentionally retained.
Compensating controls in this hardening pass:

- Automatic session timeout and lock.
- Argon2id-based Stronghold key derivation for new wallets and migrated legacy
  wallets.
- Tighter Tauri command allowlist and capability scope.
- Restricted opener permission to trusted URL scope.
- Explicit production CSP.
- Release builds no longer auto-load local `.env*` files.

## Storage policy follow-up

The wallet now treats Stronghold as secret storage first. Private metadata that
must remain hidden behind unlock can stay in Stronghold snapshots, while
non-secret account state moves to
`wallet_data/accounts/<account_id>/account_state.json`.

- Source of truth:
  [`../../references/storage-policy.md`](../../references/storage-policy.md)
