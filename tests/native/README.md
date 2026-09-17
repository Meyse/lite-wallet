# Native appearance smoke test

Runs against the real macOS Tauri development app, using WebdriverIO's
standalone client and the embedded WebDriver plugin. No mock backend or OCR is
involved.

The sole allowed fixture is the disposable **mijn app** testnet wallet. The test
uses the existing
[guarded Keychain helper](../../docs/references/test-wallet-keychain.md), never
a password in JavaScript, arguments, environment variables, logs, or files. The
helper must already be configured for this exact checkout and account.

## Run

Use the repository-pinned Node and pnpm versions. Stop any existing Tauri dev
session before starting the test-enabled one (single-instance protection remains
on).

```sh
pnpm dev:e2e:appearance
```

In another terminal:

```sh
pnpm test:e2e:appearance
```

Start on the locked login screen. The test selects mijn app, unlocks through the
normal UI, verifies the named dashboard and the backend unlock timestamp, opens
Settings > Display and language, selects Light, checks the rendered color scheme
and both persisted appearance values, reloads the webview, and checks again.
English and Dutch labels are supported. It intentionally leaves appearance
light. Appearance is an app-wide preference, not a setting isolated to this
profile.

For manual password entry, use `E2E_LOGIN=manual pnpm test:e2e:appearance` and
unlock in the app when prompted. The test never reads the input's value. An
optional `E2E_ARTIFACT_DIR` saves a screenshot only after the settings-page
assertions pass. There are no automatic screenshots or DOM dumps on failure.

## Test endpoint boundary

The pinned plugin binds to 127.0.0.1. Registration requires both the explicit
`e2e-webdriver` Cargo feature and `TAURI_WEBDRIVER_PORT`. Release builds with
the feature are rejected at compile time. Normal builds have no WebDriver
endpoint. This endpoint permits UI/script automation; close the test-enabled app
when done and restart with `pnpm tauri dev`. Do not open other wallets in the
test-enabled app. The test client deletes its WebDriver session but does not
stop the app/server.

The integration adds only the embedded driver; it does not enable global Tauri
APIs, IPC mocking, or the separate WDIO backend-execution plugin. Edge/Firefox
driver postinstall downloads are denied because this test uses neither browser.

This smoke test proves the exercised login and appearance behavior, not wallet
signing, transaction submission, native dialogs, or other app flows.
