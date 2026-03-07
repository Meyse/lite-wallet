#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { access, readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, '..');

const requiredBaseIds = ['VRSC', 'VRSCTEST', 'BTC', 'ETH', 'USDC'];

const expectedCounts = {
  totalCoins: 87,
  directLogoHits: 85,
  generatedIcons: 2,
};

async function ensureFileExists(filePath) {
  await access(filePath);
}

function fail(message) {
  console.error(`check:verus-coins failed: ${message}`);
  process.exit(1);
}

function runSyncCheck() {
  const syncScriptPath = path.join(repoRoot, 'scripts/sync-verus-coin-catalog.mjs');
  const result = spawnSync(process.execPath, [syncScriptPath, '--check'], {
    cwd: repoRoot,
    stdio: 'pipe',
    env: process.env,
    encoding: 'utf8',
  });

  if (result.status !== 0) {
    const stderr = result.stderr?.trim();
    const stdout = result.stdout?.trim();
    const details = [stdout, stderr].filter(Boolean).join('\n');
    fail(`catalog drift detected.\n${details}`);
  }
}

function validateCoinEntry(entry) {
  if (!entry || typeof entry !== 'object') {
    throw new Error('Invalid catalog entry object');
  }

  if (!entry.id || typeof entry.id !== 'string') {
    throw new Error('Entry missing string id');
  }

  if (!entry.displayName || typeof entry.displayName !== 'string') {
    throw new Error(`Entry ${entry.id} missing displayName`);
  }

  if (!entry.displayTicker || typeof entry.displayTicker !== 'string') {
    throw new Error(`Entry ${entry.id} missing displayTicker`);
  }

  if (
    Object.prototype.hasOwnProperty.call(entry, 'coinPaprikaId') &&
    entry.coinPaprikaId != null &&
    typeof entry.coinPaprikaId !== 'string'
  ) {
    throw new Error(`Entry ${entry.id} has non-string coinPaprikaId`);
  }

  if (!entry.icon || typeof entry.icon !== 'object') {
    throw new Error(`Entry ${entry.id} missing icon`);
  }

  const kind = entry.icon.kind;
  if (!['asset', 'fiat-symbol', 'generated'].includes(kind)) {
    throw new Error(`Entry ${entry.id} has unsupported icon kind ${String(kind)}`);
  }
}

async function validateCatalog(catalogPath, metaPath) {
  const [catalogRaw, metaRaw] = await Promise.all([
    readFile(catalogPath, 'utf8'),
    readFile(metaPath, 'utf8'),
  ]);

  const catalog = JSON.parse(catalogRaw);
  const meta = JSON.parse(metaRaw);

  if (!Array.isArray(catalog)) {
    throw new Error('Catalog JSON is not an array');
  }

  for (const entry of catalog) {
    validateCoinEntry(entry);
  }

  for (const requiredId of requiredBaseIds) {
    if (!catalog.some((entry) => entry.id === requiredId)) {
      throw new Error(`Catalog missing required coin id ${requiredId}`);
    }
  }

  const directLogoHits = catalog.filter((entry) => entry.icon.logoMapped === true).length;
  const generatedIcons = catalog.filter((entry) => entry.icon.kind === 'generated').length;
  const totalCoins = catalog.length;

  if (totalCoins !== expectedCounts.totalCoins) {
    throw new Error(
      `Unexpected totalCoins count: expected ${expectedCounts.totalCoins}, got ${totalCoins}`
    );
  }

  if (directLogoHits !== expectedCounts.directLogoHits) {
    throw new Error(
      `Unexpected directLogoHits count: expected ${expectedCounts.directLogoHits}, got ${directLogoHits}`
    );
  }

  if (generatedIcons !== expectedCounts.generatedIcons) {
    throw new Error(
      `Unexpected generatedIcons count: expected ${expectedCounts.generatedIcons}, got ${generatedIcons}`
    );
  }

  if (!meta?.counts || typeof meta.counts !== 'object') {
    throw new Error('Metadata is missing counts section');
  }

  if (meta.counts.totalCoins !== totalCoins) {
    throw new Error('Metadata totalCoins does not match catalog total');
  }

  if (meta.counts.directLogoHits !== directLogoHits) {
    throw new Error('Metadata directLogoHits does not match catalog value');
  }

  if (meta.counts.generatedIcons !== generatedIcons) {
    throw new Error('Metadata generatedIcons does not match catalog value');
  }

  for (const entry of catalog) {
    if (entry.icon.kind !== 'asset') continue;

    const light = entry.icon.light;
    const dark = entry.icon.dark;

    if (typeof light !== 'string' || typeof dark !== 'string') {
      throw new Error(`Asset icon for ${entry.id} is missing light/dark paths`);
    }

    if (!light.startsWith('/images/coin-logos/') || !dark.startsWith('/images/coin-logos/')) {
      throw new Error(`Asset icon for ${entry.id} has unexpected path prefix`);
    }

    const lightAbs = path.join(repoRoot, 'static', light.replace(/^\//, ''));
    const darkAbs = path.join(repoRoot, 'static', dark.replace(/^\//, ''));

    await ensureFileExists(lightAbs);
    await ensureFileExists(darkAbs);
  }

  return {
    totalCoins,
    directLogoHits,
    generatedIcons,
  };
}

async function main() {
  runSyncCheck();

  const catalogPath = path.join(repoRoot, 'src/lib/coins/verusCoinCatalog.generated.json');
  const metaPath = path.join(repoRoot, 'src/lib/coins/verusCoinCatalog.meta.json');

  const stats = await validateCatalog(catalogPath, metaPath);

  console.info(
    `check:verus-coins passed (${stats.totalCoins} coins, ${stats.directLogoHits} direct logo hits, ${stats.generatedIcons} generated fallbacks).`
  );
}

await main();
