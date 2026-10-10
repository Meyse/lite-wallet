---
owner: lite-wallet-team
last_reviewed: 2026-10-10
---

# Request testing

This runbook tells you how to test requests from websites and apps on desktop
against VRSCTEST. The [requests spec](../product-specs/generic-requests.md)
defines the behaviour, and the
[plan](../plans/active/generic-requests-guided-flow.md) defines the phases.

Test on testnet only. The owner approves each broadcast and each identity
registration at the time.

## Harness

- **Daemon:** a local VRSCTEST `verusd` with RPC on port 18843. The harnesses
  read the daemon's own configuration and hold no RPC credentials.
- **Deep Link Sim:** `/Users/maxtheyse/dev/deeplink-sim-deps-update`, branch
  `codex/gr3-generators`. The API is on `127.0.0.1:8787` and the UI on
  `127.0.0.1:5173`.
- **Provisioning webhook:**
  `/Users/maxtheyse/dev/verus-fake-provisioning-webhook`. Its dashboard is on
  `localhost:3010`.

Start Deep Link Sim from its folder:

```sh
nvm use
corepack pnpm build:encryptor
corepack pnpm dev
```

The command-line generators need only the API. To start only the API, use
`corepack pnpm start`.

## Generators

Run each command in the Deep Link Sim folder as
`corepack pnpm --silent <command>`. Each command prints JSON with the `deeplink`
and the session `id`.

| Request                         | Command or tool                                                                            |
| ------------------------------- | ------------------------------------------------------------------------------------------ |
| Sign in                         | `create:sign-in`                                                                           |
| Sign in plus update identity    | `create:identity-update --target <identity@> --variant content-add --sign-in --disposable` |
| Update identity                 | `create:identity-update --target <identity@> --variant <variant> --disposable`             |
| New VerusID                     | Provisioning webhook dashboard                                                             |
| Pay invoice                     | `create:invoice --identity <i-address> --destination <address> --amount 0.01`              |
| Pay invoice without sign in     | Add `--invoice-only` to `create:invoice`                                                   |
| Sign data, Share data           | `create:data --identity <i-address> --preset <preset>`                                     |
| Encryption key                  | `create:app-encryption`                                                                    |
| Requests the wallet must reject | `create:negative --all`                                                                    |
| Gift card claim                 | Added in phase 5                                                                           |
| Legacy sign in and invoices     | Added in phase 6                                                                           |

Each generator accepts `--help`. The sign-in, update, encryption key, and
data-packet generators accept
`--delivery post|fail-once|fail-always|redirect|none`.

Read the result of a session from the API:

- sign in, update identity, encryption key, and rejected requests:
  `GET /api/request-lab/<id>`
- sign data and share data: `GET /api/data-lab/<id>`
- invoices: `GET /api/invoices/<id>`

Each session shows the behaviour that Verus Mobile has. Desktop must match its
protocol and its response, not its screens
([ADR 0003](../adr/0003-mobile-parity-scope.md)). For a sign-in response, the
receipt says which request ID the wallet sent: the sign-in detail ID or the
envelope ID. That choice is the open question GR3-P1 in the spec.

Approving any update identity request broadcasts a real VRSCTEST transaction.
Use only an identity that "mijn app" controls, and decline the high-risk
variants.

## Test wallet

Use the disposable "mijn app" wallet, only through the
[Keychain helper](./test-wallet-keychain.md). Do not automate a secret screen.
The helper needs macOS Accessibility permission for the app that runs it.

State on 2026-10-10, read from the native screens and the daemon:

- **Funds:** about 2.633 VRSCTEST.
- **Linked VerusIDs:** `game564498.gamesession2@`, `gamesession@`,
  `gamesession2@`, `player1@`, `player6@`, and `player7@`. Each has the wallet's
  own address as its only primary address, so the wallet controls all six.
- **Private Verus:** set up with a separate privacy secret. The wallet's
  shielded address is
  `zs13akhfdhypx9a8t2efkfgxpd5d9h3glz7d5pug2ys65mrzt4mw5vjqx0vw29gx82y3e0xjjuhrwn`.
  "Reuse primary Secret Recovery Phrase" is not possible, because the primary
  secret of "mijn app" is not a 24-word phrase.
- **Private address on an identity:** `player7@` has the wallet's shielded
  address (transaction
  `08dc8363d710607560f48471bbfc4ea88c34e22ab3d5e9e42f66b545fa759bd4`). Use
  `player7@` for credential and encryption key tests, and for update identity
  tests.

To give another identity the wallet's private address, use
`create:identity-update --variant high-risk-private-address --private-address <zs1…>`
and approve the request in the wallet.

## Read the native screens

To read the app's screens from a script, start it with the debug WebDriver
endpoint:

```sh
TAURI_WEBDRIVER_PORT=4445 pnpm tauri dev --features e2e-webdriver
```

The endpoint listens on `127.0.0.1` only, and a release build with this feature
does not compile. Unlock with the Keychain helper first. Then connect with the
pinned `webdriverio`, with `browserName: 'tauri'` and the window label `main`.

Do not open **Recovery and keys** or **Private Verus** in automation. When you
finish, stop the app and check that the port is closed.

## Paste a request

Use this path for daily work.

1. Start the wallet with `pnpm tauri dev`.
2. Unlock "mijn app" with the Keychain helper.
3. In the sidebar, select **Open request**.
4. Paste the `deeplink` value, then select **Open**.
5. Compare the screens with the spec.
6. Compare the receipt in Deep Link Sim with the expected response.

Today the wallet sends POST responses to `http` and `https` addresses in every
build, so the local receivers work. After phase 1, plain HTTP works only in
debug builds. Both `pnpm tauri dev` and the bundled debug build are debug
builds.

## Open a real verus:// link

`pnpm tauri dev` does not register the `verus` scheme with macOS. To test a real
link:

1. Build the bundle with `pnpm tauri build --debug`.
2. Open `src-tauri/target/debug/bundle/macos/Verus Express.app` once, so that
   macOS registers it.
3. Open the link with `open '<deeplink>'`.

Limits:

- The Keychain helper supports only `src-tauri/target/debug/verus_express`. The
  bundled app runs a copy at another path, so the owner unlocks the bundled app.
- Run one wallet build at a time. Both builds use the same app identifier and
  the same wallet data.

## Record evidence

For each run, record these as separate facts:

- the generator command and the session ID
- the screens that you saw
- the receipt from the harness
- each transaction ID, and its confirmation on-chain

The harness proves the response. It does not prove what the screens showed.
