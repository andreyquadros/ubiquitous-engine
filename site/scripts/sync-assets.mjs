#!/usr/bin/env node
// Copies the mascot model from the desktop app into site/public so the 3 MB file is not duplicated
// in git (site/.gitignore excludes it). Runs before `dev` and `build`. The Draco decoder comes from the
// three package itself (bundled by Vite), so it needs no copy.
import { copyFileSync, existsSync, mkdirSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const src = resolve(here, '../../apps/desktop/public');
const dst = resolve(here, '../public');

const jobs = [['ubi/Ubi.glb', 'ubi/Ubi.glb']];

let copied = 0;
for (const [from, to] of jobs) {
  const a = join(src, from);
  const b = join(dst, to);
  if (!existsSync(a)) {
    console.warn(`sync-assets: missing ${a} (the 3D hero will use the PNG fallback)`);
    continue;
  }
  mkdirSync(dirname(b), { recursive: true });
  if (existsSync(b) && statSync(b).size === statSync(a).size) continue;
  copyFileSync(a, b);
  copied += 1;
}
console.log(`sync-assets: ${copied} file(s) copied, ${jobs.length - copied} up to date (${readdirSync(dst).length} entries in public/)`);
