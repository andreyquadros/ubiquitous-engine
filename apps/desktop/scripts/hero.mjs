#!/usr/bin/env node
// Renders UBI as a large transparent PNG for the README hero.
//
//   node scripts/hero.mjs [--out ../../docs/ubi-hero.png] [--size 640]
//
// Starts an in-process Vite dev server in mock mode, mounts Hoje and isolates the mascot from the hero:
// the user's PNG (public/ubi/ubi.png, background already knocked out by the app) when installed,
// otherwise the inline SVG. Scales it up and screenshots it with a transparent background.

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
    colorScheme: 'dark',
    reducedMotion: 'reduce',
    locale: 'pt-BR',
  });
  const page = await context.newPage();
  page.on('pageerror', (err) => console.error('page error:', err.message));
  // The hero is the flat art: hide the GLB so 'auto' resolves to the PNG (or the SVG) instead of the 3D canvas.
  await page.route('**/ubi/Ubi.glb', (route) => route.fulfill({ status: 404, body: '' }));
  await page.goto('http://localhost:1423/?theme=dark#/', { waitUntil: 'networkidle' });
  const HERO = '[data-testid="ubi-hero"]';
  await page.waitForSelector(`${HERO} [data-testid="ubi-png"] img, ${HERO} [data-testid="ubi-svg"]`, { timeout: 20000 });
  // give the PNG probe + cutout a moment to replace the SVG when the picture is installed
  await page.waitForTimeout(1200);
  const kind = await page.evaluate(
    ([hero, px]) => {
      const img = document.querySelector(`${hero} [data-testid="ubi-png"] img`);
      const svg = document.querySelector(`${hero} [data-testid="ubi-svg"]`);
      const el = img ?? svg;
      if (!el) return null;
      let w = px;
      let h = px;
      if (img) {
        const ratio = img.naturalWidth && img.naturalHeight ? img.naturalWidth / img.naturalHeight : 200 / 240;
        if (ratio >= 1) h = Math.round(px / ratio);
        else w = Math.round(px * ratio);
      } else {
        w = Math.round((px * 200) / 240); // keep the 200x240 viewBox aspect ratio
      }
      el.setAttribute('width', String(w));
      el.setAttribute('height', String(h));
      el.style.width = `${w}px`;
      el.style.height = `${h}px`;
      el.style.maxWidth = 'none';
      el.style.maxHeight = 'none';
      el.style.filter = 'none';
      el.setAttribute('data-hero', '1');
      document.body.replaceChildren(el);
      for (const node of [document.documentElement, document.body]) {
        node.style.background = 'transparent';
        node.style.margin = '0';
        node.style.padding = '20px';
      }
      return img ? 'png' : 'svg';
    },
    [HERO, size],
  );
  if (!kind) throw new Error('UBI not found in the hero');
  await page.waitForTimeout(300);
  await page.setViewportSize({ width: size + 40, height: size + 40 });
  await page.locator('body > [data-hero="1"]').screenshot({ path: out, omitBackground: true });
  console.log(`✓ ${out} (${kind})`);
} finally {
  await browser.close();
  await server.close();
}
