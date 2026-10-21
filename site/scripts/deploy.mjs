import { execFileSync } from 'node:child_process';
import { cpSync, existsSync, mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const site = fileURLToPath(new URL('../', import.meta.url));
const root = fileURLToPath(new URL('../../', import.meta.url));
const git = (args, cwd = root) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
execFileSync('npm', ['run', 'check'], {
  cwd: site, stdio: 'inherit', env: { ...process.env, ASTRO_TELEMETRY_DISABLED: '1' },
});
const remote = git(['remote', 'get-url', 'origin']);
const revision = git(['rev-parse', '--short', 'HEAD']);
const destination = mkdtempSync(join(tmpdir(), 'gpui-pages-'));
try {
  git(['init', '--initial-branch=gh-pages'], destination);
  git(['remote', 'add', 'origin', remote], destination);
  git(['config', 'user.name', git(['config', 'user.name'])], destination);
  git(['config', 'user.email', git(['config', 'user.email'])], destination);
  // Preserve deployment history and never force-push over a concurrent update.
  if (git(['ls-remote', '--heads', 'origin', 'gh-pages'], destination)) {
    git(['fetch', '--depth=1', 'origin', 'gh-pages'], destination);
    git(['reset', '--hard', 'FETCH_HEAD'], destination);
  }
  for (const file of readdirSync(destination)) {
    if (file !== '.git') rmSync(join(destination, file), { recursive: true });
  }
  cpSync(join(site, 'dist'), destination, { recursive: true });
  if (!existsSync(join(destination, '.nojekyll'))) throw new Error('Missing .nojekyll');
  git(['add', '--all'], destination);
  if (!git(['diff', '--cached', '--name-only'], destination)) {
    console.log('Published files are unchanged.');
  } else {
    git(['commit', '-m', `docs: deploy website from ${revision}`], destination);
    execFileSync('git', ['push', 'origin', 'HEAD:refs/heads/gh-pages'], { cwd: destination, stdio: 'inherit' });
  }
} finally {
  rmSync(destination, { recursive: true, force: true });
}
