---
owner: lite-wallet-team
last_reviewed: 2026-09-21
---

# Product specs index

Read this when implementing or validating user-visible behavior and parity
requirements.

## Current specs and trackers

- Wallet asset sorting and search (approved Paper design):
  [`./wallet-asset-sorting.md`](./wallet-asset-sorting.md)
- Contacts and VerusID profiles (draft; includes interactive profile previews):
  [`./contacts-and-verusid-profiles.md`](./contacts-and-verusid-profiles.md)
- VerusID profile screen 05 (selected Paper layout and image recommendations):
  [`./verusid-profile-screen.md`](./verusid-profile-screen.md)
- VerusID profile publishing UX corrections (implemented; local checks and
  native macOS VRSCTEST retest passed):
  [`./verusid-profile-publishing-ux.md`](./verusid-profile-publishing-ux.md)
- VerusID lookup and public profile (implemented; focused linked-list, lookup,
  profile, contact-save, and Send-entry contract):
  [`./verusid-lookup-and-profile.md`](./verusid-lookup-and-profile.md)
- VerusID profile discovery (background; tab/list/profile-layout portions are
  superseded by the focused implemented specification):
  [`./verusid-profile-discovery.md`](./verusid-profile-discovery.md)
- Verified websites (draft; guided proof upload, verification, and publication):
  [`./verusid-verified-websites.md`](./verusid-verified-websites.md)
- Shared contact and profile terminology: [`../../CONTEXT.md`](../../CONTEXT.md)
- Verus/PBaaS parity matrix:
  [`./verus-pbaas-core-parity-matrix.md`](./verus-pbaas-core-parity-matrix.md)
- Verus/PBaaS parity fixtures:
  [`./verus-pbaas-core-parity-fixtures.json`](./verus-pbaas-core-parity-fixtures.json)
- Identity guard signed-out UX flow:
  [`./identity-guard-signed-out-flow.md`](./identity-guard-signed-out-flow.md)
- Settings scope (desktop): [`./settings-scope.md`](./settings-scope.md)
- UI component adoption matrix:
  [`../ui-component-matrix.md`](../ui-component-matrix.md)

## Scope notes

- Parity documents track behavior versus `valu-mobile` (`newsend3`).
- Component matrix tracks implementation status in this repo.

## Gaps to fill next

- Onboarding behavior spec split by step.
- Send and receive UX spec with edge-case copy.
