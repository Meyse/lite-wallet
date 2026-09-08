#!/usr/bin/env node

import { execFile as execFileCallback } from 'node:child_process';
import { copyFile, mkdir, readdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';
import vm from 'node:vm';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import * as acorn from 'acorn';
import { build } from 'esbuild';

const execFile = promisify(execFileCallback);
const require = createRequire(import.meta.url);

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, '..');

const sourceRoot = process.env.VALU_MOBILE_PATH ?? '/Users/maxtheyse/dev/valu-mobile';

const args = new Set(process.argv.slice(2));
const checkMode = args.has('--check');

const sourcePaths = {
  coinsList: path.join(sourceRoot, 'src/utils/CoinData/CoinsList.js'),
  coinData: path.join(sourceRoot, 'src/utils/CoinData/CoinData.js'),
  btcIndex: path.join(sourceRoot, 'src/images/cryptologo/default/btc/index.js'),
  web3Index: path.join(sourceRoot, 'src/images/cryptologo/default/web3/index.js'),
  pbaasIndex: path.join(sourceRoot, 'src/images/cryptologo/default/pbaas/index.js'),
  fiatIndex: path.join(sourceRoot, 'src/images/cryptologo/default/fiat/index.js'),
};

const outputPaths = {
  catalog: path.join(repoRoot, 'src/lib/coins/verusCoinCatalog.generated.json'),
  meta: path.join(repoRoot, 'src/lib/coins/verusCoinCatalog.meta.json'),
  iconRoot: path.join(repoRoot, 'static/images/coin-logos'),
};

