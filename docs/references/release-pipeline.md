---
owner: lite-wallet-team
last_reviewed: 2026-02-26
---

# Release pipeline (GitHub Actions)

This runbook defines CI and release automation for cross-platform desktop
builds.

## Scope

- In scope: build/test automation and draft release packaging for macOS,
  Windows, and Linux.
- Out of scope: updater integration, proxy-based key protection, code signing,
  and notarization.

## Workflows

### CI

- File: `/Users/maxtheyse/dev/lite-wallet/.github/workflows/ci.yml`
- Triggers: `pull_request`, `push` (non-tag refs)
- Runner: `ubuntu-latest`
- Steps:
  1. Checkout
  2. Setup Node 22.23.2
  3. Enable Corepack
  4. Activate the `pnpm` version pinned in `package.json`
  5. `pnpm install --frozen-lockfile`
  6. `pnpm lint`
  7. `pnpm check`
  8. `pnpm test`
  9. `pnpm build`
  10. `cargo test --locked`
  11. `cargo check --locked`

### Release

- File: `/Users/maxtheyse/dev/lite-wallet/.github/workflows/release.yml`
- Triggers:
  - Tag push: `v*`
  - Manual: `workflow_dispatch` with `tag` input
- Permission: `contents: write`
- Release output: draft GitHub release with generated notes

#### Preflight contract

The workflow fails before matrix builds unless all checks pass:

1. Tag format is `vX.Y.Z` or prerelease variant (`vX.Y.Z-...`).
2. Tag version must exactly match:
   - `/Users/maxtheyse/dev/lite-wallet/package.json`
   - `/Users/maxtheyse/dev/lite-wallet/src-tauri/tauri.conf.json`
   - `/Users/maxtheyse/dev/lite-wallet/src-tauri/Cargo.toml`

#### Target matrix

1. `macos-15-intel` → `x86_64-apple-darwin`
2. `macos-14` → `aarch64-apple-darwin`
3. `windows-latest` → `x86_64-pc-windows-msvc`
4. `ubuntu-22.04` → `x86_64-unknown-linux-gnu`

Linux installs these packages before building:

- `libwebkit2gtk-4.1-dev`
- `libgtk-3-dev`
- `libayatana-appindicator3-dev`
- `librsvg2-dev`
- `patchelf`

## Release operator steps

1. Ensure versions are aligned in:
   - `/Users/maxtheyse/dev/lite-wallet/package.json`
   - `/Users/maxtheyse/dev/lite-wallet/src-tauri/Cargo.toml`
   - `/Users/maxtheyse/dev/lite-wallet/src-tauri/tauri.conf.json`
2. Commit and push.
3. Create and push tag `vX.Y.Z`.
4. Wait for `Release` workflow to complete.
5. Review the draft release artifacts.
6. Publish the draft release manually when verified.

Manual trigger option:

1. Open Actions → `Release` workflow.
2. Run workflow with an existing `vX.Y.Z` tag.
3. Preflight validation and matrix build behavior stays the same.

## Key and env policy for this phase

- `INFURA_PROJECT_ID` and `ETHERSCAN_API_KEY` are treated as leakable in this
  phase.
- No secret-hardening changes are required to ship this pipeline.
- ETH features may remain disabled at runtime when keys are not configured.
- Follow-up work can add BYO key UX or a proxy service.

## Notes

- Artifacts are unsigned in this iteration.
- macOS and Windows trust warnings are expected until signing/notarization is
  added.
