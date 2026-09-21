# Browser fixtures

These development-only pages mount production components against synthetic Tauri
responses. They do not connect to a real wallet or submit transactions. Use the
Node and pnpm versions pinned in `package.json`, then run from the repository
root:

```sh
pnpm exec vite --config dev/browser-fixtures/vite.config.js
```

Open either page on `http://127.0.0.1:1428`:

- `/dev/browser-fixtures/profile.html`: selected Paper 05 profile, using
  production components and synthetic public data. Add `?rich&header`, `&saved`,
  `&linked`, `&long`, `&detailsError`, `&unavailable`, or `&failOnce` for the
  corresponding states. Rich content is a UI contract fixture, not evidence of
  working claim readers. `?editor` opens the profile draft overview with empty
  synthetic profile data. Staged changes can reach a synthetic review with a
  fixed fixture fee; `?editor&split` shows the two-update quote. Publish returns
  a synthetic receipt without signing or network submission. A development-only
  command adapter also allows this page in an isolated native Tauri fixture:
  profile/fee commands stay synthetic while image encoding uses the real Rust
  encoder. Do not treat fixture costs or partitioning as daemon evidence.
  `?owner` opens the linked-owner profile with Edit profile and unlink controls.
- `/dev/browser-fixtures/contacts.html`: isolated Contacts and Send states,
  profile loading, long content, saving, and retry. See the
  [Contacts verification notes](../../docs/references/contacts-implementation.md).
- `/dev/browser-fixtures/transfer-navigation.html`: the production wallet shell
  and retained Send/Convert drafts, including profile-to-Contacts navigation.
  See the
  [transfer verification notes](../../docs/references/transfer-navigation-verification.md).
- `/dev/browser-fixtures/manage-assets.html`: asset management at 920×620 with a
  custom UNI registration that overlaps the catalog. `?stall=uni` holds one
  token balance; `?stall=discovery` holds Verus discovery. Other rows remain
  usable. The normal deadlines lead to Unavailable and Retry; Retry succeeds.
  Both themes and locales are supported. All responses are synthetic and no
  wallet is accessed.

Both accept `?theme=dark&locale=nl`; the default is English and light mode.
Outside the profile publication fixture, signing is rejected by default.
`simulateReceipt=1` enables a synthetic receipt without signing or network
submission.

Save screenshots, generated assets, and run reports under the ignored `output/`
directory. Keep reusable fixture code here and durable findings in `docs/`.
Documentation must not require local output files to exist.

## Profile publication corrections

`profile.html?editor` renders the real detail/editor composition against
synthetic publication services. Add `draft` for all three staged fields, `split`
for two updates and a beneficial comparison, or `published&removal` for
removals. The page supports `theme=dark` and `locale=nl`.

Progress fixtures use `state=single-pending`, `state=first-pending`,
`state=header-ready`, `state=header-pending` or `state=recovery`. Add
`feeChanged`, `expired`, `insufficient`, `refreshFail`, `replanFail`,
`ambiguous` or `confirmed` to exercise their respective transitions. These
profile fixtures return synthetic receipts after Publish; they cannot sign or
broadcast. Other fixture signing policies above are unchanged. Browser
file-input attachment does not exercise the native OS chooser. Fee grouping,
image quality and real confirmation still require the native VRSCTEST acceptance
run.
