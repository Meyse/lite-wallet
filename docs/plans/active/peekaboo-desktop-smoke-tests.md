---
owner: lite-wallet-team
last_reviewed: 2026-03-15
---

# Plan: Peekaboo desktop smoke tests

- Status: active
- Owner: lite-wallet-team
- Last updated: 2026-03-15

## Goal

Add a small macOS-only Peekaboo smoke suite for desktop-only Tauri behavior that existing Rust unit tests and current CI do not cover.

## Constraints

- Keep Peekaboo scoped to desktop interaction coverage, not backend correctness already covered by `src-tauri` tests.
- Do not block the current Ubuntu CI path on Peekaboo; run it separately on a local macOS machine or a dedicated macOS workflow later.
- Prefer disposable funded testnet wallets for automated send coverage.
- Small-balance mainnet wallets are acceptable only for explicitly manual scenarios with operator review.
- Do not automate seed phrase display, recovery key export, private key reveal, or any screen that intentionally exposes secret material.
- Pin locale and environment for deterministic runs; default to English unless a locale-specific scenario is under test.
- Treat Peekaboo as scripted automation (`peekaboo run` / CLI JSON output), not as an AI-agent test harness.
- Keep screenshots and snapshot artifacts out of git; retain them only as local or CI artifacts when debugging failures.

## Proposed scope

### Phase 1: stable non-broadcast smoke checks

1. Launch the app, unlock a known fixture wallet, and verify the main wallet shell is interactive.
2. Open wallet switcher, choose a second fixture wallet, and unlock it.
3. Trigger a Verus deep link and verify the generic request flow opens and renders the expected review surface.
4. Open a trusted external link and verify the opener flow hands off cleanly.
5. Open the transfer wizard, fill source amount and recipient, and verify review/preflight renders without broadcasting.

### Phase 2: automated funded testnet coverage

1. Run a funded testnet unlock smoke against a disposable wallet.
2. Complete one controlled testnet send on the highest-confidence path first:
   VRSC native send.
3. After that is stable, extend to one additional route only if it stays deterministic:
   BTC, ETH, or bridge preflight.

### Phase 3: manual-only tiny mainnet checks

1. Unlock a low-balance mainnet wallet reserved for smoke testing.
2. Run review/preflight checks without sending by default.
3. Allow an operator-triggered micro-send only for release validation or incident reproduction.

## Initial scenarios to implement first

1. `unlock-existing-wallet`
   Covers app launch, password entry, wallet shell readiness, and top-level focus issues.
2. `switch-wallet`
   Covers drawer interactions and account switching regressions on the signed-out surface.
3. `deep-link-request-review`
   Covers desktop-only Tauri deep-link plumbing and the generic request entry path.
4. `transfer-preflight-testnet`
   Covers amount entry, recipient entry, route selection, and review state with a funded disposable wallet.
5. `external-link-opener`
   Covers `tauri-plugin-opener` integration and foreground/window handoff behavior.

## Explicit non-goals

- Replacing `cargo test`, frontend type checks, or future API-level tests.
- Broad visual-diff coverage across the entire app.
- Running large matrix coverage across every chain/channel in Peekaboo.
- Using natural-language agent mode for routine regression checks.

## Decisions

- Peekaboo complements this repo because Tauri desktop WebDriver does not cover macOS, while this app is desktop-first and uses native Tauri integrations such as deep links and opener flows.
- The suite should stay small and high-signal: a few deterministic desktop journeys are better than a wide, flaky matrix.
- We accept screenshot and snapshot artifacts for funded smoke wallets, but we still exclude secret-reveal surfaces because those artifacts have no regression value proportional to the exposure.
- Broadcast coverage should start on testnet only. Mainnet broadcast remains manual because the failure mode is financial, not just flaky.
- The first CI improvement outside Peekaboo should still be adding Rust test execution to automated CI.

## Implementation outline

1. Add a local runbook under `docs/` or `scripts/` describing:
   app startup command, wallet fixture selection, locale, theme, and Peekaboo permissions.
2. Add a deterministic fixture strategy:
   one signed-out profile, one funded testnet wallet, one optional tiny mainnet wallet.
3. Add scriptable Peekaboo scenarios with JSON output and named artifact directories.
4. Add one wrapper command for local execution so the suite can be run without remembering raw Peekaboo CLI invocations.
5. Once the local flow is stable, add an optional macOS-only GitHub Actions workflow that does not gate normal pull requests initially.

## Verification

- Local dry run can complete all Phase 1 scenarios on macOS without manual intervention.
- Artifact output includes enough evidence to diagnose failures:
  screenshots, Peekaboo JSON output, and app logs when needed.
- Testnet send smoke proves at least one funded end-to-end transfer path beyond unit tests.
- Existing repo checks still pass unchanged:
  `yarn lint`, `yarn check`, `yarn build`, and `cargo test` in `src-tauri`.

## Exit criteria

- At least four deterministic macOS Peekaboo scenarios exist and are documented.
- One funded testnet send scenario is reliable enough to run repeatedly.
- Mainnet usage, if any, is documented as manual-only and uses a dedicated low-balance wallet.
- The suite has a clear owner, runbook, and artifact policy.
