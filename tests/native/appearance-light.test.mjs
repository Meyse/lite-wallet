import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync, mkdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { remote } from 'webdriverio';

// This live test intentionally leaves the app in light mode. No transactions,
// credential values, recovery screens, full DOM dumps, or automatic failure screenshots.
const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const fixtureName = 'mijn app';
const helper = resolve(repo, 'scripts/test-wallet-keychain.sh');
const metadataPath = resolve(
  homedir(),
  'Library/Application Support/com.maxtheyse.verus-express/wallet_data/mijn app_metadata.json'
);
const manualLogin = process.env.E2E_LOGIN === 'manual';
const port = Number(process.env.TAURI_WEBDRIVER_PORT || 4445);
const settingsButton =
  '//button[normalize-space(.)="Settings" or normalize-space(.)="Instellingen"]';
const displayButton =
  '//button[.//span[normalize-space(.)="Display and language" or normalize-space(.)="Weergave en taal"]]';
const lightButton = '[role="radiogroup"] [role="radio"]';

function fixture() {
  const value = JSON.parse(readFileSync(metadataPath, 'utf8'));
  assert.equal(value.network, 'testnet', 'Only the designated testnet fixture is allowed');
  assert.match(value.id, /^[0-9a-f-]{36}$/i, 'Fixture needs a valid account ID');
  return { id: value.id, lastUnlockedAt: value.last_unlocked_at };
}

function runHelper(command) {
  // The helper owns credential retrieval and native entry. Nothing secret is
  // passed through WebDriver, environment variables, arguments, or test logs.
  execFileSync(helper, [command], { cwd: repo, stdio: 'inherit', timeout: 90_000 });
}

async function openAppearance(browser) {
  const settings = await browser.$(settingsButton);
  await settings.waitForDisplayed();
  await settings.click();
  const display = await browser.$(displayButton);
  await display.waitForDisplayed();
  await display.click();
  await browser.$('[role="radiogroup"]').waitForDisplayed();
}

async function lightControl(browser) {
  const radios = await browser.$$(lightButton);
  for (const radio of radios) {
    if (['Light', 'Licht'].includes((await radio.getText()).trim())) return radio;
  }
  throw new Error('The light appearance control was not found');
}

async function assertLight(browser) {
  await browser.waitUntil(
    async () => {
      const state = await browser.execute(() => ({
        dark: document.documentElement.classList.contains('dark'),
        colorScheme: getComputedStyle(document.documentElement).colorScheme,
        saved: JSON.parse(localStorage.getItem('lite_wallet_settings_v1') || '{}').theme,
        mode: localStorage.getItem('mode-watcher-mode'),
      }));
      return (
        !state.dark &&
        state.colorScheme === 'light' &&
        state.saved === 'light' &&
        state.mode === 'light'
      );
    },
    { timeout: 15_000, timeoutMsg: 'Rendered and persisted appearance did not become light' }
  );
  assert.equal(await (await lightControl(browser)).getAttribute('aria-checked'), 'true');
}

test('mijn app: log in and persist light appearance', { timeout: 300_000 }, async (t) => {
  assert.equal(process.platform, 'darwin', 'This fixture uses the macOS Keychain helper');
  assert.ok(Number.isInteger(port) && port > 1024 && port <= 65535, 'Invalid local driver port');
  const before = fixture();
  if (!manualLogin) runHelper('status');

  const browser = await remote({
    hostname: '127.0.0.1',
    port,
    path: '/',
    capabilities: { browserName: 'tauri', 'wdio:tauriServiceOptions': { windowLabel: 'main' } },
    logLevel: 'silent',
    connectionRetryCount: 0,
    connectionRetryTimeout: 30_000,
    waitforTimeout: 20_000,
  });

  try {
    await browser.waitUntil(
      async () => new URL(await browser.getUrl()).origin === 'http://localhost:1420',
      { timeout: 30_000, timeoutMsg: 'The development webview did not load; check the Vite server' }
    );
    const password = await browser.$('#unlock-password');
    await password.waitForDisplayed({ timeoutMsg: 'Start the app on its locked login screen' });
    assert.equal(await password.getAttribute('type'), 'password');

    // Open the wallet selector when multiple profiles exist, then choose the
    // exact named fixture. A single-wallet screen is checked separately.
    const switcher = await browser.$('main button[aria-haspopup="dialog"]');
    if (await switcher.isExisting()) {
      await switcher.click();
      const choice = await browser.$('//button[.//p[normalize-space(.)="mijn app"]]');
      await choice.waitForDisplayed();
      await choice.click();
      await browser.waitUntil(async () => !(await browser.$('[role="dialog"]').isDisplayed()), {
        timeoutMsg: 'Wallet selector did not close',
      });
      assert.match(
        await (await browser.$('main button[aria-haspopup="dialog"]')).getAttribute('aria-label'),
        /: mijn app$/
      );
    } else {
      assert.equal(
        await browser.$('//main//span[normalize-space(.)="mijn app"]').getText(),
        fixtureName
      );
    }
    assert.equal(fixture().id, before.id, 'Fixture account changed during selection');
    t.diagnostic('Selected the mijn app testnet profile');

    if (manualLogin) {
      console.info(
        'Enter the password and unlock in the app; the test will continue automatically'
      );
    } else {
      // WebKit's accessibility tree can lag behind the DOM when the profile
      // drawer closes. Poll the unchanged, non-secret guard; never weaken it.
      await browser.waitUntil(
        () => {
          try {
            execFileSync(helper, ['preflight'], { cwd: repo, stdio: 'pipe', timeout: 5_000 });
            return true;
          } catch {
            return false;
          }
        },
        {
          timeout: 15_000,
          interval: 500,
          timeoutMsg: 'The guarded native login field did not become ready',
        }
      );
      runHelper('unlock');
    }
    await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname === '/wallet', {
      timeout: manualLogin ? 180_000 : 30_000,
      timeoutMsg: 'The normal login flow did not reach the wallet',
    });
    const after = fixture();
    assert.equal(after.id, before.id);
    assert.ok(
      after.lastUnlockedAt > (before.lastUnlockedAt || 0),
      'Backend did not record a new fixture unlock'
    );
    assert.equal(await browser.$('[data-sidebar="header"] p').getText(), fixtureName);
    t.diagnostic('Login verified by the dashboard and backend unlock timestamp');

    await openAppearance(browser);
    await (await lightControl(browser)).click();
    await assertLight(browser);
    t.diagnostic('Light selected; rendered color scheme and saved preference verified');

    await browser.refresh();
    await openAppearance(browser);
    await assertLight(browser);
    t.diagnostic('Light appearance survived a webview reload');

    if (process.env.E2E_ARTIFACT_DIR) {
      const artifacts = resolve(process.env.E2E_ARTIFACT_DIR);
      mkdirSync(artifacts, { recursive: true });
      // Only this verified settings page is captured, never login or recovery.
      await browser.saveScreenshot(resolve(artifacts, 'mijn-app-light-mode.png'));
    }
  } finally {
    await browser.deleteSession();
  }
});
