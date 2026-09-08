#!/usr/bin/env node

import { access, readdir, readFile } from 'node:fs/promises';
import path from 'node:path';

const requiredPaths = [
  'docs/index.md',
  'docs/architecture/index.md',
  'docs/product-specs/index.md',
  'docs/product-specs/verus-pbaas-core-parity-matrix.md',
  'docs/product-specs/verus-pbaas-core-parity-fixtures.json',
  'docs/plans/index.md',
  'docs/context-packs/index.md',
  'docs/references/index.md',
];

const docsIndexRequiredLinks = [
  './architecture/index.md',
  './product-specs/index.md',
  './plans/index.md',
  './context-packs/index.md',
  './references/index.md',
];

const markdownRoots = ['docs'];
const extraMarkdownFiles = ['AGENTS.md', 'src/AGENTS.md', 'src-tauri/AGENTS.md'];

async function pathExists(targetPath) {
  try {
    await access(targetPath);
    return true;
  } catch {
    return false;
  }
}

async function collectMarkdownFiles(dir) {
  const out = [];
  const entries = await readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    const absolute = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      out.push(...(await collectMarkdownFiles(absolute)));
      continue;
    }
    if (entry.isFile() && absolute.endsWith('.md')) {
      out.push(absolute);
    }
  }
  return out;
}

function normalizeLinkTarget(rawTarget) {
  let target = rawTarget.trim();
  if (target.startsWith('<') && target.endsWith('>')) {
    target = target.slice(1, -1);
  }
  if (target.includes(' ')) {
    target = target.split(' ')[0];
  }
  return target;
}

async function validateLinks(markdownFile, errors) {
  const content = await readFile(markdownFile, 'utf8');
  validateDocMetadata(markdownFile, content, errors);
  const linkRegex = /\[[^\]]+\]\(([^)]+)\)/g;
  const linkTargets = [...content.matchAll(linkRegex)].map((match) =>
    normalizeLinkTarget(match[1])
  );

  for (const target of linkTargets) {
    if (!target || target.startsWith('#')) continue;
    if (
      target.startsWith('http://') ||
      target.startsWith('https://') ||
      target.startsWith('mailto:')
    ) {
      continue;
    }

    const [pathPart] = target.split('#');
    if (!pathPart) continue;

    const resolved = path.resolve(path.dirname(markdownFile), pathPart);
    if (await pathExists(resolved)) continue;
    if (await pathExists(`${resolved}.md`)) continue;

    errors.push(`${markdownFile}: broken link target "${target}"`);
  }
}

function validateDocMetadata(markdownFile, content, errors) {
  if (!markdownFile.startsWith(`docs${path.sep}`)) {
    return;
  }

  const lines = content.split('\n');
  if (lines[0]?.trim() !== '---') {
    errors.push(`${markdownFile}: missing metadata front matter (--- ... ---)`);
    return;
  }

  const closeIdxRelative = lines.slice(1).findIndex((line) => line.trim() === '---');
  if (closeIdxRelative === -1) {
    errors.push(`${markdownFile}: unterminated metadata front matter`);
    return;
  }

  const closeIdx = closeIdxRelative + 1;
  const meta = lines.slice(1, closeIdx).join('\n');

  if (!/^owner:\s*[a-z0-9._-]+\s*$/m.test(meta)) {
    errors.push(`${markdownFile}: metadata must include owner (example: owner: lite-wallet-team)`);
  }

  if (!/^last_reviewed:\s*\d{4}-\d{2}-\d{2}\s*$/m.test(meta)) {
    errors.push(`${markdownFile}: metadata must include last_reviewed in YYYY-MM-DD format`);
  }
}

async function main() {
  const errors = [];

  for (const p of requiredPaths) {
    if (!(await pathExists(p))) {
      errors.push(`Missing required path: ${p}`);
    }
  }

  if (await pathExists('AGENTS.md')) {
    const lines = (await readFile('AGENTS.md', 'utf8')).split('\n').length;
    if (lines > 180) {
      errors.push(`AGENTS.md too long (${lines} lines). Keep it a short map.`);
    }
  }

  if (await pathExists('docs/index.md')) {
    const docsIndex = await readFile('docs/index.md', 'utf8');
    for (const expected of docsIndexRequiredLinks) {
      if (!docsIndex.includes(expected)) {
        errors.push(`docs/index.md missing link: ${expected}`);
      }
    }
  }

  const markdownFiles = [];
  for (const root of markdownRoots) {
    if (await pathExists(root)) {
      markdownFiles.push(...(await collectMarkdownFiles(root)));
    }
  }
  markdownFiles.push(...extraMarkdownFiles.filter((p) => markdownFiles.indexOf(p) === -1));

  for (const mdFile of markdownFiles) {
    if (await pathExists(mdFile)) {
      await validateLinks(mdFile, errors);
    }
  }

  if (errors.length > 0) {
    console.error('docs:check failed:\n');
    for (const err of errors) {
      console.error(`- ${err}`);
    }
    process.exit(1);
  }

  console.info(`docs:check passed (${markdownFiles.length} markdown files validated).`);
}

await main();
