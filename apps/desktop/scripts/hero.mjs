#!/usr/bin/env node
// Renders UBI (SVG fallback) as a large transparent PNG for the README hero.
//
//   node scripts/hero.mjs [--out ../../docs/ubi-hero.png] [--size 640]
//
// Starts an in-process Vite dev server in mock mode, mounts the dashboard, isolates the
// UBI <svg>, scales it up and screenshots it with a transparent background.

import { createServer } from 'vite';
import { chromium } from 'playwright';
import { mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const arg = (name, def) => {
  const i = args.indexOf(name);
  return i >= 0 && args[i + 1] ? args[i + 1] : def;
};
const out = resolve(here, arg('--out', '../../../docs/ubi-hero.png'));
const size = Number(arg('--size', '640'));

async function launch() {
  try {
    return await chromium.launch();
  } catch (e) {
    console.warn('default chromium launch failed, retrying with executablePath:', e.message.split('\n')[0]);
    return chromium.launch({ executablePath: process.env.CHROMIUM_PATH ?? '/opt/pw-browsers/chromium' });
  }
}

mkdirSync(dirname(out), { recursive: true });
const server = await createServer({ root: resolve(here, '..'), server: { port: 1423, strictPort: true }, logLevel: 'error' });
await server.listen();
const browser = await launch();
try {
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 1,
    colorScheme: 'light',
    reducedMotion: 'reduce',
    locale: 'pt-BR',
  });
  const page = await context.newPage();
  page.on('pageerror', (err) => console.error('page error:', err.message));
  await page.goto('http://localhost:1423/?theme=light#/', { waitUntil: 'networkidle' });
  const SEL = '[data-testid="ubi-card"] [data-testid="ubi-svg"]';
  await page.waitForSelector(SEL, { timeout: 15000 });
  await page.evaluate(([sel, px]) => {
    const svg = document.querySelector(sel);
    // keep the 200x240 viewBox aspect ratio
    const w = Math.round((px * 200) / 240);
    svg.setAttribute('width', String(w));
    svg.setAttribute('height', String(px));
    svg.style.width = `${w}px`;
    svg.style.height = `${px}px`;
    document.body.replaceChildren(svg);
    for (const el of [document.documentElement, document.body]) {
      el.style.background = 'transparent';
      el.style.margin = '0';
      el.style.padding = '20px';
    }
  }, [SEL, size]);
  await page.waitForTimeout(300);
  await page.setViewportSize({ width: size + 40, height: size + 40 });
  await page.locator('body > svg[data-testid="ubi-svg"]').screenshot({ path: out, omitBackground: true });
  console.log(`✓ ${out}`);
} finally {
  await browser.close();
  await server.close();
}
