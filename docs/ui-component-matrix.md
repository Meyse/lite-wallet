---
owner: lite-wallet-team
last_reviewed: 2026-03-07
---

# shadcn-svelte UI Component Matrix

This file converts the product spec into an implementation tracker for this
repository.

## Status Legend

- `Done`: Component exists in `src/lib/components/ui` and is already used in at
  least one wallet flow.
- `Available`: Component exists in `src/lib/components/ui` but is not
  consistently used in target flows yet.
- `In progress`: Pattern is actively used in wallet flows, but dedicated
  component adoption is still being completed.
- `Not started`: Component is not in `src/lib/components/ui` yet.

## Current Matrix

| Component              | Priority | Screen(s)                  | Status      | Owner    |
| ---------------------- | -------- | -------------------------- | ----------- | -------- |
| Button                 | Critical | Global                     | Done        | Frontend |
| Card                   | Critical | Dashboard, Send/Receive    | Done        | Frontend |
| Input                  | Critical | Send                       | Done        | Frontend |
| Dialog                 | Critical | Confirmations              | Available   | Frontend |
| Badge                  | Critical | Status indicators          | Done        | Frontend |
| Sidebar                | Critical | Wallet layout              | Done        | Frontend |
| Tabs                   | Critical | Wallet filters             | Not started | Frontend |
| Toast / Sonner         | Critical | Action feedback            | Done        | Frontend |
| Tooltip                | Critical | Context hints              | Done        | Frontend |
| Separator              | Critical | Visual hierarchy           | Available   | Frontend |
| Scroll Area            | Critical | Sidebar, lists             | Not started | Frontend |
| Avatar                 | High     | Wallet identity display    | Done        | Frontend |
| Collapsible            | High     | Sidebar nested groups      | Not started | Frontend |
| Alert                  | High     | Persistent warnings        | Not started | Frontend |
| Alert Dialog           | High     | Destructive confirmations  | Not started | Frontend |
| Select                 | High     | Coin/chain selection       | Not started | Frontend |
| Form + Label           | High     | Send/Import flows          | Done        | Frontend |
| Sheet                  | High     | Request/detail panels      | Done        | Frontend |
| Skeleton               | High     | Loading states             | Available   | Frontend |
| Dropdown Menu          | High     | Settings/actions           | Done        | Frontend |
| Switch                 | High     | Settings toggles           | Not started | Frontend |
| Breadcrumb             | High     | Top navigation context     | Not started | Frontend |
| Command                | Medium   | Global command palette     | Not started | Frontend |
| Checkbox               | Medium   | Confirmations              | Available   | Frontend |
| Radio Group            | Medium   | Mutually exclusive choices | Not started | Frontend |
| Combobox               | Medium   | Currency/address search    | Not started | Frontend |
| Data Table             | Medium   | Activity view              | Not started | Frontend |
| Pagination             | Medium   | Long transaction history   | Not started | Frontend |
| Hover Card             | Medium   | Rich hover previews        | Not started | Frontend |
| Toggle / Toggle Group  | Medium   | View modes and filters     | Not started | Frontend |
| Textarea               | Medium   | Seed phrase import         | Done        | Frontend |
| Context Menu           | Medium   | Power-user actions         | Not started | Frontend |
| Popover                | Medium   | Quick actions and filters  | Not started | Frontend |
| Progress               | Medium   | Sync and multi-step flows  | Not started | Frontend |
| Accordion              | Low      | Settings/help groups       | Done        | Frontend |
| Date Picker / Calendar | Low      | Activity date filters      | Not started | Frontend |
| Carousel               | Low      | Onboarding                 | Not started | Frontend |
| Drawer                 | Low      | Narrow-window/mobile UX    | In progress | Frontend |
| Resizable              | Low      | Sidebar width              | Not started | Frontend |
| Input OTP              | Low      | PIN lock                   | Not started | Frontend |
| Menubar                | Low      | Desktop-native menu        | Not started | Frontend |
| Slider                 | Low      | Fee tuning                 | Not started | Frontend |
| Aspect Ratio           | Low      | QR/image framing           | Not started | Frontend |

## Next Install Batch

Run these to add the next highest-value missing primitives:

```bash
yarn dlx shadcn-svelte@latest add tabs scroll-area alert alert-dialog select switch radio-group
yarn dlx shadcn-svelte@latest add breadcrumb command table pagination popover progress toggle toggle-group
```

## First Refactor Slice

- `src/routes/+layout.svelte`: add global `Toaster` provider.
- `src/lib/components/wallet/sections/Receive.svelte`: migrate copy feedback to
  toast and finish migrating field markup onto existing UI primitives.
