---
owner: lite-wallet-team
last_reviewed: 2026-10-10
---

# TypeScript parses request details; Rust re-checks what it acts on

Rust verifies every request's signature but has no Verus primitives library, so
it understands the contents of only a few detail types. Writing Rust parsers for
every detail type would duplicate `verus-typescript-primitives` and invite
parity bugs. TypeScript therefore parses details with the primitives library for
review, and Rust re-derives from the verified request bytes every field that
affects what the wallet signs, pays, shares, or broadcasts: amounts,
destinations, identities, expiry, and flags. Rust never trusts these values when
they arrive from TypeScript; a mismatch rejects the request.

## Consequences

The identity update path changes: today TypeScript supplies the requested
identity JSON and request metadata to Rust. Under this decision Rust checks them
against the verified request before preflight.
