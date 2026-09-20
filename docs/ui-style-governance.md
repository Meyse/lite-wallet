---
owner: lite-wallet-team
last_reviewed: 2026-09-15
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
- Copy actions never render `Copy`, `Copied`, or an equivalent label in the
  interface. Use `CopyButton` for the icon and changed-icon feedback, with a
  localized accessible name and tooltip.
- Standard inline text actions should use the local `InlineTextActionButton`
  primitive in `src/lib/components/common/InlineTextActionButton.svelte` instead
  of ad hoc underlined buttons or links. Its default accent is `text-action`;
  use its muted and destructive tones only when the action's meaning requires
  those semantic states.

## Control heights

- Use 32px buttons for compact toolbar, list-row and small inline actions.
- Use 36px buttons for ordinary form, footer, dialog and confirmation actions.
- Use 40px buttons for prominent wallet actions and for buttons paired with a
  40px field in the same control group.
- Use 40px fields for standard unlocked-wallet forms and right-sheet searches.
  Narrow sidebar or list filters may use 32px fields. Entry and onboarding
  screens may retain the shared Input's larger 44px default.
- Adjacent fields, buttons and select triggers in one control group should have
  equal heights. Avoid arbitrary intermediate heights unless a documented design
  source explicitly requires them. The wallet Overview's approved 34px
  search-and-sort toolbar remains such an exception.

## Back navigation

- Contextual back actions at the top-left of a page or nested right-sheet view
  use `NavigationBackButton`. It standardizes a 32px control height, 16px
  `ArrowLeft` icon, 13px normal-weight label, 6px icon gap, transparent hover,
  and a visible focus ring.
- Use a destination label such as “Back to settings” or “Back to search” when
  the destination is known. The containing page or sheet owns the outer inset
  and vertical spacing.
- Multi-step flows keep Back in their footer action group as a normal secondary
  button. Do not replace wizard footer navigation with the contextual control.

## Native cursor convention

All clickable actions use the normal arrow (`cursor: default`), including
buttons, wallet selectors, clickable cards, icon actions, and external links
(such as Discord). Do not use a hand cursor, even for links that open a browser.
Communicate interactivity through hover, pressed, and visible focus states.
Preserve text-entry, resize, and disabled-state cursors when appropriate.

The base styles and shared Button implement this convention. `pnpm lint:ui`
rejects pointer/hand cursor declarations and Tailwind utilities in `src/**`.

## Identifier typography and truncation

Raw machine-readable identifiers use the shared
`src/lib/components/common/IdentifierText.svelte` component. This includes
transparent and shielded addresses, identity i-addresses, transaction IDs,
contract addresses, and preflight IDs. Human-readable VerusIDs, contact names,
network names, badges, and prose stay in the regular interface font.

Use the component modes by context:

- `compact` shows the first and last 6 characters for dense lists, pickers,
  narrow summary rails, and secondary authority disclosures where the full value
  would add bulk without helping the decision.
- `review` shows the first and last 12 characters where a user is checking a
  transaction or consequential change.
- `full` never truncates and may wrap. Use it when the exact identifier is the
  primary detail being inspected. A compact identifier may still have a nearby
  copy action when the full value is available from the title and copy target.

Editable identifier inputs must never truncate; apply the shared
`identifier-text` class to the input instead. Copy actions must receive their
full source value directly, never the shortened display string. Keep suffixes
such as “(self),” network names, and explanatory text outside the identifier
component.

## Semantic tokens in `src/app.css`

| Token                            | Light     | Dark      | Purpose                                      |
| -------------------------------- | --------- | --------- | -------------------------------------------- |
| `app-canvas`                     | `#FBFBFB` | `#111111` | Entry screens and dark wallet shell surfaces |
| `sidebar-surface`                | `#EDEDED` | `#28282B` | Sidebar and aside surface                    |
| `sidebar-item-hover`             | `#E0E0E0` | `#36373B` | Sidebar hover state                          |
| `sidebar-item-pressed`           | `#D8D8D8` | `#323338` | Sidebar pressed state                        |
| `sidebar-item-active`            | `#E5E5E5` | `#303136` | Sidebar active state                         |
| `text-action`                    | `#3165D4` | `#89AEFF` | Standalone, non-destructive text actions     |
| `guard-revoke`                   | `#D4313E` | `#D4313E` | Guard revoke accent                          |
| `guard-recover`                  | `#4AA658` | `#4AA658` | Guard recover accent                         |
| `brand-discord`                  | `#5865F2` | `#5865F2` | Community hangout background                 |
| `brand-discord-foreground`       | `#4752C4` | `#7B86F8` | Community hangout text/icon color            |
| `brand-discord-foreground-hover` | `#3F49B7` | `#A0A8FF` | Community hangout hover text/icon color      |

## Approved literal registries

The wallet overview balance banner follows the approved Paper design at
`https://app.paper.design/file/01M2QVY623ETT6FS77D9T0RMDN`:
`wallet-balance-gradient-start` and `wallet-balance-gradient-end` define its
theme-specific blue gradient. `wallet-balance-foreground` and
`wallet-balance-muted` define the amount and secondary controls. The
`wallet-balance-edge` token is applied only to the bottom edge in light mode and
the top edge in dark mode. The banner keeps live totals and their loading,
partial-total, and hide/show states; Paper sample amounts are not app data.

- Wallet palette: `src/lib/constants/walletColors.ts`
- Identity avatar gradients: `src/lib/styles/identityAvatarGradients.ts`
- Core theme tokens and semantic UI tokens: `src/app.css`
- Enforcement allowlist: `scripts/ui-style-registry.mjs`

## Enforcement

- `pnpm lint:ui` scans `src/**` for unapproved hex literals and color functions.
- `pnpm lint:ui` also rejects native `input`, `textarea`, and `select` usage
  outside `src/lib/components/ui/**`.
- `pnpm lint:ui` rejects raw `<label>` usage in feature code so label styling
  stays centralized.
- `pnpm lint:ui` rejects direct static `identifier-text` classes so identifier
  display behavior stays centralized in `IdentifierText`. Editable `Input`
  fields remain the exception.
- `pnpm lint:ui` rejects the labeled `CopyActionButton` outside its legacy
  component directory. Use `CopyButton` instead.
- `pnpm lint:ui` rejects direct `ArrowLeft` and `ChevronLeft` icon imports
  outside `NavigationBackButton`, preventing new ad hoc contextual Back
  controls.
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
