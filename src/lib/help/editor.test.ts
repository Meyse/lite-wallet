// Node DOM checks only. The local editor file is intentionally not opened through
// a browser tool: its URL was denied in the originating task.
import { readFileSync } from 'node:fs';
// @ts-expect-error The existing jsdom test dependency does not ship declarations.
import { JSDOM, VirtualConsole } from 'jsdom';
import { afterEach, describe, expect, it } from 'vitest';
import destinations from './destinations.json';
import { helpEn } from '$lib/i18n/locales/help/en';
import { parseHelpText } from './links';

const html = readFileSync('docs/references/help-copy/editor.html', 'utf8');
const data = JSON.parse(
  required(
    html.match(/<script id="help-copy-data" type="application\/json">([\s\S]*?)<\/script>/)
  )[1]
);
const key = 'wallet-help-copy:' + data.sourceId;
const doms: JSDOM[] = [];
function editor(stored?: string) {
  let downloaded: Blob | undefined;
  const errors: unknown[] = [];
  const virtualConsole = new VirtualConsole();
  virtualConsole.on('jsdomError', (error: unknown) => errors.push(error));
  const dom = new JSDOM(html, {
    url: 'https://help-editor.invalid',
    runScripts: 'dangerously',
    virtualConsole,
    beforeParse(window: Window & typeof globalThis) {
      window.structuredClone = structuredClone;
      window.Blob = Blob;
      window.URL.createObjectURL = (blob: Blob | MediaSource) => {
        downloaded = blob as Blob;
        return 'blob:test';
      };
      window.URL.revokeObjectURL = () => {};
      window.HTMLAnchorElement.prototype.click = () => {};
      window.confirm = () => true;
      if (stored !== undefined) window.localStorage.setItem(key, stored);
    },
  });
  doms.push(dom);
  const doc = dom.window.document;
  const el = <T extends HTMLElement = HTMLButtonElement>(id: string) => doc.getElementById(id) as T;
  const article = (id: string) =>
    required(doc.querySelector<HTMLButtonElement>(`[data-article="${id}"]`)).click();
  const select = (start: number, end: number) => {
    const text = el<HTMLTextAreaElement>('article-text');
    text.setSelectionRange(start, end);
    text.dispatchEvent(new dom.window.Event('select'));
  };
  const input = (value: string) => {
    el<HTMLTextAreaElement>('article-text').value = value;
    el('article-text').dispatchEvent(new dom.window.Event('input'));
  };
  const download = async () => {
    el('download-edits').click();
    expect(downloaded).toBeDefined();
    return JSON.parse(await required(downloaded).text());
  };
  const restore = async (value: unknown) => {
    const field = el<HTMLInputElement>('import-file');
    Object.defineProperty(field, 'files', {
      configurable: true,
      value: [{ size: 100, text: async () => JSON.stringify(value) }],
    });
    field.dispatchEvent(new dom.window.Event('change'));
    await new Promise((resolve) => setTimeout(resolve, 0));
  };
  expect(errors).toEqual([]);
  return { dom, doc, el, article, select, input, download, restore, errors };
}
afterEach(() => {
  for (const dom of doms.splice(0)) dom.window.close();
});