function toJson(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function getObjectKey(node) {
  if (!node) return null;
  if (node.type === 'Identifier') return node.name;
  if (node.type === 'Literal') return String(node.value);
  return null;
}

function getMemberPath(node) {
  if (!node) return null;
  if (node.type === 'Identifier') return [node.name];
  if (node.type === 'MemberExpression') {
    const objectPath = getMemberPath(node.object);
    if (!objectPath) return null;

    let propertyPart = null;
    if (node.computed) {
      if (node.property.type === 'Literal') {
        propertyPart = String(node.property.value);
      }
    } else if (node.property.type === 'Identifier') {
      propertyPart = node.property.name;
    }

    if (!propertyPart) return null;
    return [...objectPath, propertyPart];
  }

  return null;
}

function parseAst(code, filePath) {
  return acorn.parse(code, {
    ecmaVersion: 'latest',
    sourceType: 'module',
    allowHashBang: true,
    locations: false,
    sourceFile: filePath,
  });
}

function findExportedConstObject(ast, name) {
  for (const node of ast.body) {
    if (node.type !== 'ExportNamedDeclaration' || !node.declaration) continue;
    if (node.declaration.type !== 'VariableDeclaration') continue;

    for (const declaration of node.declaration.declarations) {
      if (declaration.id?.type !== 'Identifier' || declaration.id.name !== name) continue;
      if (!declaration.init || declaration.init.type !== 'ObjectExpression') {
        throw new Error(`${name} is not an object literal`);
      }
      return declaration.init;
    }
  }

  throw new Error(`Unable to find exported object ${name}`);
}

function parseCoinLogoRefs(coinDataCode) {
  const ast = parseAst(coinDataCode, sourcePaths.coinData);
  const coinLogosObject = findExportedConstObject(ast, 'CoinLogos');
  const refs = new Map();

  for (const property of coinLogosObject.properties) {
    if (property.type !== 'Property') continue;

    const key = getObjectKey(property.key);
    if (!key) continue;

    const memberPath = getMemberPath(property.value);
    if (!memberPath || memberPath.length !== 3 || memberPath[0] !== 'CoinLogoIcons') {
      continue;
    }

    refs.set(key, {
      family: memberPath[1],
      symbol: memberPath[2],
    });
  }

  return refs;
}

function buildImportMap(code) {
  const importMap = new Map();
  const importRegex = /import\s+([A-Za-z0-9_]+)\s+from\s+['"]([^'"]+)['"]/g;
  let match = importRegex.exec(code);
  while (match) {
    importMap.set(match[1], match[2]);
    match = importRegex.exec(code);
  }
  return importMap;
}

async function parseFamilyAssetIndex(indexPath) {
  const code = await readFile(indexPath, 'utf8');
  const importMap = buildImportMap(code);
  const familyAssets = new Map();

  const assetEntryRegex =
    /([A-Za-z0-9_]+)\s*:\s*\{\s*light\s*:\s*([A-Za-z0-9_]+)\s*,\s*dark\s*:\s*([A-Za-z0-9_]+)\s*\}/g;

  let match = assetEntryRegex.exec(code);
  while (match) {
    const symbol = match[1];
    const lightImport = importMap.get(match[2]);
    const darkImport = importMap.get(match[3]);

    if (lightImport && darkImport) {
      familyAssets.set(symbol, {
        lightAbs: path.resolve(path.dirname(indexPath), lightImport),
        darkAbs: path.resolve(path.dirname(indexPath), darkImport),
      });
    }

    match = assetEntryRegex.exec(code);
  }

  return familyAssets;
}

async function parseFiatSymbolIndex(indexPath) {
  const code = await readFile(indexPath, 'utf8');
  const fiatSymbols = new Map();
  const fiatRegex = /([A-Za-z0-9_]+)\s*:\s*RenderFiatCoinLogo\((['"])(.*?)\2\)/g;

  let match = fiatRegex.exec(code);
  while (match) {
    fiatSymbols.set(match[1], match[3]);
    match = fiatRegex.exec(code);
  }

  return fiatSymbols;
}

async function loadCoinsList() {
  const bundle = await build({
    entryPoints: [sourcePaths.coinsList],
    bundle: true,
    write: false,
    platform: 'node',
    format: 'cjs',
    logLevel: 'silent',
  });

  const output = bundle.outputFiles?.[0]?.text;
  if (!output) {
    throw new Error('Failed to bundle valu-mobile CoinsList.js');
  }

  const module = { exports: {} };
  const sandbox = {
    module,
    exports: module.exports,
    require,
    process,
    console,
  };

  vm.runInNewContext(output, sandbox, { filename: 'coinslist.bundle.cjs' });

  const coinsList = sandbox.module.exports?.coinsList ?? sandbox.exports?.coinsList;

  if (!coinsList || typeof coinsList !== 'object') {
    throw new Error('Bundled CoinsList.js did not export coinsList object');
  }

  return coinsList;
}

async function getGitInfo(repoPath) {
  const runGit = async (gitArgs) => {
    const { stdout } = await execFile('git', ['-C', repoPath, ...gitArgs]);
    return stdout.trim();
  };

  try {
    const [branch, commit, commitDate] = await Promise.all([
      runGit(['rev-parse', '--abbrev-ref', 'HEAD']),
      runGit(['rev-parse', 'HEAD']),
      runGit(['log', '-1', '--date=iso-strict', '--pretty=%cI']),
    ]);

    return { branch, commit, commitDate };
  } catch {
    return {
      branch: 'unknown',
      commit: 'unknown',
      commitDate: null,
    };
  }
}

async function assertReadable(targetPath) {
  await stat(targetPath);
}

function normalizeCoin(rawCoin) {
  const coinPaprikaIdRaw =
    rawCoin?.rate_url_params && typeof rawCoin.rate_url_params === 'object'
      ? rawCoin.rate_url_params.coin_paprika
      : null;
  const coinPaprikaId =
    typeof coinPaprikaIdRaw === 'string' && coinPaprikaIdRaw.trim().length > 0
      ? coinPaprikaIdRaw.trim()
      : null;

  return {
    id: String(rawCoin.id ?? ''),
    currencyId: String(rawCoin.currency_id ?? ''),
    systemId: String(rawCoin.system_id ?? ''),
    displayTicker: String(rawCoin.display_ticker ?? ''),
    displayName: String(rawCoin.display_name ?? ''),
    ...(coinPaprikaId ? { coinPaprikaId } : {}),
    proto: String(rawCoin.proto ?? ''),
    mappedTo: rawCoin.mapped_to == null ? null : String(rawCoin.mapped_to),
    isTestnet: Boolean(rawCoin.testnet),
  };
}

async function listFilesRecursively(rootDir) {
  const results = [];

  async function walk(currentDir) {
    let entries = [];
    try {
      entries = await readdir(currentDir, { withFileTypes: true });
    } catch {
      return;
    }

    for (const entry of entries) {
      const absolute = path.join(currentDir, entry.name);
      if (entry.isDirectory()) {
        await walk(absolute);
      } else if (entry.isFile()) {
        results.push(absolute);
      }
    }
  }

  await walk(rootDir);
  return results;
}

function toPosixRelative(basePath, absolutePath) {
  return path.relative(basePath, absolutePath).split(path.sep).join('/');
}

async function generateCatalogPayload() {
  await Promise.all(Object.values(sourcePaths).map((targetPath) => assertReadable(targetPath)));

  const [coinsList, coinDataCode, btcAssets, web3Assets, pbaasAssets, fiatSymbols, gitInfo] =
    await Promise.all([
      loadCoinsList(),
      readFile(sourcePaths.coinData, 'utf8'),
      parseFamilyAssetIndex(sourcePaths.btcIndex),
      parseFamilyAssetIndex(sourcePaths.web3Index),
      parseFamilyAssetIndex(sourcePaths.pbaasIndex),
      parseFiatSymbolIndex(sourcePaths.fiatIndex),
      getGitInfo(sourceRoot),
    ]);

  const coinLogoRefs = parseCoinLogoRefs(coinDataCode);
  const assetFamilies = new Map([
    ['btc', btcAssets],
    ['web3', web3Assets],
    ['pbaas', pbaasAssets],
  ]);

  const coins = Object.values(coinsList)
    .map((coin) => normalizeCoin(coin))
    .sort((a, b) => a.id.localeCompare(b.id));

  const assetCopies = new Map();
  const catalog = [];

  for (const coin of coins) {
    const logoRef = coinLogoRefs.get(coin.id) ?? null;

    let icon = {
      kind: 'generated',
      logoMapped: false,
    };

    if (logoRef) {
      if (logoRef.family === 'fiat') {
        icon = {
          kind: 'fiat-symbol',
          symbol: fiatSymbols.get(logoRef.symbol) ?? logoRef.symbol,
          logoMapped: true,
          family: logoRef.family,
          symbolKey: logoRef.symbol,
        };
      } else {
        const familyAssets = assetFamilies.get(logoRef.family);
        const symbolAssets = familyAssets?.get(logoRef.symbol);

        if (symbolAssets) {
          const lightTargetRel = path.posix.join('images', 'coin-logos', logoRef.family, path.basename(symbolAssets.lightAbs));
          const darkTargetRel = path.posix.join('images', 'coin-logos', logoRef.family, path.basename(symbolAssets.darkAbs));

          const lightTargetAbs = path.join(repoRoot, 'static', ...lightTargetRel.split('/'));
          const darkTargetAbs = path.join(repoRoot, 'static', ...darkTargetRel.split('/'));

          assetCopies.set(lightTargetAbs, symbolAssets.lightAbs);
          assetCopies.set(darkTargetAbs, symbolAssets.darkAbs);

          icon = {
            kind: 'asset',
            light: `/${lightTargetRel}`,
            dark: `/${darkTargetRel}`,
            logoMapped: true,
            family: logoRef.family,
            symbolKey: logoRef.symbol,
          };
        } else {
          icon = {
            kind: 'generated',
            logoMapped: true,
            family: logoRef.family,
            symbolKey: logoRef.symbol,
          };
        }
      }
    }

    catalog.push({
      ...coin,
      icon,
    });
  }

  const counts = {
    totalCoins: catalog.length,
    directLogoHits: catalog.filter((entry) => entry.icon.logoMapped).length,
    generatedIcons: catalog.filter((entry) => entry.icon.kind === 'generated').length,
    assetIcons: catalog.filter((entry) => entry.icon.kind === 'asset').length,
    fiatSymbolIcons: catalog.filter((entry) => entry.icon.kind === 'fiat-symbol').length,
  };

  const metadata = {
    generatedAt: gitInfo.commitDate ?? null,
    source: {
      repoPath: sourceRoot,
      branch: gitInfo.branch,
      commit: gitInfo.commit,
      commitDate: gitInfo.commitDate,
    },
    counts,
    output: {
      catalogPath: toPosixRelative(repoRoot, outputPaths.catalog),
      iconRoot: toPosixRelative(repoRoot, outputPaths.iconRoot),
    },
  };

  return {
    catalog,
    metadata,
    assetCopies,
    counts,
  };
}

async function writeOutputs(payload) {
  await mkdir(path.dirname(outputPaths.catalog), { recursive: true });
  await writeFile(outputPaths.catalog, toJson(payload.catalog), 'utf8');
  await writeFile(outputPaths.meta, toJson(payload.metadata), 'utf8');

  await rm(outputPaths.iconRoot, { recursive: true, force: true });
  await mkdir(outputPaths.iconRoot, { recursive: true });

  const sortedTargets = [...payload.assetCopies.entries()].sort(([a], [b]) => a.localeCompare(b));

  for (const [targetAbs, sourceAbs] of sortedTargets) {
    await mkdir(path.dirname(targetAbs), { recursive: true });
    await copyFile(sourceAbs, targetAbs);
  }
}

async function checkOutputs(payload) {
  const errors = [];

  const expectedCatalog = toJson(payload.catalog);
  const expectedMeta = toJson(payload.metadata);

  const existingCatalog = await readFile(outputPaths.catalog, 'utf8').catch(() => null);
  const existingMeta = await readFile(outputPaths.meta, 'utf8').catch(() => null);

  if (existingCatalog !== expectedCatalog) {
    errors.push(`Catalog out of date: ${toPosixRelative(repoRoot, outputPaths.catalog)}`);
  }

  if (existingMeta !== expectedMeta) {
    errors.push(`Metadata out of date: ${toPosixRelative(repoRoot, outputPaths.meta)}`);
  }

  const expectedTargetSet = new Set(
    [...payload.assetCopies.keys()].map((absolutePath) => toPosixRelative(outputPaths.iconRoot, absolutePath))
  );

  const existingAssetFiles = await listFilesRecursively(outputPaths.iconRoot);
  const existingTargetSet = new Set(
    existingAssetFiles.map((absolutePath) => toPosixRelative(outputPaths.iconRoot, absolutePath))
  );

  for (const targetRel of expectedTargetSet) {
    if (!existingTargetSet.has(targetRel)) {
      errors.push(`Missing icon asset: static/images/coin-logos/${targetRel}`);
    }
  }

  for (const targetRel of existingTargetSet) {
    if (!expectedTargetSet.has(targetRel)) {
      errors.push(`Unexpected icon asset: static/images/coin-logos/${targetRel}`);
    }
  }

  const sortedTargets = [...payload.assetCopies.entries()].sort(([a], [b]) => a.localeCompare(b));
  for (const [targetAbs, sourceAbs] of sortedTargets) {
    const [sourceBuffer, targetBuffer] = await Promise.all([
      readFile(sourceAbs),
      readFile(targetAbs).catch(() => null),
    ]);

    if (!targetBuffer) continue;

    if (!sourceBuffer.equals(targetBuffer)) {
      errors.push(`Changed icon asset content: ${toPosixRelative(repoRoot, targetAbs)}`);
    }
  }

  if (errors.length > 0) {
    throw new Error(errors.join('\n'));
  }
}

async function main() {
  const payload = await generateCatalogPayload();

  if (checkMode) {
    await checkOutputs(payload);
    console.info(
      `verus coin catalog up-to-date (${payload.counts.totalCoins} coins, ${payload.counts.directLogoHits} direct logo hits, ${payload.counts.generatedIcons} generated fallbacks).`
    );
    return;
  }

  await writeOutputs(payload);
  console.info(
    `verus coin catalog synced (${payload.counts.totalCoins} coins, ${payload.counts.directLogoHits} direct logo hits, ${payload.counts.generatedIcons} generated fallbacks).`
  );
}

await main();
