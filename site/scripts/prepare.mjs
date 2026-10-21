import { mkdir, copyFile } from 'node:fs/promises';
const root = new URL('../', import.meta.url);
for (const file of ['Cargo.toml', 'src/main.rs']) {
  const destination = new URL(`public/example/${file}`, root);
  await mkdir(new URL('.', destination), { recursive: true });
  await copyFile(new URL(`example/${file}`, root), destination);
}