describe('English Help editor screen authoring', () => {
  it('embeds the shared destination allowlist and correct localized source ranges without changing its text baseline', () => {
    expect(data.destinations).toEqual(destinations);
    for (const [key, ranges] of Object.entries(data.baselineLinks)) {
      let position = 0;
      const expected = parseHelpText(helpEn[key as keyof typeof helpEn]).flatMap((part) => {
        const start = position;
        position += part.text.length;
        return part.destination ? [{ start, end: position, destination: part.destination }] : [];
      });
      expect(ranges).toEqual(expected);
    }
    expect(data.sourceId).toBe('9e189cb7b7a3f2597775');
  });

  it.each([1, 2])(
    'removes the article without losing version %s drafts or exports',
    async (version) => {
      const pending = {
        'helpCenter.article.send.title': 'My unfinished send edit',
        'helpCenter.article.balances.body': 'My earlier removed-article edit',
      };
      const draft = {
        version,
        sourceId: data.sourceId,
        copyRevision: data.copyRevision,
        changes: pending,
        linkChanges: {},
        selected: 'balances',
        mode: 'article',
      };
      const stored = JSON.stringify(draft);
      const e = editor(stored);
      expect(e.doc.querySelector('[data-article="balances"]')).toBeNull();
      expect(e.doc.querySelectorAll('[data-article]')).toHaveLength(29);
      expect(e.el<HTMLInputElement>('article-title').value).toBe(
        data.baseline['helpCenter.article.wallet.title']
      );
      expect(e.dom.window.localStorage.getItem(key)).toBe(stored);
      e.article('send');
      expect(e.el<HTMLInputElement>('article-title').value).toBe(
        pending['helpCenter.article.send.title']
      );
      const output = await e.download();
      expect(output.removedArticleIds).toEqual(['balances']);
      for (const [key, after] of Object.entries(pending))
        expect(output.changes).toContainEqual({ key, before: data.baseline[key], after });
      const restored = editor();
      await restored.restore(output);
      restored.article('send');
      expect(restored.el<HTMLInputElement>('article-title').value).toBe(
        pending['helpCenter.article.send.title']
      );
      expect(restored.doc.querySelector('[data-article="balances"]')).toBeNull();
      restored.article('networks');
      restored.el('next').click();
      expect(restored.el<HTMLInputElement>('article-title').value).toBe(
        data.baseline['helpCenter.article.receive.title']
      );
    }
  );

  it('adds a link, preserves its target when renamed, exports it, and restores/removes it', async () => {
    const e = editor();
    e.article('wallet');
    const text = e.el<HTMLTextAreaElement>('article-text').value;
    e.select(0, 11);
    e.el<HTMLSelectElement>('link-destination').value = 'manage-assets';
    e.el('apply-link').click();
    e.input('This wallet' + text.slice(11));
    expect(e.el('article-links').textContent).toContain('This wallet → Manage assets');
    const output = await e.download();
    expect(output.version).toBe(2);
    expect(
      output.linkChanges.find((edit: { key: string }) => edit.key.endsWith('wallet.summary')).after
    ).toEqual([{ start: 0, end: 11, destination: 'manage-assets' }]);
    const restored = editor();
    await restored.restore(output);
    expect(restored.el('article-links').textContent).toContain('This wallet → Manage assets');
    restored.select(0, 11);
    restored.el('remove-link').click();
    expect(restored.el('article-links').textContent).toBe('');
  });

  it('inspects an existing link and changes its screen without changing the text', async () => {
    const e = editor();
    e.article('assets');
    required(e.el('article-links').querySelector<HTMLButtonElement>('button')).click();
    expect(e.el('remove-link').hidden).toBe(false);
    e.el<HTMLSelectElement>('link-destination').value = 'receive';
    e.el('apply-link').click();
    const out = await e.download();
    expect(
      out.linkChanges.find((edit: { key: string }) => edit.key.endsWith('assets.summary')).after[0]
        .destination
    ).toBe('receive');
    expect(out.changes.some((edit: { key: string }) => edit.key.endsWith('assets.summary'))).toBe(
      false
    );
  });

  it('moves ranges with preceding text edits and removes a deleted link', async () => {
    const e = editor();
    e.article('assets');
    const text = e.el<HTMLTextAreaElement>('article-text').value;
    e.input('Here: ' + text);
    expect(e.el('article-links').textContent).toContain('Manage assets → Manage assets');
    e.input('Here: ' + text.slice('Manage assets'.length));
    expect(e.el('article-links').textContent).toBe('');
    expect((await e.download()).linkChanges[0].after).toEqual([]);
  });

  it('keeps older text-only drafts and imports, including one-time copy corrections', async () => {
    const changes = {
      'helpCenter.article.wallet.body':
        data.baseline['helpCenter.article.wallet.body'] + '\n\nMy pending edit.',
    };
    const e = editor(
      JSON.stringify({
        version: 1,
        sourceId: data.sourceId,
        changes,
        selected: 'wallet',
        mode: 'article',
      })
    );
    expect(e.el<HTMLTextAreaElement>('article-text').value).toContain('My pending edit.');
    expect(e.el<HTMLTextAreaElement>('article-text').value).toContain('Secret Recovery Phrase');
    const output = {
      format: 'wallet-help-edits',
      version: 1,
      locale: 'en',
      sourceId: data.sourceId,
      changes: [
        {
          key: 'helpCenter.article.wallet.title',
          before: data.baseline['helpCenter.article.wallet.title'],
          after: 'My older title',
        },
      ],
    };
    await e.restore(output);
    expect(e.el<HTMLInputElement>('article-title').value).toBe('My older title');
    expect(JSON.parse(required(e.dom.window.localStorage.getItem(key))).version).toBe(2);
  });

  it('restores locally saved link drafts and keeps malicious-looking text inert', async () => {
    const e = editor();
    e.article('wallet');
    e.input('<img src=x onerror=alert(1)>\n\nText');
    e.select(0, 26);
    e.el<HTMLSelectElement>('link-destination').value = 'send';
    e.el('apply-link').click();
    e.dom.window.dispatchEvent(new e.dom.window.Event('pagehide'));
    const restored = editor(required(e.dom.window.localStorage.getItem(key)));
    expect(restored.el<HTMLTextAreaElement>('article-text').value).toContain('<img');
    expect(restored.doc.querySelector('img')).toBeNull();
    expect(restored.el('article-links').textContent).toContain('→ Send');
  });

  it('rejects unknown targets, overlapping ranges, conflicting baselines and unknown versions without replacing the draft', async () => {
    const e = editor();
    const valid = await e.download();
    const field = 'helpCenter.article.assets.summary';
    for (const bad of [
      { ...valid, version: 99 },
      { ...valid, sourceId: 'wrong' },
      { ...valid, linkChanges: [{ key: field, before: [], after: [] }] },
      {
        ...valid,
        linkChanges: [
          {
            key: field,
            before: data.baselineLinks[field],
            after: [{ start: 0, end: 6, destination: 'recovery-keys' }],
          },
        ],
      },
      {
        ...valid,
        linkChanges: [
          {
            key: field,
            before: data.baselineLinks[field],
            after: [
              { start: 0, end: 6, destination: 'send' },
              { start: 5, end: 8, destination: 'receive' },
            ],
          },
        ],
      },
    ]) {
      const before = e.el<HTMLTextAreaElement>('article-text').value;
      await e.restore(bad);
      expect(e.el('notice').hidden).toBe(false);
      expect(e.el<HTMLTextAreaElement>('article-text').value).toBe(before);
    }
  });

  it('preserves a corrupt browser draft and prevents links across paragraph boundaries', () => {
    const e = editor('{broken');
    e.select(0, e.el<HTMLTextAreaElement>('article-text').value.length);
    expect(e.el<HTMLButtonElement>('apply-link').disabled).toBe(true);
    e.input('A new edit');
    e.dom.window.dispatchEvent(new e.dom.window.Event('pagehide'));
    expect(e.dom.window.localStorage.getItem(key)).toBe('{broken');
  });
});

function required<T>(value: T | null | undefined): T {
  if (value === null || value === undefined) throw new Error('Expected test element or value');
  return value;
}
