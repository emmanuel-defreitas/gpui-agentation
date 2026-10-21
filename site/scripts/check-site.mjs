import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const dist = fileURLToPath(new URL('../dist/', import.meta.url));
const html = readFileSync(resolve(dist, 'index.html'), 'utf8');
const base = '/gpui-agentation/';
const ids = [...html.matchAll(/\bid="([^"]+)"/g)].map(match => match[1]);
assert.equal(new Set(ids).size, ids.length, 'Duplicate element IDs');
const attributes = [...html.matchAll(/\b(?:href|src|data-gif|data-poster)="([^"]+)"/g)]
  .map(match => match[1]);
for (const value of attributes) {
  if (value.startsWith('#')) {
    if (value.length > 1) assert(ids.includes(value.slice(1)), `Missing anchor: ${value}`);
  } else if (value.startsWith('/')) {
    assert(value.startsWith(base), `Asset bypasses project base: ${value}`);
    const relative = decodeURIComponent(value.slice(base.length).split(/[?#]/)[0]);
    const path = resolve(dist, relative);
    assert(path.startsWith(dist.endsWith(sep) ? dist : dist + sep), `Asset escapes output: ${value}`);
    assert(existsSync(path), `Missing asset: ${value}`);
  }
}
const decode = value => value.replace(/<[^>]*>/g, '')
  .replace(/&#(?:x([0-9a-f]+)|(\d+));/gi, (_, hex, decimal) => String.fromCodePoint(parseInt(hex || decimal, hex ? 16 : 10)))
  .replace(/&quot;/g, '"').replace(/&apos;/g, "'")
  .replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&');
const snippets = [...html.matchAll(/<pre\b[^>]*data-language="([^"]+)"[^>]*>([\s\S]*?)<\/pre>/g)];
assert.equal(snippets.length, 5, 'Expected five highlighted, copyable setup snippets');
for (const [, language, markup] of snippets) {
  assert(markup.includes('<span'), `Missing syntax highlighting: ${language}`);
  if (language === 'json') {
    const config = JSON.parse(decode(markup));
    assert.equal(config.mcpServers.gpui_agentation.command, 'gpui-agentation');
    assert.deepEqual(config.mcpServers.gpui_agentation.args, ['--url', 'http://127.0.0.1:4748/mcp']);
  }
}
assert(existsSync(resolve(dist, '.nojekyll')), 'Pages must disable Jekyll');
assert(!existsSync(resolve(dist, 'example')), 'Removed example page must not ship');
assert(!html.includes('http://localhost:'), 'Local preview URL leaked into output');
console.log(`Site checks passed: ${ids.length} unique IDs, ${attributes.length} links/assets, ${snippets.length} highlighted snippets.`);
