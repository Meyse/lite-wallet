---
owner: lite-wallet-team
last_reviewed: 2026-03-07
---

# UI style governance

Phase 1 freezes the current product look as the design-system baseline. The goal
is not to neutralize the UI. The goal is to stop new ad hoc colors from
appearing without first deciding that they are now part of the style.

## Phase 1 rules

- Existing approved colors are the system of record.
- New UI colors must come from a semantic token in `src/app.css` or an approved
  registry file.
- Approved raw colors may only be declared in token or registry source files.
  Feature code should reference tokens or helpers instead of redeclaring them.
- `bits-ui` primitives must stay inside `src/lib/components/ui/**`. Feature code
  uses local wrappers.
- Native `input`, `textarea`, and `select` elements belong in
  `src/lib/components/ui/**`. Feature code should consume local UI primitives.
- Form labels should use the local `Label` primitive in
  `src/lib/components/ui/label`.
- Standard icon-only copy actions should use the local `CopyButton` primitive in
  `src/lib/components/ui/copy-button`.
- Standard labeled copy actions should use the local `CopyActionButton`
  primitive in `src/lib/components/ui/copy-action-button`.

## Semantic tokens in `src/app.css`

| Token                            | Light     | Dark      | Purpose                                      |
| -------------------------------- | --------- | --------- | -------------------------------------------- |
| `app-canvas`                     | `#FBFBFB` | `#111111` | Entry screens and dark wallet shell surfaces |
| `sidebar-surface`                | `#EDEDED` | `#28282B` | Sidebar and aside surface                    |
| `sidebar-item-hover`             | `#E0E0E0` | `#36373B` | Sidebar hover state                          |
| `sidebar-item-pressed`           | `#D8D8D8` | `#323338` | Sidebar pressed state                        |
| `sidebar-item-active`            | `#E5E5E5` | `#303136` | Sidebar active state                         |
| `guard-revoke`                   | `#D4313E` | `#D4313E` | Guard revoke accent                          |
| `guard-recover`                  | `#4AA658` | `#4AA658` | Guard recover accent                         |
| `brand-discord`                  | `#5865F2` | `#5865F2` | Community hangout background                 |
| `brand-discord-foreground`       | `#4752C4` | `#7B86F8` | Community hangout text/icon color            |
| `brand-discord-foreground-hover` | `#3F49B7` | `#A0A8FF` | Community hangout hover text/icon color      |

## Approved literal registries

- Wallet palette: `src/lib/constants/walletColors.ts`
- Identity avatar gradients: `src/lib/styles/identityAvatarGradients.ts`
- Core theme tokens and semantic UI tokens: `src/app.css`
- Enforcement allowlist: `scripts/ui-style-registry.mjs`

## Enforcement

- `yarn lint:ui` scans `src/**` for unapproved hex literals and color functions.
- `yarn lint:ui` also rejects native `input`, `textarea`, and `select` usage
  outside `src/lib/components/ui/**`.
- `yarn lint:ui` rejects raw `<label>` usage in feature code so label styling
  stays centralized.
- The linter fails when a color is outside the approved palette or when an
  approved color is declared outside its token or registry source file.
- Context-specific errors are surfaced for inline styles and arbitrary Tailwind
  color utilities when those usages introduce an unapproved color.

## Adding a new color

1. Check whether an existing token or approved palette entry already fits.
2. If not, decide whether this is a semantic app color, a product palette entry,
   or a brand exception.
3. Add the color to the right source file first.
4. Update `scripts/ui-style-registry.mjs`.
5. Use the token or approved registry value deliberately, then verify light and
   dark mode.
