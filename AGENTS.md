# Lite wallet agent map

Use this file as a table of contents, not as an encyclopedia.

## Global rules (always apply)

- Use the repository-pinned `pnpm` version for package-manager commands.
- Do not use `npm` or Yarn.
- Any new user-facing UI text must use translation keys via `i18n.t(...)` from
  `src/lib/i18n`.
- Use sentence case for user-facing UI copy.
- Always verify both light and dark mode for changed UI.
- Default to desktop UX over mobile parity.
- Use the normal arrow cursor (`cursor: default`) for all clickable actions,
  including buttons, wallet selectors, icons, and external links. Never use a
  hand/pointer cursor. Keep text-entry, resize, and disabled-state cursors where
  appropriate; see `docs/ui-style-governance.md`.
- Use the `frontend-design` skill for frontend UI, layout, styling, and visual
  polish work in this repository.
- Keep cognitive load low: one primary task at a time and minimal simultaneous
  UI elements.
- Keep visual language consistent with existing screens (especially
  `UnlockScreen` input treatment).
- Use Lucide icons (`@lucide/svelte/icons/*`) for UI iconography; avoid custom
  inline SVG icons when a Lucide equivalent exists.
- For parity research against `valu-mobile` (`newsend3`), use
  `/Users/maxtheyse/dev/valu-mobile` on branch `newsend3`.
- Centralize repeated user-facing content definitions instead of duplicating
  them across screens.
- For shared help topics, FAQs, and copy blocks used in more than one screen,
  create a shared helper in `src/lib` and consume it from all screens.
- Keep changes small and focused when possible.
- Keep public interfaces stable unless there is a clear reason to change them.
- Document assumptions near the code that depends on them.
- Review changes with a focus on regressions.

## Disposable testnet automation

- For the user-designated disposable `mijn app` wallet, use the
  [Keychain helper runbook](docs/references/test-wallet-keychain.md).
- Never request or print its password in chat, arguments, environment variables,
  logs, or screenshots. Use the native setup dialog and guarded helper commands.

## Primary maps

- Repo knowledge map: [`docs/index.md`](docs/index.md)
- Frontend-local guidance: [`src/AGENTS.md`](src/AGENTS.md)
- Backend-local guidance: [`src-tauri/AGENTS.md`](src-tauri/AGENTS.md)

## Task-first context packs

- Send flow:
  [`docs/context-packs/send-flow.md`](docs/context-packs/send-flow.md)
- Identity + guard flow:
  [`docs/context-packs/identity-guard.md`](docs/context-packs/identity-guard.md)
- UI + i18n updates:
  [`docs/context-packs/ui-i18n.md`](docs/context-packs/ui-i18n.md)

## Source-of-truth docs

- Architecture map: [`docs/architecture/index.md`](docs/architecture/index.md)
- Product specs index:
  [`docs/product-specs/index.md`](docs/product-specs/index.md)
- Plans index: [`docs/plans/index.md`](docs/plans/index.md)
- References index: [`docs/references/index.md`](docs/references/index.md)

## Drift rule

If docs and code disagree, trust code, then open a docs follow-up update in the
same change whenever practical.
