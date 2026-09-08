import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  approvedUiColorFunctions,
  approvedUiColorFunctionsBySource,
  approvedUiHexColors,
  approvedUiHexColorsBySource,
  colorFunctionTokenFiles,
  governedFormPrimitives,
  governedNativeControls,
  nativeControlUiRoot,
  normalizeColorFunction,
  normalizeHexColor,
  scanExtensions,
  scanRoots,
} from './ui-style-registry.mjs';

const HEX_COLOR_PATTERN = /(?<![\w-])#(?:[0-9A-Fa-f]{3,4}|[0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})(?![\w-])/g;
const COLOR_FUNCTION_PATTERN = /\b(?:rgb|rgba|hsl|hsla|oklch|oklab|lab|lch)\([^)]*\)/gi;
const NATIVE_CONTROL_PATTERN = new RegExp(`<(${governedNativeControls.join('|')})\\b`, 'g');
const FORM_PRIMITIVE_PATTERN = new RegExp(`<(${governedFormPrimitives.join('|')})\\b`, 'g');

const HAND_CURSOR_PATTERN = /\bcursor-(?:pointer\b|\[(?:pointer|hand)\])|\bcursor\s*:\s*['"]?(?:pointer|hand)\b/g;

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const violations = [];

for (const root of scanRoots) {
  const rootPath = path.join(projectRoot, root);
  if (!fs.existsSync(rootPath)) continue;
  walk(rootPath);
}

if (violations.length > 0) {
  console.error(`UI style lint failed with ${violations.length} violation(s).\n`);
  for (const violation of violations) {
    console.error(
      `${violation.rule} ${violation.file}:${violation.line}:${violation.column} ${violation.value}`
    );
    console.error(`  ${violation.message}`);
    console.error(`  ${violation.snippet}`);
  }
  console.error(
    '\nAdd a new color to scripts/ui-style-registry.mjs only if it is now part of the product style.'
  );
  process.exitCode = 1;
} else {
  console.info('UI style lint passed.');
}

function walk(currentPath) {
  const stats = fs.statSync(currentPath);
  if (stats.isDirectory()) {
    for (const entry of fs.readdirSync(currentPath)) {
      walk(path.join(currentPath, entry));
    }
    return;
  }

  if (!scanExtensions.has(path.extname(currentPath))) return;

  const relativePath = toProjectPath(currentPath);
  const source = fs.readFileSync(currentPath, 'utf8');
  const lineStarts = getLineStarts(source);

  for (const match of source.matchAll(HAND_CURSOR_PATTERN)) {
    pushViolation({
      file: relativePath,
      lineStarts,
      source,
      index: match.index ?? 0,
      value: match[0],
      defaultRule: 'ui/native-arrow-cursor',
      message: 'Use cursor: default for clickable actions, including external links. See docs/ui-style-governance.md.',
    });
  }

  if (path.extname(currentPath) === '.svelte' && !relativePath.startsWith(nativeControlUiRoot)) {
    for (const match of source.matchAll(NATIVE_CONTROL_PATTERN)) {
      const tagName = match[1];
      pushViolation({
        file: relativePath,
        lineStarts,
        source,
        index: match.index ?? 0,
        value: `<${tagName}>`,
        defaultRule: 'ui/native-control-outside-ui-layer',
        message: `Use a local UI primitive for <${tagName}>. Add or extend a wrapper in src/lib/components/ui before using the native control in feature code.`,
      });
    }

    for (const match of source.matchAll(FORM_PRIMITIVE_PATTERN)) {
      const tagName = match[1];
      pushViolation({
        file: relativePath,
        lineStarts,
        source,
        index: match.index ?? 0,
        value: `<${tagName}>`,
        defaultRule: 'ui/form-primitive-outside-ui-layer',
        message: `Use the local ${tagName === 'label' ? 'Label' : tagName} primitive from src/lib/components/ui instead of a raw <${tagName}> in feature code.`,
      });
    }
  }

  for (const match of source.matchAll(HEX_COLOR_PATTERN)) {
    const value = match[0];
    const normalized = normalizeHexColor(value);
    if (approvedUiHexColors.has(normalized)) {
      if (sourceAllowsHex(relativePath, normalized)) continue;
      pushViolation({
        file: relativePath,
        lineStarts,
        source,
        index: match.index ?? 0,
        value,
        defaultRule: 'ui/approved-color-outside-source',
        message:
          'This color is part of the approved palette, but it must be defined in a token or shared registry file instead of repeated here.',
      });
      continue;
    }
    pushViolation({
      file: relativePath,
      lineStarts,
      source,
      index: match.index ?? 0,
      value,
      defaultRule: 'ui/unapproved-color-literal',
      message:
        'Use an existing semantic token or register this color as part of the approved UI palette.',
    });
  }

  for (const match of source.matchAll(COLOR_FUNCTION_PATTERN)) {
    const value = match[0];
    const normalized = normalizeColorFunction(value);
    if (colorFunctionTokenFiles.has(relativePath)) continue;
    if (approvedUiColorFunctions.has(normalized)) {
      if (sourceAllowsColorFunction(relativePath, normalized)) continue;
      pushViolation({
        file: relativePath,
        lineStarts,
        source,
        index: match.index ?? 0,
        value,
        defaultRule: 'ui/approved-color-function-outside-source',
        message:
          'This approved color function belongs in a shared token or utility definition instead of being repeated inline.',
      });
      continue;
    }
    pushViolation({
      file: relativePath,
      lineStarts,
      source,
      index: match.index ?? 0,
      value,
      defaultRule: 'ui/unapproved-color-function',
      message:
        'Color functions are only allowed in token files or approved registries. Promote this value before reuse.',
    });
  }
}

function pushViolation({ file, lineStarts, source, index, value, defaultRule, message }) {
  const { line, column } = getLineColumn(lineStarts, index);
  const snippet = source.slice(lineStarts[line - 1], lineStarts[line] ?? source.length).trim();
  const contextRule = getContextRule(snippet, defaultRule);

  violations.push({
    rule: contextRule,
    file,
    line,
    column,
    value,
    message,
    snippet,
  });
}

function getContextRule(snippet, defaultRule) {
  if (/\bstyle\s*=/.test(snippet)) {
    return 'ui/unapproved-inline-style-color';
  }

  if (
    /\[[^\]]*(?:#|rgba?\(|hsla?\(|oklch\(|oklab\(|lab\(|lch\()/i.test(snippet) &&
    (/\bclass\s*=/.test(snippet) || /\bclass:/.test(snippet) || /\bclassName\b/.test(snippet))
  ) {
    return 'ui/unapproved-arbitrary-color-utility';
  }

  return defaultRule;
}

function getLineStarts(source) {
  const starts = [0];
  for (let index = 0; index < source.length; index += 1) {
    if (source[index] === '\n') {
      starts.push(index + 1);
    }
  }
  return starts;
}

function getLineColumn(lineStarts, index) {
  let low = 0;
  let high = lineStarts.length - 1;

  while (low <= high) {
    const mid = Math.floor((low + high) / 2);
    const start = lineStarts[mid];
    const next = lineStarts[mid + 1] ?? Number.POSITIVE_INFINITY;
    if (index < start) {
      high = mid - 1;
      continue;
    }
    if (index >= next) {
      low = mid + 1;
      continue;
    }
    return {
      line: mid + 1,
      column: index - start + 1,
    };
  }

  return { line: 1, column: index + 1 };
}

function toProjectPath(filePath) {
  return path.relative(projectRoot, filePath).split(path.sep).join('/');
}

function sourceAllowsHex(filePath, value) {
  return approvedUiHexColorsBySource.get(filePath)?.has(value) ?? false;
}

function sourceAllowsColorFunction(filePath, value) {
  return approvedUiColorFunctionsBySource.get(filePath)?.has(value) ?? false;
}
