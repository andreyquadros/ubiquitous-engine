#!/usr/bin/env node
// Generates the static images the site ships with, from the site itself:
//   public/favicon-32.png, favicon-128.png, apple-touch-icon.png  (from the app icon)
//   public/ubi-hero.png   still render of the 3D mascot at 2x, transparent (the hero fallback and the OG art)
//   public/og.png         1200x630 Open Graph card rendered from the /og view
//
//   pnpm images [--only icons|hero|og]
import { createServer } from 'vite';
import { mkdirSync, statSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import sharp from 'sharp';
import { argParser, launch } from './pw.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const arg = argParser(process.argv.slice(2));
const only = arg('--only', '');
const pub = resolve(here, '../public');
mkdirSync(pub, { recursive: true });
const kb = (f) => `${Math.round(statSync(f).size / 1024)} kB`;

if (!only || only === 'icons') {
  const icon = resolve(here, '../../apps/desktop/src-tauri/icons/app-icon.png');
  for (const [name, size] of [['favicon-32.png', 32], ['favicon-128.png', 128], ['apple-touch-icon.png', 180]]) {
    const f = resolve(pub, name);
    await sharp(icon).resize(size, size).png({ compressionLevel: 9 }).toFile(f);
    console.log(`${f} (${kb(f)})`);
  }
}

if (!only || only === 'hero' || only === 'og') {
  const server = await createServer({ root: resolve(here, '..'), server: { port: 5174, strictPort: true }, logLevel: 'error' });
  await server.listen();
  const browser = await launch();
  try {
    if (!only || only === 'hero') {
      const context = await browser.newContext({ viewport: { width: 520, height: 600 }, deviceScaleFactor: 2, colorScheme: 'dark', locale: 'pt-BR' });
      const page = await context.newPage();
      page.on('pageerror', (err) => console.error('page error:', err.message));
      await page.goto('http://localhost:5174/?capture=hero', { waitUntil: 'networkidle' });
      await page.waitForSelector('[data-testid="ubi-hero"][data-ready="1"]', { timeout: 60000 });
      await page.waitForTimeout(600);
      const f = resolve(pub, 'ubi-hero.png');
      const buf = await page.locator('[data-testid="ubi-hero"]').screenshot({ omitBackground: true });
      await sharp(buf).png({ compressionLevel: 9, palette: true, quality: 90, effort: 9 }).toFile(f);
      console.log(`${f} (${kb(f)})`);
      await context.close();
    }
    if (!only || only === 'og') {
      const context = await browser.newContext({ viewport: { width: 1200, height: 630 }, deviceScaleFactor: 1, colorScheme: 'dark', locale: 'pt-BR' });
      const page = await context.newPage();
      page.on('pageerror', (err) => console.error('page error:', err.message));
      await page.goto('http://localhost:5174/og?lang=pt-BR', { waitUntil: 'networkidle' });
      await page.waitForSelector('[data-testid="og-card"]', { timeout: 30000 });
      await page.evaluate(() => document.fonts.ready);
      await page.waitForTimeout(400);
      const f = resolve(pub, 'og.png');
      const buf = await page.locator('[data-testid="og-card"]').screenshot();
      await sharp(buf).png({ compressionLevel: 9, palette: true, quality: 90, effort: 9 }).toFile(f);
      console.log(`${f} (${kb(f)})`);
      await context.close();
    }
  } finally {
    await browser.close();
    await server.close();
  }
}
