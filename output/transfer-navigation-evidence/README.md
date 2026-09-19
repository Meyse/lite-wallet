# Send and Convert navigation evidence

Captured 2026-09-19 in the Codex in-app browser using the production
`WalletLayout`, `AppSidebar` and `TransferWizard`, with synthetic Tauri
responses and example contacts. No wallet was unlocked and no real transaction
was signed or broadcast. The conversion path is a synthetic layout fixture, not
network or bridge proof.

## Reproduce

Use the repository Node and pnpm versions, install dependencies, and run:

```sh
pnpm exec vite --config output/playwright/vite.config.js --host 127.0.0.1 --port 1439 --strictPort
```

Open `/output/playwright/transfer-navigation.html` for English/light or add
`?theme=dark&locale=nl` for Dutch/dark. Start Send or Convert from Wallet,
choose the example public Verus source, enter an amount and `alex.example@`. The
fixture rejects transaction signing by default.

## Captures

- `send-light-920.png`, `send-dark-nl-920.png`: compact top tabs, permanent
  244px sidebar, form and footer at 920 × 620.
- `review-light-920.png`, `review-dark-nl-920.png`: compact review title,
  recipient acknowledgment and submission footer at the minimum desktop size.
- `convert-light-920.png`, `convert-dark-nl-920.png`: stacked conversion panels
  with body scrolling and fixed header/footer.
- `convert-light-1200.png`, `convert-dark-nl-1200.png`: two-column conversion
  details at 1200 × 800.
- `resume-dialog-dark-nl-920.png`: explicit resume/replace draft decision.

## Interaction evidence

Verified in the browser: sidebar → Contacts → Back to send preserves amount and
recipient and restores focus; leaving a review requires Refresh review; the
return action sits below the native drag region; Cmd+B leaves the sidebar
visible; no document-level horizontal overflow at 920px. Verified keyboard
default on the resume dialog and Escape cancellation after the final focus
correction.

Mounted regression coverage in `preflightLifecycle.mounted.ts` covers draft
preservation/replacement, Cancel/Escape, refreshed review before send, sidebar
submission lock with Lock available, ignored stale preflight completion, and
wallet/network/unlock-session replacement. These tests use mocked transaction
services.

Native Tauri execution and real wallet/chain transactions were not exercised.
