#!/usr/bin/env node
// Refreshes the gallery in public/shots from the desktop app's mock mode (no Tauri needed).
//
//   pnpm screenshots [--port 1420]         # starts ../apps/desktop `pnpm dev` on that port, captures, stops it
//   pnpm screenshots --url http://localhost:1420   # uses a server that is already running
//
// Six pages in the dark theme (pt-BR) plus the dashboard in the light theme, 1440x900, lossless PNG through sharp
// (roughly 50-200 kB each; --palette quantises). Requires `pnpm install` in apps/desktop.
import { spawn } from 'node:child_process';
import { mkdirSync, statSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import sharp from 'sharp';
import { argParser, launch, waitFor } from './pw.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const arg = argParser(process.argv.slice(2));
const out = resolve(here, arg('--out', '../public/shots'));
const lang = arg('--lang', 'pt-BR');
const port = arg('--port', '1420');
// PNGs are lossless by default (roughly 50-200 kB each); --palette quantises them (--quality, default 85) when size matters more
const palette = process.argv.includes('--palette');
const quality = Number(arg('--quality', '85'));
let base = arg('--url', '');
let server = null;

if (!base) {
  base = `http://localhost:${port}`;
  server = spawn('pnpm', ['dev', '--port', port], { cwd: resolve(here, '../../apps/desktop'), stdio: ['ignore', 'ignore', 'inherit'], detached: true });
  process.on('exit', () => server && process.kill(-server.pid, 'SIGTERM'));
}

const onlyPage = arg('--only', '');
const PAGES = [
  ['dashboard', '/', 'page-dashboard', 'dark'],
  ['timeline', '/timeline', 'page-timeline', 'dark'],
  ['review', '/review', 'page-review', 'dark'],
  ['reports', '/reports', 'page-reports', 'dark'],
  ['categories', '/categories', 'page-categories', 'dark'],
  ['insights', '/insights', 'page-insights', 'dark'],
  ['dashboard-light', '/', 'page-dashboard', 'light'],
];

mkdirSync(out, { recursive: true });
await waitFor(base);
const browser = await launch();
try {
  for (const [name, route, testId, theme] of PAGES) {
    if (onlyPage && name !== onlyPage) continue;
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1, colorScheme: theme, reducedMotion: 'reduce', locale: lang === 'en' ? 'en-US' : 'pt-BR' });
    // The unlicensed reminder is a product nag, not something the gallery should advertise: mark it dismissed
    // (the app keeps the unix ms of the last dismissal in localStorage) before the app boots.
    await context.addInitScript(() => {
      try {
        localStorage.setItem('ubiqx.license_nag', String(Date.now()));
      } catch {
        /* storage unavailable: the banner simply stays */
      }
    });
    const page = await context.newPage();
    page.on('pageerror', (err) => console.error(`[${name}] page error:`, err.message));
    await page.goto(`${base}/?theme=${theme}&lang=${lang}#${route}`, { waitUntil: 'networkidle' });
    await page.waitForSelector(`[data-testid="${testId}"]`, { timeout: 20000 });
    await page.waitForSelector('[data-testid="ubi-3d"][data-ready="1"], [data-testid="ubi-png"], [data-testid="ubi-svg"]', { timeout: 20000 }).catch(() => {});
    if (name.startsWith('dashboard') || name === 'insights') await page.waitForSelector('.recharts-surface', { timeout: 15000 }).catch(() => {});
    if (name === 'insights') {
      await page.getByRole('button', { name: /^(Gerar|Generate)$/ }).click().catch(() => {});
      await page.waitForTimeout(1800);
    }
    if (name === 'review') await page.locator('[data-testid="review-row"]').first().click().catch(() => {});
    // reduced motion skips the load animation, so the dial and bars are already at their final state
    await page.waitForTimeout(900);
    const file = resolve(out, `${name}.png`);
    const buf = await page.screenshot({ fullPage: false });
    await sharp(buf).png(palette ? { palette: true, quality, effort: 9, compressionLevel: 9 } : { compressionLevel: 9 }).toFile(file);
    console.log(`${file} (${Math.round(statSync(file).size / 1024)} kB)`);
    await context.close();
  }
} finally {
  await browser.close();
  if (server) {
    process.kill(-server.pid, 'SIGTERM');
    server = null;
  }
}
