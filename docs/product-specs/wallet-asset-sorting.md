---
owner: lite-wallet-team
last_reviewed: 2026-09-19
---

# Wallet asset sorting and search

Implement the approved overview toolbar and ordering from
[Paper: Asset order & discovery](https://app.paper.design/file/01M2VMMRHYZ9QQZH7C86GGW944/1-0).
The latest user decision is authoritative: **search and sorting are always
visible, including a new wallet with four currencies**. There is no count or
overflow threshold. Use **Currency amount**, never “Token amount”.

## Behavior

- Default to **Verus first**: native Verus, Verus PRIVATE when enabled, other
  currencies hosted on Verus, native Ethereum and its currencies, then Bitcoin.
  Put other supported networks afterward. Sort additional currencies within each
  network group by name. Use canonical currency/network metadata rather than
  guessing from display names. Apply the equivalent priorities on testnet.
- A new mainnet wallet reads: Verus → USDC on Verus → Ethereum → Bitcoin.
  Default ordering must not change merely because balances or prices update.
- The menu has one selected sort and an independent **With balance** switch:

  | Menu label                   | Behavior                                                         |
  | ---------------------------- | ---------------------------------------------------------------- |
  | Verus first                  | The default network order above                                  |
  | Value: high to low           | Total holding value in the selected display currency, descending |
  | Name: A–Z                    | Localized display name, ascending                                |
  | Currency amount: high to low | Numeric currency units held, descending; not monetary worth      |

- Name, value, and currency amount sorts have a reverse-order button beside the
  current-sort menu. Name toggles A–Z / Z–A; value and amount toggle high-to-low
  / low-to-high. Labels reflect the current direction. A newly selected sort
  starts in its default direction. Verus first remains the fixed network order.
  This subsequent user decision extends the Paper toolbar without changing its
  other controls or geometry.
- Explicit value, name, or amount sorting applies to the entire list, without
  pinning Verus above the selected order. Break ties by name, then a stable row
  identifier. Compare numeric data, never formatted strings; put unavailable
  values after known values without treating them as zero.
- Search enabled rows by display name, ticker, and network, ignoring case and
  surrounding whitespace. Combine search and the balance filter.
- **With balance** defaults off. Hide only confirmed, loaded zero balances;
  retain pending, syncing, and unknown balances. Show its removable chip beside
  search when enabled, as in Paper. Removing the chip preserves search and sort.
- The query's clear control clears only the query. Show **No assets found** and
  **Clear search** when search has no matches. If only the balance filter leaves
  no rows, provide an action to clear that filter instead.
- The balance banner and its loading/partial state always use the full enabled
  wallet, before filtering. Search and filtering never change the wallet total,
  currency enablement, row destinations, or the existing hide-holdings behavior.
- Persist sort, direction, and balance-filter preferences per wallet and network
  using the existing preference infrastructure. Keep search transient and clear
  it when leaving the overview or switching wallets. Validate stored sort values
  and fall back to Verus first. Do not store holdings or queries with
  preferences.

## Paper fidelity

Read Paper through MCP, including `get_jsx` and `get_computed_styles` for exact
values; use screenshots for visual comparison. The numbered screens are the
design authority. The two “Reference · Previous wallet” boards are historical.

| State                             | Light node | Dark node |
| --------------------------------- | ---------- | --------- |
| 01 · New wallet                   | `AL-0`     | `EU-0`    |
| 02 · Larger wallet                | `N8-0`     | `R4-0`    |
| 03 · Sort and filter menu         | `16E-0`    | `1DX-0`   |
| 04 · Highest value / With balance | `1MY-0`    | `1X6-0`   |
| 05 · Search / Verus               | `27E-0`    | `2DZ-0`   |
| 06 · Search / No matches          | `2KK-0`    | `2P3-0`   |

At the **920 × 620** minimum window, retain the 244px sidebar and 612px content
width. The toolbar starts at x276/y176, is 36px high, and the list starts at
y224. The search field is 288 × 34px (256px wide with the active filter chip).
The menu is 264px wide with 6px padding, 34px option rows, and 10px corners. Use
flex layout and existing components rather than copying Paper's absolute canvas
positions.

Keep the banner, four main actions, and toolbar fixed while rows scroll using
the existing ScrollArea. Match both themes, Google Sans, restrained surfaces,
13px controls, Lucide icons, and the current-sort label in the trigger
(including **Highest value** for value sorting). Preserve keyboard access,
visible focus, accessible search/clear labels, radio selection semantics, and
arrow cursors. All new UI strings require English and Dutch translation keys.

## Implementation and acceptance

Start with `src/lib/components/wallet/sections/Overview.svelte` and
`src/lib/utils/walletOverview.ts`. Reuse current row aggregation and currency
metadata; do not introduce extra balance requests or change transaction logic.
Keep the change scoped to the overview controls, ordering/filtering helpers,
preferences, translations, and meaningful regression coverage.

Verify default and explicit sorts in both directions, stable ties, missing
rates/balances, filter composition, unchanged totals, and preference isolation.
Exercise menu, search, clear actions, and keyboard use. Compare all six states
with Paper in light and dark at 920 × 620, plus a larger window and Dutch copy.
Run focused tests, Svelte check, UI lint, formatting, and the relevant build
checks. Report actual rendered evidence separately from tests; browser fixtures
are not native wallet proof.
