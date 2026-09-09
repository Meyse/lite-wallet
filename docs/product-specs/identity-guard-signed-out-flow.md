---
owner: lite-wallet-team
last_reviewed: 2026-02-13
---

# Identity guard signed-out flow

This spec defines the signed-out `VerusID Guard` UX on desktop-first screens.

## Goal

Provide an always-available signed-out path to revoke or recover a VerusID using
temporary in-memory guard sessions.

## Entry points

- `src/lib/components/wallet/WelcomeScreen.svelte`
- `src/lib/components/wallet/UnlockScreen.svelte`

Both screens include a fixed bottom-right dock control labeled `VerusID Guard`.

## Visual behavior

- Dock button is attached to right and bottom edges.
- Shape: rounded top-left corner only (`rounded-tl-[24px]`).
- Border emphasis on top and left edges.
- Hover/focus increases contrast while staying visually secondary to primary
  onboarding actions.

## Interaction flow

1. User clicks the dock button.
2. Right-side sheet opens with two actions:
   - `Revoke VerusID`
   - `Recover VerusID`
3. Choosing an action opens full-screen overlay flow.

## Revoke flow

1. Import authority secret and choose network using one of:
   - `Paste Secret Recovery Phrase` (24-word mnemonic)
   - `Type one by one` (24-word mnemonic)
   - `WIF, private key, or custom seed`
2. Enter target VerusID.
3. Run preflight with operation `revoke`.
4. Review warnings/high-risk changes and fee.
5. Submit and show result with txid.

## Recover flow

1. Import authority secret and choose network using one of:
   - `Paste Secret Recovery Phrase` (24-word mnemonic)
   - `Type one by one` (24-word mnemonic)
   - `WIF, private key, or custom seed`
2. Enter target VerusID.
3. Enter required new primary address and optional advanced patch fields:
   - recovery authority
   - revocation authority
   - private address
4. Run preflight with operation `recover`.
5. Review patch + warnings/high-risk changes.
6. Submit and show result with txid.

## Backend/API contract

Uses existing commands via frontend service wrappers:

- `begin_guard_session`
- `preflight_guard_identity_update`
- `send_guard_identity_update`
- `end_guard_session`

No UI-supplied transaction hex is accepted.

## Safety and lifecycle requirements

- Guard session is in-memory only.
- Step-1 import mode is strict:
  - mnemonic options require valid 24-word English BIP39
  - text option allows WIF/private-key-hex/seed-text auto classification
- Session is ended on:
  - explicit close/cancel
  - flow unmount
  - final completion path
- Expired/missing session and preflight are surfaced as user-safe errors with
  restart guidance.

## i18n and copy

- All user-facing text uses `i18n.t(...)`.
- Sentence case is required.
- EN/NL keys are kept in sync for each new guard string.

## Verification checklist

- Dock appears on both signed-out screens in light and dark modes.
- Sheet opens and routes to correct flow mode.
- Revoke and recover happy paths execute with txid result.
- Recover enforces primary address requirement in basic mode.
- Advanced fields remain optional but validate if partially provided.
- `end_guard_session` cleanup runs on close/unmount.
