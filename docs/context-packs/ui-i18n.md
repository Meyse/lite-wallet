---
owner: lite-wallet-team
last_reviewed: 2026-02-12
---

# Context pack: UI and i18n

Read this before changing user-facing copy or wallet UI structure.

## Invariants

- New user-facing text must use `i18n.t(...)`.
- Keep sentence case for headings, labels, and helper text.
- Verify both light and dark mode states.
- Prefer desktop-first layout decisions for this app.

## Open these files first

- `src/lib/i18n/index.ts`
- `src/lib/i18n/locales/en.ts`
- `src/lib/i18n/locales/nl.ts`
- `src/lib/components/wallet/UnlockScreen.svelte`
- `src/lib/components/wallet/WalletLayout.svelte`
- `src/lib/components/wallet/sections/*`
- `docs/ui-style-governance.md`

## Verification checklist

- Confirm every new string key exists in both `en` and `nl` locale files.
- Confirm interactive states remain legible in dark and light themes.
- Confirm copy stays concise and task-focused.
