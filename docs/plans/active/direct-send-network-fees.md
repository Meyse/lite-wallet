---
owner: lite-wallet-team
last_reviewed: 2026-09-15
---

# Plan: Economy and standard direct-send network fees

- Status: implemented candidate; integration review pending
- Owner: lite-wallet-team
- Source plan:
  `/private/tmp/lite-wallet-send-integration-01a0a1ce/direct-send-network-fees.md`
- Paper reference: Page 1, F family, 18 artboards (read-only)

## Goal

Offer Economy and Standard fee policies for ordinary direct BTC, ETH, and ERC20
sends. Keep all conversion, cross-chain export, bridge, VRPC, and private-send
fee behavior automatic and unchanged.

## Product amendment to the source plan

The source plan placed the selector on Details. The approved Paper
reconciliation changes that interaction:

- Details contains amount, real spendable source, destination network, and
  recipient.
- Review remains a required screen and contains the one authoritative network
  fee row.
- For eligible direct sends, that row opens a right-side fee drawer. There is no
  extra fee step and no duplicate fee control on Details.
- The drawer offers only Economy and Standard. Standard is the default for each
  new wizard session. Paper's Fast/custom controls, countdowns, and predictions
  are intentionally omitted.
- Applying a changed mode clears the displayed preflight and disables Send
  immediately. The backend then resolves a fresh authoritative fee and
  `preflight_id`; Send is restored only for that result.
- EVM copy describes the displayed amount as a maximum network-fee envelope. The
  UI does not invent an expected final fee.

## Eligibility boundary

A single pure frontend helper includes `feeMode` only when all conditions hold:

1. The operation is ordinary `preflight_send`.
2. Conversion is off.
3. No cross-chain export or bridge destination is selected.
4. The channel prefix is `btc`, `eth`, or `erc20`.

The backend independently defaults an omitted eligible mode to Standard and
rejects an explicit mode on any ineligible channel. No fee field is added to
bridge or VRPC request shapes.

| Route                           | Fee choice         | Behavior                           |
| ------------------------------- | ------------------ | ---------------------------------- |
| Direct BTC                      | Economy / Standard | Mempool-backed sat/vB quote        |
| Direct ETH                      | Economy / Standard | EIP-1559 maximum envelope          |
| Direct ERC20                    | Economy / Standard | EIP-1559 maximum envelope          |
| VRPC / PBaaS                    | None               | Existing automatic behavior        |
| Private dLight                  | None               | Existing automatic behavior        |
| Conversion / cross-chain export | None               | Existing routed preflight          |
| Either bridge direction         | None               | Existing composite bridge behavior |

## Backend contract

- `PreflightParams.feeMode` and `PreflightResult.feeMode` are optional and
  backward compatible.
- Send still accepts only the session-scoped, single-use `preflight_id`.
- The complete resolved mode, fee inputs, and amount remain in the backend-owned
  preflight payload.
- Invalid quotes fail retryably; they never fall back to a silent constant.

### BTC

- Fetch `/v1/fees/recommended` from the configured mempool provider.
- Require positive integer targets and clamp Economy/Standard to the provider's
  minimum fee.
- Use Economy's `economyFee`; use Standard's `halfHourFee`.
- Compute deterministic legacy P2PKH input shape plus the actual destination and
  change output scripts with checked integer satoshi arithmetic.
- Store the sat/vB rate and total fee. Dust change is added to the fee and
  surfaced as a warning. Max-send deducts the shaped fee.
- At send time, revalidate input/output totals and the reviewed shaped-fee
  bounds before signing.

### ETH and ERC20

- Resolve both direct paths through one EIP-1559 policy helper.
- Economy uses the provider maximum fee without 33% headroom and halves the
  provider priority fee.
- Standard adds one-third headroom and preserves the provider priority fee.
- Keep the 1 gwei maximum-fee floor and clamp priority at or below maximum.
- Preserve the existing 20% ETH and 33% ERC20 gas-limit margins.
- Display `gasLimit * maxFeePerGas` as the maximum fee envelope.
- Direct ERC20 submission must remain type-2 EIP-1559 and preserve distinct
  maximum and priority fee fields. Keep the send-time drift guard.

## UI and receipt behavior

- The source picker lists positive-balance wallet channel sources, including
  public and private address scopes; it does not synthesize identity balances.
- Prototype amounts, fiat values, title-bar decoration, and footnotes are not
  app data and are not copied into production UI.
- A successful command renders an immutable submitted snapshot, not live form
  state, with route-aware localized timing guidance.
- Receipt titles are exactly `Send has been submitted` or
  `Conversion has been submitted` in English.
- The receipt exposes Transaction id, Copy, a quiet explorer action when a
  supported source-chain explorer exists, and Done. It does not claim
  confirmation or completion and does not add a confirmation tracker.

## Concurrency and invalidation

- Amount, recipient, memo, route, source, or applied fee-mode changes invalidate
  the current preflight.
- Each request has a monotonically increasing generation and input signature.
- Superseded, late, session-changed, or route-changed responses cannot restore
  review state.
- Rapid mode changes result in one current winner. A failed refresh leaves Send
  blocked until the user obtains a new authoritative preflight.

## Hard exclusions

- No changes under `src-tauri/src/core/channels/eth/bridge/**`.
- No bridge or VRPC fee-choice fields.
- No Paper edits.
- No mainnet value-spending tests.

## Verification target

- Rust format and focused policy, BTC shape/max/dust/send guard, EVM envelope,
  ERC20 type-2, route eligibility, session and bridge tests.
- Frontend type-check, tests, lint, format, production build, and docs check.
- Light and dark UI evidence at approximately 920 × 620 when a safe disposable
  wallet runtime is available. Static or mocked evidence must be labeled and
  does not prove a live wallet or external network.
