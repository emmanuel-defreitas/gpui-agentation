import { readFileSync, writeFileSync } from 'node:fs';
import TurndownService from 'turndown';

const output = new URL('../dist/', import.meta.url);
const html = readFileSync(new URL('index.html', output), 'utf8');
const main = html.match(/<main\b[^>]*>([\s\S]*?)<\/main>/)?.[1];
if (!main) throw new Error('Page content missing from production output');
const base = 'https://emmanuel-defreitas.github.io/gpui-agentation/';
const markdown = new TurndownService({ headingStyle: 'atx', codeBlockStyle: 'fenced', bulletListMarker: '-' });
markdown.remove(['button', 'svg', 'aside', 'figcaption']);
markdown.addRule('decoration', {
  filter: node => ['hero-aside', 'feature-art'].some(name => node.classList?.contains(name)),
  replacement: () => '',
});
markdown.addRule('requirements', {
  filter: node => node.classList?.contains('requirements'),
  replacement: (_, node) => `\n\n${Array.from(node.children).map(child => child.textContent).join(' · ')}\n\n`,
});
markdown.addRule('definition', { filter: 'dt', replacement: content => `\n\n**${content}**\n\n` });
markdown.addRule('code', {
  filter: 'pre',
  replacement: (_, node) => `\n\n\`\`\`${node.getAttribute('data-language') || ''}\n${node.textContent.trim()}\n\`\`\`\n\n`,
});
markdown.addRule('links', {
  filter: 'a',
  replacement: (content, node) => `[${content}](${new URL(node.getAttribute('href'), base).href}) `,
});
markdown.addRule('images', {
  filter: 'img',
  replacement: (_, node) => `![${node.getAttribute('alt') || ''}](${new URL(node.getAttribute('src'), base).href})`,
});
markdown.addRule('lineBreak', { filter: 'br', replacement: () => ' ' });
writeFileSync(new URL('index.md', output), `# gpui-agentation\n\nSource: ${base}\n\n${markdown.turndown(main).replace(/^# .+\n+/, '')}\n`);
console.log('Exported page content to index.md');
