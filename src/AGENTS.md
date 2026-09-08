# Frontend agent map (`src`)

This file applies to frontend work under `src/`.

## Core frontend rules

- Use translation keys via `i18n.t(...)` for any user-facing copy.
- Keep sentence case for user-facing text.
- Validate light and dark mode for all changed UI states.
- Default to desktop UX decisions for this app.
- Use the `frontend-design` skill for frontend UI work in `src/`, especially
  layout, hierarchy, spacing, styling, states, and responsiveness.
- Keep visual patterns consistent with
  `src/lib/components/wallet/UnlockScreen.svelte`.
- For scrollable lists/panels in app UI, use shadcn-svelte `ScrollArea`
  (`$lib/components/ui/scroll-area`) instead of ad-hoc `overflow-y-auto`, unless
  native browser scrolling is explicitly required.
- Treat repeated user-facing content (for example, Need help Q&A blocks) as
  shared sources instead of duplicated inline arrays in multiple screens.
- Prefer shared utilities under `src/lib` (for example, `src/lib/utils/*`) that
  accept `i18n.t(...)` and return localized content, then compose per-screen
  variations in one place.
- Keep error messages actionable.

## Start points by task

- General wallet route lifecycle: `src/routes/wallet/+page.svelte`
- Send flow UI and orchestration:
  `src/lib/components/wallet/sections/Send.svelte`,
  `src/lib/components/wallet/sections/TransferWizard.svelte`,
  `src/lib/components/wallet/sections/transfer-wizard`
- Event updates and stores: `src/lib/services/eventBridge.ts`, `src/lib/stores`
- Locale and formatting: `src/lib/i18n/index.ts`, `src/lib/i18n/locales/en.ts`,
  `src/lib/i18n/locales/nl.ts`

## Related docs

- Docs index: [`../docs/index.md`](../docs/index.md)
- UI + i18n context pack:
  [`../docs/context-packs/ui-i18n.md`](../docs/context-packs/ui-i18n.md)
- Send flow context pack:
  [`../docs/context-packs/send-flow.md`](../docs/context-packs/send-flow.md)
