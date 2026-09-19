# Browser fixtures

These development-only pages mount production components against synthetic Tauri
responses. They do not connect to a real wallet or submit transactions. Use the
Node and pnpm versions pinned in `package.json`, then run from the repository
root:

```sh
pnpm exec vite --config dev/browser-fixtures/vite.config.js
```

Open either page on `http://127.0.0.1:1428`:

- `/dev/browser-fixtures/contacts.html`: isolated Contacts and Send states,
  profile loading, long content, saving, and retry. See the
  [Contacts verification notes](../../docs/references/contacts-implementation.md).
- `/dev/browser-fixtures/transfer-navigation.html`: the production wallet shell
  and retained Send/Convert drafts, including profile-to-Contacts navigation.
  See the
  [transfer verification notes](../../docs/references/transfer-navigation-verification.md).

Both accept `?theme=dark&locale=nl`; the default is English and light mode.
Signing is rejected by default. `simulateReceipt=1` enables a synthetic receipt
without signing or network submission.

Save screenshots, generated assets, and run reports under the ignored `output/`
directory. Keep reusable fixture code here and durable findings in `docs/`.
Documentation must not require local output files to exist.
